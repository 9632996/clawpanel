use serde_json::Value;
use std::collections::HashMap;

use super::config_model_common::{
    model_api_key_env_ref_from_value, normalize_base_url_for_api, normalize_model_api_type, resolve_model_api_key_value,
    strip_config_value,
};
use super::config_model_response::{
    extract_error_message, extract_openai_assistant_response, extract_single_json_reply, extract_sse_reply,
};

pub(super) fn parse_simple_config_blocks(raw: &str) -> HashMap<String, HashMap<String, String>> {
    let mut blocks: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current = String::from("");
    blocks.entry(current.clone()).or_default();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current = trimmed.trim_matches(&['[', ']'][..]).trim().to_string();
            blocks.entry(current.clone()).or_default();
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        blocks
            .entry(current.clone())
            .or_default()
            .insert(key.trim().to_string(), strip_config_value(value));
    }
    blocks
}

fn model_entry_id(entry: &Value) -> Option<&str> {
    entry
        .as_str()
        .or_else(|| entry.get("id").and_then(|v| v.as_str()))
        .or_else(|| entry.get("name").and_then(|v| v.as_str()))
        .map(str::trim)
        .filter(|v| !v.is_empty())
}

fn first_provider_model_id(provider: &Value) -> Option<String> {
    provider
        .get("models")
        .and_then(|v| v.as_array())
        .and_then(|models| models.iter().filter_map(model_entry_id).next())
        .map(str::to_string)
}

fn provider_model_api(provider: &Value, model_id: &str) -> Option<String> {
    provider
        .get("models")
        .and_then(|v| v.as_array())
        .and_then(|models| {
            models.iter().find_map(|entry| {
                if model_entry_id(entry)? == model_id {
                    entry.get("api").and_then(|v| v.as_str()).map(|v| v.trim().to_string())
                } else {
                    None
                }
            })
        })
        .filter(|v| !v.is_empty())
}

#[tauri::command]
pub fn get_assistant_default_model_config() -> Result<Value, String> {
    let config = crate::commands::config::load_openclaw_json()?;
    let Some(providers) = config.pointer("/models/providers").and_then(|value| value.as_object()) else {
        return Ok(crate::jv!({
            "configured": false,
            "reason": "openclaw.json 未配置 models.providers"
        }));
    };

    let primary = config
        .pointer("/agents/defaults/model/primary")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let mut selected: Option<(String, String)> = primary.and_then(|value| {
        let (provider_key, model_id) = value.split_once('/')?;
        let provider_key = provider_key.trim();
        let model_id = model_id.trim();
        if provider_key.is_empty() || model_id.is_empty() {
            None
        } else {
            Some((provider_key.to_string(), model_id.to_string()))
        }
    });

    if selected.is_none() {
        selected = providers.iter().find_map(|(provider_key, provider)| {
            first_provider_model_id(provider).map(|model_id| (provider_key.clone(), model_id))
        });
    }

    let Some((provider_key, model_id)) = selected else {
        return Ok(crate::jv!({
            "configured": false,
            "reason": "openclaw.json 未配置默认模型"
        }));
    };

    let Some(provider) = providers.get(&provider_key) else {
        return Ok(crate::jv!({
            "configured": false,
            "reason": format!("默认模型服务商不存在: {provider_key}")
        }));
    };

    let base_url = provider
        .get("baseUrl")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("");
    if base_url.is_empty() {
        return Ok(crate::jv!({
            "configured": false,
            "providerKey": provider_key,
            "model": model_id,
            "reason": "默认模型服务商未配置 baseUrl"
        }));
    }

    let raw_api_type = provider_model_api(provider, &model_id)
        .or_else(|| {
            provider
                .get("api")
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
        })
        .unwrap_or_else(|| "openai-completions".to_string());
    let api_type = normalize_model_api_type(&raw_api_type);

    let (api_key, api_key_source) = if let Some(api_key_value) = provider.get("apiKey") {
        let source = model_api_key_env_ref_from_value(api_key_value)?
            .map(|key| format!("env:{key}"))
            .unwrap_or_else(|| "openclaw".to_string());
        match resolve_model_api_key_value(api_key_value) {
            Ok(value) => (value, source),
            Err(err) => {
                return Ok(crate::jv!({
                    "configured": false,
                    "providerKey": provider_key,
                    "model": model_id,
                    "baseUrl": normalize_base_url_for_api(base_url, api_type),
                    "apiType": api_type,
                    "apiKeySource": source,
                    "reason": err
                }));
            }
        }
    } else {
        (String::new(), String::new())
    };

    Ok(crate::jv!({
        "configured": true,
        "providerKey": provider_key,
        "model": model_id,
        "modelRef": format!("{provider_key}/{model_id}"),
        "baseUrl": normalize_base_url_for_api(base_url, api_type),
        "apiType": api_type,
        "apiKey": api_key,
        "apiKeySource": api_key_source
    }))
}

