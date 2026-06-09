// ===== macOS 实现 =====

#[cfg(target_os = "macos")]
pub(super) mod platform {
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    const OPENCLAW_PREFIXES: &[&str] = &["ai.openclaw."];

    fn common_cli_candidates() -> Vec<PathBuf> {
        let mut candidates = Vec::new();
        // standalone 安装目录（集中管理，避免多处硬编码）
        for sa_dir in crate::commands::config::all_standalone_dirs() {
            candidates.push(sa_dir.join("openclaw"));
        }
        // Homebrew 路径（非 standalone，保留）
        candidates.push(PathBuf::from("/opt/homebrew/bin/openclaw"));
        candidates.push(PathBuf::from("/usr/local/bin/openclaw"));
        candidates
    }

    /// macOS 上 CLI 是否安装（兼容手动安装 / standalone / Homebrew）
    pub fn is_cli_installed() -> bool {
        crate::utils::resolve_openclaw_cli_path().is_some() || common_cli_candidates().into_iter().any(|p| p.exists())
    }

    pub fn current_uid() -> Result<u32, String> {
        let output = Command::new("id")
            .arg("-u")
            .output()
            .map_err(|e| format!("获取 UID 失败: {e}"))?;
        let uid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        uid_str.parse::<u32>().map_err(|e| format!("解析 UID 失败: {e}"))
    }

