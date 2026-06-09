
#[tauri::command]
pub async fn reload_gateway(app: tauri::AppHandle) -> Result<String, String> {
    restart_gateway_guarded(Some(&app), false).await
}

/// 重启 Gateway 服务
#[tauri::command]
pub async fn restart_gateway(app: tauri::AppHandle) -> Result<String, String> {
    restart_gateway_guarded(Some(&app), true).await
}

/// 运行 openclaw doctor --fix 自动修复配置问题
#[tauri::command]
pub async fn doctor_fix() -> Result<Value, String> {
    use crate::utils::openclaw_command_async;

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        openclaw_command_async().args(["doctor", "--fix"]).output(),
    )
    .await;

    match result {
        Ok(Ok(o)) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            let success = o.status.success();
            Ok(crate::jv!({
                "success": success,
                "output": stdout.trim(),
                "errors": stderr.trim(),
                "exitCode": o.status.code(),
            }))
        }
        Ok(Err(e)) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err("OpenClaw CLI 未找到，请先安装".to_string())
            } else {
                Err(format!("执行 doctor 失败: {e}"))
            }
        }
        Err(_) => Err("doctor --fix 执行超时 (30s)".to_string()),
    }
}

/// 运行 openclaw doctor（仅诊断，不修复）
#[tauri::command]
pub async fn doctor_check() -> Result<Value, String> {
    use crate::utils::openclaw_command_async;

    let result =
        tokio::time::timeout(std::time::Duration::from_secs(20), openclaw_command_async().args(["doctor"]).output()).await;

    match result {
        Ok(Ok(o)) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            Ok(crate::jv!({
                "success": o.status.success(),
                "output": stdout.trim(),
                "errors": stderr.trim(),
            }))
        }
        Ok(Err(e)) => Err(format!("执行 doctor 失败: {e}")),
        Err(_) => Err("doctor 执行超时 (20s)".to_string()),
    }
}

/// 安装 Gateway 服务（执行 openclaw gateway install）
#[tauri::command]
pub async fn install_gateway() -> Result<String, String> {
    use crate::utils::openclaw_command_async;
    let _guardian_pause = GuardianPause::new("install gateway");
    // 先检测 openclaw CLI 是否可用
    let cli_check = openclaw_command_async().arg("--version").output().await;
    match cli_check {
        Ok(o) if o.status.success() => {}
        _ => {
            return Err("openclaw CLI 未安装。请先执行以下命令安装：\n\n\
                 npm install -g openclaw\n\n\
                 安装完成后再点击此按钮安装 Gateway 服务。"
                .into());
        }
    }

    let output = openclaw_command_async()
        .args(["gateway", "install"])
        .output()
        .await
        .map_err(|e| format!("安装失败: {e}"))?;

    if output.status.success() {
        Ok("Gateway 服务已安装".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("安装失败: {stderr}"))
    }
}

/// 卸载 Gateway 服务
/// macOS: launchctl bootout + 删除 plist
/// Windows: 直接 taskkill
/// Linux: pkill
#[tauri::command]
pub fn uninstall_gateway() -> Result<String, String> {
    let _guardian_pause = GuardianPause::new("uninstall gateway");
    crate::commands::service::guardian_mark_manual_stop();
    #[cfg(target_os = "macos")]
    {
        let uid = get_uid()?;
        let target = format!("gui/{uid}/ai.openclaw.gateway");

        // 先停止服务
        let _ = Command::new("launchctl").args(["bootout", &target]).output();

        // 删除 plist 文件
        let home = dirs::home_dir().unwrap_or_default();
        let plist = home.join("Library/LaunchAgents/ai.openclaw.gateway.plist");
        if plist.exists() {
            fs::remove_file(&plist).map_err(|e| format!("删除 plist 失败: {e}"))?;
        }
    }
    #[cfg(target_os = "windows")]
    {
        // 直接杀死 gateway 相关的 node.exe 进程，不走慢 CLI
        let _ = Command::new("taskkill")
            .args(["/f", "/im", "node.exe", "/fi", "WINDOWTITLE eq openclaw*"])
            .creation_flags(0x08000000)
            .output();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("pkill").args(["-f", "openclaw.*gateway"]).output();
    }
    Ok("Gateway 服务已卸载".to_string())
}

