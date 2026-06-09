async fn start_service_impl_internal(label: &str, app: Option<&tauri::AppHandle>) -> Result<(), String> {
    match start_service_impl_internal_once(label).await {
        Ok(()) => Ok(()),
        Err(err) => match try_auto_fix_gateway_config(&err, app).await {
            Ok(true) => {
                guardian_log("自动修复完成，准备重试启动 Gateway");
                emit_guardian_event(app, "auto_fix_retry", "已自动修复配置，正在重试启动 Gateway…");
                #[cfg(target_os = "windows")]
                {
                    platform::cleanup_zombie_gateway_processes();
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
                match start_service_impl_internal_once(label).await {
                    Ok(()) => {
                        emit_guardian_event(app, "auto_fix_success", "已自动修复配置并成功重试启动 Gateway。");
                        Ok(())
                    }
                    Err(retry_err) => {
                        // 二级回退：doctor --fix 没解决问题，尝试直接修改 JSON
                        if looks_like_gateway_config_mismatch(&retry_err) {
                            guardian_log("doctor --fix 后仍失败，尝试直接修复 openclaw.json");
                            match try_direct_config_strip() {
                                Ok(true) => {
                                    emit_guardian_event(app, "auto_fix_retry", "已直接修复配置文件，正在再次重试启动 Gateway…");
                                    #[cfg(target_os = "windows")]
                                    {
                                        platform::cleanup_zombie_gateway_processes();
                                    }
                                    tokio::time::sleep(Duration::from_millis(500)).await;
                                    match start_service_impl_internal_once(label).await {
                                        Ok(()) => {
                                            emit_guardian_event(app, "auto_fix_success", "已直接修复配置并成功启动 Gateway。");
                                            return Ok(());
                                        }
                                        Err(e) => {
                                            emit_guardian_event(app, "auto_fix_failure", format!("直接修复后仍启动失败：{e}"));
                                        }
                                    }
                                }
                                Ok(false) => {
                                    guardian_log("直接修复未找到可清理的配置项");
                                }
                                Err(e) => {
                                    guardian_log(&format!("直接修复失败: {e}"));
                                }
                            }
                        }
                        emit_guardian_event(
                            app,
                            "auto_fix_failure",
                            format!("已自动执行 openclaw doctor --fix 并重试启动 Gateway，但仍失败：{retry_err}"),
                        );
                        Err(format!("{retry_err}\n（已自动执行 openclaw doctor --fix + 直接修复并重试启动 Gateway）"))
                    }
                }
            }
            Ok(false) => Err(err),
            Err(fix_err) => Err(format!("{err}\n{fix_err}")),
        },
    }
}

async fn start_service_impl_internal_once(label: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        platform::start_service_impl(label)?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        platform::start_service_impl(label).await?;
    }
    wait_for_gateway_running(label, Duration::from_secs(15)).await
}

async fn stop_service_impl_internal(label: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        platform::stop_service_impl(label)?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        platform::stop_service_impl(label).await?;
    }
    wait_for_gateway_stopped(label, Duration::from_secs(10)).await
}

async fn restart_service_impl_internal(label: &str, app: Option<&tauri::AppHandle>) -> Result<(), String> {
    stop_service_impl_internal(label).await?;
    start_service_impl_internal(label, app).await
}

pub fn start_backend_guardian(app: tauri::AppHandle) {
    if GUARDIAN_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    // Windows 重启后清理残留的僵尸 Gateway 进程（防止多进程堆积）
    #[cfg(target_os = "windows")]
    {
        platform::cleanup_zombie_gateway_processes();
    }

    guardian_log("后端守护循环已启动");
    tauri::async_runtime::spawn(async move {
        loop {
            guardian_tick(&app).await;
            tokio::time::sleep(GUARDIAN_INTERVAL).await;
        }
    });
}

pub fn invalidate_cli_detection_cache() {
    platform::invalidate_cli_detection_cache();
}

#[tauri::command]
pub fn guardian_status() -> Result<GuardianStatus, String> {
    Ok(guardian_snapshot())
}

// ===== 跨平台公共接口 =====

