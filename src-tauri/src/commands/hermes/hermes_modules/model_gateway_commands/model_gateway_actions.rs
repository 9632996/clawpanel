#[tauri::command]
pub async fn hermes_update_model(model: String, provider: Option<String>) -> Result<String, String> {
    use super::hermes_providers;

    let home = hermes_home();
    let config_path = home.join("config.yaml");
    let config_raw = std::fs::read_to_string(&config_path).map_err(|e| format!("读取 config.yaml 失败: {e}"))?;

    let model_str = model.clone();

    // Provider 决定策略：
    //   1. 调用方显式提供 → 直接使用
    //   2. 从静态 catalog 反查唯一匹配 → 使用反查结果
    //   3. 找不到 / 模糊 → 保持现有 provider（不改）
    let resolved_provider: Option<String> =
        provider.or_else(|| hermes_providers::find_provider_by_model(&model).map(String::from));

    // 一次性扫描并替换 model 区块中的 default / provider 字段。
    let lines: Vec<&str> = config_raw.lines().collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len() + 1);
    let mut in_model = false;
    let mut default_written = false;
    let mut provider_written = false;
    let mut default_indent: String = "  ".into();

    for line in lines.iter() {
        let trimmed = line.trim();
        if trimmed.starts_with("model:") {
            in_model = true;
            out.push(line.to_string());
            continue;
        }
        if in_model {
            let is_indented = line.starts_with("  ") || line.starts_with('\t');
            if !is_indented && !trimmed.is_empty() && !trimmed.starts_with('#') {
                // 离开 model 区块 —— 先补齐未写入的 provider 行
                if let Some(pid) = resolved_provider.as_ref() {
                    if !provider_written && !pid.is_empty() && pid != "custom" {
                        out.push(format!("{default_indent}provider: {pid}"));
                        provider_written = true;
                    }
                }
                in_model = false;
                out.push(line.to_string());
                continue;
            }

            if trimmed.starts_with("default:") {
                let indent_len = line.len() - line.trim_start().len();
                default_indent = " ".repeat(indent_len);
                out.push(format!("{default_indent}default: {model_str}"));
                default_written = true;
                continue;
            }
            if trimmed.starts_with("provider:") {
                if let Some(pid) = resolved_provider.as_ref() {
                    if !pid.is_empty() && pid != "custom" {
                        let indent_len = line.len() - line.trim_start().len();
                        let indent = " ".repeat(indent_len);
                        out.push(format!("{indent}provider: {pid}"));
                        provider_written = true;
                        continue;
                    }
                    // custom → 删除 provider 行
                    continue;
                }
                // 未提供新 provider，保留旧值
                out.push(line.to_string());
                provider_written = true;
                continue;
            }
            // 与 Hermes 内核 8ac351407 保持一致：切模型时清掉旧 context_length，
            // 否则新模型会沿用上一个模型的 context window（典型表现：context 报错
            // / 输出被截断）。删除该行即可，Hermes 会按新模型默认窗口生效。
            if trimmed.starts_with("context_length:") {
                continue;
            }
        }
        out.push(line.to_string());
    }

    // 文件末尾还在 model 块里：补 provider 行
    if in_model {
        if let Some(pid) = resolved_provider.as_ref() {
            if !provider_written && !pid.is_empty() && pid != "custom" {
                out.push(format!("{default_indent}provider: {pid}"));
            }
        }
    }

    if !default_written {
        return Err("config.yaml 中未找到 model.default 字段".into());
    }

    let mut new_content = out.join("\n");
    if !new_content.ends_with('\n') {
        new_content.push('\n');
    }

    std::fs::write(&config_path, new_content).map_err(|e| format!("写入 config.yaml 失败: {e}"))?;
    let _ = sanitize_hermes_openrouter_custom_mismatch()?;
    Ok(format!("模型已切换为 {model_str}"))
}

