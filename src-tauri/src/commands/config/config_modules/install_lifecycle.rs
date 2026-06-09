
async fn upgrade_openclaw_inner(
    app: tauri::AppHandle,
    source: String,
    version: Option<String>,
    method: String,
) -> Result<String, String> {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    use tauri::Emitter;
    let _guardian_pause = GuardianPause::new("upgrade");

    let current_source = detect_installed_source();
    let pkg_name = npm_package_name(&source);
    let requested_version = version.clone();
    let recommended_version = recommended_version_for(&source).await;
    let ver = requested_version
        .as_deref()
        .or(recommended_version.as_deref())
        .unwrap_or("latest");
    let pkg = format!("{}@{}", pkg_name, ver);

    // ── standalone 安装（auto / standalone-r2 / standalone-github） ──
    let try_standalone = source != "official" && (method == "auto" || method == "standalone-r2" || method == "standalone-github");

    if try_standalone {
        let github_release_base = super::zhizhua_url(&format!("/openclaw-standalone/releases/download/v{}", ver));

        if method == "standalone-github" {
            // standalone-github 模式：只走 GitHub
            match try_standalone_install(&app, ver, Some(&github_release_base)).await {
                Ok(installed_ver) => {
                    let _ = app.emit("upgrade-progress", 100);
                    super::refresh_enhanced_path();
                    crate::commands::service::invalidate_cli_detection_cache();
                    let msg = format!("✅ standalone (GitHub) 安装完成，当前版本: {installed_ver}");
                    let _ = app.emit("upgrade-log", &msg);
                    return Ok(msg);
                }
                Err(reason) => {
                    return Err(format!("standalone 安装失败: {reason}"));
                }
            }
        } else {
            // auto / standalone-r2 模式：R2 CDN → GitHub Releases fallback
            match try_standalone_install(&app, ver, None).await {
                Ok(installed_ver) => {
                    let _ = app.emit("upgrade-progress", 100);
                    super::refresh_enhanced_path();
                    crate::commands::service::invalidate_cli_detection_cache();
                    let msg = format!("✅ standalone (CDN) 安装完成，当前版本: {installed_ver}");
                    let _ = app.emit("upgrade-log", &msg);
                    return Ok(msg);
                }
                Err(cdn_reason) => {
                    let _ = app.emit("upgrade-log", format!("CDN 下载失败（{cdn_reason}），尝试从 GitHub Releases 下载..."));
                    let _ = app.emit("upgrade-progress", 5);
                    // Fallback: GitHub Releases
                    match try_standalone_install(&app, ver, Some(&github_release_base)).await {
                        Ok(installed_ver) => {
                            let _ = app.emit("upgrade-progress", 100);
                            super::refresh_enhanced_path();
                            crate::commands::service::invalidate_cli_detection_cache();
                            let msg = format!("✅ standalone (GitHub) 安装完成，当前版本: {installed_ver}");
                            let _ = app.emit("upgrade-log", &msg);
                            return Ok(msg);
                        }
                        Err(gh_reason) => {
                            if method == "auto" {
                                let _ = app
                                    .emit("upgrade-log", format!("standalone 不可用（GitHub: {gh_reason}），降级到 npm 安装..."));
                                let _ = app.emit("upgrade-progress", 5);
                            } else {
                                return Err(format!("standalone 安装失败: CDN={cdn_reason}, GitHub={gh_reason}"));
                            }
                        }
                    }
                }
            }
        }
    }

    // ── npm install（兜底或用户明确选择） ──

    // 切换源时需要卸载旧包，但为避免安装失败导致 CLI 丢失，
    // 先安装新包，成功后再卸载旧包
    let old_pkg = npm_package_name(&current_source);
    let need_uninstall_old = current_source != source && old_pkg != pkg_name;

    if requested_version.is_none() {
        if let Some(recommended) = &recommended_version {
            let _ = app.emit(
                "upgrade-log",
                format!("当前工作台 {} 默认绑定 OpenClaw 稳定版: {}", panel_version(), recommended),
            );
        } else {
            let _ = app.emit("upgrade-log", "未找到绑定稳定版，将回退到 latest");
        }
    }
    let configured_rules = configure_git_https_rules();
    let _ = app.emit(
        "upgrade-log",
        format!("Git HTTPS 规则已就绪 ({}/{})", configured_rules, GIT_HTTPS_REWRITES.len()),
    );

    // 安装前：停止 Gateway 并清理可能冲突的 bin 文件
    let _ = app.emit("upgrade-log", "正在停止 Gateway 并清理旧文件...");
    pre_install_cleanup();

    let _ = app.emit("upgrade-log", format!("$ npm install -g {pkg} --force"));
    #[cfg(target_os = "linux")]
    {
        if !nix_is_root() {
            if npm_prefix_is_user_writable() {
                let _ = app.emit("upgrade-log", "npm prefix 在用户目录下，无需提权");
            } else {
                let has_pkexec = Command::new("which")
                    .arg("pkexec")
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false);
                if has_pkexec {
                    let _ = app.emit("upgrade-log", "需要管理员权限，将通过 pkexec 弹出认证窗口...");
                } else {
                    let _ = app.emit("upgrade-log", "⚠️ 需要管理员权限但 pkexec 不可用，可能需要手动安装");
                }
            }
        }
    }
    let _ = app.emit("upgrade-progress", 10);

    // 汉化版只支持官方源和淘宝源
    let configured_registry = get_configured_registry();
    let registry = if pkg_name.contains(&legacy_openclaw_zh_package()) {
        // 中文增强版：淘宝源或官方源
        if configured_registry.contains("npmmirror.com") || configured_registry.contains("taobao.org") {
            configured_registry.as_str()
        } else {
            "https://registry.npmjs.org"
        }
    } else {
        // 官方版：使用用户配置的镜像源
        configured_registry.as_str()
    };

    let mut install_cmd = npm_command_elevated();
    install_cmd.args(["install", "-g", &pkg, "--force", "--registry", registry, "--verbose"]);
    apply_git_install_env(&mut install_cmd);
    let mut child = install_cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("执行升级命令失败: {e}"))?;

    let stderr = child.stderr.take();
    let stdout = child.stdout.take();

    // stderr 每行递增进度（10→80 区间），让用户看到进度在动
    // 同时收集 stderr 用于失败时返回给前端诊断
    let app2 = app.clone();
    let stderr_lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let stderr_lines2 = stderr_lines.clone();
    let handle = std::thread::spawn(move || {
        let mut progress: u32 = 15;
        if let Some(pipe) = stderr {
            for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                let _ = app2.emit("upgrade-log", &line);
                stderr_lines2
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(line);
                if progress < 75 {
                    progress += 2;
                    let _ = app2.emit("upgrade-progress", progress);
                }
            }
        }
    });

    if let Some(pipe) = stdout {
        for line in BufReader::new(pipe).lines().map_while(Result::ok) {
            let _ = app.emit("upgrade-log", &line);
        }
    }

    let _ = handle.join();
    let _ = app.emit("upgrade-progress", 80);

    let status = child.wait().map_err(|e| format!("等待进程失败: {e}"))?;
    let _ = app.emit("upgrade-progress", 100);

    if !status.success() {
        let code = status.code().map(|c| c.to_string()).unwrap_or("unknown".into());

        // 如果使用了镜像源失败，自动降级到官方源重试
        let used_mirror = registry.contains("npmmirror.com") || registry.contains("taobao.org");
        if used_mirror {
            let _ = app.emit("upgrade-log", "");
            let _ = app.emit("upgrade-log", "⚠️ 镜像源安装失败，自动切换到官方源重试...");
            let _ = app.emit("upgrade-progress", 15);
            let fallback = "https://registry.npmjs.org";
            let mut install_cmd2 = npm_command_elevated();
            install_cmd2.args(["install", "-g", &pkg, "--force", "--registry", fallback, "--verbose"]);
            apply_git_install_env(&mut install_cmd2);
            let mut child2 = install_cmd2
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| format!("执行重试命令失败: {e}"))?;
            let stderr2 = child2.stderr.take();
            let stdout2 = child2.stdout.take();
            let app3 = app.clone();
            let stderr_lines3 = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
            let stderr_lines4 = stderr_lines3.clone();
            let handle2 = std::thread::spawn(move || {
                if let Some(pipe) = stderr2 {
                    let mut p: u32 = 20;
                    for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                        let _ = app3.emit("upgrade-log", &line);
                        stderr_lines4
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner())
                            .push(line);
                        if p < 75 {
                            p += 2;
                            let _ = app3.emit("upgrade-progress", p);
                        }
                    }
                }
            });
            if let Some(pipe) = stdout2 {
                for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                    let _ = app.emit("upgrade-log", &line);
                }
            }
            let _ = handle2.join();
            let _ = app.emit("upgrade-progress", 80);
            let status2 = child2.wait().map_err(|e| format!("等待重试进程失败: {e}"))?;
            let _ = app.emit("upgrade-progress", 100);
            if !status2.success() {
                let code2 = status2.code().map(|c| c.to_string()).unwrap_or("unknown".into());
                let tail = stderr_lines3
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .iter()
                    .rev()
                    .take(15)
                    .rev()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n");
                return Err(format!("升级失败（镜像源和官方源均失败），exit code: {code2}\n{tail}"));
            }
            let _ = app.emit("upgrade-log", "✅ 官方源安装成功");
        } else {
            let _ = app.emit("upgrade-log", format!("❌ 升级失败 (exit code: {code})"));
            let tail = stderr_lines
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .iter()
                .rev()
                .take(15)
                .rev()
                .cloned()
                .collect::<Vec<_>>()
                .join("\n");
            return Err(format!("升级失败，exit code: {code}\n{tail}"));
        }
    }

    // 安装成功后再卸载旧包（确保 CLI 始终可用）
    // 清理步骤采用错误隔离：任何清理失败都不影响安装成功的最终结果
    if need_uninstall_old {
        let _ = app.emit("upgrade-log", format!("清理旧版本 ({old_pkg})..."));
        // npm uninstall 加 30s 超时，避免无限卡住
        let uninstall_child = npm_command_elevated()
            .args(["uninstall", "-g", old_pkg])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        match uninstall_child {
            Ok(mut child) => {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
                loop {
                    match child.try_wait() {
                        Ok(Some(_status)) => break,
                        Ok(None) => {
                            if std::time::Instant::now() >= deadline {
                                let _ = child.kill();
                                let _ = app.emit("upgrade-log", "⚠️ 清理旧版本超时（30s），已跳过");
                                break;
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        }
                        Err(_) => break,
                    }
                }
            }
            Err(e) => {
                let _ = app.emit("upgrade-log", format!("⚠️ 清理旧版本启动失败: {e}，已跳过"));
            }
        }

        // 清理 standalone 安装目录（不论从 standalone 切走还是切到 standalone，
        // npm 路径已经安装了新 CLI，standalone 残留会干扰源检测）
        for sa_dir in all_standalone_dirs() {
            if sa_dir.exists() {
                let _ = app.emit("upgrade-log", format!("清理 standalone 残留: {}", sa_dir.display()));

                // Windows: 终止占用该目录的 node.exe 进程
                // 使用 PowerShell Get-Process（兼容 Windows 11，wmic 已废弃）
                #[cfg(target_os = "windows")]
                {
                    let dir_lower = sa_dir.to_string_lossy().to_lowercase().replace('\\', "\\\\");
                    let ps_script = format!(
                        "Get-Process -Name node -ErrorAction SilentlyContinue | Where-Object {{ $_.Path -and $_.Path.ToLower().Contains('{}') }} | Select-Object -ExpandProperty Id",
                        dir_lower
                    );
                    if let Ok(output) = Command::new("powershell")
                        .args(["-NoProfile", "-Command", &ps_script])
                        .output()
                    {
                        let text = String::from_utf8_lossy(&output.stdout);
                        for line in text.lines() {
                            if let Ok(pid) = line.trim().parse::<u32>() {
                                let _ = app.emit("upgrade-log", format!("终止占用进程 PID {pid}..."));
                                let _ = Command::new("taskkill").args(["/F", "/PID", &pid.to_string()]).output();
                            }
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }

                match std::fs::remove_dir_all(&sa_dir) {
                    Ok(()) => {
                        let _ = app.emit("upgrade-log", "standalone 残留已清理 ✓");
                    }
                    Err(_) => {
                        let _ = app.emit("upgrade-log", "文件被占用，等待后重试...");
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                        if let Err(e) = std::fs::remove_dir_all(&sa_dir) {
                            let _ = app.emit(
                                "upgrade-log",
                                format!("⚠️ 清理 standalone 残留失败: {e}（可手动删除 {}）", sa_dir.display()),
                            );
                        } else {
                            let _ = app.emit("upgrade-log", "standalone 残留已清理（重试成功）✓");
                        }
                    }
                }
            }
        }
    }

    // 切换源后重装 Gateway 服务
    if need_uninstall_old {
        let _ = app.emit("upgrade-log", "正在重装 Gateway 服务（更新启动路径）...");

        // 刷新 PATH 缓存和 CLI 检测缓存，确保找到新安装的二进制
        super::refresh_enhanced_path();
        crate::commands::service::invalidate_cli_detection_cache();

        // 先停掉旧的
        #[cfg(target_os = "macos")]
        {
            let uid = get_uid().unwrap_or(501);
            let _ = Command::new("launchctl")
                .args(["bootout", &format!("gui/{uid}/ai.openclaw.gateway")])
                .output();
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = openclaw_command().args(["gateway", "stop"]).output();
        }
        // 等待旧 Gateway 进程退出
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        // 重新安装（刷新后的 PATH 会找到新二进制）
        use crate::utils::openclaw_command_async;
        let gw_out = openclaw_command_async().args(["gateway", "install"]).output().await;
        match gw_out {
            Ok(o) if o.status.success() => {
                let _ = app.emit("upgrade-log", "Gateway 服务已重装");
            }
            _ => {
                let _ = app.emit("upgrade-log", "⚠️ Gateway 重装失败，请手动执行 openclaw gateway install");
            }
        }
    }

    // #Compat-4: npm 首次安装场景下，前面 `if need_uninstall_old` 块被跳过，
    // PATH 缓存和 CLI 检测缓存都是装 openclaw 之前的旧快照。必须在这里统一刷新一次，
    // 否则前端 `check_installation`/`get_services_status` 拿到的仍是「CLI 未安装」
    // —— 用户反馈「一键装完日志显示成功，但面板不识别，重启客户端才能用」。
    // 切换源场景前面已刷过，这里重刷无害（几十 ms 扫描开销可接受）。
    super::refresh_enhanced_path();
    crate::commands::service::invalidate_cli_detection_cache();

    let new_ver = get_local_version().await.unwrap_or_else(|| "未知".into());
    let msg = format!("✅ 安装完成，当前版本: {new_ver}");
    let _ = app.emit("upgrade-log", &msg);
    Ok(msg)
}

/// 卸载 OpenClaw（后台执行，通过 event 推送进度）
/// 立即返回，不阻塞前端。完成后 emit "upgrade-done" 或 "upgrade-error"。
#[tauri::command]
pub async fn uninstall_openclaw(app: tauri::AppHandle, clean_config: bool) -> Result<String, String> {
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        use tauri::Emitter;
        let result = uninstall_openclaw_inner(app2.clone(), clean_config).await;
        match result {
            Ok(msg) => {
                let _ = app2.emit("upgrade-done", &msg);
            }
            Err(err) => {
                let _ = app2.emit("upgrade-error", &err);
            }
        }
    });
    Ok("任务已启动".into())
}

