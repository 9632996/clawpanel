    pub async fn stop_service_impl(_label: &str) -> Result<(), String> {
        let port = crate::commands::gateway_listen_port();

        // 端口不通 → 已停止
        if !check_service_status(0, "").0 {
            cleanup_legacy_gateway_window();
            // 清空已记录的 PID
            {
                let mut known = LAST_KNOWN_GATEWAY_PID.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                *known = None;
            }
            {
                let mut active = ACTIVE_GATEWAY_CHILD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                *active = None;
            }
            return Ok(());
        }

        // 先尝试 openclaw gateway stop
        let _ = crate::utils::openclaw_command_async()
            .args(["gateway", "stop"])
            .output()
            .await;

        for _ in 0..10 {
            tokio::time::sleep(Duration::from_millis(300)).await;
            if !check_service_status(0, "").0 {
                cleanup_legacy_gateway_window();
                let mut known = LAST_KNOWN_GATEWAY_PID.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                *known = None;
                let mut active = ACTIVE_GATEWAY_CHILD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                *active = None;
                return Ok(());
            }
        }

        // 精确 kill：只杀 Gateway 进程，不杀所有 node.exe
        // 1. 用记录的活跃子进程 PID
        let pids_to_kill: Vec<u32> = {
            let active = ACTIVE_GATEWAY_CHILD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let known = LAST_KNOWN_GATEWAY_PID.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            [active.as_ref(), known.as_ref()].into_iter().flatten().copied().collect()
        };

        for &pid in &pids_to_kill {
            if pid > 0 && is_process_alive(pid) {
                kill_process_tree(pid);
            }
        }

        // 2. 再用 netstat 找当前端口上的 Gateway PID（兜底）
        if let Some(gw_pid) = get_gateway_pid_by_port(port) {
            if !pids_to_kill.contains(&gw_pid) {
                kill_process_tree(gw_pid);
            }
        }

        cleanup_legacy_gateway_window();

        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if !check_service_status(0, "").0 {
                // 清空记录
                let mut known = LAST_KNOWN_GATEWAY_PID.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                *known = None;
                let mut active = ACTIVE_GATEWAY_CHILD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                *active = None;
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }

        Err("停止 Gateway 失败，请手动检查进程".into())
    }

    #[allow(dead_code)]
    pub async fn restart_service_impl(_label: &str) -> Result<(), String> {
        stop_service_impl(_label).await?;
        start_service_impl(_label).await
    }