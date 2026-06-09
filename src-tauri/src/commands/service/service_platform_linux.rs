// ===== Linux 实现（与 Windows 类似，使用 openclaw CLI） =====

#[cfg(target_os = "linux")]
pub(super) mod platform {
    use std::env;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::time::Duration;

    static CLI_CACHE: Mutex<Option<(bool, std::time::Instant)>> = Mutex::new(None);
    const CLI_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(60);

    pub fn current_uid() -> Result<u32, String> {
        let output = std::process::Command::new("id")
            .arg("-u")
            .output()
            .map_err(|e| format!("获取 UID 失败: {e}"))?;
        let uid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        uid_str.parse::<u32>().map_err(|e| format!("解析 UID 失败: {e}"))
    }

    /// Linux 上检测 CLI 是否安装（带缓存）
    pub fn is_cli_installed() -> bool {
        if let Ok(guard) = CLI_CACHE.lock() {
            if let Some((val, ts)) = *guard {
                if ts.elapsed() < CLI_CACHE_TTL {
                    return val;
                }
            }
        }
        let result = candidate_cli_paths().into_iter().any(|p| p.exists())
            || std::process::Command::new("which")
                .arg("openclaw")
                .env("PATH", crate::commands::enhanced_path())
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
        if let Ok(mut guard) = CLI_CACHE.lock() {
            *guard = Some((result, std::time::Instant::now()));
        }
        result
    }

    fn candidate_cli_paths() -> Vec<PathBuf> {
        let mut candidates = Vec::new();
        if let Ok(home) = env::var("HOME") {
            candidates.push(PathBuf::from(&home).join(".openclaw").join("openclaw"));
            candidates.push(PathBuf::from(&home).join(".npm-global").join("bin").join("openclaw"));
            candidates.push(PathBuf::from(&home).join("node_modules").join(".bin").join("openclaw"));
        }
        // standalone 安装目录（集中管理，避免多处硬编码）
        for sa_dir in crate::commands::config::all_standalone_dirs() {
            candidates.push(sa_dir.join("openclaw"));
        }
        candidates.push(PathBuf::from("/usr/local/bin/openclaw"));
        candidates.push(PathBuf::from("/usr/bin/openclaw"));
        for segment in crate::commands::enhanced_path().split(':') {
            let dir = segment.trim();
            if dir.is_empty() {
                continue;
            }
            let base = PathBuf::from(dir);
            candidates.push(base.join("openclaw"));
        }
        candidates
    }

    pub fn scan_service_labels() -> Vec<String> {
        vec!["ai.openclaw.gateway".to_string()]
    }

    /// 跨平台统一检测：TCP 连端口
    #[allow(dead_code)]
    pub async fn check_service_status(_uid: u32, _label: &str) -> (bool, Option<u32>) {
        let port = crate::commands::gateway_listen_port();
        let addr = format!("127.0.0.1:{port}");
        let socket_addr: std::net::SocketAddr = match addr.parse() {
            Ok(a) => a,
            Err(_) => return (false, None),
        };
        // 使用 spawn_blocking 避免阻塞 Tokio 运行时
        let result = tokio::task::spawn_blocking(move || {
            std::net::TcpStream::connect_timeout(&socket_addr, std::time::Duration::from_secs(1)).is_ok()
        })
        .await
        .unwrap_or(false);
        if result {
            (true, None)
        } else {
            (false, None)
        }
    }