async fn uninstall_openclaw_inner(app: tauri::AppHandle, clean_config: bool) -> Result<String, String> {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    use tauri::Emitter;
    let _guardian_pause = GuardianPause::new("uninstall openclaw");
    crate::commands::service::guardian_mark_manual_stop();

    let source = detect_installed_source();
    let pkg = npm_package_name(&source);

    // 1. 先停止 Gateway
    let _ = app.emit("upgrade-log", "正在停止 Gateway...");
    #[cfg(target_os = "macos")]
    {
        let uid = get_uid().unwrap_or(501);
        let _ = Command::new("launchctl")
            .args(["bootout", &format!("gui/{uid}/ai.openclaw.gateway")])
            .output();
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = openclaw_command().args(["gateway", "stop"]).output();
    }

    // 2. 卸载 Gateway 服务
    let _ = app.emit("upgrade-log", "正在卸载 Gateway 服务...");
    #[cfg(not(target_os = "macos"))]
    {
        let _ = openclaw_command().args(["gateway", "uninstall"]).output();
    }

    // 等待进程完全退出（Gateway stop 是异步的，需要等文件锁释放）
    let _ = app.emit("upgrade-log", "等待进程退出...");
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // 3. 清理 standalone 安装（所有可能的位置）
    for sa_dir in &all_standalone_dirs() {
        if sa_dir.exists() {
            let _ = app.emit("upgrade-log", format!("清理 standalone 安装: {}", sa_dir.display()));

            // Windows: 先尝试终止占用该目录的 node.exe 进程
            // 使用 PowerShell Get-Process（兼容 Windows 11，wmic 已废弃）
            #[cfg(target_os = "windows")]
            {
                let dir_lower = sa_dir.to_string_lossy().to_lowercase().replace('\\', "\\\\");
                let ps_script = format!(
                    "Get-Process -Name node -ErrorAction SilentlyContinue | Where-Object {{ $_.Path -and $_.Path.ToLower().Contains('{}') }} | Select-Object -ExpandProperty Id",
                    dir_lower
                );
                if let Ok(output) = Command::new("powershell")
                    .args(["-NoProfile", "-Command", &ps_script])
                    .output()
                {
                    let text = String::from_utf8_lossy(&output.stdout);
                    for line in text.lines() {
                        if let Ok(pid) = line.trim().parse::<u32>() {
                            let _ = app.emit("upgrade-log", format!("终止占用进程 PID {pid}..."));
                            let _ = Command::new("taskkill").args(["/F", "/PID", &pid.to_string()]).output();
                        }
                    }
                }
                // 短暂等待进程退出
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }

            // 尝试删除，失败则重试一次
            match std::fs::remove_dir_all(sa_dir) {
                Ok(()) => {
                    let _ = app.emit("upgrade-log", "standalone 安装已清理 ✓");
                }
                Err(_) => {
                    // 重试：等待后再删一次
                    let _ = app.emit("upgrade-log", "文件被占用，等待后重试...");
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    if let Err(e) = std::fs::remove_dir_all(sa_dir) {
                        let _ =
                            app.emit("upgrade-log", format!("⚠️ 清理 standalone 失败: {e}（可手动删除 {}）", sa_dir.display()));
                    } else {
                        let _ = app.emit("upgrade-log", "standalone 安装已清理（重试成功）✓");
                    }
                }
            }
        }
    }

    // 4. npm uninstall
    let _ = app.emit("upgrade-log", format!("$ npm uninstall -g {pkg}"));
    let _ = app.emit("upgrade-progress", 20);

    let mut child = npm_command_elevated()
        .args(["uninstall", "-g", pkg])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("执行卸载命令失败: {e}"))?;

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
    let _ = app.emit("upgrade-progress", 60);

    let status = child.wait().map_err(|e| format!("等待进程失败: {e}"))?;
    if !status.success() {
        let code = status.code().map(|c| c.to_string()).unwrap_or("unknown".into());
        return Err(format!("卸载失败，exit code: {code}"));
    }

    // 4. 两个包都尝试卸载（确保干净）
    let other_pkg = "openclaw";
    let _ = app.emit("upgrade-log", format!("清理 {other_pkg}..."));
    let _ = npm_command_elevated().args(["uninstall", "-g", other_pkg]).output();
    let _ = app.emit("upgrade-progress", 80);

    // 5. 可选：清理配置目录
    if clean_config {
        let config_dir = super::openclaw_dir();
        if config_dir.exists() {
            let _ = app.emit("upgrade-log", format!("清理配置目录: {}", config_dir.display()));
            if let Err(e) = std::fs::remove_dir_all(&config_dir) {
                let _ = app.emit("upgrade-log", format!("⚠️ 清理配置目录失败: {e}（可能有文件被占用）"));
            }
        }
    }

    let _ = app.emit("upgrade-progress", 100);
    // #Compat-4: 卸载后刷缓存，否则 is_cli_installed（60s TTL）/ enhanced_path
    // 仍是旧快照，UI 会在 60 秒内继续显示「CLI 已安装」或 Gateway 还在运行。
    super::refresh_enhanced_path();
    crate::commands::service::invalidate_cli_detection_cache();
    let msg = if clean_config {
        "✅ OpenClaw 已完全卸载（包括配置文件）"
    } else {
        "✅ OpenClaw 已卸载（配置文件保留在 ~/.openclaw/）"
    };
    let _ = app.emit("upgrade-log", msg);
    Ok(msg.into())
}

