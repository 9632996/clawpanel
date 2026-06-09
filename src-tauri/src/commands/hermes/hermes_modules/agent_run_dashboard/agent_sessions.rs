#[derive(serde::Deserialize, Clone)]
pub struct HermesAttachment {
    pub kind: String,
    pub mime: String,
    /// 原始文件名（前端可选传入，用于日志/调试展示）— 当前未读取，保留供后续展开附件清单 UI 使用
    #[serde(default)]
    #[allow(dead_code)]
    pub name: Option<String>,
    /// base64 编码的内容（不含 data:image/...,base64, 前缀，仅纯 base64）
    pub data_base64: String,
}

/// 构造 OpenAI 多模态 content：[{type:"text"}, {type:"image_url"}, ...]
fn build_multimodal_input(text: &str, attachments: &[HermesAttachment]) -> Value {
    let mut parts: Vec<Value> = Vec::new();
    parts.push(crate::jv!({ "type": "text", "text": text }));
    for a in attachments {
        if a.kind == "image" {
            let url = format!("data:{};base64,{}", a.mime, a.data_base64);
            parts.push(crate::jv!({
                "type": "image_url",
                "image_url": { "url": url },
            }));
        }
    }
    Value::Array(parts)
}

#[tauri::command]
pub async fn hermes_agent_run(
    app: tauri::AppHandle,
    input: String,
    session_id: Option<String>,
    conversation_history: Option<Value>,
    instructions: Option<String>,
    attachments: Option<Vec<HermesAttachment>>,
) -> Result<String, String> {
    let gw_url = hermes_gateway_url();
    let runs_url = format!("{gw_url}/v1/runs");

    ensure_managed_gateway_ready(&app, &gw_url).await?;

    // 读取 API_SERVER_KEY
    let home = hermes_home();
    let api_key = {
        let env_path = home.join(".env");
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

    // Batch 3 §K: 有 attachments 时 input 改成多模态格式
    let mut payload = if let Some(atts) = attachments.as_ref().filter(|v| !v.is_empty()) {
        crate::jv!({ "input": build_multimodal_input(&input, atts) })
    } else {
        crate::jv!({ "input": input })
    };
    if let Some(sid) = &session_id {
        payload["session_id"] = Value::String(sid.clone());
    }
    if let Some(hist) = &conversation_history {
        payload["conversation_history"] = hist.clone();
    }
    if let Some(inst) = &instructions {
        payload["instructions"] = Value::String(inst.clone());
    }

    // 优先 /v1/runs：该端点显式支持 body.session_id，按 client 传的 session id 复用 session，
    // 避免 Hermes 服务端 `sessions list` 中每条消息生成一个新 session（issue #275）。
    // /v1/responses 会忽略 body.session_id 并对每次请求新建 session_id，所以不作为主路径。
    let client =
        hermes_gateway_http_client(std::time::Duration::from_secs(10)).map_err(|e| format!("HTTP 客户端创建失败: {e}"))?;

    // 1. POST /v1/runs → 获取 run_id
    let mut req = client
        .post(&runs_url)
        .header("Content-Type", "application/json")
        .body(payload.to_string());
    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {api_key}"));
    }

    let resp = match req.send().await {
        Ok(resp) => resp,
        Err(error) => {
            return Err(hermes_run_failure_message("启动 run 失败", &gw_url, reqwest_error_detail(&error)).await);
        }
    };
    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        // 404 → 老版本 Hermes Agent 没有 /v1/runs，降级到 /v1/responses 兼容
        // （代价：session 会暴增，但至少能用；建议用户升级 Hermes Agent）
        if status == 404 {
            if let Some(response_run_id) =
                try_hermes_responses_run(&app, &gw_url, &api_key, &payload, session_id.as_deref()).await?
            {
                return Ok(response_run_id);
            }
        }
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {status}: {text}"));
    }
    let body: Value = resp.json().await.map_err(|e| format!("解析响应失败: {e}"))?;
    let run_id = body["run_id"].as_str().ok_or("响应中没有 run_id")?.to_string();

    let _ = app.emit("hermes-run-started", crate::jv!({ "run_id": &run_id }));

    // 2. GET /v1/runs/{run_id}/events — SSE 事件流
    let events_url = format!("{gw_url}/v1/runs/{run_id}/events");
    let sse_client =
        hermes_gateway_http_client(std::time::Duration::from_secs(300)).map_err(|e| format!("SSE 客户端创建失败: {e}"))?;

    let mut sse_req = sse_client.get(&events_url);
    if !api_key.is_empty() {
        sse_req = sse_req.header("Authorization", format!("Bearer {api_key}"));
    }

    let sse_resp = match sse_req.send().await {
        Ok(resp) => resp,
        Err(error) => {
            return Err(hermes_run_failure_message("SSE 连接失败", &gw_url, reqwest_error_detail(&error)).await);
        }
    };

    if !sse_resp.status().is_success() {
        let status = sse_resp.status().as_u16();
        let text = sse_resp.text().await.unwrap_or_default();
        return Err(format!("SSE HTTP {status}: {text}"));
    }

    use futures_util::StreamExt;
    let mut stream = sse_resp.bytes_stream();
    let mut buffer = String::new();
    let mut final_output = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("SSE 读取失败: {e}"))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(newline_pos) = buffer.find('\n') {
            let line = buffer[..newline_pos].to_string();
            buffer = buffer[newline_pos + 1..].to_string();

            let trimmed = line.trim();
            let data = if let Some(rest) = trimmed.strip_prefix("data:") {
                rest.trim()
            } else if trimmed.starts_with('{') {
                trimmed
            } else {
                continue;
            };
            if data.is_empty() || data == "[DONE]" {
                let _ = app.emit(
                    "hermes-run-done",
                    crate::jv!({
                        "run_id": &run_id,
                        "output": &final_output,
                    }),
                );
                return Ok(run_id);
            }

            if let Ok(evt) = serde_json::from_str::<Value>(data) {
                if let Some(normalized) = normalize_hermes_stream_event(&evt, &run_id, session_id.as_deref()) {
                    if emit_hermes_stream_event(&app, normalized, &run_id, &mut final_output)? {
                        return Ok(run_id);
                    }
                }
            }
        }
    }

    let _ = app.emit(
        "hermes-run-done",
        crate::jv!({
            "run_id": &run_id,
            "output": &final_output,
        }),
    );
    Ok(run_id)
}