#[tauri::command]
pub async fn test_model(base_url: String, api_key: Value, model_id: String, api_type: Option<String>) -> Result<String, String> {
    let api_type = normalize_model_api_type(api_type.as_deref().unwrap_or("openai-completions"));
    let base = normalize_base_url_for_api(&base_url, api_type);
    let api_key = resolve_model_api_key_value(&api_key)?;

    let client = crate::commands::build_http_client_no_proxy(std::time::Duration::from_secs(30), None)
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let resp = match api_type {
        "anthropic-messages" => {
            let url = format!("{}/messages", base);
            let body = crate::jv!({
                "model": model_id,
                "messages": [{"role": "user", "content": "Hi"}],
                "max_tokens": 16,
            });
            let mut req = client.post(&url).header("anthropic-version", "2023-06-01").json(&body);
            if !api_key.is_empty() {
                req = req.header("x-api-key", api_key.clone());
            }
            req.send()
        }
        "google-gemini" => {
            let url = format!("{}/models/{}:generateContent?key={}", base, model_id, api_key);
            let body = crate::jv!({
                "contents": [{"role": "user", "parts": [{"text": "Hi"}]}]
            });
            client.post(&url).json(&body).send()
        }
        _ => {
            let url = format!("{}/chat/completions", base);
            let body = crate::jv!({
                "model": model_id,
                "messages": [{"role": "user", "content": "Hi"}],
                "max_tokens": 16,
                "stream": false
            });
            let mut req = client.post(&url).json(&body);
            if !api_key.is_empty() {
                req = req.header("Authorization", format!("Bearer {api_key}"));
            }
            req.send()
        }
    }
    .await
    .map_err(|e| {
        if e.is_timeout() {
            "请求超时 (30s)".to_string()
        } else if e.is_connect() {
            format!("连接失败: {e}")
        } else {
            format!("请求失败: {e}")
        }
    })?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        let msg = extract_error_message(&text, status);
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(msg);
        }
        return Ok(format!("连接正常（API 返回 {status}，部分模型对简单测试不兼容，不影响实际使用）\n{msg}"));
    }

    let reply = extract_single_json_reply(&text);
    if reply.is_empty() {
        Ok("（模型已响应）".into())
    } else {
        Ok(reply)
    }
}