/// 自动初始化配置文件（CLI 已装但 openclaw.json 不存在时）
#[tauri::command]
pub fn init_openclaw_config() -> Result<Value, String> {
    let dir = super::openclaw_dir();
    let config_path = dir.join("openclaw.json");
    let backup_path = dir.join("openclaw.json.bak");
    let mut result = serde_json::Map::new();

    if config_path.exists() {
        result.insert("created".into(), Value::Bool(false));
        result.insert("message".into(), Value::String("配置文件已存在".into()));
        return Ok(Value::Object(result));
    }

    // 确保目录存在
    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败: {e}"))?;
    }

    if backup_path.exists() {
        let backup_content = std::fs::read_to_string(&backup_path).map_err(|e| format!("读取配置备份失败: {e}"))?;
        serde_json::from_str::<Value>(&backup_content).map_err(|e| format!("配置备份损坏，无法恢复: {e}"))?;
        std::fs::write(&config_path, backup_content).map_err(|e| format!("恢复配置备份失败: {e}"))?;

        result.insert("created".into(), Value::Bool(false));
        result.insert("restored".into(), Value::Bool(true));
        result.insert("message".into(), Value::String("已从 openclaw.json.bak 恢复配置文件".into()));
        return Ok(Value::Object(result));
    }

    let default_config = strip_ui_fields(normalize_calibrated_config(build_calibration_baseline()));

    let content = serde_json::to_string_pretty(&default_config).map_err(|e| format!("序列化失败: {e}"))?;
    std::fs::write(&config_path, content).map_err(|e| format!("写入失败: {e}"))?;

    result.insert("created".into(), Value::Bool(true));
    result.insert("restored".into(), Value::Bool(false));
    result.insert("message".into(), Value::String("配置文件已创建".into()));
    Ok(Value::Object(result))
}