// ---------------------------------------------------------------------------
// Hermes Sessions / Logs / Skills / Memory — 文件系统 + CLI 命令
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn hermes_sessions_list(
    source: Option<String>,
    limit: Option<usize>,
    profile: Option<String>,
) -> Result<Value, String> {
    let mut args: Vec<String> = Vec::new();
    if let Some(p) = profile.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        args.push("--profile".into());
        args.push(p.to_string());
    }
    args.extend(["sessions", "export", "-"].iter().map(|s| s.to_string()));
    if let Some(s) = source.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        args.push("--source".into());
        args.push(s.to_string());
    }
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = match run_silent("hermes", &refs) {
        Ok(s) => s,
        Err(_) => return Ok(crate::jv!([])),
    };
    let mut sessions: Vec<Value> = Vec::new();
    for line in output.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if let Ok(obj) = serde_json::from_str::<Value>(t) {
            // Extra numeric fields for Usage analytics. Carry through as-is so
            // the frontend can aggregate without another round-trip. Missing
            // fields fall back to 0 / null rather than breaking the shape.
            //
            // `started_at` is a POSIX seconds timestamp produced by the
            // official Hermes CLI export. We also surface it under that name
            // (matching the web UI contract) so the Usage store can group
            // sessions by day without needing a separate parse.
            let started_at = obj.get("started_at").and_then(|v| v.as_u64()).unwrap_or_else(|| {
                // Fallback: parse `created_at` as ISO8601 → epoch seconds.
                obj.get("created_at")
                    .and_then(|v| v.as_str())
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.timestamp() as u64)
                    .unwrap_or(0)
            });
            sessions.push(crate::jv!({
                "id": obj.get("session_id").or(obj.get("id")).and_then(|v| v.as_str()).unwrap_or(""),
                "title": obj.get("title").or(obj.get("name")).and_then(|v| v.as_str()).unwrap_or(""),
                "source": obj.get("source").and_then(|v| v.as_str()).unwrap_or(""),
                "model": obj.get("model").and_then(|v| v.as_str()).unwrap_or(""),
                "created_at": obj.get("created_at").or(obj.get("createdAt")).and_then(|v| v.as_str()).unwrap_or(""),
                "updated_at": obj.get("updated_at").or(obj.get("updatedAt")).and_then(|v| v.as_str()).unwrap_or(""),
                "message_count": obj.get("message_count").and_then(|v| v.as_u64()).unwrap_or(0),
                // --- Usage analytics fields ---
                "started_at": started_at,
                "input_tokens": obj.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                "output_tokens": obj.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                "cache_read_tokens": obj.get("cache_read_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                "cache_write_tokens": obj.get("cache_write_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                "estimated_cost_usd": obj.get("estimated_cost_usd").and_then(|v| v.as_f64()),
                "actual_cost_usd": obj.get("actual_cost_usd").and_then(|v| v.as_f64()),
            }));
        }
    }
    sessions.sort_by(|a, b| {
        let ca = a["created_at"].as_str().unwrap_or("");
        let cb = b["created_at"].as_str().unwrap_or("");
        cb.cmp(ca)
    });
    if let Some(lim) = limit {
        if lim > 0 {
            sessions.truncate(lim);
        }
    }
    Ok(Value::Array(sessions))
}