#[tauri::command]
pub async fn test_model_verbose(
    base_url: String,
    api_key: Value,
    model_id: String,
    api_type: Option<String>,
) -> Result<Value, String> {
    let api_type = normalize_model_api_type(api_type.as_deref().unwrap_or("openai-completions"));
    let base = normalize_base_url_for_api(&base_url, api_type);
    let api_key = resolve_model_api_key_value(&api_key)?;
    let client = crate::commands::build_http_client_no_proxy(std::time::Duration::from_secs(30), None)
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    let start = std::time::Instant::now();

    let (used_api, req_url, req_body_json, req_builder) = match api_type {
        "anthropic-messages" => {
            let url = format!("{}/messages", base);
            let body = crate::jv!({
                "model": model_id,
                "messages": [{"role": "user", "content": "你好，请用一句话回复"}],
                "max_tokens": 200,
            });
            let mut req = client
                .post(&url)
                .header("anthropic-version", "2023-06-01")
                .header("Accept-Encoding", "identity")
                .json(&body);
            if !api_key.is_empty() {
                req = req.header("x-api-key", api_key.clone());
            }
            ("Anthropic Messages", url, body, req)
        }
        "google-gemini" => {
            let url_display = format!("{}/models/{}:generateContent?key=***", base, model_id);
            let url_real = format!("{}/models/{}:generateContent?key={}", base, model_id, api_key);
            let body = crate::jv!({
                "contents": [{"role": "user", "parts": [{"text": "你好，请用一句话回复"}]}]
            });
            let req = client.post(&url_real).header("Accept-Encoding", "identity").json(&body);
            ("Gemini", url_display, body, req)
        }
        _ => {
            let url = format!("{}/chat/completions", base);
            let body = crate::jv!({
                "model": model_id,
                "messages": [{"role": "user", "content": "你好，请用一句话回复"}],
                "max_tokens": 200,
                "stream": true
            });
            let mut req = client
                .post(&url)
                .header("Accept-Encoding", "identity")
                .header("Accept", "text/event-stream")
                .json(&body);
            if !api_key.is_empty() {
                req = req.header("Authorization", format!("Bearer {api_key}"));
            }
            ("Chat Completions (SSE)", url, body, req)
        }
    };

    let resp_result = req_builder.send().await;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    let resp = match resp_result {
        Ok(r) => r,
        Err(e) => {
            let error = if e.is_timeout() {
                "请求超时 (30s)".to_string()
            } else if e.is_connect() {
                format!("连接失败: {e}")
            } else {
                format!("请求失败: {e}")
            };
            return Ok(crate::jv!({
                "success": false,
                "status": 0,
                "reqUrl": req_url,
                "reqBody": req_body_json,
                "respBody": "",
                "reply": "",
                "error": error,
                "elapsedMs": elapsed_ms,
                "usedApi": used_api,
            }));
        }
    };

    let status = resp.status();
    let status_code = status.as_u16();
    let resp_headers = {
        let mut map = serde_json::Map::new();
        for (k, v) in resp.headers().iter() {
            map.insert(k.to_string(), serde_json::Value::String(v.to_str().unwrap_or("<non-utf8>").to_string()));
        }
        serde_json::Value::Object(map)
    };
    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            let mut err_chain = format!("{e}");
            let mut src: Option<&dyn std::error::Error> = std::error::Error::source(&e);
            while let Some(s) = src {
                err_chain.push_str(&format!(" -> {s}"));
                src = std::error::Error::source(s);
            }
            return Ok(crate::jv!({
                "success": false,
                "status": status_code,
                "reqUrl": req_url,
                "reqBody": req_body_json,
                "respHeaders": resp_headers,
                "respBody": "",
                "respRawHex": "",
                "respByteCount": 0,
                "reply": "",
                "error": format!("读取响应字节失败: {err_chain}"),
                "elapsedMs": elapsed_ms,
                "usedApi": used_api,
            }));
        }
    };
    let byte_count = bytes.len();
    let hex_preview = bytes
        .iter()
        .take(200)
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    let text = match std::str::from_utf8(&bytes) {
        Ok(s) => s.to_string(),
        Err(e) => {
            let lossy = String::from_utf8_lossy(&bytes).into_owned();
            let ascii_preview: String = bytes
                .iter()
                .take(80)
                .map(|&b| if (0x20..=0x7e).contains(&b) { b as char } else { '.' })
                .collect();
            return Ok(crate::jv!({
                "success": false,
                "status": status_code,
                "reqUrl": req_url,
                "reqBody": req_body_json,
                "respHeaders": resp_headers,
                "respBody": lossy,
                "respRawHex": hex_preview,
                "respByteCount": byte_count,
                "reply": "",
                "error": format!("响应体 UTF-8 解码失败: {e} | 字节数 {byte_count} | 前 80 字节 ASCII='{ascii_preview}'"),
                "elapsedMs": elapsed_ms,
                "usedApi": used_api,
            }));
        }
    };
    let reply = {
        let sse_reply = extract_sse_reply(&text);
        if !sse_reply.is_empty() {
            sse_reply
        } else {
            extract_single_json_reply(&text)
        }
    };
    let success = status.is_success() && !reply.is_empty();
    let error = if !status.is_success() {
        Some(extract_error_message(&text, status))
    } else if reply.is_empty() {
        Some("API 已响应但未解析出内容".to_string())
    } else {
        None
    };

    Ok(crate::jv!({
        "success": success,
        "status": status_code,
        "reqUrl": req_url,
        "reqBody": req_body_json,
        "respHeaders": resp_headers,
        "respBody": text,
        "respRawHex": hex_preview,
        "respByteCount": byte_count,
        "reply": reply,
        "error": error,
        "elapsedMs": elapsed_ms,
        "usedApi": used_api,
    }))
}

