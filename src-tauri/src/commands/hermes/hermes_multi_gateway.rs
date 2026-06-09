use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::Emitter;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

use super::hermes_runtime::{apply_hermes_runtime_env_std, hermes_enhanced_path, hermes_home};

// ============================================================================
// Batch 2 §G: 多 Gateway 看板
//
// 让用户同时运行多个 Hermes Gateway 实例（每个绑不同 profile）。
// 用 `hermes --profile <name> gateway run` 启动，PID 跟踪在内存里。
//
// 持久化：~/.openclaw/clawpanel.json 的 hermes.multiGateways 数组
//   [{ name: "main", profile: "default" }, { name: "coder", profile: "coder" }]
//
// 端口：从 profile 的 config.yaml 读 model.gateway.port（每个 profile 独立配置）。
//
// 状态：TCP 探测每个端口 + 检查 PID 是否仍活着。
// ============================================================================

static MULTI_GW_PIDS: Mutex<Option<HashMap<String, u32>>> = Mutex::new(None);

fn multi_gw_pids_get(name: &str) -> Option<u32> {
    MULTI_GW_PIDS.lock().ok().and_then(|guard| guard.as_ref()?.get(name).copied())
}

fn multi_gw_pids_set(name: &str, pid: u32) {
    if let Ok(mut guard) = MULTI_GW_PIDS.lock() {
        guard.get_or_insert_with(HashMap::new).insert(name.to_string(), pid);
    }
}

fn multi_gw_pids_remove(name: &str) {
    if let Ok(mut guard) = MULTI_GW_PIDS.lock() {
        if let Some(map) = guard.as_mut() {
            map.remove(name);
        }
    }
}

/// 读取 panel config 的 multiGateways 列表
fn read_multi_gateways_config() -> Vec<Value> {
    super::read_panel_config_value()
        .and_then(|v| v.get("hermes")?.get("multiGateways").cloned())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
}

/// 写入 panel config 的 multiGateways 列表（保留其他字段）
fn write_multi_gateways_config(gateways: Vec<Value>) -> Result<(), String> {
    let config_path = super::panel_config_path();
    if let Some(parent) = config_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut root: serde_json::Map<String, Value> = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).map_err(|e| format!("读取 panel 配置失败: {e}"))?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        serde_json::Map::new()
    };
    // root.hermes.multiGateways = gateways
    let mut hermes_obj = root.get("hermes").and_then(|v| v.as_object()).cloned().unwrap_or_default();
    hermes_obj.insert("multiGateways".into(), Value::Array(gateways));
    root.insert("hermes".into(), Value::Object(hermes_obj));
    let json = serde_json::to_string_pretty(&Value::Object(root)).map_err(|e| format!("序列化失败: {e}"))?;
    std::fs::write(&config_path, json).map_err(|e| format!("写入失败: {e}"))?;
    Ok(())
}

/// 读 profile config.yaml 的 model.gateway.port（缩进感知）
fn read_profile_gateway_port(profile: &str) -> u16 {
    let home = if profile == "default" {
        hermes_home()
    } else {
        hermes_home().join("profiles").join(profile)
    };
    let config_path = home.join("config.yaml");
    let Ok(content) = std::fs::read_to_string(&config_path) else {
        return 8642;
    };
    // 简单缩进感知解析：model: → gateway: → port:
    let mut in_model = false;
    let mut in_gateway = false;
    for line in content.lines() {
        let raw_indent = line.len() - line.trim_start().len();
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if raw_indent == 0 {
            in_model = trimmed.starts_with("model:");
            in_gateway = false;
        } else if in_model && raw_indent == 2 {
            in_gateway = trimmed.starts_with("gateway:");
        } else if in_model && in_gateway && raw_indent == 4 {
            if let Some(p) = trimmed.strip_prefix("port:") {
                if let Ok(n) = p.trim().parse::<u16>() {
                    return n;
                }
            }
        }
    }
    8642
}

/// 检测 PID 是否仍然存活
fn pid_is_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(target_os = "windows")]
    {
        let out = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        match out {
            Ok(o) => {
                let s = String::from_utf8_lossy(&o.stdout);
                s.lines().any(|l| l.contains(&pid.to_string()))
            }
            Err(_) => false,
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        // kill -0 signal 0 不杀进程，只检查存在性
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[tauri::command]
pub async fn hermes_multi_gateway_list() -> Result<Value, String> {
    let configs = read_multi_gateways_config();
    let mut result = Vec::new();
    for cfg in configs {
        let name = cfg.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let profile = cfg.get("profile").and_then(|v| v.as_str()).unwrap_or("default").to_string();
        if name.is_empty() {
            continue;
        }
        let port = read_profile_gateway_port(&profile);
        // PID-based liveness
        let pid_opt = multi_gw_pids_get(&name);
        let pid_alive = pid_opt.map(pid_is_alive).unwrap_or(false);
        // TCP probe（即使 PID 死了，也可能其他进程占着端口）
        let addr = format!("127.0.0.1:{port}");
        let tcp_running = addr
            .parse::<std::net::SocketAddr>()
            .ok()
            .and_then(|sa| std::net::TcpStream::connect_timeout(&sa, std::time::Duration::from_millis(300)).ok())
            .is_some();
        result.push(crate::jv!({
            "name": name,
            "profile": profile,
            "port": port,
            "running": pid_alive || tcp_running,
            "pid": pid_opt.unwrap_or(0),
            "owned": pid_alive,  // 是否是 ClawPanel spawn 的
        }));
    }
    Ok(Value::Array(result))
}

#[tauri::command]
pub async fn hermes_multi_gateway_add(name: String, profile: String) -> Result<Value, String> {
    let name = name.trim().to_string();
    let profile = profile.trim().to_string();
    if name.is_empty() {
        return Err("名称不能为空".into());
    }
    if profile.is_empty() {
        return Err("Profile 不能为空".into());
    }
    // 名称合法性检查（同 hermes profile 规则）
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err("名称只能含字母/数字/下划线/连字符".into());
    }
    let mut configs = read_multi_gateways_config();
    if configs.iter().any(|c| c.get("name").and_then(|v| v.as_str()) == Some(&name)) {
        return Err(format!("名称 \"{name}\" 已存在"));
    }
    configs.push(crate::jv!({ "name": name, "profile": profile }));
    write_multi_gateways_config(configs)?;
    Ok(crate::jv!({ "ok": true }))
}

#[tauri::command]
pub async fn hermes_multi_gateway_remove(name: String) -> Result<Value, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("名称不能为空".into());
    }
    // 先停掉（如果在跑）
    let _ = hermes_multi_gateway_stop(name.clone()).await;
    let configs: Vec<Value> = read_multi_gateways_config()
        .into_iter()
        .filter(|c| c.get("name").and_then(|v| v.as_str()) != Some(&name))
        .collect();
    write_multi_gateways_config(configs)?;
    Ok(crate::jv!({ "ok": true }))
}

