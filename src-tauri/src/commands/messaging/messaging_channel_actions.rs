use serde_json::Value;

/// 检测微信插件安装状态与版本
#[tauri::command]
pub async fn check_weixin_plugin_status() -> Result<Value, String> {
    let ext_dir = super::openclaw_dir().join("extensions").join("openclaw-weixin");
    let mut installed = false;
    let mut installed_version: Option<String> = None;

    // 检查本地安装
    let pkg_json = ext_dir.join("package.json");
    if pkg_json.is_file() {
        installed = true;
        if let Ok(content) = std::fs::read_to_string(&pkg_json) {
            if let Ok(pkg) = serde_json::from_str::<Value>(&content) {
                installed_version = pkg.get("version").and_then(|v| v.as_str()).map(|s| s.to_string());
            }
        }
    }

    // 从 npm registry 获取最新版本
    let mut latest_version: Option<String> = None;
    let client = super::build_http_client(std::time::Duration::from_secs(8), None).unwrap_or_else(|_| reqwest::Client::new());
    if let Ok(resp) = client
        .get("https://registry.npmjs.org/@tencent-weixin/openclaw-weixin/latest")
        .header("Accept", "application/json")
        .send()
        .await
    {
        if let Ok(body) = resp.json::<Value>().await {
            latest_version = body.get("version").and_then(|v| v.as_str()).map(|s| s.to_string());
        }
    }

    let update_available = match (&installed_version, &latest_version) {
        (Some(cur), Some(lat)) if cur != lat => {
            // 简单 semver 比较：按 . 分割为数字段逐段比较
            let parse = |s: &str| -> Vec<u32> { s.split('.').filter_map(|p| p.parse().ok()).collect() };
            let cv = parse(cur);
            let lv = parse(lat);
            lv > cv
        }
        _ => false,
    };

    // 兼容性检查：微信插件要求 OpenClaw >= 2026.3.22，通过版本号判断
    let mut compatible = true;
    let mut compat_error = String::new();
    if installed {
        let oc_ver = crate::utils::resolve_openclaw_cli_path()
            .and_then(|_| {
                let out = crate::utils::openclaw_command().arg("--version").output().ok()?;
                let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
                raw.split_whitespace()
                    .find(|w| w.chars().next().is_some_and(|c| c.is_ascii_digit()))
                    .map(String::from)
            })
            .unwrap_or_default();
        let oc_nums: Vec<u32> = oc_ver
            .split(|c: char| !c.is_ascii_digit())
            .filter_map(|s| s.parse().ok())
            .collect();
        if oc_nums < vec![2026, 3, 22] {
            compatible = false;
            compat_error = format!(
                "插件版本与当前 OpenClaw {} 不兼容（要求 >= 2026.3.22），请先升级 OpenClaw 或在终端执行: npx -y @tencent-weixin/openclaw-weixin-cli@latest install",
                oc_ver
            );
        }
    }

    Ok(crate::jv!({
        "installed": installed,
        "installedVersion": installed_version,
        "latestVersion": latest_version,
        "updateAvailable": update_available,
        "extensionDir": ext_dir.to_string_lossy(),
        "compatible": compatible,
        "compatError": compat_error,
    }))
}