#[tauri::command]
pub fn check_installation() -> Result<Value, String> {
    let dir = super::openclaw_dir();
    let installed = dir.join("openclaw.json").exists();
    let mut result = serde_json::Map::new();
    result.insert("installed".into(), Value::Bool(installed));
    result.insert("path".into(), Value::String(dir.to_string_lossy().to_string()));
    Ok(Value::Object(result))
}

/// 检测 Node.js 是否已安装，返回版本号和检测到的路径
#[tauri::command]
pub fn check_node() -> Result<Value, String> {
    let mut result = serde_json::Map::new();
    let enhanced = super::enhanced_path();

    // 尝试通过 which/where 命令找到 node 的实际路径
    let node_path = find_node_path(&enhanced);

    if let Some(path) = node_path {
        let mut cmd = Command::new(&path);
        cmd.arg("--version");
        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        match cmd.output() {
            Ok(o) if o.status.success() => {
                let ver = String::from_utf8_lossy(&o.stdout).trim().to_string();
                let detected_from = detect_node_source(&path);
                result.insert("installed".into(), Value::Bool(true));
                result.insert("version".into(), Value::String(ver));
                result.insert("path".into(), Value::String(path));
                result.insert("detectedFrom".into(), Value::String(detected_from));
            }
            _ => {
                result.insert("installed".into(), Value::Bool(false));
                result.insert("version".into(), Value::Null);
                result.insert("path".into(), Value::Null);
                result.insert("detectedFrom".into(), Value::Null);
            }
        }
    } else {
        result.insert("installed".into(), Value::Bool(false));
        result.insert("version".into(), Value::Null);
        result.insert("path".into(), Value::Null);
        result.insert("detectedFrom".into(), Value::Null);
    }
    Ok(Value::Object(result))
}