// ---------------------------------------------------------------------------
// hermes_gateway_action — Gateway 管理
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn hermes_gateway_action(app: tauri::AppHandle, action: String) -> Result<String, String> {
    let enhanced = hermes_enhanced_path();
    match action.as_str() {
        "start" => {
            // Guardian: ensure platforms.api_server.enabled:true is present
            // before every start. Auto-heal if missing (with a .bak backup).
            // See `ensure_api_server_enabled` for rationale.
            ensure_api_server_enabled(&app)?;
            let _ = sanitize_hermes_openrouter_custom_mismatch()?;
            if gateway_quick_health_check().await {
                start_guardian(&app);
                emit_gateway_status(true);
                return Ok("Gateway 已在运行".into());
            }
            let _start_guard = if let Some(guard) = try_gateway_start_guard() {
                guard
            } else {
                for _ in 0..40 {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    if gateway_quick_health_check().await {
                        start_guardian(&app);
                        emit_gateway_status(true);
                        return Ok("Gateway 已在运行".into());
                    }
                }
                return Err("Gateway 正在启动中，请稍后重试".into());
            };
            if gateway_quick_health_check().await {
                start_guardian(&app);
                emit_gateway_status(true);
                return Ok("Gateway 已在运行".into());
            }

            #[cfg(target_os = "windows")]
            {
                let home = hermes_home();
                let port = hermes_gateway_port();
                let addr: std::net::SocketAddr = match format!("127.0.0.1:{port}").parse() {
                    Ok(addr) => addr,
                    Err(_) => return Err("解析 Hermes Gateway 地址失败".into()),
                };

                // 1. 如果端口已经可达，说明 Gateway 已在运行
                if std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(300)).is_ok() {
                    // 即使已在运行也启动 Guardian 守护
                    start_guardian(&app);
                    emit_gateway_status(true);
                    return Ok("Gateway 已在运行".into());
                }

                // 2. 先精准杀掉之前我们 spawn 的进程
                kill_gateway_pid();
                cleanup_stale_gateway_runtime_files(&home);
                // 如果仍有残留（非我们启动的），再 taskkill
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                if std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(200)).is_ok() {
                    // 端口仍被占用，有残留进程
                    let _ = std::process::Command::new("taskkill")
                        .args(["/F", "/IM", "hermes.exe"])
                        .creation_flags(CREATE_NO_WINDOW)
                        .output();
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }

                // 3. 清理过期 PID 文件（绕过 Hermes Windows bug）
                let pid_file = home.join("gateway.pid");
                if pid_file.exists() {
                    let _ = std::fs::remove_file(&pid_file);
                }

                // 4. 启动 Gateway 进程
                let log_path = home.join("gateway-run.log");
                let log_file = std::fs::File::create(&log_path).map_err(|e| format!("创建日志文件失败: {e}"))?;
                let log_err = log_file.try_clone().map_err(|e| format!("克隆日志句柄失败: {e}"))?;

                let hermes_cmd = hermes_executable_path().unwrap_or_else(|| PathBuf::from("hermes"));
                let mut cmd = std::process::Command::new(&hermes_cmd);
                cmd.args(["gateway", "run"])
                    .current_dir(&home)
                    .env("PATH", &enhanced)
                    .stdin(std::process::Stdio::null())
                    .stdout(log_file)
                    .stderr(log_err)
                    .creation_flags(CREATE_NO_WINDOW);
                apply_hermes_runtime_env_std(&mut cmd);
                // 注入 .env 环境变量
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
                match cmd.spawn() {
                    Ok(child) => {
                        // 记录 PID 供后续精准 kill
                        GW_PID.store(child.id(), Ordering::SeqCst);

                        // 5. 等待 Gateway 端口可达（最多 20s）
                        let mut ok = false;
                        for i in 0..40 {
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            if std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(500)).is_ok() {
                                ok = true;
                                break;
                            }
                            // 前 3 秒每次都检查，之后检查日志是否有错误
                            if i > 6 {
                                if let Ok(log) = std::fs::read_to_string(&log_path) {
                                    if log.contains("failed to connect") || log.contains("Port") && log.contains("already in use")
                                    {
                                        break; // 进程已报错，不再等待
                                    }
                                }
                            }
                        }
                        if ok {
                            // 启动 Guardian 后台守护
                            start_guardian(&app);
                            emit_gateway_status(true);
                            Ok("Gateway 已启动".into())
                        } else {
                            let log_tail = std::fs::read_to_string(&log_path).unwrap_or_default();
                            let tail: String = log_tail
                                .lines()
                                .rev()
                                .take(20)
                                .collect::<Vec<_>>()
                                .into_iter()
                                .rev()
                                .collect::<Vec<_>>()
                                .join("\n");
                            Err(format!(
                                "Gateway 启动失败。\n日志:\n{}",
                                if tail.is_empty() { "(日志为空)".to_string() } else { tail }
                            ))
                        }
                    }
                    Err(e) => Err(format!("启动 hermes gateway run 失败: {e}")),
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                let home = hermes_home();
                // 先精准杀掉之前我们 spawn 的进程
                kill_gateway_pid();
                cleanup_stale_gateway_runtime_files(&home);

                let hermes_cmd = hermes_executable_path().unwrap_or_else(|| PathBuf::from("hermes"));
                let mut cmd = std::process::Command::new(&hermes_cmd);
                cmd.args(["gateway", "run"])
                    .current_dir(&home)
                    .env("PATH", &enhanced)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null());
                apply_hermes_runtime_env_std(&mut cmd);

                // 注入 .env 环境变量
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

                match cmd.spawn() {
                    Ok(child) => {
                        GW_PID.store(child.id(), Ordering::SeqCst);
                        // 等待端口可达（最多 15s）
                        let port = hermes_gateway_port();
                        let addr: std::net::SocketAddr = match format!("127.0.0.1:{port}").parse() {
                            Ok(addr) => addr,
                            Err(_) => return Err("解析 Hermes Gateway 地址失败".into()),
                        };
                        let mut ok = false;
                        for _ in 0..30 {
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            if std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(500)).is_ok() {
                                ok = true;
                                break;
                            }
                        }
                        if ok {
                            start_guardian(&app);
                            emit_gateway_status(true);
                            Ok("Gateway 已启动".into())
                        } else {
                            Err("Gateway 启动后端口未就绪".into())
                        }
                    }
                    Err(e) => {
                        // fallback: hermes gateway start
                        let mut fallback = tokio::process::Command::new("hermes");
                        fallback.args(["gateway", "start"]).env("PATH", &enhanced);
                        apply_hermes_runtime_env_tokio(&mut fallback);
                        let out = fallback
                            .output()
                            .await
                            .map_err(|e2| format!("启动失败: {e} / fallback: {e2}"))?;
                        if out.status.success() {
                            start_guardian(&app);
                            emit_gateway_status(true);
                            Ok("Gateway 已启动".into())
                        } else {
                            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                            Err(if stderr.is_empty() {
                                format!("Gateway 启动失败 (exit {})", out.status.code().unwrap_or(-1))
                            } else {
                                stderr
                            })
                        }
                    }
                }
            }
        }
        "stop" => {
            // 停止 Guardian 守护
            stop_guardian();

            // 1. 先精准杀掉我们 spawn 的进程
            let killed = kill_gateway_pid();

            // 2. 尝试 hermes gateway stop（作为补充）
            let mut cmd = tokio::process::Command::new("hermes");
            cmd.args(["gateway", "stop"]).env("PATH", &enhanced);
            apply_hermes_runtime_env_tokio(&mut cmd);
            #[cfg(target_os = "windows")]
            cmd.creation_flags(CREATE_NO_WINDOW);
            let stop_result = cmd.output().await;

            // 3. 如果以上都没成功，Windows 上 taskkill 兜底
            #[cfg(target_os = "windows")]
            if !killed {
                let port = hermes_gateway_port();
                let addr: std::net::SocketAddr = match format!("127.0.0.1:{port}").parse() {
                    Ok(addr) => addr,
                    Err(_) => return Err("解析 Hermes Gateway 地址失败".into()),
                };
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                if std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(300)).is_ok() {
                    let _ = std::process::Command::new("taskkill")
                        .args(["/F", "/IM", "hermes.exe"])
                        .creation_flags(CREATE_NO_WINDOW)
                        .output();
                }
            }

            emit_gateway_status(false);

            match stop_result {
                Ok(out) if out.status.success() || killed => Ok("Gateway 已停止".into()),
                Ok(_) if killed => Ok("Gateway 已停止".into()),
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    if stderr.is_empty() {
                        Ok("Gateway 已停止".into())
                    } else {
                        Err(stderr)
                    }
                }
                Err(_) if killed => Ok("Gateway 已停止".into()),
                Err(e) => Err(format!("停止失败: {e}")),
            }
        }
        "status" => {
            let mut cmd = tokio::process::Command::new("hermes");
            cmd.args(["gateway", "status"]).env("PATH", &enhanced);
            apply_hermes_runtime_env_tokio(&mut cmd);
            #[cfg(target_os = "windows")]
            cmd.creation_flags(CREATE_NO_WINDOW);
            let out = cmd.output().await.map_err(|e| format!("查询失败: {e}"))?;
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            Ok(stdout)
        }
        "install" => {
            let mut cmd = tokio::process::Command::new("hermes");
            cmd.args(["gateway", "install"]).env("PATH", &enhanced);
            apply_hermes_runtime_env_tokio(&mut cmd);
            #[cfg(target_os = "windows")]
            cmd.creation_flags(CREATE_NO_WINDOW);
            let out = cmd.output().await.map_err(|e| format!("安装失败: {e}"))?;
            if out.status.success() {
                Ok("Gateway 服务已安装".into())
            } else {
                Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
            }
        }
        "uninstall" => {
            let mut cmd = tokio::process::Command::new("hermes");
            cmd.args(["gateway", "uninstall"]).env("PATH", &enhanced);
            apply_hermes_runtime_env_tokio(&mut cmd);
            #[cfg(target_os = "windows")]
            cmd.creation_flags(CREATE_NO_WINDOW);
            let out = cmd.output().await.map_err(|e| format!("卸载失败: {e}"))?;
            if out.status.success() {
                Ok("Gateway 服务已卸载".into())
            } else {
                Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
            }
        }
        _ => Err(format!("不支持的操作: {action}")),
    }
}

// ---------------------------------------------------------------------------
// hermes_health_check — Gateway 健康检查
// ---------------------------------------------------------------------------