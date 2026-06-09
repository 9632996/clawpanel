
#[tauri::command]
pub async fn hermes_health_check() -> Result<Value, String> {
    let url = format!("{}/health", hermes_gateway_url());

    let client =
        hermes_gateway_http_client(std::time::Duration::from_secs(5)).map_err(|e| format!("HTTP 客户端创建失败: {e}"))?;

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let body: Value = resp.json().await.unwrap_or(Value::Null);
            Ok(body)
        }
        Ok(resp) => Err(format!("Gateway 返回 HTTP {}", resp.status())),
        Err(e) => Err(format!("Gateway 不可达: {e}")),
    }
}

// ---------------------------------------------------------------------------
// hermes_capabilities — 探测 Gateway 暴露的 API 能力描述（GET /v1/capabilities）
//
// Hermes 内核 v2026.5.x 起暴露的「机器可读 capability 描述」，给外部 UI 用来
// 动态适配可用功能，避免在前端写死哪些 endpoint/feature 存在。例：
// 老版本的 Gateway 没有 `/v1/runs/{id}/approval`，新版有 → 用 capabilities 判
// 断而不是用版本号匹配。
//
// 不可达 / 老版 Gateway 没有该 endpoint → 返回 Err，调用方应优雅降级。
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn hermes_capabilities() -> Result<Value, String> {
    let url = format!("{}/v1/capabilities", hermes_gateway_url());

    let client =
        hermes_gateway_http_client(std::time::Duration::from_secs(5)).map_err(|e| format!("HTTP 客户端创建失败: {e}"))?;

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let body: Value = resp.json().await.unwrap_or(Value::Null);
            Ok(body)
        }
        Ok(resp) => Err(format!("Gateway 返回 HTTP {}", resp.status())),
        Err(e) => Err(format!("Gateway 不可达: {e}")),
    }
}

