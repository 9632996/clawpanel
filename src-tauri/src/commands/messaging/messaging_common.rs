use serde_json::{Map, Value};
pub(super) fn platform_storage_key(platform: &str) -> &str {
    match platform {
        "dingtalk" | "dingtalk-connector" => "dingtalk-connector",
        "weixin" => "openclaw-weixin",
        _ => platform,
    }
}

pub(super) fn platform_list_id(platform: &str) -> &str {
    match platform {
        "dingtalk-connector" => "dingtalk",
        "openclaw-weixin" => "weixin",
        _ => platform,
    }
}

pub(super) fn ensure_chat_completions_enabled(cfg: &mut Value) -> Result<(), String> {
    let root = cfg.as_object_mut().ok_or("配置格式错误")?;
    let gateway = root.entry("gateway").or_insert_with(|| crate::jv!({}));
    let gateway_obj = gateway.as_object_mut().ok_or("gateway 节点格式错误")?;
    let http = gateway_obj.entry("http").or_insert_with(|| crate::jv!({}));
    let http_obj = http.as_object_mut().ok_or("gateway.http 节点格式错误")?;
    let endpoints = http_obj.entry("endpoints").or_insert_with(|| crate::jv!({}));
    let endpoints_obj = endpoints.as_object_mut().ok_or("gateway.http.endpoints 节点格式错误")?;
    let chat = endpoints_obj.entry("chatCompletions").or_insert_with(|| crate::jv!({}));
    let chat_obj = chat
        .as_object_mut()
        .ok_or("gateway.http.endpoints.chatCompletions 节点格式错误")?;
    chat_obj.insert("enabled".into(), Value::Bool(true));
    Ok(())
}

pub(super) fn form_string(form_obj: &Map<String, Value>, key: &str) -> String {
    form_obj.get(key).and_then(|v| v.as_str()).unwrap_or("").trim().to_string()
}

pub(super) fn insert_string_if_present(form: &mut Map<String, Value>, source: &Value, key: &str) {
    if let Some(v) = source.get(key).and_then(|v| v.as_str()) {
        form.insert(key.into(), Value::String(v.into()));
    }
}