#[tauri::command]
pub async fn hermes_multi_gateway_start(app: tauri::AppHandle, name: String) -> Result<Value, String> {
    let name = name.trim().to_string();
    let configs = read_multi_gateways_config();
    let cfg = configs
        .iter()
        .find(|c| c.get("name").and_then(|v| v.as_str()) == Some(&name))
        .ok_or_else(|| format!("Gateway \"{name}\" 未配置"))?;
    let profile = cfg.get("profile").and_then(|v| v.as_str()).unwrap_or("default").to_string();
    let port = read_profile_gateway_port(&profile);

    // 已运行？
    if let Some(pid) = multi_gw_pids_get(&name) {
        if pid_is_alive(pid) {
            return Ok(crate::jv!({
                "started": true, "already_running": true, "pid": pid, "port": port
            }));
        }
    }
    let addr = format!("127.0.0.1:{port}");
    if let Ok(sa) = addr.parse::<std::net::SocketAddr>() {
        if std::net::TcpStream::connect_timeout(&sa, std::time::Duration::from_millis(300)).is_ok() {
            return Err(format!(
                "端口 {port} 已被占用（非当前工作台拉起的进程，无法接管。请用 services 页停掉默认 Gateway 后重试）"
            ));
        }
    }

    let enhanced = hermes_enhanced_path();
    let home = hermes_home();
    let log_path = home.join(format!("gateway-{name}-run.log"));
    let log_file = std::fs::File::create(&log_path).map_err(|e| format!("创建日志文件失败: {e}"))?;
    let log_err = log_file.try_clone().map_err(|e| format!("克隆日志句柄失败: {e}"))?;

    let mut cmd = std::process::Command::new("hermes");
    cmd.args(["--profile", &profile, "gateway", "run"])
        .current_dir(&home)
        .env("PATH", &enhanced)
        .stdin(std::process::Stdio::null())
        .stdout(log_file)
        .stderr(log_err);
    apply_hermes_runtime_env_std(&mut cmd);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    // 注入 profile 的 .env
    let profile_env = if profile == "default" {
        home.join(".env")
    } else {
        home.join("profiles").join(&profile).join(".env")
    };
    if let Ok(env_content) = std::fs::read_to_string(&profile_env) {
        for line in env_content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                cmd.env(k.trim(), v.trim());
            }
        }
    }

    let child = cmd.spawn().map_err(|e| format!("启动失败: {e}"))?;
    let pid = child.id();
    std::mem::forget(child); // 不等待进程，由 PID 跟踪
    multi_gw_pids_set(&name, pid);

    let _ = app.emit("hermes-multi-gateway-changed", crate::jv!({ "name": &name, "action": "started" }));

    // 等端口起来（最多 8 秒）
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        if let Ok(sa) = addr.parse::<std::net::SocketAddr>() {
            if std::net::TcpStream::connect_timeout(&sa, std::time::Duration::from_millis(200)).is_ok() {
                return Ok(crate::jv!({
                    "started": true, "pid": pid, "port": port
                }));
            }
        }
    }
    Ok(crate::jv!({
        "started": true, "pid": pid, "port": port, "warning": "端口未在 8 秒内可达，可能仍在初始化"
    }))
}

#[tauri::command]
pub async fn hermes_multi_gateway_stop(name: String) -> Result<Value, String> {
    let name = name.trim().to_string();
    let Some(pid) = multi_gw_pids_get(&name) else {
        multi_gw_pids_remove(&name);
        return Ok(crate::jv!({ "stopped": true, "was_running": false }));
    };
    if !pid_is_alive(pid) {
        multi_gw_pids_remove(&name);
        return Ok(crate::jv!({ "stopped": true, "was_running": false }));
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("kill").args(["-TERM", &pid.to_string()]).output();
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if pid_is_alive(pid) {
            let _ = std::process::Command::new("kill").args(["-9", &pid.to_string()]).output();
        }
    }
    multi_gw_pids_remove(&name);
    Ok(crate::jv!({ "stopped": true, "was_running": true, "pid": pid }))
}
