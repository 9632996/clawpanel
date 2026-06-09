// ===== Windows 实现 =====

#[cfg(target_os = "windows")]
pub(super) mod platform {
    use std::env;
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::Command as StdCommand;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use crate::commands::service::{guardian_log, read_gateway_error_log_excerpt};

    /// 缓存 is_cli_installed 结果，避免每 15 秒 polling 都 spawn cmd.exe
    static CLI_CACHE: Mutex<Option<(bool, std::time::Instant)>> = Mutex::new(None);
    const CLI_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(60);
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    /// 记录最后一次成功启动的 Gateway PID，避免误判旧进程为新进程
    static LAST_KNOWN_GATEWAY_PID: Mutex<Option<u32>> = Mutex::new(None);

    /// 记录当前活跃的 Gateway 子进程（用于 stop 时精确 kill）
    static ACTIVE_GATEWAY_CHILD: Mutex<Option<u32>> = Mutex::new(None);
    static GATEWAY_START_IN_PROGRESS: Mutex<bool> = Mutex::new(false);

    // 检查 Gateway /health 是否已经可用。
    /// TCP 端口监听不代表 Gateway 完成初始化；启动完成判定必须等 HTTP health。
    fn is_gateway_health_responsive(port: u16, timeout: Duration) -> bool {
        use std::io::{Read, Write as IoWrite};
        use std::net::TcpStream;
        let Ok(addr) = format!("127.0.0.1:{port}").parse() else {
            return false;
        };
        let mut stream = match TcpStream::connect_timeout(&addr, timeout) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let _ = stream.set_read_timeout(Some(timeout));
        let _ = stream.set_write_timeout(Some(timeout));
        let req = format!("GET /health HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
        if stream.write_all(req.as_bytes()).is_err() {
            return false;
        }
        let mut buf = [0u8; 512];
        match stream.read(&mut buf) {
            Ok(n) if n > 0 => {
                let resp = String::from_utf8_lossy(&buf[..n]);
                resp.starts_with("HTTP/1.1 200") || resp.starts_with("HTTP/1.0 200") || resp.contains("\"ok\":true")
            }
            _ => false,
        }
    }

    // 带重试的 /health 健康检查：issue #244 的关键修复
    //
    // 原 cleanup 只做 1 次 /health 判断，若 Gateway 刚启动仍在做初始化（加载插件、
    // 连接数据库、等 network warm-up），一次请求就可能超时，被误判为僵尸并 kill —
    // 接着 start_service_impl 又会 Hidden-start 一个新实例，循环往复。
    //
    /// 改为 retries 次重试、每次间隔 interval 后才定性，给健康 Gateway 更宽容的启动窗口。
    fn is_gateway_port_responsive_with_retry(port: u16, retries: u32, interval: Duration) -> bool {
        for attempt in 0..retries {
            if attempt > 0 {
                std::thread::sleep(interval);
            }
            if is_gateway_health_responsive(port, Duration::from_secs(3)) {
                return true;
            }
        }
        false
    }

    fn load_portable_model_credentials(cmd: &mut StdCommand, openclaw_dir: &std::path::Path) {
        let credentials_path = openclaw_dir.join("model-credentials.env");
        let Ok(content) = fs::read_to_string(credentials_path) else {
            return;
        };
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let Some((key, raw_value)) = trimmed.split_once('=') else {
                continue;
            };
            let key = key.trim();
            if key.is_empty() || !key.chars().all(|ch| ch == '_' || ch.is_ascii_alphanumeric()) {
                continue;
            }
            let value = raw_value.trim().trim_matches('"').trim_matches('\'').to_string();
            cmd.env(key, value);
        }
    }

