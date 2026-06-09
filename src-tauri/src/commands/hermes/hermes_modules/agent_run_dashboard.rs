
async fn try_hermes_responses_run(
    app: &tauri::AppHandle,
    gw_url: &str,
    api_key: &str,
    payload: &Value,
    session_id: Option<&str>,
) -> Result<Option<String>, String> {
    let client =
        hermes_gateway_http_client(std::time::Duration::from_secs(300)).map_err(|e| format!("HTTP 客户端创建失败: {e}"))?;
    let mut response_payload = payload.clone();
    response_payload["stream"] = Value::Bool(true);
    let mut req = client
        .post(format!("{gw_url}/v1/responses"))
        .header("Content-Type", "application/json")
        .body(response_payload.to_string());
    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {api_key}"));
    }
    let resp = match req.send().await {
        Ok(resp) => resp,
        Err(_) => return Ok(None),
    };
    let status = resp.status();
    if !status.is_success() {
        if status.as_u16() == 401 || status.as_u16() == 403 {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {text}", status.as_u16()));
        }
        return Ok(None);
    }
    let run_id = resp
        .headers()
        .get("x-request-id")
        .or_else(|| resp.headers().get("x-response-id"))
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .unwrap_or_else(|| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or_default();
            format!("response-{now}")
        });
    let _ = app.emit("hermes-run-started", crate::jv!({ "run_id": &run_id }));
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    if content_type.contains("application/json") {
        let body: Value = resp.json().await.unwrap_or(Value::Null);
        let output = hermes_response_text(&body);
        let _ = app.emit(
            "hermes-run-done",
            crate::jv!({
                "run_id": &run_id,
                "output": output,
            }),
        );
        return Ok(Some(run_id));
    }

    use futures_util::StreamExt;
    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();
    let mut final_output = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("SSE 读取失败: {e}"))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(newline_pos) = buffer.find('\n') {
            let line = buffer[..newline_pos].trim().to_string();
            buffer = buffer[newline_pos + 1..].to_string();
            let data = if let Some(rest) = line.strip_prefix("data:") {
                rest.trim()
            } else if line.starts_with('{') {
                line.as_str()
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
                return Ok(Some(run_id));
            }
            if let Ok(evt) = serde_json::from_str::<Value>(data) {
                if let Some(normalized) = normalize_hermes_stream_event(&evt, &run_id, session_id) {
                    if emit_hermes_stream_event(app, normalized, &run_id, &mut final_output)? {
                        return Ok(Some(run_id));
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
    Ok(Some(run_id))
}

/// 读取 Hermes API_SERVER_KEY（从 ~/.hermes/.env），与 hermes_agent_run 共用。
fn read_hermes_api_key() -> String {
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
}

// ---------------------------------------------------------------------------
// Batch 1 §D: hermes_run_stop — 真正中断 run（POST /v1/runs/{run_id}/stop）
//
// 原本 chat-store 的 stopStreaming() 只 abort 本地 SSE，后端 agent 继续跑完
// 「Stop 假停」问题：从 hermes 源码确认真实端点是 /v1/runs/{run_id}/stop（用 run_id 不是 session_id）。
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn hermes_run_stop(run_id: String) -> Result<Value, String> {
    if run_id.is_empty() {
        return Err("run_id 不能为空".to_string());
    }
    let gw_url = hermes_gateway_url();
    let url = format!("{gw_url}/v1/runs/{run_id}/stop");
    let api_key = read_hermes_api_key();
    let client =
        hermes_gateway_http_client(std::time::Duration::from_secs(5)).map_err(|e| format!("HTTP 客户端创建失败: {e}"))?;
    let mut req = client.post(&url);
    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {api_key}"));
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("stop 请求失败: {}", reqwest_error_detail(&e)))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("stop 失败 HTTP {}: {}", status.as_u16(), body));
    }
    Ok(resp.json::<Value>().await.unwrap_or(crate::jv!({ "ok": true })))
}