    /// 动态扫描 LaunchAgents 目录，只返回 OpenClaw 核心服务
    pub fn scan_service_labels() -> Vec<String> {
        let home = dirs::home_dir().unwrap_or_default();
        let agents_dir = home.join("Library/LaunchAgents");
        let mut labels = Vec::new();

        if let Ok(entries) = fs::read_dir(&agents_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.ends_with(".plist") {
                    continue;
                }
                let label = name.trim_end_matches(".plist");
                if OPENCLAW_PREFIXES.iter().any(|p| label.starts_with(p)) {
                    labels.push(label.to_string());
                }
            }
        }
        labels.sort();
        if labels.is_empty() {
            labels.push("ai.openclaw.gateway".to_string());
        }
        labels
    }

    fn plist_path(label: &str) -> String {
        let home = dirs::home_dir().unwrap_or_default();
        format!("{}/Library/LaunchAgents/{}.plist", home.display(), label)
    }

    /// 跨平台统一检测：TCP 连端口 + lsof 获取 PID
    pub fn check_service_status(_uid: u32, _label: &str) -> (bool, Option<u32>) {
        let port = crate::commands::gateway_listen_port();
        let addr = format!("127.0.0.1:{port}");
        let socket_addr = match addr.parse() {
            Ok(a) => a,
            Err(_) => return (false, None),
        };
        // 两次尝试：第一次 1 秒，失败后短暂等待再用 2 秒重试，避免瞬态超时误判
        let connected = std::net::TcpStream::connect_timeout(&socket_addr, std::time::Duration::from_secs(1)).is_ok() || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            std::net::TcpStream::connect_timeout(&socket_addr, std::time::Duration::from_secs(2)).is_ok()
        };
        if connected {
            let pid = get_pid_by_lsof(port);
            (true, pid)
        } else {
            (false, None)
        }
    }

    /// 通过 lsof 获取监听指定端口的进程 PID
    fn get_pid_by_lsof(port: u16) -> Option<u32> {
        let output = Command::new("lsof")
            .args(["-i", &format!("TCP:{}", port), "-sTCP:LISTEN", "-t"])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        text.lines().next()?.trim().parse::<u32>().ok()
    }

    /// launchctl 失败时的回退：直接通过 CLI spawn Gateway 进程
    fn start_gateway_direct() -> Result<(), String> {
        // 启动前再次检查端口（防止 launchctl→direct 回退链路中重复拉起）
        let port = crate::commands::gateway_listen_port();
        if let Ok(addr) = format!("127.0.0.1:{port}").parse::<std::net::SocketAddr>() {
            if std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(500)).is_ok() {
                return Err(format!("端口 {} 已被占用，跳过 direct 启动", port));
            }
        }

        let log_dir = crate::commands::openclaw_dir().join("logs");
        fs::create_dir_all(&log_dir).ok();

        let stdout_log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join("gateway.log"))
            .map_err(|e| format!("创建日志文件失败: {e}"))?;

        let stderr_log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join("gateway.err.log"))
            .map_err(|e| format!("创建错误日志文件失败: {e}"))?;

        let mut cmd = crate::utils::openclaw_command();
        cmd.arg("gateway")
            .stdin(std::process::Stdio::null())
            .stdout(stdout_log)
            .stderr(stderr_log);
        cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "OpenClaw CLI 未找到，请确认便携运行时完整并重启智爪平台。".to_string()
            } else {
                format!("启动 Gateway 失败: {e}")
            }
        })?;

        // 等 Gateway 初始化（最多 10s，轮询端口就绪）
        let port = crate::commands::gateway_listen_port();
        let addr = format!("127.0.0.1:{port}");
        let addr = match addr.parse() {
            Ok(a) => a,
            Err(_) => {
                return Err(format!("端口 {port} 解析失败"));
            }
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(500));
            if std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(200)).is_ok() {
                return Ok(());
            }
        }

        Err(format!("Gateway 启动超时，请查看 {}", log_dir.join("gateway.err.log").display()))
    }

    pub fn start_service_impl(label: &str) -> Result<(), String> {
        // 启动前检查端口是否已被占用，防止重复拉起导致端口冲突和内存浪费
        let port = crate::commands::gateway_listen_port();
        let pre_check_addr: std::net::SocketAddr = format!("127.0.0.1:{port}")
            .parse()
            .map_err(|_| format!("端口 {port} 解析失败"))?;
        if std::net::TcpStream::connect_timeout(&pre_check_addr, std::time::Duration::from_millis(500)).is_ok() {
            return Err(format!("端口 {} 已被占用，Gateway 可能已在运行中（或其他程序占用了该端口）", port));
        }

        let uid = current_uid()?;
        let path = plist_path(label);
        let domain_target = format!("gui/{}", uid);
        let service_target = format!("gui/{}/{}", uid, label);

        // 先尝试 plist 文件是否存在
        if !std::path::Path::new(&path).exists() {
            return start_gateway_direct();
        }

        // Issue #91: 先检查服务是否已注册，避免重复 bootstrap 触发 macOS "后台项已添加" 通知
        let already_registered = Command::new("launchctl")
            .args(["print", &service_target])
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false);

        if !already_registered {
            let bootstrap_out = Command::new("launchctl")
                .args(["bootstrap", &domain_target, &path])
                .output()
                .map_err(|e| format!("bootstrap 失败: {e}"))?;

            if !bootstrap_out.status.success() {
                let stderr = String::from_utf8_lossy(&bootstrap_out.stderr);
                if !stderr.contains("already bootstrapped") && !stderr.trim().is_empty() {
                    return start_gateway_direct();
                }
            }
        }

        let kickstart_out = Command::new("launchctl")
            .args(["kickstart", &service_target])
            .output()
            .map_err(|e| format!("kickstart 失败: {e}"))?;

        if !kickstart_out.status.success() {
            let stderr = String::from_utf8_lossy(&kickstart_out.stderr);
            if !stderr.trim().is_empty() {
                // kickstart 也失败，回退到直接启动
                return start_gateway_direct();
            }
        }

        Ok(())
    }

    pub fn stop_service_impl(label: &str) -> Result<(), String> {
        let uid = current_uid()?;
        let service_target = format!("gui/{}/{}", uid, label);

        let output = Command::new("launchctl")
            .args(["bootout", &service_target])
            .output()
            .map_err(|e| format!("停止失败: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("No such process")
                && !stderr.contains("Could not find specified service")
                && !stderr.trim().is_empty()
            {
                return Err(format!("停止 {label} 失败: {stderr}"));
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn restart_service_impl(label: &str) -> Result<(), String> {
        let uid = current_uid()?;
        let path = plist_path(label);
        let domain_target = format!("gui/{}", uid);
        let service_target = format!("gui/{}/{}", uid, label);

        // 先停
        let _ = Command::new("launchctl").args(["bootout", &service_target]).output();

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            let (running, _) = check_service_status(uid, label);
            if !running || std::time::Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }

        // plist 不存在，直接用 CLI 启动
        if !std::path::Path::new(&path).exists() {
            return start_gateway_direct();
        }

        let bootstrap_out = Command::new("launchctl")
            .args(["bootstrap", &domain_target, &path])
            .output()
            .map_err(|e| format!("重启 bootstrap 失败: {e}"))?;

        if !bootstrap_out.status.success() {
            let stderr = String::from_utf8_lossy(&bootstrap_out.stderr);
            if !stderr.contains("already bootstrapped") && !stderr.trim().is_empty() {
                // launchctl 失败，回退到直接启动
                return start_gateway_direct();
            }
        }

        let kickstart_out = Command::new("launchctl")
            .args(["kickstart", "-k", &service_target])
            .output()
            .map_err(|e| format!("重启 kickstart 失败: {e}"))?;

        if !kickstart_out.status.success() {
            let stderr = String::from_utf8_lossy(&kickstart_out.stderr);
            if !stderr.trim().is_empty() {
                // kickstart 也失败，回退到直接启动
                return start_gateway_direct();
            }
        }

        Ok(())
    }
}
