
// ---------------------------------------------------------------------------
// Gateway Guardian — 进程守护 + 状态追踪
// ---------------------------------------------------------------------------

/// 我们 spawn 的 Gateway 进程 PID（0 表示没有）
static GW_PID: AtomicU32 = AtomicU32::new(0);
/// Guardian 是否正在运行
static GW_GUARDIAN_ACTIVE: AtomicBool = AtomicBool::new(false);
/// 通知 guardian 停止的 flag
static GW_GUARDIAN_STOP: AtomicBool = AtomicBool::new(false);
static GW_STARTING: AtomicBool = AtomicBool::new(false);
/// 缓存 AppHandle 供 guardian 发送事件
static GW_APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();

struct GatewayStartGuard;

impl Drop for GatewayStartGuard {
    fn drop(&mut self) {
        GW_STARTING.store(false, Ordering::SeqCst);
    }
}

fn try_gateway_start_guard() -> Option<GatewayStartGuard> {
    GW_STARTING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .ok()
        .map(|_| GatewayStartGuard)
}

/// 获取 Gateway 的完整 URL（当前本地，未来可扩展为远程）
fn hermes_gateway_custom_url() -> Option<String> {
    super::read_panel_config_value()
        .and_then(|v| v.get("hermes")?.get("gatewayUrl")?.as_str().map(String::from))
        .filter(|s| !s.trim().is_empty())
        .map(|url| url.trim_end_matches('/').to_string())
}

fn is_loopback_gateway_url(url: &str) -> bool {
    let rest = url
        .trim()
        .strip_prefix("http://")
        .or_else(|| url.trim().strip_prefix("https://"))
        .unwrap_or(url.trim());
    let host = if let Some(stripped) = rest.strip_prefix('[') {
        stripped.split(']').next().unwrap_or("")
    } else {
        rest.split('/').next().unwrap_or("").split(':').next().unwrap_or("")
    };
    let lower = host.trim().to_ascii_lowercase();
    if lower == "localhost" || lower.ends_with(".localhost") {
        return true;
    }
    lower.parse::<std::net::IpAddr>().map(|ip| ip.is_loopback()).unwrap_or(false)
}

fn hermes_gateway_url() -> String {
    if let Some(url) = hermes_gateway_custom_url() {
        return url;
    }
    let port = hermes_gateway_port();
    format!("http://127.0.0.1:{port}")
}

async fn ensure_managed_gateway_ready(app: &tauri::AppHandle, gw_url: &str) -> Result<(), String> {
    if let Some(url) = hermes_gateway_custom_url() {
        if !is_loopback_gateway_url(&url) {
            return Ok(());
        }
    }
    let _ = sanitize_hermes_openrouter_custom_mismatch()?;
    if gateway_quick_health_check().await {
        start_guardian(app);
        emit_gateway_status(true);
        return Ok(());
    }
    hermes_gateway_action(app.clone(), "start".into())
        .await
        .map(|_| ())
        .map_err(|e| format!("Gateway 未运行且自动启动失败: {e}\nGateway: {gw_url}\n{}", hermes_gateway_log_tail(20)))
}

fn hermes_gateway_http_client(timeout: std::time::Duration) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(timeout)
        .user_agent("ZhizhuaWorkbench")
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())
}

fn reqwest_error_detail(error: &reqwest::Error) -> String {
    use std::error::Error as _;
    let mut detail = error.to_string();
    let mut source = error.source();
    while let Some(item) = source {
        let text = item.to_string();
        if !text.is_empty() && !detail.contains(&text) {
            detail.push_str(": ");
            detail.push_str(&text);
        }
        source = item.source();
    }
    detail
}

fn hermes_gateway_log_tail(limit: usize) -> String {
    let log_path = hermes_home().join("gateway-run.log");
    let content = std::fs::read_to_string(log_path).unwrap_or_default();
    content
        .lines()
        .rev()
        .take(limit)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
}

