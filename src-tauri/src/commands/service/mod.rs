/// 服务管理命令
///
/// 检测策略（跨平台统一）：
///   1. TCP 连 127.0.0.1:{port}，超时 1.5s
///   2. 连通 → 认为 Gateway 在运行
///
/// 不依赖任何系统命令（无 netstat / PowerShell / launchctl / openclaw health），
/// 无权限问题，逻辑一致。
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::models::types::ServiceStatus;
use serde::Serialize;
use tauri::Emitter;

use super::service_gateway_owner::{
    clear_gateway_owner, ensure_owned_gateway_or_err, gateway_owner_pid_needs_refresh, is_current_gateway_owner,
    read_gateway_owner, should_auto_claim_gateway, write_gateway_owner,
};
use super::service_platform as platform;

/// OpenClaw 官方服务的友好名称映射
fn description_map() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("ai.openclaw.gateway", "OpenClaw Gateway"),
        ("ai.openclaw.node", "OpenClaw Node Host"),
    ])
}

const GUARDIAN_INTERVAL: Duration = Duration::from_secs(15);
const GUARDIAN_RESTART_COOLDOWN: Duration = Duration::from_secs(60);
const GUARDIAN_STABLE_WINDOW: Duration = Duration::from_secs(120);
const GUARDIAN_MAX_AUTO_RESTART: u32 = 3;
const GATEWAY_CONFIG_AUTO_FIX_COOLDOWN: Duration = Duration::from_secs(120);

#[derive(Debug, Default)]
struct GuardianRuntimeState {
    last_seen_running: Option<bool>,
    running_since: Option<Instant>,
    auto_restart_count: u32,
    last_restart_time: Option<Instant>,
    manual_hold: bool,
    pause_reason: Option<String>,
    give_up: bool,
}