/// 为 openclaw.json 中所有模型添加 input: ["text", "image"]，使 Gateway 识别模型支持图片输入
#[tauri::command]
pub fn patch_model_vision() -> Result<bool, String> {
    let path = super::openclaw_dir().join("openclaw.json");
    let content = fs::read_to_string(&path).map_err(|e| format!("读取配置失败: {e}"))?;
    let mut config: Value = serde_json::from_str(&content).map_err(|e| format!("解析 JSON 失败: {e}"))?;

    let vision_input = Value::Array(vec![Value::String("text".into()), Value::String("image".into())]);

    let mut changed = false;

    if let Some(obj) = config.as_object_mut() {
        if let Some(models_val) = obj.get_mut("models") {
            if let Some(models_obj) = models_val.as_object_mut() {
                if let Some(providers_val) = models_obj.get_mut("providers") {
                    if let Some(providers_obj) = providers_val.as_object_mut() {
                        for (_provider_name, provider_val) in providers_obj.iter_mut() {
                            if let Some(provider_obj) = provider_val.as_object_mut() {
                                if let Some(Value::Array(arr)) = provider_obj.get_mut("models") {
                                    for model in arr.iter_mut() {
                                        if let Some(mobj) = model.as_object_mut() {
                                            if !mobj.contains_key("input") {
                                                mobj.insert("input".into(), vision_input.clone());
                                                changed = true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if changed {
        let bak = super::openclaw_dir().join("openclaw.json.bak");
        let _ = fs::copy(&path, &bak);
        let json = serde_json::to_string_pretty(&config).map_err(|e| format!("序列化失败: {e}"))?;
        fs::write(&path, json).map_err(|e| format!("写入失败: {e}"))?;
    }

    Ok(changed)
}

/// 检查产品基座自身是否有新版本。
#[tauri::command]
pub async fn check_panel_update() -> Result<Value, String> {
    let client = crate::commands::build_http_client(std::time::Duration::from_secs(8), Some("Workbench"))
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let sources = [(
        super::zhizhua_url("/api/zhizhua-workbench/releases/latest"),
        super::zhizhua_url(""),
        "product",
    )];

    let mut last_err = String::new();
    for (api_url, releases_url, source) in &sources {
        match client.get(api_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let json: Value = resp.json().await.map_err(|e| format!("解析响应失败: {e}"))?;

                let tag = json
                    .get("tag_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim_start_matches('v')
                    .to_string();

                if tag.is_empty() {
                    last_err = format!("{source}: 未找到版本号");
                    continue;
                }

                let mut result = serde_json::Map::new();
                result.insert("latest".into(), Value::String(tag));
                result.insert("url".into(), json.get("html_url").cloned().unwrap_or(Value::String(releases_url.clone())));
                result.insert("source".into(), Value::String(source.to_string()));
                result.insert("downloadUrl".into(), Value::String(super::zhizhua_url("")));
                return Ok(Value::Object(result));
            }
            Ok(resp) => {
                last_err = format!("{source}: HTTP {}", resp.status());
            }
            Err(e) => {
                last_err = format!("{source}: {e}");
            }
        }
    }

    Err(last_err)
}

// === 面板配置 (clawpanel.json) ===

/// 获取当前生效的 OpenClaw 配置目录路径
#[tauri::command]
pub fn get_openclaw_dir() -> Result<Value, String> {
    let resolved = super::openclaw_dir();
    let is_custom = super::read_panel_config_value()
        .and_then(|v| v.get("openclawDir")?.as_str().map(String::from))
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    let config_exists = resolved.join("openclaw.json").exists();
    Ok(crate::jv!({
        "path": resolved.to_string_lossy(),
        "isCustom": is_custom,
        "configExists": config_exists,
    }))
}

#[tauri::command]
pub fn read_panel_config() -> Result<Value, String> {
    let path = super::panel_config_path();
    if !path.exists() {
        return Ok(crate::jv!({}));
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("读取失败: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("解析失败: {e}"))
}

#[tauri::command]
pub fn write_panel_config(config: Value) -> Result<(), String> {
    let path = super::panel_config_path();
    if let Some(dir) = path.parent() {
        if !dir.exists() {
            fs::create_dir_all(dir).map_err(|e| format!("创建目录失败: {e}"))?;
        }
    }
    let json = serde_json::to_string_pretty(&config).map_err(|e| format!("序列化失败: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("写入失败: {e}"))
}

/// 重启应用（用于设置变更后自动重启）
#[tauri::command]
pub async fn relaunch_app(app: tauri::AppHandle) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("获取可执行文件路径失败: {e}"))?;
    std::process::Command::new(&exe)
        .spawn()
        .map_err(|e| format!("重启失败: {e}"))?;
    // 短暂延迟后退出当前进程
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    app.exit(0);
    Ok(())
}

/// 测试代理连通性：通过配置的代理访问指定 URL，返回状态码和耗时
#[tauri::command]
pub async fn test_proxy(url: Option<String>) -> Result<Value, String> {
    let proxy_url = crate::commands::configured_proxy_url().ok_or("未配置代理地址，请先在面板设置中保存代理地址")?;

    let target = url.unwrap_or_else(|| "https://registry.npmjs.org/-/ping".to_string());

    let client = crate::commands::build_http_client(std::time::Duration::from_secs(10), Some("Workbench"))
        .map_err(|e| format!("创建代理客户端失败: {e}"))?;

    let start = std::time::Instant::now();
    let resp = client.get(&target).send().await.map_err(|e| {
        let elapsed = start.elapsed().as_millis();
        format!("代理连接失败 ({elapsed}ms): {e}")
    })?;

    let elapsed = start.elapsed().as_millis();
    let status = resp.status().as_u16();

    Ok(crate::jv!({
        "ok": status < 500,
        "status": status,
        "elapsed_ms": elapsed,
        "proxy": proxy_url,
        "target": target,
    }))
}

#[tauri::command]
pub fn get_npm_registry() -> Result<String, String> {
    Ok(get_configured_registry())
}

#[tauri::command]
pub fn set_npm_registry(registry: String) -> Result<(), String> {
    let path = super::openclaw_dir().join("npm-registry.txt");
    fs::write(&path, registry.trim()).map_err(|e| format!("保存失败: {e}"))
}

/// 检测 Git 是否已安装
#[tauri::command]
pub fn check_git() -> Result<Value, String> {
    let mut result = serde_json::Map::new();
    let configured = configured_git_path();
    let git = configured.clone().unwrap_or_else(|| "git".into());
    let is_custom = configured.is_some();
    let git_path = if is_custom { Some(git.clone()) } else { find_git_path() };
    // #Compat-4: 优先用 find_git_path 拿到的绝对路径执行 --version（避免依赖子进程 PATH），
    // 回退到 "git" 时也把 enhanced_path 注入子进程 PATH，让刚装完 git 的场景立即可识别。
    let exec = git_path.as_deref().unwrap_or(&git);
    let mut cmd = Command::new(exec);
    cmd.arg("--version");
    cmd.env("PATH", super::enhanced_path());
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);
    match cmd.output() {
        Ok(o) if o.status.success() => {
            let ver = String::from_utf8_lossy(&o.stdout).trim().to_string();
            result.insert("installed".into(), Value::Bool(true));
            result.insert("version".into(), Value::String(ver));
            result.insert("path".into(), git_path.map(Value::String).unwrap_or(Value::Null));
            result.insert("isCustom".into(), Value::Bool(is_custom));
        }
        _ => {
            result.insert("installed".into(), Value::Bool(false));
            result.insert("version".into(), Value::Null);
            result.insert("path".into(), Value::Null);
            result.insert("isCustom".into(), Value::Bool(is_custom));
        }
    }
    Ok(Value::Object(result))
}

/// 扫描常见路径，返回所有找到的 Git 安装
#[tauri::command]
pub fn scan_git_paths() -> Result<Value, String> {
    let mut found: Vec<Value> = vec![];
    let mut candidates: Vec<(String, String)> = vec![]; // (path, source)

    #[cfg(target_os = "windows")]
    {
        let pf = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".into());
        let pf86 = std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| r"C:\Program Files (x86)".into());
        let localappdata = std::env::var("LOCALAPPDATA").unwrap_or_default();

        // 标准安装路径
        candidates.push((format!(r"{}\Git\cmd\git.exe", pf), "SYSTEM".into()));
        candidates.push((format!(r"{}\Git\cmd\git.exe", pf86), "SYSTEM".into()));

        // 常见盘符
        for drive in &["C", "D", "E", "F", "G"] {
            candidates.push((format!(r"{}:\Git\cmd\git.exe", drive), "MANUAL".into()));
            candidates.push((format!(r"{}:\Program Files\Git\cmd\git.exe", drive), "SYSTEM".into()));
            // 工具目录
            for sub in &["Tools", "Dev", "AI", "Apps", "Software"] {
                candidates.push((format!(r"{}:\{}\Git\cmd\git.exe", drive, sub), "MANUAL".into()));
            }
        }

        // 自定义应用目录（如 D:\Data\exeApp\Git）
        for drive in &["C", "D", "E", "F"] {
            candidates.push((format!(r"{}:\Data\exeApp\Git\cmd\git.exe", drive), "MANUAL".into()));
        }

        // GitHub Desktop 内置 Git
        if !localappdata.is_empty() {
            let gh_dir = std::path::Path::new(&localappdata).join("GitHubDesktop");
            if gh_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&gh_dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.is_dir() {
                            let git_exe = p.join("resources").join("app").join("git").join("cmd").join("git.exe");
                            if git_exe.exists() {
                                candidates.push((git_exe.to_string_lossy().to_string(), "GITHUB_DESKTOP".into()));
                            }
                        }
                    }
                }
            }
        }

        // VS Code 内置 Git
        if !localappdata.is_empty() {
            let vscode_git = std::path::Path::new(&localappdata)
                .join(r"Programs\Microsoft VS Code\resources\app\node_modules.asar.unpacked\vscode-git\git\cmd\git.exe");
            if vscode_git.exists() {
                candidates.push((vscode_git.to_string_lossy().to_string(), "VSCODE".into()));
            }
        }

        // MinGW / MSYS2 / Git Bash
        candidates.push((format!(r"{}\Git\mingw64\bin\git.exe", pf), "MINGW".into()));
        for drive in &["C", "D"] {
            candidates.push((format!(r"{}:\msys64\usr\bin\git.exe", drive), "MSYS2".into()));
            candidates.push((format!(r"{}:\msys2\usr\bin\git.exe", drive), "MSYS2".into()));
        }

        // Scoop
        let home = dirs::home_dir().unwrap_or_default();
        candidates.push((format!(r"{}\scoop\apps\git\current\cmd\git.exe", home.display()), "SCOOP".into()));
        candidates.push((format!(r"{}\scoop\shims\git.exe", home.display()), "SCOOP".into()));

        // Chocolatey
        let choco_dir = std::env::var("ChocolateyInstall").unwrap_or_else(|_| r"C:\ProgramData\chocolatey".into());
        candidates.push((format!(r"{}\bin\git.exe", choco_dir), "CHOCOLATEY".into()));
    }

    #[cfg(not(target_os = "windows"))]
    {
        candidates.push(("/usr/bin/git".into(), "SYSTEM".into()));
        candidates.push(("/usr/local/bin/git".into(), "SYSTEM".into()));
        candidates.push(("/opt/homebrew/bin/git".into(), "BREW".into()));
        // Xcode
        candidates.push(("/Library/Developer/CommandLineTools/usr/bin/git".into(), "XCODE_CLT".into()));
        candidates.push(("/Applications/Xcode.app/Contents/Developer/usr/bin/git".into(), "XCODE".into()));
        // Snap / Flatpak
        candidates.push(("/snap/bin/git".into(), "SNAP".into()));
        // Nix
        let home = dirs::home_dir().unwrap_or_default();
        candidates.push((format!("{}/.nix-profile/bin/git", home.display()), "NIX".into()));
        // Linuxbrew
        candidates.push((format!("{}/.linuxbrew/bin/git", home.display()), "BREW".into()));
        candidates.push(("/home/linuxbrew/.linuxbrew/bin/git".into(), "BREW".into()));
    }

    // 去重并检测
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (path, source) in &candidates {
        let p = std::path::Path::new(path);
        if !p.exists() {
            continue;
        }
        let canonical = p.to_string_lossy().to_string();
        if seen.contains(&canonical) {
            continue;
        }
        seen.insert(canonical.clone());

        let mut cmd = Command::new(path);
        cmd.arg("--version");
        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000);
        if let Ok(o) = cmd.output() {
            if o.status.success() {
                let ver = String::from_utf8_lossy(&o.stdout).trim().to_string();
                let mut entry = serde_json::Map::new();
                entry.insert("path".into(), Value::String(canonical));
                entry.insert("version".into(), Value::String(ver));
                entry.insert("source".into(), Value::String(source.clone()));
                found.push(Value::Object(entry));
            }
        }
    }

    Ok(Value::Array(found))
}

/// 尝试自动安装 Git（Windows: winget; macOS: xcode-select; Linux: apt/yum）
#[tauri::command]
pub async fn auto_install_git(app: tauri::AppHandle) -> Result<String, String> {
    use std::process::Stdio;
    use tauri::Emitter;

    let _ = app.emit("upgrade-log", "正在尝试自动安装 Git...");

    #[cfg(target_os = "windows")]
    {
        use std::io::{BufRead, BufReader};
        // 尝试 winget
        let _ = app.emit("upgrade-log", "尝试使用 winget 安装 Git...");
        let mut child = Command::new("winget")
            .args([
                "install",
                "--id",
                "Git.Git",
                "-e",
                "--source",
                "winget",
                "--accept-package-agreements",
                "--accept-source-agreements",
            ])
            .creation_flags(0x08000000)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("winget 不可用，请手动安装 Git: {e}"))?;

        let stderr = child.stderr.take();
        let stdout = child.stdout.take();
        let app2 = app.clone();
        let handle = std::thread::spawn(move || {
            if let Some(pipe) = stderr {
                for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                    let _ = app2.emit("upgrade-log", &line);
                }
            }
        });
        if let Some(pipe) = stdout {
            for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                let _ = app.emit("upgrade-log", &line);
            }
        }
        let _ = handle.join();
        let status = child.wait().map_err(|e| format!("等待 winget 完成失败: {e}"))?;
        if status.success() {
            let _ = app.emit("upgrade-log", "Git 安装成功！");
            // #Compat-4: 刷新 PATH 缓存，使 check_git 能立即检测到新装的 git，
            // 避免用户反馈「装完不识别，重启客户端才能用」
            super::refresh_enhanced_path();
            crate::commands::service::invalidate_cli_detection_cache();
            return Ok("Git 已通过 winget 安装".to_string());
        }
        Err("winget 安装 Git 失败，请手动下载安装: https://git-scm.com/downloads".to_string())
    }

    #[cfg(target_os = "macos")]
    {
        let _ = app.emit("upgrade-log", "尝试通过 xcode-select 安装 Git...");
        let mut child = Command::new("xcode-select")
            .arg("--install")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("xcode-select 不可用: {e}"))?;
        let status = child.wait().map_err(|e| format!("等待安装完成失败: {e}"))?;
        if status.success() {
            let _ = app.emit("upgrade-log", "Git 安装已触发，请在弹出的窗口中确认安装。");
            // #Compat-4: 刷新缓存（即便是"触发"而非同步完成，下次检测时缓存也已清）
            super::refresh_enhanced_path();
            crate::commands::service::invalidate_cli_detection_cache();
            return Ok("已触发 xcode-select 安装，请在弹窗中确认".to_string());
        }
        Err("xcode-select 安装失败，请手动安装 Xcode Command Line Tools 或 brew install git".to_string())
    }

    #[cfg(target_os = "linux")]
    {
        use std::io::{BufRead, BufReader};
        // 检测包管理器
        let pkg_mgr = if Command::new("apt-get")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            "apt"
        } else if Command::new("yum")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            "yum"
        } else if Command::new("dnf")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            "dnf"
        } else if Command::new("pacman")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            "pacman"
        } else {
            return Err("未找到包管理器，请手动安装 Git: sudo apt install git 或 sudo yum install git".to_string());
        };

        let (cmd_name, args): (&str, Vec<&str>) = match pkg_mgr {
            "apt" => ("sudo", vec!["apt-get", "install", "-y", "git"]),
            "yum" => ("sudo", vec!["yum", "install", "-y", "git"]),
            "dnf" => ("sudo", vec!["dnf", "install", "-y", "git"]),
            "pacman" => ("sudo", vec!["pacman", "-S", "--noconfirm", "git"]),
            _ => return Err("不支持的包管理器".to_string()),
        };

        let _ = app.emit("upgrade-log", format!("执行: {} {}", cmd_name, args.join(" ")));
        let mut child = Command::new(cmd_name)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("安装命令执行失败: {e}"))?;

        let stderr = child.stderr.take();
        let stdout = child.stdout.take();
        let app2 = app.clone();
        let handle = std::thread::spawn(move || {
            if let Some(pipe) = stderr {
                for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                    let _ = app2.emit("upgrade-log", &line);
                }
            }
        });
        if let Some(pipe) = stdout {
            for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                let _ = app.emit("upgrade-log", &line);
            }
        }
        let _ = handle.join();
        let status = child.wait().map_err(|e| format!("等待安装完成失败: {e}"))?;
        if status.success() {
            let _ = app.emit("upgrade-log", "Git 安装成功！");
            // #Compat-4: 刷新 PATH 缓存，使 check_git 立即识别新装的 git
            super::refresh_enhanced_path();
            crate::commands::service::invalidate_cli_detection_cache();
            return Ok("Git 已安装".to_string());
        }
        Err("Git 安装失败，请手动执行: sudo apt install git".to_string())
    }
}

/// 配置 Git 使用 HTTPS 替代 SSH，解决国内用户 SSH 不通的问题
#[tauri::command]
pub fn configure_git_https() -> Result<String, String> {
    let success = configure_git_https_rules();
    if success > 0 {
        Ok(format!("已配置 Git 使用 HTTPS（{success}/{} 条规则）", GIT_HTTPS_REWRITES.len()))
    } else {
        Err("Git 未安装或配置失败".to_string())
    }
}