// ---------------------------------------------------------------------------
// Batch 1 §C-bis: hermes_run_approval — 批准/拒绝 Hermes 内核的工具调用
//
// Hermes 跑高危工具（terminal / code_execution）默认是 ask once 模式，
// 触发 approval.request SSE 事件，前端要弹给用户 4 个选项：
//   - "once"    一次性批准（默认）
//   - "session" 本 session 内都批准
//   - "always"  全局总是批准（极少用）
//   - "deny"    拒绝（run 会被 cancelled）
//
// 端点：POST /v1/runs/{run_id}/approval { choice }
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn hermes_run_approval(run_id: String, choice: String) -> Result<Value, String> {
    if run_id.is_empty() {
        return Err("run_id 不能为空".to_string());
    }
    let normalized_choice = match choice.as_str() {
        "once" | "session" | "always" | "deny" => choice,
        other => return Err(format!("approval choice 必须是 once/session/always/deny，收到 {other}")),
    };
    let gw_url = hermes_gateway_url();
    let url = format!("{gw_url}/v1/runs/{run_id}/approval");
    let api_key = read_hermes_api_key();
    let client =
        hermes_gateway_http_client(std::time::Duration::from_secs(5)).map_err(|e| format!("HTTP 客户端创建失败: {e}"))?;
    let mut req = client.post(&url).json(&crate::jv!({ "choice": normalized_choice }));
    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {api_key}"));
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("approval 请求失败: {}", reqwest_error_detail(&e)))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("approval 失败 HTTP {}: {}", status.as_u16(), body));
    }
    Ok(resp.json::<Value>().await.unwrap_or(crate::jv!({ "ok": true })))
}

// ---------------------------------------------------------------------------
// Batch 2 §I: hermes_run_status — 查 run 当前状态（流恢复用）
//
// GET /v1/runs/{run_id} 返回 { run_id, status, last_event, output?, ... }
// status 取值：running / stopping / completed / failed / cancelled / waiting_for_approval
// 切页 / 刷新后用这个判断是否还需要重连 SSE 事件流
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn hermes_run_status(run_id: String) -> Result<Value, String> {
    if run_id.is_empty() {
        return Err("run_id 不能为空".to_string());
    }
    let gw_url = hermes_gateway_url();
    let url = format!("{gw_url}/v1/runs/{run_id}");
    let api_key = read_hermes_api_key();
    let client =
        hermes_gateway_http_client(std::time::Duration::from_secs(5)).map_err(|e| format!("HTTP 客户端创建失败: {e}"))?;
    let mut req = client.get(&url);
    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {api_key}"));
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("status 请求失败: {}", reqwest_error_detail(&e)))?;
    let status = resp.status();
    if status.as_u16() == 404 {
        // run 已过期或不存在 — 返回明确状态而不是错
        return Ok(crate::jv!({ "run_id": run_id, "status": "not_found" }));
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("status 失败 HTTP {}: {}", status.as_u16(), body));
    }
    resp.json::<Value>().await.map_err(|e| format!("解析 JSON 失败: {e}"))
}

// ---------------------------------------------------------------------------
// Batch 1 §E: hermes_session_export — 导出会话消息（走 dashboard 9119）
//
// 校对稿订正：不走 CLI `hermes sessions export`，直接调
// `GET http://127.0.0.1:{dashboard_port}/api/sessions/{session_id}/messages`
// 拿 JSON 后由前端打包下载（避免 CLI 子进程开销 + Web 模式不可达）。
//
// 注意：dashboard server 需要先启动（用户没启的话调 hermes_dashboard_start）
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn hermes_session_export(app: tauri::AppHandle, session_id: String) -> Result<Value, String> {
    if session_id.is_empty() {
        return Err("session_id 不能为空".to_string());
    }
    let port = ensure_managed_dashboard_ready(&app).await?;
    let url = format!("http://127.0.0.1:{port}/api/sessions/{session_id}/messages");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP 客户端创建失败: {e}"))?;

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("export 请求失败: {}", reqwest_error_detail(&e)))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("export 失败 HTTP {}: {}", status.as_u16(), body));
    }
    // 让前端拿原始 JSON 自己打包下载（保留完整结构）
    resp.json::<Value>().await.map_err(|e| format!("解析 JSON 失败: {e}"))
}

// ---------------------------------------------------------------------------
// Batch 2 §H 基础设施: hermes_dashboard_api_proxy
//
// 通用 Dashboard 9119 HTTP 代理 — 让前端直接调任意 /api/* 端点。
// Profiles / Kanban / OAuth / Sessions（高级）等都走这一个入口，
// 避免给每个端点都写专属 Tauri 命令。
//
// 与 hermes_api_proxy 区别：
//   - hermes_api_proxy 走 Gateway 8642（含 API_SERVER_KEY 认证）
//   - hermes_dashboard_api_proxy 走 Dashboard 9119（无需 token，本地绑定）
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Dashboard session token 缓存
//
// Hermes Dashboard 9119 大部分 /api/* 端点需要 token 鉴权（_require_token）。
// token 来源：进程启动时 secrets.token_urlsafe(32) 生成，注入到 SPA HTML 的
//   <script>window.__HERMES_SESSION_TOKEN__="..."</script>
// 没有公开获取 API，只能 GET / 抓 HTML 提取。
//
// 缓存策略：
//   - 全局静态 Mutex<Option<String>> 保存
//   - 401 时 invalidate 重抓一次（dashboard 进程重启会重生成 token）
// ---------------------------------------------------------------------------