fn secret_ref_parts(value: &Value) -> Option<(&str, &str, &str)> {
    let obj = value.as_object()?;
    let source = obj.get("source").and_then(|v| v.as_str())?.trim();
    if !matches!(source, "env" | "file" | "exec") {
        return None;
    }
    let provider = obj
        .get("provider")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("default");
    let id = obj
        .get("id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())?;
    Some((source, provider, id))
}

pub(super) fn secret_ref_placeholder(value: &Value) -> Option<String> {
    let (source, provider, id) = secret_ref_parts(value)?;
    Some(format!("SecretRef({}:{}:{})", source, provider, id))
}

pub(super) fn insert_secret_aware_form_value(form: &mut Map<String, Value>, source: &Value, key: &str) {
    if let Some(v) = source.get(key).and_then(|v| v.as_str()) {
        form.insert(key.into(), Value::String(v.into()));
        return;
    }

    let Some(value) = source.get(key) else {
        return;
    };
    let Some(placeholder) = secret_ref_placeholder(value) else {
        return;
    };
    form.insert(key.into(), Value::String(placeholder));
    let refs = form.entry("__secretRefs").or_insert_with(|| Value::Object(Map::new()));
    if let Some(obj) = refs.as_object_mut() {
        obj.insert(key.into(), value.clone());
    }
}

pub(super) fn insert_secret_aware_form_alias(form: &mut Map<String, Value>, source: &Value, source_key: &str, form_key: &str) {
    if let Some(v) = source.get(source_key).and_then(|v| v.as_str()) {
        form.insert(form_key.into(), Value::String(v.into()));
        return;
    }

    let Some(value) = source.get(source_key) else {
        return;
    };
    let Some(placeholder) = secret_ref_placeholder(value) else {
        return;
    };
    form.insert(form_key.into(), Value::String(placeholder));
    let refs = form.entry("__secretRefs").or_insert_with(|| Value::Object(Map::new()));
    if let Some(obj) = refs.as_object_mut() {
        obj.insert(form_key.into(), value.clone());
    }
}

pub(super) fn resolve_messaging_credential_value_for_save(
    form_obj: &Map<String, Value>,
    current: &Value,
    key: &str,
) -> Option<Value> {
    let raw_value = form_obj.get(key)?;
    let Value::String(raw) = raw_value else {
        return Some(raw_value.clone());
    };
    let value = raw.trim();
    if let Some(current_value) = current.get(key) {
        if let Some(placeholder) = secret_ref_placeholder(current_value) {
            if value.is_empty() || value == placeholder {
                return Some(current_value.clone());
            }
        }
    }
    if value.is_empty() {
        None
    } else {
        Some(Value::String(value.to_string()))
    }
}

pub(super) fn resolve_messaging_credential_value_for_save_alias(
    form_obj: &Map<String, Value>,
    current: &Value,
    form_key: &str,
    current_key: &str,
) -> Option<Value> {
    let raw_value = form_obj.get(form_key)?;
    let Value::String(raw) = raw_value else {
        return Some(raw_value.clone());
    };
    let value = raw.trim();
    if let Some(current_value) = current.get(current_key) {
        if let Some(placeholder) = secret_ref_placeholder(current_value) {
            if value.is_empty() || value == placeholder {
                return Some(current_value.clone());
            }
        }
    }
    if value.is_empty() {
        None
    } else {
        Some(Value::String(value.to_string()))
    }
}

pub(super) fn preserve_messaging_credential_refs(entry: &mut Map<String, Value>, form_obj: &Map<String, Value>, current: &Value) {
    entry.remove("__secretRefs");
    for key in [
        "accessToken",
        "appId",
        "appPassword",
        "appSecret",
        "appToken",
        "apiPassword",
        "apiPasswordFile",
        "botSecret",
        "botSecretFile",
        "botToken",
        "channelAccessToken",
        "channelSecret",
        "code",
        "clientId",
        "clientSecret",
        "refreshToken",
        "gatewayPassword",
        "gatewayToken",
        "password",
        "passwordFile",
        "privateKey",
        "secretFile",
        "serviceAccount",
        "serviceAccountFile",
        "serviceAccountRef",
        "signingSecret",
        "token",
        "tokenFile",
        "webhookSecret",
    ] {
        if !form_obj.contains_key(key) {
            continue;
        }
        match resolve_messaging_credential_value_for_save(form_obj, current, key) {
            Some(value) => {
                entry.insert(key.into(), value);
            }
            None => {
                entry.remove(key);
            }
        }
    }
}

pub(super) fn has_configured_messaging_value(value: Option<&Value>) -> bool {
    match value {
        Some(Value::String(raw)) => !raw.trim().is_empty(),
        Some(value) if secret_ref_parts(value).is_some() => true,
        Some(Value::Null) | None => false,
        Some(_) => true,
    }
}

pub(super) fn is_enabled_form_flag(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Bool(v)) => *v,
        Some(Value::Number(v)) => v.as_i64().map(|n| n != 0).unwrap_or(false),
        Some(Value::String(raw)) => matches!(raw.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes" | "on" | "enabled"),
        _ => false,
    }
}

pub(super) fn channel_root_has_messaging_credential(root: &Map<String, Value>) -> bool {
    [
        "accessToken",
        "appId",
        "appPassword",
        "appSecret",
        "appToken",
        "apiPassword",
        "apiPasswordFile",
        "botSecret",
        "botSecretFile",
        "botToken",
        "channelAccessToken",
        "channelSecret",
        "code",
        "clientId",
        "clientSecret",
        "refreshToken",
        "gatewayPassword",
        "gatewayToken",
        "password",
        "privateKey",
        "secretFile",
        "serviceAccount",
        "serviceAccountFile",
        "serviceAccountRef",
        "signingSecret",
        "token",
        "tokenFile",
        "webhookSecret",
    ]
    .iter()
    .any(|key| has_configured_messaging_value(root.get(*key)))
}

pub(super) fn value_has_messaging_credential(value: &Value) -> bool {
    value.as_object().map(channel_root_has_messaging_credential).unwrap_or(false)
}