async fn hermes_run_failure_message(action: &str, gw_url: &str, detail: String) -> String {
    let health_url = format!("{gw_url}/health");
    let health = match hermes_gateway_http_client(std::time::Duration::from_secs(3)) {
        Ok(client) => match client.get(&health_url).send().await {
            Ok(resp) => format!("HTTP {}", resp.status().as_u16()),
            Err(error) => format!("不可达 ({})", reqwest_error_detail(&error)),
        },
        Err(error) => format!("无法创建客户端 ({error})"),
    };
    let log_tail = hermes_gateway_log_tail(12);
    let log_block = if log_tail.trim().is_empty() {
        "最近 Gateway 日志为空".to_string()
    } else {
        format!("最近 Gateway 日志:\n{log_tail}")
    };
    format!(
        "{action}: {detail}\nGateway: {gw_url}\nHealth: {health}\n建议：在 Hermes 服务页点击“重启 Gateway”后重试；如果刚改过模型/API Key，必须重启 Gateway。\n{log_block}"
    )
}

/// 精准杀掉我们 spawn 的 Gateway 进程
fn kill_gateway_pid() -> bool {
    let pid = GW_PID.load(Ordering::SeqCst);
    if pid == 0 {
        return false;
    }
    #[cfg(target_os = "windows")]
    {
        let mut cmd = std::process::Command::new("taskkill");
        cmd.args(["/F", "/PID", &pid.to_string()]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        let ok = cmd.output().map(|o| o.status.success()).unwrap_or(false);
        if ok {
            GW_PID.store(0, Ordering::SeqCst);
        }
        ok
    }
    #[cfg(not(target_os = "windows"))]
    {
        let ok = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ok {
            GW_PID.store(0, Ordering::SeqCst);
        }
        ok
    }
}

fn cleanup_stale_gateway_runtime_files(home: &Path) {
    for name in ["gateway.lock", "gateway.pid", "gateway_state.json", "gateway-run.log"] {
        let path = home.join(name);
        if !path.exists() {
            continue;
        }
        if let Ok(metadata) = std::fs::metadata(&path) {
            let mut permissions = metadata.permissions();
            if permissions.readonly() {
                #[cfg(windows)]
                {
                    #[allow(clippy::permissions_set_readonly_false)]
                    permissions.set_readonly(false);
                    let _ = std::fs::set_permissions(&path, permissions);
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mode = permissions.mode();
                    permissions.set_mode(mode | 0o200);
                    let _ = std::fs::set_permissions(&path, permissions);
                }
                #[cfg(not(any(windows, unix)))]
                {
                    let _ = permissions;
                }
            }
        }
        let _ = std::fs::remove_file(&path);
    }
}

/// Guardian 后台任务：定期健康检查，失败时自动重启
async fn gateway_guardian_loop() {
    const CHECK_INTERVAL_SECS: u64 = 15;
    const MAX_FAIL_BEFORE_RESTART: u32 = 3;
    const MAX_RESTART_ATTEMPTS: u32 = 5;
    const RESTART_BACKOFF_BASE_SECS: u64 = 5;

    let mut consecutive_fails: u32 = 0;
    let mut restart_count: u32 = 0;
    let mut last_known_running = true;

    loop {
        // 检查是否被要求停止
        if GW_GUARDIAN_STOP.load(Ordering::SeqCst) {
            break;
        }

        tokio::time::sleep(std::time::Duration::from_secs(CHECK_INTERVAL_SECS)).await;

        if GW_GUARDIAN_STOP.load(Ordering::SeqCst) {
            break;
        }

        // 健康检查
        let healthy = gateway_quick_health_check().await;

        if healthy {
            if !last_known_running {
                // 状态恢复
                emit_gateway_status(true);
                last_known_running = true;
            }
            consecutive_fails = 0;
            restart_count = 0; // 稳定运行一段时间后重置重启计数
        } else {
            consecutive_fails += 1;

            if last_known_running && consecutive_fails >= 2 {
                // 状态变为离线
                emit_gateway_status(false);
                last_known_running = false;
            }

            if consecutive_fails >= MAX_FAIL_BEFORE_RESTART {
                if restart_count >= MAX_RESTART_ATTEMPTS {
                    // 超过最大重启次数，放弃
                    emit_guardian_log(&format!("Gateway 已连续重启 {} 次仍然失败，Guardian 停止自动恢复", restart_count));
                    break;
                }

                // 指数退避重启
                let backoff = RESTART_BACKOFF_BASE_SECS * (1 << restart_count.min(4));
                emit_guardian_log(&format!(
                    "Gateway 连续 {} 次健康检查失败，{}s 后尝试重启 (第 {} 次)",
                    consecutive_fails,
                    backoff,
                    restart_count + 1
                ));
                tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;

                if GW_GUARDIAN_STOP.load(Ordering::SeqCst) {
                    break;
                }

                // 尝试重启
                match do_restart_gateway().await {
                    Ok(_) => {
                        emit_guardian_log("Gateway 自动重启成功");
                        emit_gateway_status(true);
                        last_known_running = true;
                        consecutive_fails = 0;
                        restart_count += 1;
                    }
                    Err(e) => {
                        emit_guardian_log(&format!("Gateway 自动重启失败: {e}"));
                        restart_count += 1;
                    }
                }
            }
        }
    }

    GW_GUARDIAN_ACTIVE.store(false, Ordering::SeqCst);
}

/// 快速健康检查（TCP + HTTP，1s 超时）
async fn gateway_quick_health_check() -> bool {
    let url = hermes_gateway_url();
    let health_url = format!("{url}/health");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .no_proxy()
        .build();
    match client {
        Ok(c) => c
            .get(&health_url)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false),
        Err(_) => false,
    }
}