#[derive(Debug, Default)]
struct GatewayConfigAutoFixState {
    last_attempt: Option<Instant>,
    in_progress: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardianStatus {
    pub backend_managed: bool,
    pub paused: bool,
    pub manual_hold: bool,
    pub give_up: bool,
    pub auto_restart_count: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuardianEventPayload {
    kind: String,
    auto_restart_count: u32,
    message: String,
}

pub(crate) async fn current_gateway_runtime(label: &str) -> (bool, Option<u32>) {
    #[cfg(target_os = "windows")]
    {
        platform::check_service_status(0, label)
    }
    #[cfg(target_os = "macos")]
    {
        platform::check_service_status(0, label)
    }
    #[cfg(target_os = "linux")]
    {
        platform::check_service_status(0, label).await
    }
}

async fn wait_for_gateway_running(label: &str, timeout: Duration) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let (running, pid) = current_gateway_runtime(label).await;
        if running && gateway_health_ready(Duration::from_secs(2)).await {
            write_gateway_owner(pid)?;
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    Err(format!(
        "Gateway 启动超时，请查看 {}",
        crate::commands::openclaw_dir().join("logs").join("gateway.err.log").display()
    ))
}

async fn gateway_health_ready(timeout: Duration) -> bool {
    let port = crate::commands::gateway_listen_port();
    let Ok(client) = reqwest::Client::builder().timeout(timeout).build() else {
        return false;
    };
    if let Ok(resp) = client.get(format!("http://127.0.0.1:{port}/readyz")).send().await {
        if resp.status().is_success() {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                return body.get("ready").and_then(|v| v.as_bool()).unwrap_or(false);
            }
            return true;
        }
    }
    match client.get(format!("http://127.0.0.1:{port}/health")).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

async fn wait_for_gateway_health(timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if gateway_health_ready(Duration::from_secs(2)).await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    false
}

async fn wait_for_gateway_stopped(label: &str, timeout: Duration) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let (running, _) = current_gateway_runtime(label).await;
        if !running {
            clear_gateway_owner();
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    Err("Gateway 停止超时，请手动检查进程".into())
}

fn gateway_err_log_path() -> std::path::PathBuf {
    crate::commands::openclaw_dir().join("logs").join("gateway.err.log")
}

pub(super) fn read_gateway_error_log_excerpt(max_bytes: usize) -> String {
    let bytes = match std::fs::read(gateway_err_log_path()) {
        Ok(content) => content,
        Err(_) => return String::new(),
    };
    if bytes.is_empty() {
        return String::new();
    }
    let tail = if bytes.len() > max_bytes {
        &bytes[bytes.len() - max_bytes..]
    } else {
        &bytes[..]
    };
    String::from_utf8_lossy(tail).to_string()
}

fn looks_like_gateway_config_mismatch(reason: &str) -> bool {
    let combined = format!("{}\n{}", reason, read_gateway_error_log_excerpt(8192)).to_lowercase();
    let has_invalid = combined.contains("config invalid") || combined.contains("invalid config");
    let has_newer_version = combined.contains("config was last written by a newer openclaw");
    let has_schema_mismatch = combined.contains("must not have additional properties")
        || combined.contains("must not have additional property")
        || combined.contains("plugins.entries.memory-core.config")
        || combined.contains("additional properties");
    let mentions_doctor_fix = combined.contains("doctor --fix");
    (has_invalid && (has_schema_mismatch || mentions_doctor_fix)) || (has_newer_version && mentions_doctor_fix)
}

/// 直接修复 openclaw.json 中 plugins.entries.*.config 的多余属性
/// 当 `openclaw doctor --fix` 无法修复时作为二级回退
fn try_direct_config_strip() -> Result<bool, String> {
    let config_path = crate::commands::openclaw_dir().join("openclaw.json");
    let raw = std::fs::read_to_string(&config_path).map_err(|e| format!("读取配置文件失败: {e}"))?;
    let mut doc: serde_json::Value = serde_json::from_str(&raw).map_err(|e| format!("解析配置文件失败: {e}"))?;

    // 从错误日志中提取哪些 plugin entry 有 additional properties
    let err_log = read_gateway_error_log_excerpt(8192).to_lowercase();
    let mut changed = false;

    // 匹配形如 "plugins.entries.XXX.config: invalid config" 的模式
    if let Some(entries) = doc.pointer_mut("/plugins/entries").and_then(|v| v.as_object_mut()) {
        let entry_names: Vec<String> = entries.keys().cloned().collect();
        for name in &entry_names {
            let pattern = format!("plugins.entries.{}.config", name).to_lowercase();
            if err_log.contains(&pattern) {
                if let Some(entry) = entries.get_mut(name) {
                    if let Some(obj) = entry.as_object_mut() {
                        if obj.contains_key("config") {
                            guardian_log(&format!("直接修复: 清空 plugins.entries.{name}.config（含多余属性）"));
                            obj.remove("config");
                            changed = true;
                        }
                    }
                }
            }
        }
    }

    // 通用回退：如果错误日志提到 additional properties 但没匹配到具体 entry，
    // 清空所有 plugin entry 的 config
    if !changed && (err_log.contains("additional properties") || err_log.contains("additional property")) {
        if let Some(entries) = doc.pointer_mut("/plugins/entries").and_then(|v| v.as_object_mut()) {
            let entry_names: Vec<String> = entries.keys().cloned().collect();
            for name in &entry_names {
                if let Some(entry) = entries.get_mut(name) {
                    if let Some(obj) = entry.as_object_mut() {
                        if let Some(config) = obj.get("config") {
                            if config.is_object() && config.as_object().map(|m| !m.is_empty()).unwrap_or(false) {
                                guardian_log(&format!("直接修复(通用回退): 清空 plugins.entries.{name}.config"));
                                obj.remove("config");
                                changed = true;
                            }
                        }
                    }
                }
            }
        }
    }

    if changed {
        let formatted = serde_json::to_string_pretty(&doc).map_err(|e| format!("序列化配置失败: {e}"))?;
        std::fs::write(&config_path, formatted).map_err(|e| format!("写入配置文件失败: {e}"))?;
        guardian_log("直接修复: 已写回 openclaw.json");
    }

    Ok(changed)
}

static GUARDIAN_STATE: OnceLock<Arc<Mutex<GuardianRuntimeState>>> = OnceLock::new();
static GUARDIAN_STARTED: AtomicBool = AtomicBool::new(false);
static GATEWAY_CONFIG_AUTO_FIX_STATE: OnceLock<Arc<Mutex<GatewayConfigAutoFixState>>> = OnceLock::new();

fn gateway_config_auto_fix_state() -> &'static Arc<Mutex<GatewayConfigAutoFixState>> {
    GATEWAY_CONFIG_AUTO_FIX_STATE.get_or_init(|| Arc::new(Mutex::new(GatewayConfigAutoFixState::default())))
}

fn finish_gateway_config_auto_fix_attempt() {
    let mut state = gateway_config_auto_fix_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.in_progress = false;
}

async fn try_auto_fix_gateway_config(reason: &str, app: Option<&tauri::AppHandle>) -> Result<bool, String> {
    if !looks_like_gateway_config_mismatch(reason) {
        return Ok(false);
    }

    {
        let mut state = gateway_config_auto_fix_state()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.in_progress {
            return Ok(false);
        }
        if let Some(last_attempt) = state.last_attempt {
            if last_attempt.elapsed() < GATEWAY_CONFIG_AUTO_FIX_COOLDOWN {
                return Ok(false);
            }
        }
        state.in_progress = true;
        state.last_attempt = Some(Instant::now());
    }

    guardian_log("检测到 Gateway 启动疑似配置失配，尝试自动执行 openclaw doctor --fix");
    emit_guardian_event(app, "auto_fix_start", "检测到 Gateway 配置异常，正在自动执行 openclaw doctor --fix…");

    let result = tokio::time::timeout(
        Duration::from_secs(30),
        crate::utils::openclaw_command_async().args(["doctor", "--fix"]).output(),
    )
    .await;

    finish_gateway_config_auto_fix_attempt();

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if output.status.success() {
                let summary = if !stderr.is_empty() { stderr } else { stdout };
                if summary.is_empty() {
                    guardian_log("自动执行 openclaw doctor --fix 成功");
                } else {
                    guardian_log(&format!("自动执行 openclaw doctor --fix 成功: {summary}"));
                }
                Ok(true)
            } else {
                let summary = if !stderr.is_empty() { stderr } else { stdout };
                let detail = if summary.is_empty() {
                    "doctor --fix 返回失败".to_string()
                } else {
                    summary
                };
                guardian_log(&format!("自动执行 openclaw doctor --fix 失败: {detail}"));
                emit_guardian_event(
                    app,
                    "auto_fix_failure",
                    format!("已尝试自动执行 openclaw doctor --fix，但修复失败：{detail}"),
                );
                Err(format!(
                    "检测到 Gateway 配置异常，已尝试自动执行 openclaw doctor --fix，但修复失败：{detail}"
                ))
            }
        }
        Ok(Err(err)) => {
            guardian_log(&format!("自动执行 openclaw doctor --fix 失败: {err}"));
            emit_guardian_event(
                app,
                "auto_fix_failure",
                format!("已尝试自动执行 openclaw doctor --fix，但命令执行失败：{err}"),
            );
            Err(format!(
                "检测到 Gateway 配置异常，已尝试自动执行 openclaw doctor --fix，但命令执行失败：{err}"
            ))
        }
        Err(_) => {
            guardian_log("自动执行 openclaw doctor --fix 超时 (30s)");
            emit_guardian_event(app, "auto_fix_failure", "已尝试自动执行 openclaw doctor --fix，但修复超时 (30s)");
            Err("检测到 Gateway 配置异常，已尝试自动执行 openclaw doctor --fix，但修复超时 (30s)".into())
        }
    }
}