// ---------------------------------------------------------------------------
// hermes_detect_environments — 检测 WSL2 / Docker 中的 Hermes Agent
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn hermes_detect_environments() -> Result<Value, String> {
    let mut result = crate::jv!({
        "wsl2": { "available": false },
        "docker": { "available": false },
    });

    // --- WSL2 检测（仅 Windows）---
    #[cfg(target_os = "windows")]
    {
        // 1. 检测 WSL 是否安装
        let wsl_check = std::process::Command::new("wsl")
            .args(["--list", "--quiet"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        if let Ok(out) = wsl_check {
            if out.status.success() {
                let distros_raw = String::from_utf8_lossy(&out.stdout);
                let distros: Vec<String> = distros_raw
                    .lines()
                    .map(|l| l.trim().replace('\0', "").trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect();

                if !distros.is_empty() {
                    result["wsl2"]["available"] = crate::jv!(true);
                    result["wsl2"]["distros"] = crate::jv!(distros);

                    // 2. 获取默认 WSL2 IP
                    let ip_cmd = std::process::Command::new("wsl")
                        .args(["-e", "hostname", "-I"])
                        .creation_flags(CREATE_NO_WINDOW)
                        .output();
                    if let Ok(ip_out) = ip_cmd {
                        if ip_out.status.success() {
                            let ip_str = String::from_utf8_lossy(&ip_out.stdout);
                            let ip = ip_str.split_whitespace().next().unwrap_or("").to_string();
                            if !ip.is_empty() {
                                result["wsl2"]["ip"] = crate::jv!(ip);
                            }
                        }
                    }

                    // 3. 检测 WSL 里是否安装了 hermes
                    let hermes_check = std::process::Command::new("wsl")
                        .args([
                            "-e",
                            "bash",
                            "-lc",
                            "command -v hermes && hermes --version 2>/dev/null || echo NOT_FOUND",
                        ])
                        .creation_flags(CREATE_NO_WINDOW)
                        .output();
                    if let Ok(h_out) = hermes_check {
                        let h_str = String::from_utf8_lossy(&h_out.stdout).trim().to_string();
                        if !h_str.contains("NOT_FOUND") && !h_str.is_empty() {
                            result["wsl2"]["hermesInstalled"] = crate::jv!(true);
                            result["wsl2"]["hermesInfo"] = crate::jv!(h_str);
                        }
                    }

                    // 4. 探测 WSL 中 Gateway 是否正在运行
                    let wsl_ip = result["wsl2"]["ip"].as_str().map(String::from);
                    if let Some(ip) = wsl_ip {
                        let port = hermes_gateway_port();
                        let addr_str = format!("{ip}:{port}");
                        if let Ok(addr) = addr_str.parse::<std::net::SocketAddr>() {
                            let reachable =
                                std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(500)).is_ok();
                            result["wsl2"]["gatewayRunning"] = crate::jv!(reachable);
                            if reachable {
                                result["wsl2"]["gatewayUrl"] = crate::jv!(format!("http://{ip}:{port}"));
                            }
                        }
                    }
                }
            }
        }
    }

    // --- Docker 检测（所有平台）---
    {
        let docker_check = {
            let mut cmd = std::process::Command::new("docker");
            cmd.args(["info", "--format", "{{.ServerVersion}}"]);
            #[cfg(target_os = "windows")]
            cmd.creation_flags(CREATE_NO_WINDOW);
            cmd.output()
        };

        if let Ok(out) = docker_check {
            if out.status.success() {
                let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
                result["docker"]["available"] = crate::jv!(true);
                result["docker"]["version"] = crate::jv!(version);

                // 查找运行中的 hermes 相关容器
                let ps_cmd = {
                    let mut cmd = std::process::Command::new("docker");
                    cmd.args([
                        "ps",
                        "--format",
                        "{{.ID}}\t{{.Names}}\t{{.Image}}\t{{.Ports}}\t{{.Status}}",
                        "--filter",
                        "status=running",
                    ]);
                    #[cfg(target_os = "windows")]
                    cmd.creation_flags(CREATE_NO_WINDOW);
                    cmd.output()
                };

                if let Ok(ps_out) = ps_cmd {
                    let ps_str = String::from_utf8_lossy(&ps_out.stdout);
                    let containers: Vec<Value> = ps_str
                        .lines()
                        .filter(|l| {
                            let lower = l.to_lowercase();
                            lower.contains("hermes") || lower.contains("8642")
                        })
                        .map(|l| {
                            let parts: Vec<&str> = l.split('\t').collect();
                            crate::jv!({
                                "id": parts.first().unwrap_or(&""),
                                "name": parts.get(1).unwrap_or(&""),
                                "image": parts.get(2).unwrap_or(&""),
                                "ports": parts.get(3).unwrap_or(&""),
                                "status": parts.get(4).unwrap_or(&""),
                            })
                        })
                        .collect();

                    if !containers.is_empty() {
                        result["docker"]["hermesContainers"] = crate::jv!(containers);
                    }
                }
            }
        }
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// hermes_set_gateway_url — 设置自定义 Gateway URL（用于远程/WSL2/Docker）
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn hermes_set_gateway_url(url: Option<String>) -> Result<String, String> {
    let config_paths = super::panel_config_candidate_paths();
    let config_path = config_paths.first().ok_or("找不到配置文件路径")?;

    let mut config = if config_path.exists() {
        let content = std::fs::read_to_string(config_path).map_err(|e| format!("读取配置失败: {e}"))?;
        serde_json::from_str::<Value>(&content).unwrap_or_else(|_| crate::jv!({}))
    } else {
        crate::jv!({})
    };

    // 确保 hermes 对象存在
    if !config.get("hermes").is_some_and(|v| v.is_object()) {
        config["hermes"] = crate::jv!({});
    }

    match &url {
        Some(u) if !u.trim().is_empty() => {
            config["hermes"]["gatewayUrl"] = crate::jv!(u.trim());
        }
        _ => {
            // 清除自定义 URL，回退到本地
            if let Some(obj) = config["hermes"].as_object_mut() {
                obj.remove("gatewayUrl");
            }
        }
    }

    let json_str = serde_json::to_string_pretty(&config).map_err(|e| format!("序列化失败: {e}"))?;
    if let Some(parent) = config_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(config_path, json_str).map_err(|e| format!("写入配置失败: {e}"))?;

    let current_url = hermes_gateway_url();
    Ok(format!("Gateway URL 已设置: {current_url}"))
}

// ---------------------------------------------------------------------------
// update_hermes — 升级
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn update_hermes(app: tauri::AppHandle) -> Result<String, String> {
    let _ = app.emit("hermes-install-log", "📦 升级 Hermes Agent...");
    let _ = app.emit("hermes-install-progress", 0u32);

    let uv_path = uv_bin_path();
    let uv = if uv_path.exists() {
        uv_path.to_string_lossy().to_string()
    } else {
        "uv".into()
    };

    let pkg = format!("hermes-agent[web] @ {}", HERMES_GIT_URL);
    let mut cmd = tokio::process::Command::new(&uv);
    cmd.args(["tool", "install", "--reinstall", &pkg, "--python", "3.11"]);
    append_hermes_runtime_extras(&mut cmd);
    apply_hermes_runtime_env_tokio(&mut cmd);
    let _ = app.emit("hermes-install-progress", 20u32);
    let _ = app.emit(
        "hermes-install-log",
        format!(
            "uv tool install --reinstall hermes-agent --python 3.11 {}",
            hermes_runtime_extras_log_segment()
        ),
    );
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    if let Some(mirror) = pypi_mirror_url() {
        cmd.args(["--index-url", &mirror]);
    }
    apply_git_mirror_env(&mut cmd);
    super::apply_proxy_env_tokio(&mut cmd);
    cmd.env("PATH", hermes_enhanced_path());
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output().await.map_err(|e| format!("升级失败: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    for line in stdout.lines().chain(stderr.lines()) {
        if !line.trim().is_empty() {
            let _ = app.emit("hermes-install-log", sanitize_hermes_install_output(line.trim()));
        }
    }

    if output.status.success() {
        // 注入 dashboard 兼容 stub（升级路径与安装路径保持一致，避免上游 wheel 漏装的子包再次缺失）
        inject_hermes_dashboard_compat_stub(&app);
        let _ = app.emit("hermes-install-log", "✅ 升级完成");
        let _ = app.emit("hermes-install-progress", 100u32);
        Ok("升级完成".into())
    } else {
        let cleaned = sanitize_hermes_install_output(stderr.trim());
        if let Some(hint) = diagnose_install_network_error(&cleaned) {
            let _ = app.emit("hermes-install-log", &hint);
            return Err(format!("升级失败: {}\n\n{}", cleaned, hint));
        }
        Err(format!("升级失败: {}", cleaned))
    }
}

// ---------------------------------------------------------------------------
// uninstall_hermes — 卸载
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn uninstall_hermes(app: tauri::AppHandle, clean_config: bool) -> Result<String, String> {
    let _ = app.emit("hermes-install-log", "🗑️ 卸载 Hermes Agent...");
    let _ = app.emit("hermes-install-progress", 10u32);

    let uv_path = uv_bin_path();
    let uv = if uv_path.exists() {
        uv_path.to_string_lossy().to_string()
    } else {
        "uv".into()
    };

    // uv tool uninstall
    let mut cmd = tokio::process::Command::new(&uv);
    cmd.args(["tool", "uninstall", "hermes-agent"]);
    apply_hermes_runtime_env_tokio(&mut cmd);
    let _ = app.emit("hermes-install-log", "> uv tool uninstall hermes-agent");
    cmd.env("PATH", hermes_enhanced_path());
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output().await.map_err(|e| format!("卸载失败: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    for line in stdout.lines().chain(stderr.lines()) {
        if !line.trim().is_empty() {
            let _ = app.emit("hermes-install-log", line.trim());
        }
    }

    if !output.status.success() {
        return Err(format!("卸载失败: {}", stderr.trim()));
    }
    let _ = app.emit("hermes-install-progress", 65u32);

    // 清理 venv（如果存在）
    let venv_dir = hermes_venv_dir();
    if venv_dir.exists() {
        let _ = app.emit("hermes-install-log", format!("清理虚拟环境: {}", venv_dir.display()));
        let _ = std::fs::remove_dir_all(&venv_dir);
    }

    // 可选：清理配置
    if clean_config {
        let home = hermes_home();
        if home.exists() {
            let _ = app.emit("hermes-install-log", format!("清理配置目录: {}", home.display()));
            let _ = std::fs::remove_dir_all(&home);
        }
    }

    let _ = app.emit("hermes-install-log", "✅ Hermes Agent 已卸载");
    let _ = app.emit("hermes-install-progress", 100u32);
    Ok("Hermes Agent 已卸载".into())
}

// ---------------------------------------------------------------------------
// hermes_api_proxy — 代理前端对 Gateway REST API 的请求（绕过 CORS）
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn hermes_api_proxy(
    method: String,
    path: String,
    body: Option<String>,
    headers: Option<Value>,
) -> Result<Value, String> {
    let url = format!("{}{path}", hermes_gateway_url());

    // 读取 API_SERVER_KEY
    let api_key = {
        let env_path = hermes_home().join(".env");
        let mut key = String::new();
        if let Ok(content) = std::fs::read_to_string(&env_path) {
            for line in content.lines() {
                let line = line.trim();
                if let Some(val) = line.strip_prefix("API_SERVER_KEY=") {
                    key = val.trim().to_string();
                    break;
                }
            }
        }
        key
    };

    let timeout = if path.contains("/chat/completions") || path.contains("/responses") {
        std::time::Duration::from_secs(120)
    } else {
        std::time::Duration::from_secs(30)
    };
    let client = hermes_gateway_http_client(timeout).map_err(|e| format!("HTTP 客户端创建失败: {e}"))?;

    let mut req = match method.to_uppercase().as_str() {
        "GET" => client.get(&url),
        "POST" => {
            let mut r = client.post(&url);
            if let Some(b) = &body {
                r = r.header("Content-Type", "application/json").body(b.clone());
            }
            r
        }
        "PATCH" => {
            let mut r = client.patch(&url);
            if let Some(b) = &body {
                r = r.header("Content-Type", "application/json").body(b.clone());
            }
            r
        }
        "PUT" => {
            let mut r = client.put(&url);
            if let Some(b) = &body {
                r = r.header("Content-Type", "application/json").body(b.clone());
            }
            r
        }
        "DELETE" => {
            let mut r = client.delete(&url);
            if let Some(b) = &body {
                r = r.header("Content-Type", "application/json").body(b.clone());
            }
            r
        }
        _ => return Err(format!("不支持的方法: {method}")),
    };

    // 注入 API_SERVER_KEY 认证
    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {api_key}"));
    }

    // 注入自定义 headers（如 X-Hermes-Session-Id）
    if let Some(Value::Object(map)) = &headers {
        for (k, v) in map {
            if let Some(s) = v.as_str() {
                req = req.header(k.as_str(), s);
            }
        }
    }

    let resp = req.send().await.map_err(|e| format!("Gateway 请求失败: {e}"))?;
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();

    // 尝试解析为 JSON，否则包装为字符串
    let json_val: Value = serde_json::from_str(&text).unwrap_or_else(|_| crate::jv!({ "raw": text }));

    if status >= 400 {
        // 提取错误信息：支持 {"error": "msg"} 和 {"error": {"message": "msg"}} 两种格式
        let err_msg = json_val
            .get("error")
            .and_then(|v| {
                v.as_str()
                    .map(String::from)
                    .or_else(|| v.get("message").and_then(|m| m.as_str()).map(String::from))
            })
            .unwrap_or_else(|| text.clone());
        return Err(err_msg);
    }

    Ok(json_val)
}

// ---------------------------------------------------------------------------
// hermes_agent_run — streaming compatibility layer for Hermes Agent
// ---------------------------------------------------------------------------

fn hermes_response_text(value: &Value) -> String {
    let response = value.get("response").unwrap_or(value);
    if let Some(text) = response.get("output_text").and_then(|v| v.as_str()) {
        return text.to_string();
    }
    if let Some(text) = response.get("text").and_then(|v| v.as_str()) {
        return text.to_string();
    }
    let mut out = String::new();
    if let Some(items) = response.get("output").and_then(|v| v.as_array()) {
        for item in items {
            if let Some(parts) = item.get("content").and_then(|v| v.as_array()) {
                for part in parts {
                    let kind = part.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    if matches!(kind, "output_text" | "text") {
                        if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                            out.push_str(text);
                        }
                    }
                }
            }
        }
    }
    out
}

fn hermes_response_delta(evt: &Value) -> String {
    evt.get("delta")
        .and_then(|v| v.as_str())
        .or_else(|| evt.get("text").and_then(|v| v.as_str()))
        .or_else(|| evt.get("content").and_then(|v| v.as_str()))
        .or_else(|| evt.get("delta").and_then(|v| v.get("text")).and_then(|v| v.as_str()))
        .or_else(|| evt.get("delta").and_then(|v| v.get("value")).and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string()
}

fn normalize_hermes_stream_event(evt: &Value, run_id: &str, session_id: Option<&str>) -> Option<Value> {
    let event_type = evt
        .get("event")
        .and_then(|v| v.as_str())
        .or_else(|| evt.get("type").and_then(|v| v.as_str()))
        .unwrap_or("");
    if event_type.is_empty() {
        return None;
    }
    let sid = session_id.map(|s| Value::String(s.to_string())).unwrap_or(Value::Null);
    match event_type {
        "message.delta"
        | "run.completed"
        | "run.failed"
        | "run.cancelled"
        | "tool.started"
        | "tool.completed"
        | "tool.progress"
        | "tool.error"
        | "reasoning.available"
        | "approval.request"
        | "approval.responded" => {
            let mut out = evt.clone();
            if out.get("run_id").is_none() {
                out["run_id"] = Value::String(run_id.to_string());
            }
            if out.get("session_id").is_none() {
                out["session_id"] = sid;
            }
            Some(out)
        }
        "response.output_text.delta" | "response.text.delta" => {
            let delta = hermes_response_delta(evt);
            if delta.is_empty() {
                None
            } else {
                Some(crate::jv!({
                    "event": "message.delta",
                    "run_id": run_id,
                    "session_id": sid,
                    "delta": delta,
                }))
            }
        }
        "response.output_item.added" => {
            let item = evt.get("item").or_else(|| evt.get("output_item")).unwrap_or(&Value::Null);
            let kind = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if !matches!(kind, "function_call" | "tool_call") {
                return None;
            }
            let tool = item
                .get("name")
                .and_then(|v| v.as_str())
                .or_else(|| item.get("function").and_then(|v| v.get("name")).and_then(|v| v.as_str()))
                .unwrap_or("tool");
            Some(crate::jv!({
                "event": "tool.started",
                "run_id": run_id,
                "session_id": sid,
                "tool": tool,
                "input": item.get("arguments").or_else(|| item.get("input")).cloned().unwrap_or(Value::Null),
            }))
        }
        "response.function_call_arguments.delta" => Some(crate::jv!({
            "event": "tool.progress",
            "run_id": run_id,
            "session_id": sid,
            "tool": evt.get("name").and_then(|v| v.as_str()).unwrap_or("tool"),
            "preview": hermes_response_delta(evt),
        })),
        "response.output_item.done" | "response.function_call_arguments.done" => {
            let item = evt.get("item").or_else(|| evt.get("output_item")).unwrap_or(&Value::Null);
            let kind = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if event_type == "response.output_item.done" && !matches!(kind, "function_call" | "tool_call") {
                return None;
            }
            Some(crate::jv!({
                "event": "tool.completed",
                "run_id": run_id,
                "session_id": sid,
                "tool": item.get("name").and_then(|v| v.as_str()).or_else(|| evt.get("name").and_then(|v| v.as_str())).unwrap_or("tool"),
                "input": item.get("arguments").or_else(|| evt.get("arguments")).cloned().unwrap_or(Value::Null),
            }))
        }
        "response.completed" => Some(crate::jv!({
            "event": "run.completed",
            "run_id": run_id,
            "session_id": sid,
            "output": hermes_response_text(evt),
        })),
        "response.failed" | "response.error" => Some(crate::jv!({
            "event": "run.failed",
            "run_id": run_id,
            "session_id": sid,
            "error": evt.get("error").and_then(|v| v.get("message")).and_then(|v| v.as_str())
                .or_else(|| evt.get("error").and_then(|v| v.as_str()))
                .or_else(|| evt.get("message").and_then(|v| v.as_str()))
                .unwrap_or("unknown error"),
        })),
        _ => {
            let mut out = evt.clone();
            out["event"] = Value::String(event_type.to_string());
            if out.get("run_id").is_none() {
                out["run_id"] = Value::String(run_id.to_string());
            }
            if out.get("session_id").is_none() {
                out["session_id"] = sid;
            }
            Some(out)
        }
    }
}

fn emit_hermes_stream_event(app: &tauri::AppHandle, evt: Value, run_id: &str, final_output: &mut String) -> Result<bool, String> {
    let event_type = evt["event"].as_str().unwrap_or("");
    match event_type {
        "message.delta" => {
            if let Some(delta) = evt["delta"].as_str() {
                final_output.push_str(delta);
                let _ = app.emit(
                    "hermes-run-delta",
                    crate::jv!({
                        "run_id": run_id,
                        "delta": delta,
                    }),
                );
            }
        }
        "tool.started" | "tool.completed" | "tool.progress" | "tool.error" => {
            let _ = app.emit("hermes-run-tool", evt.clone());
        }
        "reasoning.available" => {
            let _ = app.emit("hermes-run-reasoning", evt.clone());
        }
        // Batch 1 §C 新增：Approval Flow 4 类真实事件（已用源码 api_server.py 确认）
        "approval.request" => {
            let _ = app.emit("hermes-run-approval-request", evt.clone());
        }
        "approval.responded" => {
            let _ = app.emit("hermes-run-approval-responded", evt.clone());
        }
        "run.cancelled" => {
            let _ = app.emit("hermes-run-cancelled", evt.clone());
            // 中断也是终态 — 让流循环可以 return Ok(true) 结束读
            return Ok(true);
        }
        "run.completed" => {
            if let Some(output) = evt["output"].as_str() {
                if !output.is_empty() {
                    *final_output = output.to_string();
                }
            }
            let _ = app.emit(
                "hermes-run-done",
                crate::jv!({
                    "run_id": run_id,
                    "output": final_output.as_str(),
                }),
            );
            return Ok(true);
        }
        "run.failed" => {
            let err = evt["error"].as_str().unwrap_or("unknown error");
            let _ = app.emit(
                "hermes-run-error",
                crate::jv!({
                    "run_id": run_id,
                    "error": err,
                }),
            );
            return Err(format!("Agent run failed: {err}"));
        }
        _ => {
            let _ = app.emit("hermes-run-event", evt.clone());
        }
    }
    Ok(false)
}