fn normalize_provider_url(raw: &str) -> String {
    let mut out = raw.trim().trim_end_matches('/').to_ascii_lowercase();
    for suffix in ["/chat/completions", "/completions", "/responses", "/messages", "/models"] {
        if out.ends_with(suffix) {
            out.truncate(out.len() - suffix.len());
            break;
        }
    }
    out
}

fn normalize_hermes_provider_for_base_url(provider: &str, base_url: Option<&str>) -> String {
    let pid = provider.trim();
    if pid.eq_ignore_ascii_case("aizuopin") {
        return "custom".into();
    }
    if pid == "openrouter" {
        if let Some(url) = base_url {
            let base = normalize_provider_url(url);
            let expected = normalize_provider_url("https://openrouter.ai/api/v1");
            if !base.is_empty() && base != expected {
                return "custom".into();
            }
        }
    }
    pid.to_string()
}

fn env_file_has_value(raw: &str, key: &str) -> bool {
    raw.lines().any(|line| {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            return false;
        }
        t.split_once('=')
            .map(|(k, v)| k.trim() == key && !v.trim().is_empty())
            .unwrap_or(false)
    })
}

fn env_file_value(raw: &str, key: &str) -> Option<String> {
    raw.lines().find_map(|line| {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            return None;
        }
        t.split_once('=').and_then(|(k, v)| {
            if k.trim() == key {
                let value = v.trim();
                if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                }
            } else {
                None
            }
        })
    })
}

fn ensure_custom_openai_key_alias() -> Result<bool, String> {
    let env_path = hermes_home().join(".env");
    if !env_path.exists() {
        return Ok(false);
    }
    let raw = std::fs::read_to_string(&env_path).map_err(|e| format!("读取 .env 失败: {e}"))?;
    if env_file_has_value(&raw, "OPENAI_API_KEY") {
        return Ok(false);
    }
    let Some(custom_key) = env_file_value(&raw, "CUSTOM_API_KEY") else {
        return Ok(false);
    };
    let mut fixed = raw;
    if !fixed.ends_with('\n') {
        fixed.push('\n');
    }
    fixed.push_str(&format!("OPENAI_API_KEY={custom_key}\n"));
    std::fs::write(&env_path, fixed).map_err(|e| format!("写入 .env 失败: {e}"))?;
    Ok(true)
}