#[tauri::command]
pub async fn assistant_call_model(
    base_url: String,
    api_key: Value,
    model_id: String,
    api_type: Option<String>,
    messages: Vec<Value>,
    temperature: Option<f64>,
    tools: Option<Vec<Value>>,
) -> Result<Value, String> {
    let api_type_norm = normalize_model_api_type(api_type.as_deref().unwrap_or("openai-completions"));
    let base = normalize_base_url_for_api(&base_url, api_type_norm);
    let api_key = resolve_model_api_key_value(&api_key)?;
    let temperature = temperature.unwrap_or(0.7);

    let client = crate::commands::build_http_client_no_proxy(std::time::Duration::from_secs(120), None)
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let (used_api, req_url, req_body_json, req_builder) = match api_type_norm {
        "anthropic-messages" => {
            let url = format!("{}/messages", base);
            let system = messages
                .iter()
                .find(|message| message.get("role").and_then(|v| v.as_str()) == Some("system"))
                .and_then(|message| message.get("content").and_then(|v| v.as_str()))
                .unwrap_or("");
            let chat_messages: Vec<Value> = messages
                .iter()
                .filter(|message| message.get("role").and_then(|v| v.as_str()) != Some("system"))
                .cloned()
                .collect();
            let mut body = crate::jv!({
                "model": model_id,
                "messages": chat_messages,
                "max_tokens": 8192,
                "stream": true,
                "temperature": temperature,
            });
            if !system.is_empty() {
                body["system"] = crate::jv!(system);
            }
            let mut req = client
                .post(&url)
                .header("anthropic-version", "2023-06-01")
                .header("Accept-Encoding", "identity")
                .header("Accept", "text/event-stream")
                .json(&body);
            if !api_key.is_empty() {
                req = req.header("x-api-key", api_key.clone());
            }
            ("Anthropic Messages", url, body, req)
        }
        "google-gemini" => {
            let url_display = format!("{}/models/{}:streamGenerateContent?alt=sse&key=***", base, model_id);
            let url_real = format!("{}/models/{}:streamGenerateContent?alt=sse&key={}", base, model_id, api_key);
            let system = messages
                .iter()
                .find(|message| message.get("role").and_then(|v| v.as_str()) == Some("system"))
                .and_then(|message| message.get("content").and_then(|v| v.as_str()))
                .unwrap_or("");
            let contents: Vec<Value> = messages
                .iter()
                .filter(|message| message.get("role").and_then(|v| v.as_str()) != Some("system"))
                .map(|message| {
                    let role = if message.get("role").and_then(|v| v.as_str()) == Some("assistant") {
                        "model"
                    } else {
                        "user"
                    };
                    let content = message
                        .get("content")
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                        .unwrap_or_else(|| message.get("content").cloned().unwrap_or(Value::Null).to_string());
                    crate::jv!({ "role": role, "parts": [{ "text": content }] })
                })
                .collect();
            let mut body = crate::jv!({
                "contents": contents,
                "generationConfig": { "temperature": temperature },
            });
            if !system.is_empty() {
                body["systemInstruction"] = crate::jv!({ "parts": [{ "text": system }] });
            }
            let req = client
                .post(&url_real)
                .header("Accept-Encoding", "identity")
                .header("Accept", "text/event-stream")
                .json(&body);
            ("Gemini", url_display, body, req)
        }
        _ => {
            let url = format!("{}/chat/completions", base);
            let mut body = crate::jv!({
                "model": model_id,
                "messages": messages,
                "stream": true,
                "temperature": temperature,
            });
            if let Some(tools) = tools.filter(|items| !items.is_empty()) {
                body["tools"] = crate::jv!(tools);
            }
            let mut req = client
                .post(&url)
                .header("Accept-Encoding", "identity")
                .header("Accept", "text/event-stream")
                .json(&body);
            if !api_key.is_empty() {
                req = req.header("Authorization", format!("Bearer {api_key}"));
            }
            ("Chat Completions (SSE)", url, body, req)
        }
    };

    let resp = req_builder.send().await.map_err(|e| {
        if e.is_timeout() {
            "请求超时 (120s)".to_string()
        } else if e.is_connect() {
            format!("连接失败: {e}")
        } else {
            format!("请求失败: {e}")
        }
    })?;
    let status = resp.status();
    let status_code = status.as_u16();
    let text = resp.text().await.unwrap_or_default();

    let (reply, tool_calls, finish_reason, stream_error) = if api_type_norm == "openai-completions" {
        extract_openai_assistant_response(&text)
    } else {
        let sse_reply = extract_sse_reply(&text);
        let reply = if !sse_reply.is_empty() {
            sse_reply
        } else {
            extract_single_json_reply(&text)
        };
        (reply, Vec::new(), String::new(), String::new())
    };

    if !status.is_success() {
        return Err(format!("API 错误 {}: {}", status_code, extract_error_message(&text, status)));
    }
    if reply.is_empty() && tool_calls.is_empty() {
        let reason = if stream_error.is_empty() {
            "模型响应为空".to_string()
        } else {
            stream_error.clone()
        };
        return Err(reason);
    }

    Ok(crate::jv!({
        "success": true,
        "status": status_code,
        "reply": reply,
        "toolCalls": tool_calls,
        "finishReason": finish_reason,
        "streamError": stream_error,
        "usedApi": used_api,
        "reqUrl": req_url,
        "reqBody": req_body_json,
        "respBody": text,
    }))
}