    /// 从 netstat 输出中提取监听指定端口的所有 PID
    fn find_listening_pids(port: u16) -> Vec<u32> {
        let output = match StdCommand::new("netstat")
            .args(["-ano", "-p", "TCP"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
            Err(_) => return vec![],
        };
        let mut pids = vec![];
        for line in output.lines() {
            let line = line.trim();
            if !line.contains(&format!(":{port}")) || !line.contains("LISTENING") {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 5 {
                continue;
            }
            if let Some(pid) = parts.last().and_then(|part| part.parse::<u32>().ok()) {
                if pid > 0 && !pids.contains(&pid) {
                    pids.push(pid);
                }
            }
        }
        pids
    }

    // 清理残留的僵尸 Gateway 进程（启动时调用，防止 Windows 重启后多进程堆积）
    //
    // issue #244 修复：原实现只做 1 次 /health 检测，Gateway 刚 ready 仍在跑
    // startup hooks / channel connect 时，单次探测可能超时 → 被误杀 → 触发 Hidden-start
    /// 又起一个新的，循环往复。改为 3 次重试（间隔 800ms）才算"真僵尸"。
    pub(crate) fn cleanup_zombie_gateway_processes() {
        let port = crate::commands::gateway_listen_port();
        let pids = find_listening_pids(port);
        if pids.is_empty() {
            return;
        }

        // 带重试的 /health 检测 —— 最多等 3 * 800ms = 2.4s 才判定僵尸
        let responsive = is_gateway_port_responsive_with_retry(port, 3, std::time::Duration::from_millis(800));

        for pid in &pids {
            let pid = *pid;

            if let Some(cmdline) = read_process_command_line(pid) {
                let cmdline_lower = cmdline.to_lowercase();
                let is_gateway = cmdline_lower.contains("openclaw") && cmdline_lower.contains("gateway");
                let our_pid = *LAST_KNOWN_GATEWAY_PID.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

                if is_gateway {
                    if !responsive {
                        // 3 次 /health 全部失败 → 僵尸进程，强制终止
                        guardian_log(&format!(
                            "检测到僵尸 Gateway 进程 (PID {pid})：端口 {port} 占用但 /health 连续 3 次无响应，强制终止"
                        ));
                        kill_process_tree(pid);
                    } else if Some(pid) != our_pid {
                        // /health 有响应但不是当前实例启动的 → 采纳为已知进程，不杀
                        guardian_log(&format!("检测到健康的 Gateway 进程 (PID {pid})：/health 正常响应，已采纳"));
                        let mut known = LAST_KNOWN_GATEWAY_PID.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                        *known = Some(pid);
                    }
                    // is_gateway + responsive + 本就是我们的 PID → 无需任何操作
                }
            }
            // 读不到命令行时，不做假设，避免误杀其他进程
        }
    }

    fn read_process_command_line(pid: u32) -> Option<String> {
        // 优先用 PowerShell Get-CimInstance（wmic 在 Win11 已弃用）
        // fallback 到 wmic 以兼容旧版 Windows
        let ps_output = StdCommand::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!("(Get-CimInstance Win32_Process -Filter 'ProcessId={}').CommandLine", pid),
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        if let Ok(o) = ps_output {
            let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if !text.is_empty() {
                return Some(text);
            }
        }
        // fallback: wmic（兼容 Win10 及更早版本）
        let output = match StdCommand::new("wmic")
            .args([
                "process",
                "where",
                &format!("ProcessId={pid}"),
                "get",
                "CommandLine",
                "/format:list",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
            Err(_) => return None,
        };
        for line in output.lines() {
            let line = line.trim();
            if let Some(cmd) = line.strip_prefix("CommandLine=") {
                return Some(cmd.to_string());
            }
        }
        None
    }

    fn kill_process_tree(pid: u32) {
        // 先尝试 /ti（包含子进程）
        let _ = StdCommand::new("taskkill")
            .args(["/f", "/t", "/pid", &pid.to_string()])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }

    /// 获取 Gateway 端口对应的真实 PID（仅返回 OpenClaw Gateway 的 PID）
    fn get_gateway_pid_by_port(port: u16) -> Option<u32> {
        let output = match StdCommand::new("netstat")
            .args(["-ano", "-p", "TCP"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
            Err(_) => return None,
        };

        for line in output.lines() {
            let line = line.trim();
            if !line.contains(&format!(":{port}")) || !line.contains("LISTENING") {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 5 {
                continue;
            }
            let Some(pid) = parts.last().and_then(|part| part.parse::<u32>().ok()) else {
                continue;
            };

            // 验证命令行
            if let Some(cmdline) = read_process_command_line(pid) {
                let cmdline_lower = cmdline.to_lowercase();
                if cmdline_lower.contains("openclaw") && cmdline_lower.contains("gateway") {
                    return Some(pid);
                }
            } else {
                // 读不到命令行时，不做假设，避免误杀其他进程
                continue;
            }
        }
        None
    }

    /// 验证指定 PID 是否还活着
    fn is_process_alive(pid: u32) -> bool {
        let output = StdCommand::new("tasklist")
            .args(["/fi", &format!("PID eq {pid}"), "/nh"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                // tasklist /nh 输出格式: "node.exe  1234 Console  1  50,000 K"
                // 行首是进程名，PID 在中间，需要检查行中是否包含该 PID
                for line in stdout.lines() {
                    let trimmed = line.trim();
                    // 跳过空行和 "INFO: No tasks" 之类的提示
                    if trimmed.is_empty() || trimmed.starts_with("INFO:") {
                        continue;
                    }
                    // 检查行中是否包含该 PID（作为独立的数字字段）
                    let fields: Vec<&str> = trimmed.split_whitespace().collect();
                    if fields.len() >= 2 {
                        if let Ok(line_pid) = fields[1].parse::<u32>() {
                            if line_pid == pid {
                                return true;
                            }
                        }
                    }
                }
                false
            }
            Err(_) => false,
        }
    }

    /// Windows 不需要 UID
    pub fn current_uid() -> Result<u32, String> {
        Ok(0)
    }

    /// 检测 openclaw CLI 是否已安装（带 60s 缓存，避免频繁 spawn 进程）
    pub fn is_cli_installed() -> bool {
        // 检查缓存
        if let Ok(guard) = CLI_CACHE.lock() {
            if let Some((val, ts)) = *guard {
                if ts.elapsed() < CLI_CACHE_TTL {
                    return val;
                }
            }
        }
        let result = check_cli_installed_inner();
        if let Ok(mut guard) = CLI_CACHE.lock() {
            *guard = Some((result, std::time::Instant::now()));
        }
        result
    }

    pub fn invalidate_cli_cache() {
        if let Ok(mut guard) = CLI_CACHE.lock() {
            *guard = None;
        }
    }

    fn candidate_cli_paths() -> Vec<PathBuf> {
        let mut candidates = Vec::new();

        // standalone 安装目录（集中管理，避免多处硬编码）
        for sa_dir in crate::commands::config::all_standalone_dirs() {
            candidates.push(sa_dir.join("openclaw.cmd"));
        }

        if let Ok(appdata) = env::var("APPDATA") {
            candidates.push(Path::new(&appdata).join("npm").join("openclaw.cmd"));
        }
        if let Ok(localappdata) = env::var("LOCALAPPDATA") {
            candidates.push(
                Path::new(&localappdata)
                    .join("Programs")
                    .join("nodejs")
                    .join("node_modules")
                    .join(format!("@{}cloud", "qingchen"))
                    .join(format!("openclaw-{}", "zh"))
                    .join("bin")
                    .join("openclaw.js"),
            );
        }

        for segment in crate::commands::enhanced_path().split(';') {
            let dir = segment.trim();
            if dir.is_empty() {
                continue;
            }
            let base = Path::new(dir);
            candidates.push(base.join("openclaw.cmd"));
            candidates.push(base.join("openclaw"));
            candidates.push(
                base.join("node_modules")
                    .join(format!("@{}cloud", "qingchen"))
                    .join(format!("openclaw-{}", "zh"))
                    .join("bin")
                    .join("openclaw.js"),
            );
        }

        candidates
    }

    fn check_cli_installed_inner() -> bool {
        if let Some(path) = crate::utils::resolve_openclaw_cli_path() {
            if Path::new(&path).exists() {
                return true;
            }
        }

        // 方式1: 检查常见文件路径（零进程，最快）
        for path in candidate_cli_paths() {
            if path.exists() {
                return true;
            }
        }

        // 方式2: 通过 where 查找（兼容 nvm、自定义 prefix 等）
        // 过滤掉第三方 openclaw（如 CherryStudio 的 .cherrystudio/bin/openclaw.exe）
        let mut where_cmd = std::process::Command::new("where");
        where_cmd.arg("openclaw");
        where_cmd.env("PATH", crate::commands::enhanced_path());
        where_cmd.creation_flags(CREATE_NO_WINDOW);
        if let Ok(o) = where_cmd.output() {
            if o.status.success() {
                let stdout = String::from_utf8_lossy(&o.stdout);
                for line in stdout.lines() {
                    let p = line.trim().to_lowercase();
                    // 跳过已知第三方 openclaw 路径
                    if p.contains(".cherrystudio") || p.contains("cherry-studio") {
                        continue;
                    }
                    if !p.is_empty() {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Windows 上始终返回 Gateway 标签（不管 CLI 是否安装）
    pub fn scan_service_labels() -> Vec<String> {
        vec!["ai.openclaw.gateway".to_string()]
    }

    // 检测 Gateway 是否在运行，并返回其 PID
    /// 策略：先 TCP 端口检测连通性，再用 netstat+PowerShell 验证命令行是 OpenClaw Gateway
    pub fn check_service_status(_uid: u32, _label: &str) -> (bool, Option<u32>) {
        let port = crate::commands::gateway_listen_port();
        let addr = format!("127.0.0.1:{port}");
        let socket_addr = match addr.parse() {
            Ok(a) => a,
            Err(_) => return (false, None),
        };
        // localhost 状态探测需要快速失败，否则仪表盘会把"已停止"误判成"状态加载失败"。
        // 这里仅做 TCP connect，不依赖应用层响应；Gateway 真在监听时本地握手会非常快。
        let connected = std::net::TcpStream::connect_timeout(&socket_addr, Duration::from_millis(250)).is_ok() || {
            std::thread::sleep(Duration::from_millis(120));
            std::net::TcpStream::connect_timeout(&socket_addr, Duration::from_millis(450)).is_ok()
        };
        if !connected {
            // 端口不通，先清空已知的僵死 PID
            let mut known = LAST_KNOWN_GATEWAY_PID.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            *known = None;
            return (false, None);
        }

        // 端口通了，PID 识别仅作为增强信息
        if let Some(pid) = get_gateway_pid_by_port(port) {
            let mut known = LAST_KNOWN_GATEWAY_PID.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            *known = Some(pid);
            (true, Some(pid))
        } else {
            // 避免因命令行查询失败误判为“未运行”并触发重复拉起
            (true, None)
        }
    }

    fn cleanup_legacy_gateway_window() {
        let _ = std::process::Command::new("taskkill")
            .args(["/f", "/t", "/fi", &format!("WINDOWTITLE eq {GATEWAY_WINDOW_TITLE}")])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }

    #[allow(dead_code)]
    fn create_gateway_log_files() -> Result<(std::fs::File, std::fs::File), String> {
        let log_dir = crate::commands::openclaw_dir().join("logs");
        fs::create_dir_all(&log_dir).map_err(|e| format!("创建日志目录失败: {e}"))?;

        let mut stdout_log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join("gateway.log"))
            .map_err(|e| format!("创建日志文件失败: {e}"))?;

        let stderr_log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join("gateway.err.log"))
            .map_err(|e| format!("创建错误日志文件失败: {e}"))?;

        let _ = writeln!(
            stdout_log,
            "\n[{}] [Workbench] Hidden-start Gateway on Windows",
            chrono::Local::now().to_rfc3339()
        );

        Ok((stdout_log, stderr_log))
    }

    const GATEWAY_WINDOW_TITLE: &str = "OpenClaw Gateway";

    // 在 Windows 上打开一个可见终端启动 Gateway
    //
    // 关键：必须通过 `cmd.exe` 内置的 `start` 命令拉起新控制台。
    // 直接 `StdCommand::new("cmd").creation_flags(CREATE_NEW_CONSOLE)` 在
    // Rust 默认 `Stdio::inherit` + `STARTF_USESTDHANDLES` 影响下，CREATE_NEW_CONSOLE
    // 会被吞掉（子进程能跑起来但 MainWindowHandle=0、无可见窗口）。
    // 通过外层 `cmd /c start "<title>" cmd /K runner.cmd` 让 `start` 用全新的
    /// `CreateProcess` 拉起子进程，stdio 不继承、控制台真正分离，稳定弹出可见窗口。
    pub async fn start_service_impl(_label: &str) -> Result<(), String> {
        if !is_cli_installed() {
            return Err("openclaw CLI 未安装，请先确认便携运行时存在，或执行 npm install -g openclaw 安装".into());
        }
        {
            let mut starting = GATEWAY_START_IN_PROGRESS
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if *starting {
                return Err("Gateway 正在启动中，请稍候再试".into());
            }
            *starting = true;
        }
        let result = start_service_impl_inner().await;
        *GATEWAY_START_IN_PROGRESS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = false;
        result
    }

    async fn start_service_impl_inner() -> Result<(), String> {
        if !is_cli_installed() {
            return Err("openclaw CLI 未安装，请先确认便携运行时存在，或执行 npm install -g openclaw 安装".into());
        }

        let (running, pid) = check_service_status(0, "");
        if running {
            if pid.is_some() {
                let port = crate::commands::gateway_listen_port();
                if is_gateway_health_responsive(port, Duration::from_secs(2)) {
                    return Ok(());
                }
                return Err(format!(
                    "端口 {port} 已被 OpenClaw Gateway 占用，但 /health 暂不可用，请稍候或重启 Gateway"
                ));
            }
            return Err(format!(
                "端口 {} 被未知进程占用，请先关闭占用该端口的程序",
                crate::commands::gateway_listen_port()
            ));
        }

        cleanup_zombie_gateway_processes();

        let before_pid = *LAST_KNOWN_GATEWAY_PID.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let cli = crate::utils::resolve_openclaw_cli_path().unwrap_or_else(|| "openclaw".into());
        let openclaw_dir = crate::commands::openclaw_dir();
        let config_path = openclaw_dir.join("openclaw.json");
        let portable_home_dir = openclaw_dir
            .parent()
            .filter(|parent| parent.join("config").is_dir())
            .map(|parent| parent.to_path_buf())
            .unwrap_or_else(|| openclaw_dir.clone());
        let log_dir = portable_home_dir.join("logs");
        let state_dir = openclaw_dir
            .parent()
            .filter(|parent| parent.join("state").is_dir())
            .map(|parent| parent.join("state"))
            .unwrap_or_else(|| openclaw_dir.clone());
        let cache_dir = openclaw_dir
            .parent()
            .map(|parent| parent.join("cache"))
            .unwrap_or_else(|| openclaw_dir.join("cache"));
        let workspace_dir = openclaw_dir
            .parent()
            .map(|parent| parent.join("workspace").join("main"))
            .unwrap_or_else(|| openclaw_dir.join("workspace").join("main"));
        let temp_dir = cache_dir.join("temp");
        fs::create_dir_all(&log_dir).map_err(|e| format!("创建日志目录失败: {e}"))?;
        fs::create_dir_all(&state_dir).map_err(|e| format!("创建状态目录失败: {e}"))?;
        fs::create_dir_all(&cache_dir).map_err(|e| format!("创建缓存目录失败: {e}"))?;
        fs::create_dir_all(&workspace_dir).map_err(|e| format!("创建工作区目录失败: {e}"))?;
        fs::create_dir_all(&temp_dir).map_err(|e| format!("创建临时目录失败: {e}"))?;
        let stdout_log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join("gateway.log"))
            .map_err(|e| format!("创建 Gateway 日志失败: {e}"))?;
        let stderr_log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join("gateway.err.log"))
            .map_err(|e| format!("创建 Gateway 错误日志失败: {e}"))?;
        let port_arg = crate::commands::gateway_listen_port().to_string();
        let cli_lower = cli.to_ascii_lowercase();
        let mut cmd = if cli_lower.ends_with(".js") {
            let mut command = StdCommand::new("node");
            command.arg(&cli);
            command
        } else if cli_lower.ends_with(".cmd") || cli_lower.ends_with(".bat") {
            let mut command = StdCommand::new("cmd");
            command.args(["/D", "/C"]).arg(&cli);
            command
        } else {
            StdCommand::new(&cli)
        };
        cmd.args(["gateway", "run", "--port"])
            .arg(&port_arg)
            .creation_flags(CREATE_NO_WINDOW)
            .env("PATH", crate::commands::enhanced_path())
            .env("OPENCLAW_HOME", &portable_home_dir)
            .env("OPENCLAW_STATE_DIR", &state_dir)
            .env("OPENCLAW_CONFIG_PATH", &config_path)
            .env("OPENCLAW_CACHE_DIR", &cache_dir)
            .env("OPENCLAW_LOG_DIR", &log_dir)
            .env("OPENCLAW_WORKSPACE_DIR", &workspace_dir)
            .env("OPENCLAW_PORTABLE", "1")
            .env("NODE_COMPILE_CACHE", cache_dir.join("node-compile-cache"))
            .env("TEMP", &temp_dir)
            .env("TMP", &temp_dir)
            .env("OPENCLAW_WRAPPER_LOG_REDIRECT", "0")
            .current_dir(&openclaw_dir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::from(stdout_log))
            .stderr(std::process::Stdio::from(stderr_log));
        load_portable_model_credentials(&mut cmd, &openclaw_dir);
        crate::commands::apply_proxy_env(&mut cmd);

        let child = cmd.spawn().map_err(|e| format!("启动 Gateway 失败: {e}"))?;
        {
            let mut active = ACTIVE_GATEWAY_CHILD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            *active = Some(child.id());
        }

        // 轮询等待：真实 Gateway PID 出现，且 /health 已响应。
        // OpenClaw 会先监听端口再加载 channels/sidecars，只看 TCP 会过早判定成功。
        let deadline = Instant::now() + Duration::from_secs(150);
        let port = crate::commands::gateway_listen_port();
        while Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let (running2, pid2) = check_service_status(0, "");

            if let (true, Some(current_pid)) = (running2, pid2) {
                let is_new = Some(current_pid) != before_pid;
                if is_new && is_process_alive(current_pid) && is_gateway_health_responsive(port, Duration::from_secs(2)) {
                    // 记录真实 Gateway PID 供 stop 时精确 kill
                    let mut active = ACTIVE_GATEWAY_CHILD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    *active = Some(current_pid);
                    return Ok(());
                }
            }
        }

        let excerpt = read_gateway_error_log_excerpt(4096);
        if excerpt.trim().is_empty() {
            Err("Gateway 启动超时，请查看 gateway.err.log".into())
        } else {
            Err(format!("Gateway 启动超时，请查看 gateway.err.log\n{excerpt}"))
        }
    }

    // 关闭 Gateway：精确 kill Gateway 进程，不误杀其他 node.exe
    include!("service_platform_windows_modules/stop_restart.rs");
}
