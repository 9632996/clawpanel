fn configured_git_path() -> Option<String> {
    super::read_panel_config_value()
        .and_then(|v| v.get("gitPath")?.as_str().map(String::from))
        .map(|custom| custom.trim().to_string())
        .filter(|custom| !custom.is_empty())
}

/// 获取用户配置的 git 可执行文件路径，回退到 "git"
pub fn git_executable() -> String {
    configured_git_path().unwrap_or_else(|| "git".into())
}

fn configure_git_https_rules() -> usize {
    let git = git_executable();
    // Collect unique target prefixes to unset old rules
    let targets: std::collections::HashSet<&str> = GIT_HTTPS_REWRITES.iter().map(|(t, _)| *t).collect();
    for target in &targets {
        let key = format!("url.{target}.insteadOf");
        let mut unset = Command::new(&git);
        unset.args(["config", "--global", "--unset-all", &key]);
        #[cfg(target_os = "windows")]
        unset.creation_flags(0x08000000);
        let _ = unset.output();
    }

    let mut success = 0;
    for (target, from) in GIT_HTTPS_REWRITES {
        let key = format!("url.{target}.insteadOf");
        let mut cmd = Command::new(&git);
        cmd.args(["config", "--global", "--add", &key, from]);
        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000);
        if cmd.output().map(|o| o.status.success()).unwrap_or(false) {
            success += 1;
        }
    }
    success
}

fn apply_git_install_env(cmd: &mut Command) {
    if let Some(custom_git) = configured_git_path() {
        let git_path = PathBuf::from(&custom_git);
        if let Some(parent) = git_path.parent() {
            let mut paths: Vec<PathBuf> = std::env::var_os("PATH")
                .map(|value| std::env::split_paths(&value).collect())
                .unwrap_or_default();
            if !paths.iter().any(|p| p == parent) {
                paths.insert(0, parent.to_path_buf());
            }
            if let Ok(joined) = std::env::join_paths(paths) {
                cmd.env("PATH", joined);
            }
        }
        cmd.env("GIT", &custom_git);
    }
    crate::commands::apply_proxy_env(cmd);
    cmd.env("GIT_TERMINAL_PROMPT", "0")
        .env(
            "GIT_SSH_COMMAND",
            "ssh -o BatchMode=yes -o StrictHostKeyChecking=no -o IdentitiesOnly=yes",
        )
        .env("GIT_ALLOW_PROTOCOL", "https:http:file");
    cmd.env("GIT_CONFIG_COUNT", GIT_HTTPS_REWRITES.len().to_string());
    for (idx, (target, from)) in GIT_HTTPS_REWRITES.iter().enumerate() {
        cmd.env(format!("GIT_CONFIG_KEY_{idx}"), format!("url.{target}.insteadOf"))
            .env(format!("GIT_CONFIG_VALUE_{idx}"), *from);
    }
}

/// Linux: 检测是否以 root 身份运行（避免 unsafe libc 调用）
#[cfg(target_os = "linux")]
fn nix_is_root() -> bool {
    std::env::var("USER")
        .or_else(|_| std::env::var("EUID"))
        .map(|v| v == "root" || v == "0")
        .unwrap_or(false)
}

/// 读取用户配置的 npm registry，fallback 到淘宝镜像
fn get_configured_registry() -> String {
    let path = super::openclaw_dir().join("npm-registry.txt");
    fs::read_to_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_REGISTRY.to_string())
}

/// 创建使用配置源的 npm Command（不带提权，用于 npm list 等只读操作）
/// Windows 上 npm 是 npm.cmd，需要通过 cmd /c 调用，并隐藏窗口
fn npm_command() -> Command {
    let registry = get_configured_registry();
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut cmd = Command::new("cmd");
        cmd.args(["/c", "npm", "--registry", &registry]);
        cmd.env("PATH", super::enhanced_path());
        crate::commands::apply_proxy_env(&mut cmd);
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = Command::new("npm");
        cmd.args(["--registry", &registry]);
        cmd.env("PATH", super::enhanced_path());
        crate::commands::apply_proxy_env(&mut cmd);
        cmd
    }
}

/// Linux: 检测 npm 全局目录是否在用户 home 下（nvm/fnm/volta 等不需要提权）
#[cfg(target_os = "linux")]
fn npm_prefix_is_user_writable() -> bool {
    if nix_is_root() {
        return true;
    }
    let home = std::env::var("HOME").unwrap_or_default();
    if home.is_empty() {
        return false;
    }
    if let Ok(o) = Command::new("npm")
        .args(["config", "get", "prefix"])
        .env("PATH", super::enhanced_path())
        .output()
    {
        if o.status.success() {
            let prefix = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if !prefix.is_empty() && prefix.starts_with(&home) {
                return true;
            }
        }
    }
    false
}

/// Linux: 收集需要透传给提权子进程的环境变量
#[cfg(target_os = "linux")]
fn collect_elevated_env_args() -> Vec<String> {
    let mut env_args = vec![format!("PATH={}", super::enhanced_path())];
    if let Ok(home) = std::env::var("HOME") {
        env_args.push(format!("HOME={home}"));
    }
    if let Some(proxy) = crate::commands::configured_proxy_url() {
        env_args.push(format!("HTTP_PROXY={proxy}"));
        env_args.push(format!("HTTPS_PROXY={proxy}"));
        env_args.push(format!("http_proxy={proxy}"));
        env_args.push(format!("https_proxy={proxy}"));
        env_args.push("NO_PROXY=localhost,127.0.0.1,::1".to_string());
        env_args.push("no_proxy=localhost,127.0.0.1,::1".to_string());
    }
    env_args
}