#[tauri::command]
pub async fn list_remote_models(base_url: String, api_key: Value, api_type: Option<String>) -> Result<Vec<String>, String> {
    let api_type = normalize_model_api_type(api_type.as_deref().unwrap_or("openai-completions"));
    let base = normalize_base_url_for_api(&base_url, api_type);
    let api_key = resolve_model_api_key_value(&api_key)?;

    let client = crate::commands::build_http_client_no_proxy(std::time::Duration::from_secs(15), None)
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let resp = match api_type {
        "anthropic-messages" => {
            let url = format!("{}/models", base);
            let mut req = client.get(&url).header("anthropic-version", "2023-06-01");
            if !api_key.is_empty() {
                req = req.header("x-api-key", api_key.clone());
            }
            req.send()
        }
        "google-gemini" => {
            let url = format!("{}/models?key={}", base, api_key);
            client.get(&url).send()
        }
        _ => {
            let url = format!("{}/models", base);
            let mut req = client.get(&url);
            if !api_key.is_empty() {
                req = req.header("Authorization", format!("Bearer {api_key}"));
            }
            req.send()
        }
    }
    .await
    .map_err(|e| {
        if e.is_timeout() {
            "请求超时 (15s)，该服务商可能不支持模型列表接口".to_string()
        } else if e.is_connect() {
            format!("连接失败，请检查接口地址是否正确: {e}")
        } else {
            format!("请求失败: {e}")
        }
    })?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        let code = status.as_u16();
        if code == 404 || code == 405 || code == 501 {
            return Err("[NOT_SUPPORTED] 该服务商不支持自动获取模型列表，请手动输入模型 ID".to_string());
        }
        let msg = extract_error_message(&text, status);
        return Err(format!("获取模型列表失败: {msg}"));
    }

    let ids = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .map(|v| {
            let mut ids: Vec<String> = if let Some(data) = v.get("data").and_then(|d| d.as_array()) {
                data.iter()
                    .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
                    .collect()
            } else if let Some(data) = v.get("models").and_then(|d| d.as_array()) {
                data.iter()
                    .filter_map(|m| {
                        m.get("name")
                            .and_then(|id| id.as_str())
                            .map(|s| s.trim_start_matches("models/").to_string())
                    })
                    .collect()
            } else {
                vec![]
            };
            ids.sort();
            ids
        })
        .unwrap_or_default();

    if ids.is_empty() {
        return Err("该服务商返回了空的模型列表，可能不支持 /models 接口".to_string());
    }

    Ok(ids)
}