/// 跨平台统一的服务状态检测：纯 TCP 端口连通性（macOS/Linux 使用）
#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
fn check_tcp_service_status(_uid: u32, _label: &str) -> (bool, Option<u32>) {
    let port = crate::commands::gateway_listen_port();
    let addr = format!("127.0.0.1:{port}");
    let socket_addr = match addr.parse() {
        Ok(a) => a,
        Err(_) => return (false, None),
    };
    match std::net::TcpStream::connect_timeout(&socket_addr, Duration::from_secs(1)) {
        Ok(_) => (true, None),
        Err(_) => (false, None),
    }
}

#[tauri::command]
pub async fn get_services_status() -> Result<Vec<ServiceStatus>, String> {
    let _uid = platform::current_uid()?;
    let labels = platform::scan_service_labels();
    let desc_map = description_map();
    let cli_installed = platform::is_cli_installed();

    let mut results = Vec::new();
    for label in labels.iter().map(String::as_str) {
        let (running, pid) = current_gateway_runtime(label).await;
        let ready = running && gateway_health_ready(Duration::from_secs(2)).await;
        let owner = read_gateway_owner();
        let mut owned_by_current_instance = running
            && owner
                .as_ref()
                .map(|record| is_current_gateway_owner(record, pid))
                .unwrap_or(false);
        if owned_by_current_instance {
            if let Some(record) = owner.as_ref() {
                if gateway_owner_pid_needs_refresh(record, pid) {
                    let _ = write_gateway_owner(pid);
                }
            }
        }
        // 自动认领：Gateway 在运行但无有效 owner，且端口 + 数据目录匹配 → 自动写入 owner
        if running && !owned_by_current_instance && should_auto_claim_gateway(&owner) {
            let _ = write_gateway_owner(pid);
            owned_by_current_instance = true;
        }
        let ownership = if !running {
            Some("stopped".to_string())
        } else if owned_by_current_instance {
            Some("owned".to_string())
        } else {
            Some("foreign".to_string())
        };
        results.push(ServiceStatus {
            label: label.to_string(),
            pid,
            running,
            ready: Some(ready),
            description: desc_map.get(label).unwrap_or(&"").to_string(),
            cli_installed,
            ownership,
            owned_by_current_instance: Some(owned_by_current_instance),
        });
    }

    Ok(results)
}

#[tauri::command]
pub async fn start_service(app: tauri::AppHandle, label: String) -> Result<(), String> {
    let (running, pid) = current_gateway_runtime(&label).await;
    if running {
        ensure_owned_gateway_or_err(pid)?;
        if wait_for_gateway_health(Duration::from_secs(150)).await {
            write_gateway_owner(pid)?;
            guardian_mark_manual_start();
            return Ok(());
        }
        return Err(format!(
            "Gateway 端口 {} 已监听，但 /health 长时间无响应，请重启 Gateway 或查看日志",
            crate::commands::gateway_listen_port()
        ));
    }
    guardian_mark_manual_start();
    start_service_impl_internal(&label, Some(&app)).await
}

#[tauri::command]
pub async fn stop_service(label: String) -> Result<(), String> {
    let (running, pid) = current_gateway_runtime(&label).await;
    if running {
        ensure_owned_gateway_or_err(pid)?;
    }
    guardian_mark_manual_stop();
    stop_service_impl_internal(&label).await
}

#[tauri::command]
pub async fn restart_service(app: tauri::AppHandle, label: String) -> Result<(), String> {
    let (running, pid) = current_gateway_runtime(&label).await;
    if running {
        ensure_owned_gateway_or_err(pid)?;
    }
    guardian_pause("manual restart");
    guardian_mark_manual_start();
    let result = restart_service_impl_internal(&label, Some(&app)).await;
    guardian_resume("manual restart");
    result
}

/// 认领外部 Gateway：将 gateway-owner.json 强制覆写为当前面板实例签名
#[tauri::command]
pub async fn claim_gateway() -> Result<(), String> {
    let (running, pid) = current_gateway_runtime("ai.openclaw.gateway").await;
    if !running {
        return Err("Gateway 未运行，无需认领".into());
    }
    write_gateway_owner(pid)?;
    Ok(())
}

/// 轻量 TCP 端口探测：检测 Gateway 端口是否可连通（用于 WS 连接前的就绪等待）
#[tauri::command]
pub async fn probe_gateway_port() -> bool {
    gateway_health_ready(Duration::from_secs(2)).await
}