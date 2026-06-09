use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

static DASH_PID: AtomicU32 = AtomicU32::new(0);

fn hermes_dashboard_port() -> u16 {
    let config_path = super::hermes_runtime::hermes_home().join("config.yaml");
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        let mut in_dashboard = false;
        for line in content.lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            let indent = line.len() - line.trim_start().len();
            if indent == 0 {
                in_dashboard = t == "dashboard:" || t.starts_with("dashboard:");
                continue;
            }
            if in_dashboard && t.starts_with("port:") {
                if let Ok(port) = t.trim_start_matches("port:").trim().parse::<u16>() {
                    if port > 0 {
                        return port;
                    }
                }
            }
        }
    }
    9119
}

fn hermes_dashboard_cli_status(port: u16) -> Option<(bool, String)> {
    let output = super::hermes_runtime::run_hermes_silent(&["dashboard", "--status"])
        .or_else(|_| super::hermes_runtime::run_hermes_silent(&["dashboard", "status"]))
        .ok()?;
    let lower = output.to_ascii_lowercase();
    if lower.contains("not running") || lower.contains("stopped") || lower.contains("inactive") || lower.contains("no dashboard")
    {
        return Some((false, output));
    }
    if lower.contains("running")
        || lower.contains("listening")
        || lower.contains("http://")
        || lower.contains("https://")
        || lower.contains(&port.to_string())
    {
        return Some((true, output));
    }
    None
}

fn hermes_dashboard_tcp_running(port: u16, timeout_ms: u64) -> bool {
    let addr = format!("127.0.0.1:{port}");
    let Ok(socket_addr) = addr.parse::<std::net::SocketAddr>() else {
        return false;
    };
    std::net::TcpStream::connect_timeout(&socket_addr, Duration::from_millis(timeout_ms)).is_ok()
}

async fn hermes_dashboard_http_ready(port: u16, timeout_ms: u64) -> bool {
    let client = match reqwest::Client::builder().timeout(Duration::from_millis(timeout_ms)).build() {
        Ok(client) => client,
        Err(_) => return false,
    };
    let Ok(resp) = client
        .get(format!("http://127.0.0.1:{port}/"))
        .header("X-Hermes-Session-Token", super::hermes_runtime::HERMES_DASHBOARD_SESSION_TOKEN)
        .send()
        .await
    else {
        return false;
    };
    if !resp.status().is_success() {
        return false;
    }
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !content_type.contains("text/html") {
        return false;
    }
    resp.text()
        .await
        .map(|body| body.contains("<!doctype html") || body.contains("<html"))
        .unwrap_or(false)
}

fn hermes_dashboard_cli_stop() -> bool {
    super::hermes_runtime::run_hermes_silent(&["dashboard", "--stop"])
        .or_else(|_| super::hermes_runtime::run_hermes_silent(&["dashboard", "stop"]))
        .is_ok()
}

pub(super) async fn ensure_managed_dashboard_ready(app: &tauri::AppHandle) -> Result<u16, String> {
    let port = hermes_dashboard_port();
    if hermes_dashboard_http_ready(port, 1500).await {
        return Ok(port);
    }

    let result = hermes_dashboard_start(app.clone()).await?;
    let started = result.get("started").and_then(|v| v.as_bool()).unwrap_or(false);
    if !started {
        let kind = result.get("kind").and_then(|v| v.as_str()).unwrap_or("spawn_failed");
        let tail = result.get("log_tail").and_then(|v| v.as_str()).unwrap_or("");
        return Err(format!(
            "Dashboard 鑷姩鍚姩澶辫触: {kind}{}",
            if tail.trim().is_empty() {
                String::new()
            } else {
                format!("\n鏈€杩戞棩蹇?\n{tail}")
            }
        ));
    }

    for _ in 0..40 {
        if hermes_dashboard_http_ready(port, 1500).await {
            return Ok(port);
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Err(format!("Dashboard 宸插惎鍔ㄤ絾 http://127.0.0.1:{port}/ 鏈氨缁?"))
}

#[tauri::command]
pub async fn hermes_dashboard_probe() -> Result<serde_json::Value, String> {
    let port = hermes_dashboard_port();
    let cli_status = hermes_dashboard_cli_status(port);
    let cli_output = cli_status.as_ref().map(|(_, output)| output.clone());
    let http_ready = hermes_dashboard_http_ready(port, 1500).await;
    Ok(crate::jv!({ "running": http_ready, "port": port, "status": cli_output }))
}

fn kill_dashboard_pid() -> bool {
    let pid = DASH_PID.load(Ordering::SeqCst);
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
            DASH_PID.store(0, Ordering::SeqCst);
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
            DASH_PID.store(0, Ordering::SeqCst);
        }
        ok
    }
}