#[tauri::command]
pub async fn run_channel_action(
    app: tauri::AppHandle,
    platform: String,
    action: String,
    version: Option<String>,
) -> Result<String, String> {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    use std::sync::{Arc, Mutex};
    use tauri::Emitter;

    let platform = platform.trim().to_string();
    let action = action.trim().to_string();
    if platform.is_empty() || action.is_empty() {
        return Err("platform 和 action 不能为空".into());
    }

    // weixin install 走 npx 而非 openclaw CLI
    if platform == "weixin" && action == "install" {
        // 微信 CLI 版本号独立于 OpenClaw（1.0.x / 2.0.x），不能用 OpenClaw 版本号 pin
        // v2.0.1 需要 OpenClaw >= 2026.3.22 的 SDK，旧版用 v1.0.3（最后兼容版）
        let weixin_spec = if version.as_deref().is_some_and(|v| !v.is_empty()) {
            format!("@tencent-weixin/openclaw-weixin-cli@{}", version.as_deref().unwrap_or_default())
        } else {
            // 检测 OpenClaw 版本，决定装哪个
            let oc_ver = crate::utils::resolve_openclaw_cli_path()
                .and_then(|_| {
                    let out = crate::utils::openclaw_command().arg("--version").output().ok()?;
                    let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    // 输出格式: "OpenClaw 2026.3.24 (hash)" → 取第二个词（版本号）
                    raw.split_whitespace()
                        .find(|w| w.chars().next().is_some_and(|c| c.is_ascii_digit()))
                        .map(String::from)
                })
                .unwrap_or_default();
            let oc_nums: Vec<u32> = oc_ver
                .split(|c: char| !c.is_ascii_digit())
                .filter_map(|s| s.parse().ok())
                .collect();
            let needs_legacy = oc_nums < vec![2026, 3, 22];
            if needs_legacy {
                // 微信插件所有版本都依赖 OpenClaw >= 2026.3.22 的 SDK
                // 给用户两个选择：升级 OpenClaw 或手动尝试安装
                let _ = app.emit(
                    "channel-action-log",
                    crate::jv!({ "platform": &platform, "action": &action, "kind": "error",
                        "message": format!("⚠ 微信插件要求 OpenClaw >= 2026.3.22，当前版本 {}。", oc_ver) }),
                );
                let _ = app.emit(
                    "channel-action-log",
                    crate::jv!({ "platform": &platform, "action": &action, "kind": "info",
                        "message": "建议方案 1（推荐）：先升级 OpenClaw，再安装微信插件" }),
                );
                let _ = app.emit(
                    "channel-action-log",
                    crate::jv!({ "platform": &platform, "action": &action, "kind": "info",
                        "message": "  → 前往「服务管理」页面点击升级" }),
                );
                let _ = app.emit(
                    "channel-action-log",
                    crate::jv!({ "platform": &platform, "action": &action, "kind": "info",
                        "message": "建议方案 2：在终端手动尝试安装（可能存在兼容问题）" }),
                );
                let _ = app.emit(
                    "channel-action-log",
                    crate::jv!({ "platform": &platform, "action": &action, "kind": "info",
                        "message": "  → npx -y @tencent-weixin/openclaw-weixin-cli@latest install" }),
                );
                let _ = app.emit(
                    "channel-action-log",
                    crate::jv!({ "platform": &platform, "action": &action, "kind": "info",
                        "message": "后续版本将升级推荐内核到最新版以完整支持微信插件。" }),
                );
                let _ = app.emit(
                    "channel-action-progress",
                    crate::jv!({ "platform": &platform, "action": &action, "progress": 100 }),
                );
                return Err(format!(
                    "微信插件要求 OpenClaw >= 2026.3.22（当前 {}），请先升级 OpenClaw 或在终端手动安装",
                    oc_ver
                ));
            }
            "@tencent-weixin/openclaw-weixin-cli@latest".to_string()
        };
        // 先清理旧的不兼容插件目录 + openclaw.json 中的残留配置
        // （否则 OpenClaw 配置校验会报 unknown channel / plugin not found）
        let weixin_ext_dir = super::openclaw_dir().join("extensions").join("openclaw-weixin");
        if weixin_ext_dir.exists() {
            let _ = app.emit(
                "channel-action-log",
                crate::jv!({ "platform": &platform, "action": &action, "kind": "info", "message": "清理旧版微信插件目录..." }),
            );
            let _ = std::fs::remove_dir_all(&weixin_ext_dir);
        }
        // 清理 openclaw.json 中的微信残留配置
        if let Ok(mut cfg) = super::config::load_openclaw_json() {
            let mut changed = false;
            if let Some(channels) = cfg.get_mut("channels").and_then(|c| c.as_object_mut()) {
                if channels.remove("openclaw-weixin").is_some() {
                    changed = true;
                }
            }
            if let Some(plugins) = cfg.get_mut("plugins").and_then(|p| p.as_object_mut()) {
                if let Some(allow) = plugins.get_mut("allow").and_then(|a| a.as_array_mut()) {
                    let before = allow.len();
                    allow.retain(|v| v.as_str() != Some("openclaw-weixin"));
                    if allow.len() != before {
                        changed = true;
                    }
                }
                if let Some(entries) = plugins.get_mut("entries").and_then(|e| e.as_object_mut()) {
                    if entries.remove("openclaw-weixin").is_some() {
                        changed = true;
                    }
                }
            }
            if changed {
                let _ = super::config::save_openclaw_json(&cfg);
                let _ = app.emit(
                    "channel-action-log",
                    crate::jv!({ "platform": &platform, "action": &action, "kind": "info", "message": "已清理 openclaw.json 中的微信插件残留配置" }),
                );
            }
        }

        let _ = app.emit(
            "channel-action-log",
            crate::jv!({
                "platform": &platform, "action": &action, "kind": "info",
                "message": format!("开始安装微信插件: npx -y {} install", weixin_spec),
            }),
        );
        let _ = app.emit(
            "channel-action-progress",
            crate::jv!({ "platform": &platform, "action": &action, "progress": 5 }),
        );

        let path_env = super::enhanced_path();
        #[cfg(target_os = "windows")]
        let mut cmd = {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let mut c = std::process::Command::new("cmd");
            c.args(["/c", "npx", "-y", &weixin_spec, "install"]);
            c.creation_flags(CREATE_NO_WINDOW);
            c
        };
        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut c = std::process::Command::new("npx");
            c.args(["-y", &weixin_spec, "install"]);
            c
        };
        cmd.env("PATH", &path_env);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        crate::commands::apply_proxy_env(&mut cmd);

        let mut child = cmd.spawn().map_err(|e| format!("启动 npx 失败: {}", e))?;

        let stderr = child.stderr.take();
        let app2 = app.clone();
        let platform2 = platform.clone();
        let action2 = action.clone();
        let lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let err_lines = lines.clone();
        let handle = std::thread::spawn(move || {
            if let Some(pipe) = stderr {
                for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                    if let Ok(mut guard) = err_lines.lock() {
                        guard.push(line.clone());
                    }
                    let _ = app2.emit(
                        "channel-action-log",
                        crate::jv!({ "platform": platform2, "action": action2, "message": line, "kind": "stderr" }),
                    );
                }
            }
        });

        let mut progress: u32 = 15;
        if let Some(pipe) = child.stdout.take() {
            for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                if let Ok(mut guard) = lines.lock() {
                    guard.push(line.clone());
                }
                let _ = app.emit(
                    "channel-action-log",
                    crate::jv!({ "platform": &platform, "action": &action, "message": line, "kind": "stdout" }),
                );
                if progress < 90 {
                    progress += 5;
                    let _ = app.emit(
                        "channel-action-progress",
                        crate::jv!({ "platform": &platform, "action": &action, "progress": progress }),
                    );
                }
            }
        }

        let _ = handle.join();
        let status = child.wait().map_err(|e| format!("等待命令结束失败: {}", e))?;
        let text = lines.lock().ok().map(|g| g.join("\n")).unwrap_or_default();
        let _ = app.emit(
            "channel-action-progress",
            crate::jv!({ "platform": &platform, "action": &action, "progress": 100 }),
        );
        if status.success() {
            let _ = app.emit("channel-action-done", crate::jv!({ "platform": &platform, "action": &action }));
            return Ok(text);
        } else {
            let _ = app.emit(
                "channel-action-error",
                crate::jv!({ "platform": &platform, "action": &action, "message": "安装失败" }),
            );
            return Err(format!("微信插件安装失败 (exit {})\n{}", status.code().unwrap_or(-1), text));
        }
    }

    // weixin login 映射到 openclaw-weixin channel id
    let channel_id = if platform == "weixin" {
        "openclaw-weixin".to_string()
    } else {
        platform.clone()
    };

    let args: Vec<String> = match action.as_str() {
        "login" => {
            vec!["channels".into(), "login".into(), "--channel".into(), channel_id]
        }
        _ => return Err(format!("不支持的渠道动作: {}", action)),
    };

    let emit_payload = |kind: &str, message: String| {
        let payload = crate::jv!({
            "platform": platform,
            "action": action,
            "message": message,
            "kind": kind,
        });
        let _ = app.emit("channel-action-log", payload);
    };

    let progress_payload = |progress: u32| {
        let payload = crate::jv!({
            "platform": platform,
            "action": action,
            "progress": progress,
        });
        let _ = app.emit("channel-action-progress", payload);
    };

    emit_payload("info", format!("开始执行 openclaw {}", args.join(" ")));
    progress_payload(5);

    let lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let spawn_result = crate::utils::openclaw_command()
        .args(args.iter().map(|s| s.as_str()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let mut child = match spawn_result {
        Ok(child) => child,
        Err(e) => {
            let payload = crate::jv!({
                "platform": platform,
                "action": action,
                "message": format!("启动 openclaw 失败: {}", e),
            });
            let _ = app.emit("channel-action-error", payload);
            return Err(format!("启动 openclaw 失败: {}", e));
        }
    };

    let stderr = child.stderr.take();
    let app2 = app.clone();
    let platform2 = platform.clone();
    let action2 = action.clone();
    let err_lines = lines.clone();
    let handle = std::thread::spawn(move || {
        if let Some(pipe) = stderr {
            for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                if let Ok(mut guard) = err_lines.lock() {
                    guard.push(line.clone());
                }
                let payload = crate::jv!({
                    "platform": platform2,
                    "action": action2,
                    "message": line,
                    "kind": "stderr",
                });
                let _ = app2.emit("channel-action-log", payload);
            }
        }
    });

    let mut progress = 15;
    if let Some(pipe) = child.stdout.take() {
        for line in BufReader::new(pipe).lines().map_while(Result::ok) {
            if let Ok(mut guard) = lines.lock() {
                guard.push(line.clone());
            }
            let payload = crate::jv!({
                "platform": platform,
                "action": action,
                "message": line,
                "kind": "stdout",
            });
            let _ = app.emit("channel-action-log", payload);
            if progress < 90 {
                progress += 5;
                progress_payload(progress);
            }
        }
    }

    let _ = handle.join();
    let status = child.wait().map_err(|e| format!("等待命令结束失败: {}", e))?;
    let message = lines
        .lock()
        .ok()
        .map(|guard| {
            let text = guard.join("\n");
            if text.trim().is_empty() {
                "操作完成".to_string()
            } else {
                text
            }
        })
        .unwrap_or_else(|| "操作完成".into());

    if status.success() {
        // 微信登录成功后写入 channels.openclaw-weixin.enabled 以便 list_configured_platforms 检测
        if platform == "weixin" && action == "login" {
            if let Ok(mut cfg) = super::config::load_openclaw_json() {
                let channels = cfg
                    .as_object_mut()
                    .map(|r| r.entry("channels").or_insert_with(|| crate::jv!({})))
                    .and_then(|c| c.as_object_mut());
                if let Some(ch) = channels {
                    let entry = ch.entry("openclaw-weixin").or_insert_with(|| crate::jv!({}));
                    if let Some(obj) = entry.as_object_mut() {
                        obj.insert("enabled".into(), crate::jv!(true));
                    }
                    let _ = super::config::save_openclaw_json(&cfg);
                }
            }
        }

        progress_payload(100);
        let payload = crate::jv!({
            "platform": platform,
            "action": action,
            "message": message,
        });
        let _ = app.emit("channel-action-done", payload);
        Ok(message)
    } else {
        let payload = crate::jv!({
            "platform": platform,
            "action": action,
            "message": message,
        });
        let _ = app.emit("channel-action-error", payload);
        Err(message)
    }
}
