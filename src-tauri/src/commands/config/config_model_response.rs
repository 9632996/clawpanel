use serde_json::Value;

pub(super) fn extract_error_message(text: &str, status: reqwest::StatusCode) -> String {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|v| {
            v.get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .map(String::from)
                .or_else(|| v.get("message").and_then(|m| m.as_str()).map(String::from))
        })
        .unwrap_or_else(|| format!("HTTP {status}"))
}

pub(super) fn extract_sse_reply(text: &str) -> String {
    let mut content = String::new();
    let mut reasoning = String::new();
    let mut saw_data_line = false;
    for line in text.lines() {
        let data = if let Some(rest) = line.strip_prefix("data: ") {
            rest
        } else if let Some(rest) = line.strip_prefix("data:") {
            rest
        } else {
            continue;
        };
        saw_data_line = true;
        let data = data.trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
            let delta = v.get("choices").and_then(|c| c.get(0)).and_then(|c| c.get("delta"));
            if let Some(d) = delta {
                if let Some(c) = d.get("content").and_then(|c| c.as_str()) {
                    content.push_str(c);
                }
                if let Some(rc) = d.get("reasoning_content").and_then(|c| c.as_str()) {
                    reasoning.push_str(rc);
                }
            }
            if v.get("type").and_then(|t| t.as_str()) == Some("content_block_delta") {
                if let Some(c) = v.get("delta").and_then(|d| d.get("text")).and_then(|t| t.as_str()) {
                    content.push_str(c);
                }
            }
        }
    }
    if !saw_data_line {
        return String::new();
    }
    if !content.is_empty() {
        content
    } else if !reasoning.is_empty() {
        format!("[reasoning] {reasoning}")
    } else {
        String::new()
    }
}

pub(super) fn extract_single_json_reply(text: &str) -> String {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|v| {
            if let Some(arr) = v.get("content").and_then(|c| c.as_array()) {
                let text = arr
                    .iter()
                    .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
                    .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                    .collect::<Vec<_>>()
                    .join("");
                if !text.is_empty() {
                    return Some(text);
                }
            }
            if let Some(t) = v
                .get("candidates")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("content"))
                .and_then(|c| c.get("parts"))
                .and_then(|p| p.get(0))
                .and_then(|p| p.get("text"))
                .and_then(|t| t.as_str())
                .filter(|s| !s.is_empty())
            {
                return Some(t.to_string());
            }
            if let Some(msg) = v.get("choices").and_then(|c| c.get(0)).and_then(|c| c.get("message")) {
                let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
                if !content.is_empty() {
                    return Some(content.to_string());
                }
                if let Some(rc) = msg
                    .get("reasoning_content")
                    .and_then(|c| c.as_str())
                    .filter(|s| !s.is_empty())
                {
                    return Some(format!("[reasoning] {rc}"));
                }
            }
            if let Some(t) = v
                .get("output")
                .and_then(|o| o.get("text"))
                .and_then(|t| t.as_str())
                .filter(|s| !s.is_empty())
            {
                return Some(t.to_string());
            }
            None
        })
        .unwrap_or_default()
}

fn default_openai_tool_call() -> Value {
    crate::jv!({
        "id": "",
        "type": "function",
        "function": {
            "name": "",
            "arguments": ""
        }
    })
}

fn push_openai_tool_delta(tool_calls: &mut Vec<Value>, delta: &Value) {
    let index = delta
        .get("index")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(tool_calls.len());
    while tool_calls.len() <= index {
        tool_calls.push(default_openai_tool_call());
    }

    let slot = &mut tool_calls[index];
    if let Some(id) = delta.get("id").and_then(|v| v.as_str()).filter(|v| !v.is_empty()) {
        slot["id"] = crate::jv!(id);
    }
    if let Some(kind) = delta.get("type").and_then(|v| v.as_str()).filter(|v| !v.is_empty()) {
        slot["type"] = crate::jv!(kind);
    }
    if let Some(function) = delta.get("function") {
        if let Some(name) = function.get("name").and_then(|v| v.as_str()).filter(|v| !v.is_empty()) {
            let current = slot.pointer("/function/name").and_then(|v| v.as_str()).unwrap_or("");
            slot["function"]["name"] = crate::jv!(format!("{current}{name}"));
        }
        if let Some(arguments) = function.get("arguments").and_then(|v| v.as_str()).filter(|v| !v.is_empty()) {
            let current = slot.pointer("/function/arguments").and_then(|v| v.as_str()).unwrap_or("");
            slot["function"]["arguments"] = crate::jv!(format!("{current}{arguments}"));
        }
    }
}

pub(super) fn extract_openai_assistant_response(text: &str) -> (String, Vec<Value>, String, String) {
    let mut content = String::new();
    let mut reasoning = String::new();
    let mut tool_calls: Vec<Value> = Vec::new();
    let mut finish_reason = String::new();
    let mut stream_error = String::new();
    let mut saw_sse = false;

    for line in text.lines() {
        let Some(data) = line.strip_prefix("data: ").or_else(|| line.strip_prefix("data:")) else {
            continue;
        };
        saw_sse = true;
        let data = data.trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(data) else {
            continue;
        };
        if let Some(err) = value.get("error") {
            stream_error = err
                .get("message")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| err.to_string());
            continue;
        }
        let Some(choice) = value.get("choices").and_then(|v| v.get(0)) else {
            continue;
        };
        if finish_reason.is_empty() {
            if let Some(reason) = choice.get("finish_reason").and_then(|v| v.as_str()) {
                finish_reason = reason.to_string();
            }
        }
        let Some(delta) = choice.get("delta") else {
            continue;
        };
        if let Some(text) = delta.get("content").and_then(|v| v.as_str()) {
            content.push_str(text);
        }
        if let Some(text) = delta.get("reasoning_content").and_then(|v| v.as_str()) {
            reasoning.push_str(text);
        }
        if let Some(calls) = delta.get("tool_calls").and_then(|v| v.as_array()) {
            for call in calls {
                push_openai_tool_delta(&mut tool_calls, call);
            }
        }
    }

    if !saw_sse {
        let reply = extract_single_json_reply(text);
        return (reply, Vec::new(), String::new(), String::new());
    }
    let reply = if !content.is_empty() {
        content
    } else if !reasoning.is_empty() {
        format!("[reasoning] {reasoning}")
    } else {
        String::new()
    };
    (reply, tool_calls, finish_reason, stream_error)
}