    /// 清理残留的 Gateway 进程（Linux 版：通过 fuser 查端口占用进程并 kill）
    fn cleanup_zombie_gateway_processes() {
        let port = crate::commands::gateway_listen_port();
        // 尝试用 fuser 找到端口占用进程
        if let Ok(output) = std::process::Command::new("fuser").args([&format!("{port}/tcp")]).output() {
            let pids = String::from_utf8_lossy(&output.stdout);
            for pid_str in pids.split_whitespace() {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    let _ = std::process::Command::new("kill").args(["-9", &pid.to_string()]).output();
                    eprintln!("[cleanup_zombie] killed PID {pid} on port {port}");
                }
            }
        }
    }

    async fn gateway_command(action: &str) -> Result<(), String> {
        if !is_cli_installed() {
            return Err("openclaw CLI 未安装，请先确认便携运行时存在，或执行 npm install -g openclaw 安装".into());
        }
        let action_owned = action.to_string();
        let mut child = crate::utils::openclaw_command_async()
            .args(["gateway", &action_owned])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("执行 openclaw gateway {action_owned} 失败: {e}"))?;

        // 带超时等待命令完成（防止 restart 时旧进程卡死导致永远阻塞）
        let timeout = if action_owned == "stop" || action_owned == "restart" {
            Duration::from_secs(20)
        } else {
            Duration::from_secs(30)
        };

        match tokio::time::timeout(timeout, child.wait()).await {
            Ok(Ok(status)) => {
                if !status.success() {
                    let stderr = if let Some(mut err) = child.stderr.take() {
                        let mut buf = String::new();
                        use tokio::io::AsyncReadExt;
                        let _ = err.read_to_string(&mut buf).await;
                        buf
                    } else {
                        String::new()
                    };
                    if action_owned == "restart" {
                        eprintln!("[gateway_command] restart 失败，尝试强制清理后重启");
                        cleanup_zombie_gateway_processes();
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        return start_service_impl("ai.openclaw.gateway").await;
                    }
                    return Err(format!("openclaw gateway {action_owned} 失败: {stderr}"));
                }
                Ok(())
            }
            Ok(Err(e)) => Err(format!("openclaw gateway {action_owned} 进程异常: {e}")),
            Err(_) => {
                let _ = child.kill().await;
                eprintln!(
                    "[gateway_command] openclaw gateway {} 超时 ({}s)，强制终止",
                    action_owned,
                    timeout.as_secs()
                );
                if action_owned == "restart" || action_owned == "stop" {
                    cleanup_zombie_gateway_processes();
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    if action_owned == "restart" {
                        return start_service_impl("ai.openclaw.gateway").await;
                    }
                    return Ok(());
                }
                Err(format!("openclaw gateway {action_owned} 超时"))
            }
        }
    }

    pub async fn start_service_impl(_label: &str) -> Result<(), String> {
        if !is_cli_installed() {
            return Err("openclaw CLI 未安装，请先确认便携运行时存在，或执行 npm install -g openclaw 安装".into());
        }

        // 启动前检查端口是否已被占用，防止重复拉起导致端口冲突和内存浪费
        let port = crate::commands::gateway_listen_port();
        let pre_check_addr: std::net::SocketAddr = format!("127.0.0.1:{port}")
            .parse()
            .map_err(|_| format!("端口 {port} 解析失败"))?;
        let already_occupied = tokio::task::spawn_blocking(move || {
            std::net::TcpStream::connect_timeout(&pre_check_addr, std::time::Duration::from_millis(500)).is_ok()
        })
        .await
        .unwrap_or(false);
        if already_occupied {
            return Err(format!("端口 {} 已被占用，Gateway 可能已在运行中（或其他程序占用了该端口）", port));
        }

        let output = crate::utils::openclaw_command_async()
            .args(["gateway", "start"])
            .output()
            .await
            .map_err(|e| format!("执行 openclaw gateway start 失败: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("openclaw gateway start 失败: {stderr}"));
        }

        // 等端口就绪（最多 15s）
        let port = crate::commands::gateway_listen_port();
        let addr: std::net::SocketAddr = match format!("127.0.0.1:{port}").parse() {
            Ok(a) => a,
            Err(_) => return Err(format!("端口 {port} 解析失败")),
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while std::time::Instant::now() < deadline {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let addr_clone = addr;
            let connected = tokio::task::spawn_blocking(move || {
                std::net::TcpStream::connect_timeout(&addr_clone, std::time::Duration::from_millis(200)).is_ok()
            })
            .await
            .unwrap_or(false);
            if connected {
                return Ok(());
            }
        }

        Err(format!(
            "Gateway 启动超时，请查看 {}",
            crate::commands::openclaw_dir().join("logs").join("gateway.err.log").display()
        ))
    }

    pub async fn stop_service_impl(_label: &str) -> Result<(), String> {
        gateway_command("stop").await
    }

    #[allow(dead_code)]
    pub async fn restart_service_impl(_label: &str) -> Result<(), String> {
        gateway_command("restart").await
    }
}