fn sanitize_hermes_openrouter_custom_mismatch() -> Result<bool, String> {
    let home = hermes_home();
    let config_path = home.join("config.yaml");
    if !config_path.exists() {
        return Ok(false);
    }

    let raw = std::fs::read_to_string(&config_path).map_err(|e| format!("读取 config.yaml 失败: {e}"))?;
    let mut provider = String::new();
    let mut base_url = String::new();
    let mut in_model = false;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("model:") {
            in_model = true;
            continue;
        }
        if in_model {
            let indented = line.starts_with(' ') || line.starts_with('\t');
            if !indented && !trimmed.is_empty() && !trimmed.starts_with('#') {
                break;
            }
            if let Some(v) = trimmed.strip_prefix("provider:") {
                provider = v.trim().trim_matches('"').trim_matches('\'').to_string();
            } else if let Some(v) = trimmed.strip_prefix("base_url:") {
                base_url = v.trim().trim_matches('"').trim_matches('\'').to_string();
            }
        }
    }

    let base = normalize_provider_url(&base_url);
    let expected = normalize_provider_url("https://openrouter.ai/api/v1");
    let uses_custom_endpoint = !base.is_empty() && base != expected;
    let alias_changed = if provider.is_empty() || provider == "custom" || uses_custom_endpoint {
        ensure_custom_openai_key_alias()?
    } else {
        false
    };
    if !uses_custom_endpoint {
        return Ok(alias_changed);
    }
    if provider == "custom" {
        return Ok(alias_changed);
    }

    let mut out = Vec::new();
    let mut in_model = false;
    let mut provider_written = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("model:") {
            in_model = true;
            provider_written = false;
            out.push(line.to_string());
            continue;
        }
        if in_model {
            let indented = line.starts_with(' ') || line.starts_with('\t');
            if !indented && !trimmed.is_empty() && !trimmed.starts_with('#') {
                in_model = false;
                if !provider_written {
                    out.push("  provider: custom".to_string());
                    provider_written = true;
                }
            } else if trimmed.starts_with("provider:") {
                out.push("  provider: custom".to_string());
                provider_written = true;
                continue;
            }
        }
        out.push(line.to_string());
    }
    if in_model && !provider_written {
        out.push("  provider: custom".to_string());
    }
    let mut fixed = out.join("\n");
    if !fixed.ends_with('\n') {
        fixed.push('\n');
    }
    std::fs::write(&config_path, fixed).map_err(|e| format!("写入 config.yaml 失败: {e}"))?;
    Ok(true)
}

/// 重启 Gateway（kill 旧进程 → 启动新进程）
async fn do_restart_gateway() -> Result<(), String> {
    // 1. 杀掉旧进程
    kill_gateway_pid();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // 2. 清理旧运行态文件
    let home = hermes_home();
    cleanup_stale_gateway_runtime_files(&home);

    // 3. 启动新进程
    let enhanced = hermes_enhanced_path();
    let log_path = home.join("gateway-run.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("打开日志失败: {e}"))?;
    let log_err = log_file.try_clone().map_err(|e| format!("克隆日志句柄失败: {e}"))?;

    let hermes_cmd = hermes_executable_path().unwrap_or_else(|| PathBuf::from("hermes"));
    let mut cmd = std::process::Command::new(&hermes_cmd);
    cmd.args(["gateway", "run"])
        .current_dir(&home)
        .env("PATH", &enhanced)
        .stdin(std::process::Stdio::null())
        .stdout(log_file)
        .stderr(log_err);
    apply_hermes_runtime_env_std(&mut cmd);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    // 注入 .env
    let env_path = home.join(".env");
    if let Ok(env_content) = std::fs::read_to_string(&env_path) {
        for line in env_content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, val)) = line.split_once('=') {
                cmd.env(key.trim(), val.trim());
            }
        }
    }

    let child = cmd.spawn().map_err(|e| format!("启动 hermes gateway run 失败: {e}"))?;
    GW_PID.store(child.id(), Ordering::SeqCst);

    // 4. 等待端口可达（最多 15s）
    let port = hermes_gateway_port();
    let addr: std::net::SocketAddr = format!("127.0.0.1:{port}")
        .parse()
        .map_err(|e| format!("解析 Hermes Gateway 地址失败: {e}"))?;
    for _ in 0..30 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(500)).is_ok() {
            return Ok(());
        }
    }
    Err("Gateway 重启后端口未就绪".into())
}