#[tauri::command]
pub async fn hermes_dashboard_start(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let port = hermes_dashboard_port();
    if hermes_dashboard_tcp_running(port, 500) || hermes_dashboard_http_ready(port, 1500).await {
        let ready = hermes_dashboard_http_ready(port, 1500).await;
        return Ok(crate::jv!({
            "started": ready,
            "already_running": ready,
            "kind": if ready { "ready" } else { "port_open_but_http_not_ready" },
            "port": port,
        }));
    }

    let _ = kill_dashboard_pid();
    super::hermes_dashboard_stub::inject_hermes_dashboard_compat_stub(&app);

    let home = super::hermes_runtime::hermes_home();
    let log_path = home.join("dashboard-run.log");
    let log_file = std::fs::File::create(&log_path).map_err(|e| format!("鍒涘缓鏃ュ織鏂囦欢澶辫触: {e}"))?;
    let log_err = log_file.try_clone().map_err(|e| format!("鍏嬮殕鏃ュ織鍙ユ焺澶辫触: {e}"))?;

    let enhanced = super::hermes_runtime::hermes_enhanced_path();
    let hermes_cmd = super::hermes_runtime::hermes_executable_path().unwrap_or_else(|| std::path::PathBuf::from("hermes"));
    let port_arg = port.to_string();
    let mut cmd = std::process::Command::new(&hermes_cmd);
    cmd.args([
        "dashboard",
        "--no-open",
        "--skip-build",
        "--host",
        "127.0.0.1",
        "--port",
        port_arg.as_str(),
    ])
    .current_dir(&home)
    .env("PATH", &enhanced)
    .stdin(std::process::Stdio::null())
    .stdout(log_file)
    .stderr(log_err);
    super::hermes_runtime::apply_hermes_runtime_env_std(&mut cmd);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

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

    let mut child = cmd.spawn().map_err(|e| format!("spawn hermes dashboard failed: {e}"))?;
    let pid = child.id();
    DASH_PID.store(pid, Ordering::SeqCst);

    let deadline = std::time::Instant::now() + Duration::from_secs(90);
    while std::time::Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(status)) => {
                DASH_PID.store(0, Ordering::SeqCst);
                let log_raw = std::fs::read_to_string(&log_path).unwrap_or_default();
                let tail = log_raw
                    .lines()
                    .rev()
                    .take(40)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<Vec<_>>()
                    .join("\n");
                let lower = log_raw.to_lowercase();
                let kind = if lower.contains("web ui dependencies not installed")
                    || lower.contains("no module named 'fastapi'")
                    || (lower.contains("import error") && lower.contains("fastapi"))
                {
                    "deps_missing"
                } else if lower.contains("address already in use")
                    || lower.contains("address in use")
                    || (lower.contains("port") && lower.contains("already in use"))
                {
                    "port_in_use"
                } else {
                    "spawn_failed"
                };
                return Ok(crate::jv!({
                    "started": false,
                    "kind": kind,
                    "exit_code": status.code(),
                    "port": port,
                    "log_tail": tail,
                }));
            }
            Ok(None) => {
                if hermes_dashboard_http_ready(port, 1500).await {
                    return Ok(crate::jv!({
                        "started": true,
                        "already_running": false,
                        "port": port,
                        "pid": pid,
                    }));
                }
            }
            Err(e) => {
                let log_raw = std::fs::read_to_string(&log_path).unwrap_or_default();
                let tail = log_raw
                    .lines()
                    .rev()
                    .take(40)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<Vec<_>>()
                    .join("\n");
                return Ok(crate::jv!({
                    "started": false,
                    "kind": "spawn_failed",
                    "port": port,
                    "log_tail": tail,
                    "error": format!("try_wait error: {e}"),
                }));
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    let log_raw = std::fs::read_to_string(&log_path).unwrap_or_default();
    let tail = log_raw
        .lines()
        .rev()
        .take(40)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    Ok(crate::jv!({
        "started": false,
        "kind": "timeout",
        "port": port,
        "pid": pid,
        "log_tail": tail,
    }))
}

#[tauri::command]
pub async fn hermes_dashboard_stop() -> Result<bool, String> {
    let port = hermes_dashboard_port();
    let cli_stopped = tokio::task::spawn_blocking(hermes_dashboard_cli_stop).await.unwrap_or(false);
    let pid_stopped = kill_dashboard_pid();
    if cli_stopped || pid_stopped {
        for _ in 0..20 {
            if !hermes_dashboard_tcp_running(port, 200) {
                return Ok(true);
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        return Ok(true);
    }
    Ok(false)
}
