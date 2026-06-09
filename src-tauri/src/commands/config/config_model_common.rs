use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn resolve_model_api_key(raw: &str) -> Result<String, String> {
    let Some(key) = model_api_key_env_ref(raw)? else {
        return Ok(raw.to_string());
    };
    let values = model_env_values();
    if let Some(value) = values.get(&key).filter(|v| !v.is_empty()) {
        return Ok(value.clone());
    }
    if let Ok(value) = std::env::var(&key) {
        if !value.is_empty() {
            return Ok(value);
        }
    }
    Err(format!(
        "API Key 引用了环境变量 {key}，但未在 openclaw.json env、model-credentials.env、.openclaw/.env 或当前进程环境中找到"
    ))
}

fn normalize_base_url(raw: &str) -> String {
    let mut base = raw.trim_end_matches('/').to_string();
    for suffix in &[
        "/api/chat",
        "/api/generate",
        "/api/tags",
        "/api",
        "/chat/completions",
        "/completions",
        "/responses",
        "/messages",
        "/models",
    ] {
        if base.ends_with(suffix) {
            base.truncate(base.len() - suffix.len());
            break;
        }
    }
    base = base.trim_end_matches('/').to_string();
    if base.ends_with(":11434") {
        return format!("{base}/v1");
    }
    base
}

pub(super) fn normalize_model_api_type(raw: &str) -> &'static str {
    match raw.trim() {
        "anthropic" | "anthropic-messages" => "anthropic-messages",
        "google-gemini" | "google-generative-ai" => "google-gemini",
        "openai" | "openai-completions" | "openai-responses" | "" => "openai-completions",
        _ => "openai-completions",
    }
}

pub(super) fn normalize_base_url_for_api(raw: &str, api_type: &str) -> String {
    let mut base = normalize_base_url(raw);
    match normalize_model_api_type(api_type) {
        "anthropic-messages" => {
            if !base.ends_with("/v1") {
                base.push_str("/v1");
            }
            base
        }
        "google-gemini" => base,
        _ => base,
    }
}

pub(super) fn is_valid_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

pub(super) fn model_api_key_env_ref(raw: &str) -> Result<Option<String>, String> {
    let value = raw.trim();
    if value.starts_with("${") && value.ends_with('}') {
        let key = &value[2..value.len() - 1];
        if is_valid_env_key(key) {
            return Ok(Some(key.to_string()));
        }
        return Err(format!("无效的环境变量引用: {value}"));
    }
    if let Some(key) = value.strip_prefix("$env:") {
        if is_valid_env_key(key) {
            return Ok(Some(key.to_string()));
        }
        return Err(format!("无效的环境变量引用: {value}"));
    }
    if let Some(key) = value.strip_prefix('$') {
        if !key.is_empty() && is_valid_env_key(key) {
            return Ok(Some(key.to_string()));
        }
    }
    Ok(None)
}

pub(super) fn model_api_key_env_ref_from_value(value: &Value) -> Result<Option<String>, String> {
    if let Some(raw) = value.as_str() {
        return model_api_key_env_ref(raw);
    }

    let Some(map) = value.as_object() else {
        return Ok(None);
    };
    let raw = map.get("$env").and_then(|v| v.as_str()).or_else(|| {
        if map.get("source").and_then(|v| v.as_str()) == Some("env") {
            map.get("id").or_else(|| map.get("env")).and_then(|v| v.as_str())
        } else {
            None
        }
    });
    let raw = raw.map(str::trim).filter(|v| !v.is_empty());
    let Some(key) = raw else {
        return Ok(None);
    };
    if is_valid_env_key(key) {
        Ok(Some(key.to_string()))
    } else {
        Err(format!("无效的环境变量引用: {key}"))
    }
}

fn insert_env_value(values: &mut HashMap<String, String>, key: String, value: String) {
    if value.is_empty() {
        return;
    }
    match values.get(&key) {
        Some(existing) if !existing.is_empty() => {}
        _ => {
            values.insert(key, value);
        }
    }
}