/// 发送 Gateway 状态事件给前端
fn emit_gateway_status(running: bool) {
    if let Some(app) = GW_APP_HANDLE.get() {
        let port = hermes_gateway_port();
        let _ = app.emit(
            "hermes-gateway-status",
            crate::jv!({
                "running": running,
                "port": port,
                "url": hermes_gateway_url(),
            }),
        );
    }
}

/// 发送 Guardian 日志事件给前端
fn emit_guardian_log(msg: &str) {
    if let Some(app) = GW_APP_HANDLE.get() {
        let _ = app.emit("hermes-guardian-log", msg);
    }
}

/// 启动 Guardian（如果尚未运行）
fn start_guardian(app: &tauri::AppHandle) {
    let _ = GW_APP_HANDLE.set(app.clone());
    if GW_GUARDIAN_ACTIVE
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        GW_GUARDIAN_STOP.store(false, Ordering::SeqCst);
        tokio::spawn(gateway_guardian_loop());
    }
}

/// 停止 Guardian
fn stop_guardian() {
    GW_GUARDIAN_STOP.store(true, Ordering::SeqCst);
}

// ---------------------------------------------------------------------------
// check_python — 检测 Python 环境
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn check_python() -> Result<Value, String> {
    let enhanced = hermes_enhanced_path();
    let mut result = serde_json::Map::new();

    // 平台标识
    result.insert("platform".into(), Value::String(current_platform_key().into()));

    // 尝试多种 Python 命令
    let python_candidates: Vec<(&str, Vec<&str>)> = {
        #[cfg(target_os = "windows")]
        {
            vec![
                ("py", vec!["-3", "--version"]),
                ("python", vec!["--version"]),
                ("python3", vec!["--version"]),
            ]
        }
        #[cfg(not(target_os = "windows"))]
        {
            vec![("python3", vec!["--version"]), ("python", vec!["--version"])]
        }
    };

    let mut found = false;
    for (cmd, args) in &python_candidates {
        if let Ok(ver_str) = run_at_path(cmd, args, &enhanced) {
            if let Some((major, minor, patch)) = parse_python_version(&ver_str) {
                let version = format!("{major}.{minor}.{patch}");
                let version_ok = major >= 3 && minor >= 11;
                result.insert("installed".into(), Value::Bool(true));
                result.insert("version".into(), Value::String(version));
                result.insert("versionOk".into(), Value::Bool(version_ok));
                result.insert("pythonCmd".into(), Value::String(cmd.to_string()));

                // 尝试获取 Python 路径
                let path_result = find_executable_path(cmd, &enhanced);
                result.insert("path".into(), path_result.map(Value::String).unwrap_or(Value::Null));

                found = true;
                break;
            }
        }
    }

    if !found {
        result.insert("installed".into(), Value::Bool(false));
        result.insert("version".into(), Value::Null);
        result.insert("versionOk".into(), Value::Bool(false));
        result.insert("path".into(), Value::Null);
        result.insert("pythonCmd".into(), Value::Null);
    }

    // 检测 pip
    let has_pip = run_at_path("pip", &["--version"], &enhanced).is_ok() || run_at_path("pip3", &["--version"], &enhanced).is_ok();
    result.insert("hasPip".into(), Value::Bool(has_pip));

    // 检测 pipx
    let has_pipx = run_at_path("pipx", &["--version"], &enhanced).is_ok();
    result.insert("hasPipx".into(), Value::Bool(has_pipx));

    // 检测 uv
    let uv_path = uv_bin_path();
    let has_uv = if uv_path.exists() {
        true
    } else {
        run_at_path("uv", &["--version"], &enhanced).is_ok()
    };
    result.insert("hasUv".into(), Value::Bool(has_uv));

    // 检测 git
    let has_git = run_at_path("git", &["--version"], &enhanced).is_ok();
    result.insert("hasGit".into(), Value::Bool(has_git));

    // 检测 brew（macOS/Linux）
    #[cfg(not(target_os = "windows"))]
    {
        let has_brew = run_at_path("brew", &["--version"], &enhanced).is_ok();
        result.insert("hasBrew".into(), Value::Bool(has_brew));
    }
    #[cfg(target_os = "windows")]
    {
        result.insert("hasBrew".into(), Value::Bool(false));
    }

    Ok(Value::Object(result))
}