fn guardian_state() -> &'static Arc<Mutex<GuardianRuntimeState>> {
    GUARDIAN_STATE.get_or_init(|| Arc::new(Mutex::new(GuardianRuntimeState::default())))
}

pub(super) fn guardian_log(message: &str) {
    let log_dir = crate::commands::openclaw_dir().join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let path = log_dir.join("guardian.log");
    let line = format!("[{}] {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), message);
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
}

fn emit_guardian_event(app: Option<&tauri::AppHandle>, kind: &str, message: impl Into<String>) {
    if let Some(app) = app {
        let payload = GuardianEventPayload {
            kind: kind.to_string(),
            auto_restart_count: 0,
            message: message.into(),
        };
        let _ = app.emit("guardian-event", payload);
    }
}

fn guardian_snapshot() -> GuardianStatus {
    let state = guardian_state().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    GuardianStatus {
        backend_managed: true,
        paused: state.pause_reason.is_some(),
        manual_hold: state.manual_hold,
        give_up: state.give_up,
        auto_restart_count: state.auto_restart_count,
    }
}

pub(crate) fn guardian_mark_manual_stop() {
    let mut state = guardian_state().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    state.manual_hold = true;
    state.give_up = false;
    state.auto_restart_count = 0;
    state.last_restart_time = None;
    state.running_since = None;
    guardian_log("用户主动停止 Gateway，后端守护进入手动停机保持状态");
}

pub(crate) fn guardian_mark_manual_start() {
    let mut state = guardian_state().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    state.manual_hold = false;
    state.give_up = false;
    state.auto_restart_count = 0;
    state.last_restart_time = None;
    state.running_since = None;
    guardian_log("用户主动启动/恢复 Gateway，后端守护已重置自动重启状态");
}

pub(crate) fn guardian_pause(reason: &str) {
    let mut state = guardian_state().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    state.pause_reason = Some(reason.to_string());
    state.give_up = false;
    guardian_log(&format!("后端守护已暂停: {reason}"));
}

pub(crate) fn guardian_resume(reason: &str) {
    let mut state = guardian_state().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    state.pause_reason = None;
    state.running_since = None;
    guardian_log(&format!("后端守护已恢复: {reason}"));
}

fn gateway_config_exists() -> bool {
    crate::commands::openclaw_dir().join("openclaw.json").exists()
}

async fn gateway_service_status() -> Result<Option<ServiceStatus>, String> {
    let mut services = get_services_status().await?;
    if let Some(index) = services.iter().position(|svc| svc.label == "ai.openclaw.gateway") {
        return Ok(Some(services.remove(index)));
    }
    Ok(services.into_iter().next())
}

async fn guardian_tick(app: &tauri::AppHandle) {
    let snapshot = match gateway_service_status().await {
        Ok(Some(svc)) => svc,
        Ok(None) => return,
        Err(err) => {
            guardian_log(&format!("读取 Gateway 状态失败: {err}"));
            return;
        }
    };

    let ready = snapshot.cli_installed && gateway_config_exists();
    let running = snapshot.running;
    let now = Instant::now();
    let (restart_attempt, emit_give_up) = {
        let mut state = guardian_state().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut restart_attempt = None::<u32>;
        let mut emit_give_up = None::<String>;

        if state.last_seen_running.is_none() {
            state.last_seen_running = Some(running);
            state.running_since = running.then_some(now);
            return;
        }

        if !ready {
            state.last_seen_running = Some(running);
            state.running_since = running.then_some(now);
            return;
        }

        if state.pause_reason.is_some() {
            state.last_seen_running = Some(running);
            state.running_since = if running { state.running_since.or(Some(now)) } else { None };
            return;
        }

        if running {
            if state.last_seen_running != Some(true) {
                if state.manual_hold || state.give_up {
                    state.manual_hold = false;
                    state.give_up = false;
                    state.auto_restart_count = 0;
                    state.last_restart_time = None;
                    guardian_log("检测到 Gateway 已重新运行，后端守护已退出手动停机/放弃状态");
                }
                state.running_since = Some(now);
            }

            if state.auto_restart_count > 0
                && state
                    .running_since
                    .map(|ts| now.duration_since(ts) >= GUARDIAN_STABLE_WINDOW)
                    .unwrap_or(false)
            {
                state.auto_restart_count = 0;
                state.last_restart_time = None;
                guardian_log("Gateway 已稳定运行，后端守护已清零自动重启计数");
            }

            state.last_seen_running = Some(true);
            return;
        }

        let was_running = state.last_seen_running == Some(true);
        state.last_seen_running = Some(false);
        state.running_since = None;

        if !was_running || state.manual_hold || state.give_up {
            return;
        }

        if std::env::consts::OS == "windows" {
            state.manual_hold = true;
            state.auto_restart_count = 0;
            state.last_restart_time = None;
            guardian_log("检测到 Windows Gateway 终端窗口已关闭，按用户停机处理，不自动重启");
            return;
        }

        if let Some(last) = state.last_restart_time {
            if now.duration_since(last) < GUARDIAN_RESTART_COOLDOWN {
                return;
            }
        }

        if state.auto_restart_count >= GUARDIAN_MAX_AUTO_RESTART {
            state.give_up = true;
            let message = format!("Gateway 连续自动重启 {} 次后仍异常，后端守护已停止自动拉起", GUARDIAN_MAX_AUTO_RESTART);
            guardian_log(&message);
            emit_give_up = Some(message);
            (restart_attempt, emit_give_up)
        } else {
            state.auto_restart_count += 1;
            state.last_restart_time = Some(now);
            restart_attempt = Some(state.auto_restart_count);
            (restart_attempt, emit_give_up)
        }
    };

    if let Some(attempt) = restart_attempt {
        guardian_log(&format!(
            "检测到 Gateway 异常退出，后端守护开始自动重启 ({attempt}/{GUARDIAN_MAX_AUTO_RESTART})"
        ));
        if let Err(err) = start_service_impl_internal("ai.openclaw.gateway", Some(app)).await {
            guardian_log(&format!("后端守护自动重启失败: {err}"));
        }
    }

    if let Some(message) = emit_give_up {
        let payload = GuardianEventPayload {
            kind: "give_up".into(),
            auto_restart_count: GUARDIAN_MAX_AUTO_RESTART,
            message,
        };
        let _ = app.emit("guardian-event", payload);
    }
}

include!("service_modules/runtime_commands.rs");