pub(super) fn parse_dotenv_line(line: &str) -> Option<(String, String)> {
    let line = line.trim().trim_start_matches('\u{feff}');
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let line = line.strip_prefix("export ").unwrap_or(line).trim();
    let (key, value) = line.split_once('=')?;
    let key = key.trim();
    if !is_valid_env_key(key) {
        return None;
    }
    let mut value = value.trim().to_string();
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"') || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'') {
            value = value[1..value.len() - 1].to_string();
        }
    }
    Some((key.to_string(), value))
}

pub(super) fn model_env_values() -> HashMap<String, String> {
    let mut values = HashMap::new();
    if let Ok(cfg) = crate::commands::config::load_openclaw_json() {
        if let Some(env) = cfg.get("env").and_then(|v| v.as_object()) {
            for (key, value) in env {
                if !is_valid_env_key(key) {
                    continue;
                }
                if let Some(s) = value.as_str() {
                    insert_env_value(&mut values, key.to_string(), s.to_string());
                } else if value.is_number() || value.is_boolean() {
                    insert_env_value(&mut values, key.to_string(), value.to_string());
                }
            }
        }
    }
    let credentials_path = crate::commands::openclaw_dir().join("model-credentials.env");
    if let Ok(content) = fs::read_to_string(credentials_path) {
        for line in content.lines() {
            if let Some((key, value)) = parse_dotenv_line(line) {
                insert_env_value(&mut values, key, value);
            }
        }
    }
    let env_path = crate::commands::openclaw_dir().join(".env");
    if let Ok(content) = fs::read_to_string(env_path) {
        for line in content.lines() {
            if let Some((key, value)) = parse_dotenv_line(line) {
                insert_env_value(&mut values, key, value);
            }
        }
    }
    values
}

pub(super) fn resolve_model_api_key_value(api_key: &Value) -> Result<String, String> {
    if api_key.is_null() {
        return Ok(String::new());
    }
    if let Some(raw) = api_key.as_str() {
        return resolve_model_api_key(raw);
    }
    if let Some(key) = model_api_key_env_ref_from_value(api_key)? {
        let values = model_env_values();
        if let Some(value) = values.get(&key).filter(|v| !v.is_empty()) {
            return Ok(value.clone());
        }
        if let Ok(value) = std::env::var(&key) {
            if !value.is_empty() {
                return Ok(value);
            }
        }
        return Err(format!(
            "API Key 引用了环境变量 {key}，但未在 openclaw.json env、model-credentials.env、.openclaw/.env 或当前进程环境中找到"
        ));
    }
    Err("API Key 必须是字符串或环境变量引用对象".to_string())
}

pub(super) fn strip_config_value(raw: &str) -> String {
    let mut out = String::new();
    let mut quote: Option<char> = None;
    for ch in raw.trim().chars() {
        if ch == '"' || ch == '\'' {
            if quote == Some(ch) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(ch);
            }
            out.push(ch);
            continue;
        }
        if ch == '#' && quote.is_none() {
            break;
        }
        out.push(ch);
    }
    let value = out.trim().trim_end_matches(',').trim();
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"') || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'') {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}

pub(super) fn first_env_ref(keys: &[&str]) -> (String, String) {
    for key in keys {
        if std::env::var(key).map(|v| !v.trim().is_empty()).unwrap_or(false) {
            return (format!("${{{key}}}"), "found".into());
        }
    }
    if let Some(key) = keys.first() {
        (format!("${{{key}}}"), "missing".into())
    } else {
        (String::new(), "none".into())
    }
}

pub(super) fn find_json_string(value: &Value, keys: &[&str], depth: usize) -> Option<String> {
    if depth > 5 {
        return None;
    }
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(v) = map.get(*key).and_then(|v| v.as_str()) {
                    if !v.trim().is_empty() {
                        return Some(v.trim().to_string());
                    }
                }
            }
            for v in map.values() {
                if let Some(found) = find_json_string(v, keys, depth + 1) {
                    return Some(found);
                }
            }
        }
        Value::Array(list) => {
            for v in list {
                if let Some(found) = find_json_string(v, keys, depth + 1) {
                    return Some(found);
                }
            }
        }
        _ => {}
    }
    None
}

pub(super) fn home_path(parts: &[&str]) -> Option<PathBuf> {
    let mut path = dirs::home_dir()?;
    for part in parts {
        path.push(part);
    }
    Some(path)
}