/// 创建需要全局写入权限的 npm Command（用于 install -g / uninstall -g）
/// Linux 非 root 用户：先检测 npm prefix 是否在用户 home 下（nvm/fnm/volta），
/// 不需要提权则直接调用；否则优先使用 pkexec（图形密码对话框），
/// 降级到 sudo（不再使用 -E，改用 env 显式传递变量）。
fn npm_command_elevated() -> Command {
    #[cfg(not(target_os = "linux"))]
    {
        npm_command()
    }
    #[cfg(target_os = "linux")]
    {
        if nix_is_root() || npm_prefix_is_user_writable() {
            return npm_command();
        }
        let registry = get_configured_registry();
        let env_args = collect_elevated_env_args();
        // 优先 pkexec：图形密码对话框，适合桌面 GUI 应用
        let has_pkexec = Command::new("which")
            .arg("pkexec")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        let mut cmd = if has_pkexec {
            let mut c = Command::new("pkexec");
            c.arg("/usr/bin/env");
            for ea in &env_args {
                c.arg(ea);
            }
            c.args(["npm", "--registry", &registry]);
            c
        } else {
            // 降级到 sudo：不再用 -E（sudo-rs 不支持），通过 env 显式传递
            let mut c = Command::new("sudo");
            c.arg("--non-interactive");
            c.arg("/usr/bin/env");
            for ea in &env_args {
                c.arg(ea);
            }
            c.args(["npm", "--registry", &registry]);
            c
        };
        cmd.env("PATH", super::enhanced_path());
        crate::commands::apply_proxy_env(&mut cmd);
        cmd
    }
}

/// 安装/升级前的清理工作：停止 Gateway、清理 npm 全局 bin 下的 openclaw 残留文件
/// 解决 Windows 上 EEXIST（文件已存在）和文件被占用的问题
fn pre_install_cleanup() {
    /// 带超时执行命令（spawn + try_wait），防止任何子进程无限阻塞
    fn run_with_timeout(mut child: std::process::Child, timeout_secs: u64) -> Option<std::process::Output> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let stdout = child
                        .stdout
                        .take()
                        .map(|mut s| {
                            let mut buf = Vec::new();
                            let _ = std::io::Read::read_to_end(&mut s, &mut buf);
                            buf
                        })
                        .unwrap_or_default();
                    return Some(std::process::Output {
                        status,
                        stdout,
                        stderr: Vec::new(),
                    });
                }
                Ok(None) => {
                    if std::time::Instant::now() >= deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        return None;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
                Err(_) => return None,
            }
        }
    }

    // 1. 先通过 CLI 正常停止 Gateway（10s 超时）
    if let Ok(child) = openclaw_command()
        .args(["gateway", "stop"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        run_with_timeout(child, 10);
    }

    // 2. 停止 Gateway 进程，释放 openclaw 相关文件锁
    #[cfg(target_os = "windows")]
    {
        // 杀死所有运行 openclaw gateway 的 node.exe 进程（通过命令行匹配）
        // 使用 PowerShell Get-CimInstance（兼容 Windows 11，wmic 已废弃）（10s 超时）
        if let Ok(child) = Command::new("powershell")
            .args(["-NoProfile", "-Command",
                "Get-CimInstance Win32_Process -Filter \"CommandLine like '%openclaw%gateway%'\" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty ProcessId"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            if let Some(output) = run_with_timeout(child, 10) {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    if let Ok(_pid) = line.trim().parse::<u32>() {
                        let _ = Command::new("taskkill").args(["/F", "/PID", line.trim()]).output();
                    }
                }
            }
        }

        // 同时杀死 standalone 目录下的 node.exe 进程（每个目录 10s 超时）
        for sa_dir in all_standalone_dirs() {
            if sa_dir.exists() {
                let dir_lower = sa_dir.to_string_lossy().to_lowercase().replace('\\', "\\\\");
                let ps_script = format!(
                    "Get-Process -Name node -ErrorAction SilentlyContinue | Where-Object {{ $_.Path -and $_.Path.ToLower().Contains('{}') }} | Select-Object -ExpandProperty Id",
                    dir_lower
                );
                if let Ok(child) = Command::new("powershell")
                    .args(["-NoProfile", "-Command", &ps_script])
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                {
                    if let Some(output) = run_with_timeout(child, 10) {
                        let text = String::from_utf8_lossy(&output.stdout);
                        for line in text.lines() {
                            if let Ok(_pid) = line.trim().parse::<u32>() {
                                let _ = Command::new("taskkill").args(["/F", "/PID", line.trim()]).output();
                            }
                        }
                    }
                }
            }
        }

        // 等文件锁释放（Node.js 进程退出需要时间）
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    #[cfg(target_os = "macos")]
    {
        let uid = get_uid().unwrap_or(501);
        if let Ok(child) = Command::new("launchctl")
            .args(["bootout", &format!("gui/{uid}/ai.openclaw.gateway")])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            run_with_timeout(child, 10);
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(child) = Command::new("pkill")
            .args(["-f", "openclaw.*gateway"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            run_with_timeout(child, 10);
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    // 3. 清理 npm 全局 bin 目录下的 openclaw 残留文件（Windows EEXIST 根因）
    #[cfg(target_os = "windows")]
    {
        if let Some(npm_bin) = npm_global_bin_dir() {
            for name in &["openclaw", "openclaw.cmd", "openclaw.ps1"] {
                let p = npm_bin.join(name);
                if p.exists() {
                    let _ = fs::remove_file(&p);
                }
            }
        }
    }
}