use std::sync::Mutex;
static DASHBOARD_SESSION_TOKEN: Mutex<Option<String>> = Mutex::new(None);

async fn fetch_dashboard_session_token(port: u16) -> Result<String, String> {
    let url = format!("http://127.0.0.1:{port}/");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("HTTP 客户端创建失败: {e}"))?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("拉 dashboard 首页失败: {}", reqwest_error_detail(&e)))?;
    if !resp.status().is_success() {
        return Err(format!("dashboard 首页 HTTP {}", resp.status().as_u16()));
    }
    let html = resp.text().await.unwrap_or_default();
    // 正则匹配 window.__HERMES_SESSION_TOKEN__="..."
    // 用简单的字符串搜索避免引入 regex crate（已有 regex 依赖但保持简单）
    let needle = "window.__HERMES_SESSION_TOKEN__=\"";
    if let Some(start) = html.find(needle) {
        let after = &html[start + needle.len()..];
        if let Some(end) = after.find('"') {
            let token = &after[..end];
            if !token.is_empty() {
                return Ok(token.to_string());
            }
        }
    }
    Err("无法从 dashboard HTML 提取 session token（dashboard 可能未启动）".to_string())
}

async fn dashboard_session_token(_port: u16, _force_refresh: bool) -> Result<String, String> {
    Ok(HERMES_DASHBOARD_SESSION_TOKEN.to_string())
}

#[allow(dead_code)]
async fn dashboard_session_token_from_html(port: u16, force_refresh: bool) -> Result<String, String> {
    if !force_refresh {
        if let Ok(guard) = DASHBOARD_SESSION_TOKEN.lock() {
            if let Some(t) = guard.as_ref() {
                return Ok(t.clone());
            }
        }
    }
    let token = fetch_dashboard_session_token(port).await?;
    if let Ok(mut guard) = DASHBOARD_SESSION_TOKEN.lock() {
        *guard = Some(token.clone());
    }
    Ok(token)
}

#[tauri::command]
pub async fn hermes_dashboard_api_proxy(
    app: tauri::AppHandle,
    method: String,
    path: String,
    body: Option<String>,
    headers: Option<Value>,
) -> Result<Value, String> {
    let port = ensure_managed_dashboard_ready(&app).await?;
    let url = format!("http://127.0.0.1:{port}{path}");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP 客户端创建失败: {e}"))?;

    let build_request = |token_opt: Option<&str>| -> Result<reqwest::RequestBuilder, String> {
        let mut req = match method.to_uppercase().as_str() {
            "GET" => client.get(&url),
            "POST" => client.post(&url),
            "PUT" => client.put(&url),
            "PATCH" => client.patch(&url),
            "DELETE" => client.delete(&url),
            _ => return Err(format!("不支持的方法: {method}")),
        };
        // 自动注入 session token
        if let Some(tok) = token_opt {
            req = req.header("X-Hermes-Session-Token", tok);
        }
        // 自定义 headers
        if let Some(Value::Object(map)) = headers.as_ref() {
            for (k, v) in map.iter() {
                if let Some(s) = v.as_str() {
                    req = req.header(k, s);
                }
            }
        }
        // body
        if let Some(b) = body.as_ref() {
            req = req.header("Content-Type", "application/json").body(b.clone());
        }
        Ok(req)
    };

    // 拿缓存的 token（首次为空，让 send 触发 401 再抓）
    let mut token = dashboard_session_token(port, false).await.ok();
    let resp = build_request(token.as_deref())?
        .send()
        .await
        .map_err(|e| format!("Dashboard 请求失败: {}", reqwest_error_detail(&e)))?;

    let status = resp.status();
    if status.as_u16() == 401 {
        // token 失效或没拿到 — 强制刷新 + 重试一次
        token = Some(dashboard_session_token(port, true).await?);
        let retry = build_request(token.as_deref())?
            .send()
            .await
            .map_err(|e| format!("Dashboard 重试失败: {}", reqwest_error_detail(&e)))?;
        let retry_status = retry.status();
        let body = retry.text().await.unwrap_or_default();
        if !retry_status.is_success() {
            return Err(format!("HTTP {}: {}", retry_status.as_u16(), body));
        }
        return Ok(serde_json::from_str::<Value>(&body).unwrap_or(Value::String(body)));
    }

    let resp_body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("HTTP {}: {}", status.as_u16(), resp_body));
    }
    Ok(serde_json::from_str::<Value>(&resp_body).unwrap_or(Value::String(resp_body)))
}

// Batch 3 §K: 多模态附件结构
//
// 前端传过来的附件描述（图片用 base64 直传）。
// 支持 kind="image"（暂时只接图片，文件附件留作后续）。
include!("agent_run_dashboard/agent_sessions.rs");