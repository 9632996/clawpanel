use serde_json::{Map, Value};

use super::messaging_common::{channel_root_has_messaging_credential, value_has_messaging_credential};

fn normalize_binding_match_value(value: &Value) -> Option<Value> {
    match value {
        Value::Null => None,
        Value::String(s) => Some(Value::String(s.trim().to_string())),
        Value::Array(items) => {
            let mut normalized: Vec<Value> = items.iter().filter_map(normalize_binding_match_value).collect();
            if normalized.iter().all(|item| item.as_str().is_some()) {
                normalized.sort_by(|a, b| a.as_str().unwrap_or_default().cmp(b.as_str().unwrap_or_default()));
            }
            Some(Value::Array(normalized))
        }
        Value::Object(map) => {
            let mut result = Map::new();
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();

            for key in keys {
                let Some(item) = map.get(key) else {
                    continue;
                };

                if key == "peer" {
                    if let Some(peer_id) = item.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                        result.insert("peer".into(), crate::jv!({ "kind": "direct", "id": peer_id }));
                    } else if let Some(peer_obj) = item.as_object() {
                        let kind = peer_obj
                            .get("kind")
                            .and_then(|v| v.as_str())
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .unwrap_or("direct");
                        let id = peer_obj
                            .get("id")
                            .and_then(|v| v.as_str())
                            .map(str::trim)
                            .filter(|s| !s.is_empty());
                        if let Some(peer_id) = id {
                            result.insert("peer".into(), crate::jv!({ "kind": kind, "id": peer_id }));
                        }
                    }
                    continue;
                }

                let Some(normalized) = normalize_binding_match_value(item) else {
                    continue;
                };
                if key == "accountId" && normalized.as_str().map(|s| s.is_empty()).unwrap_or(false) {
                    continue;
                }
                if normalized.as_str().map(|s| s.is_empty()).unwrap_or(false) {
                    continue;
                }
                result.insert(key.clone(), normalized);
            }

            Some(Value::Object(result))
        }
        _ => Some(value.clone()),
    }
}

fn build_binding_match(channel: &str, account_id: Option<&str>, binding_config: &Value) -> Value {
    let mut match_config = Map::new();
    match_config.insert("channel".into(), Value::String(channel.to_string()));

    if let Some(acct) = account_id.map(str::trim).filter(|s| !s.is_empty()) {
        match_config.insert("accountId".into(), Value::String(acct.to_string()));
    }

    if let Some(config_obj) = binding_config.as_object() {
        for (k, v) in config_obj {
            if k == "peer" {
                if let Some(peer_str) = v.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                    match_config.insert("peer".into(), crate::jv!({ "kind": "direct", "id": peer_str }));
                } else if let Some(peer_obj) = v.as_object() {
                    let kind = peer_obj
                        .get("kind")
                        .and_then(|v| v.as_str())
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .unwrap_or("direct");
                    let id = peer_obj
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(str::trim)
                        .filter(|s| !s.is_empty());
                    if let Some(peer_id) = id {
                        match_config.insert("peer".into(), crate::jv!({ "kind": kind, "id": peer_id }));
                    }
                }
            } else if k != "accountId" && k != "channel" && !v.is_null() {
                match_config.insert(k.clone(), v.clone());
            }
        }
    }

    normalize_binding_match_value(&Value::Object(match_config)).unwrap_or_else(|| Value::Object(Map::new()))
}

fn binding_identity_matches(binding: &Value, agent_id: &str, target_match: &Value) -> bool {
    let binding_agent = binding.get("agentId").and_then(|v| v.as_str()).unwrap_or("main");
    if binding_agent != agent_id {
        return false;
    }

    let existing_match =
        normalize_binding_match_value(binding.get("match").unwrap_or(&Value::Null)).unwrap_or_else(|| Value::Object(Map::new()));
    let expected_match = normalize_binding_match_value(target_match).unwrap_or_else(|| Value::Object(Map::new()));

    existing_match == expected_match
}

/// 创建 Agent 到渠道的绑定配置（OpenClaw bindings schema）
pub(super) fn create_agent_binding(
    cfg: &mut Value,
    agent_id: &str,
    channel: &str,
    account_id: Option<String>,
) -> Result<(), String> {
    let bindings = cfg
        .as_object_mut()
        .ok_or("配置格式错误")?
        .entry("bindings")
        .or_insert_with(|| crate::jv!([]));
    let bindings_arr = bindings.as_array_mut().ok_or("bindings 节点格式错误")?;

    let mut new_binding = Map::new();
    new_binding.insert("type".to_string(), Value::String("route".to_string()));
    new_binding.insert("agentId".to_string(), Value::String(agent_id.to_string()));

    let mut match_config = Map::new();
    match_config.insert("channel".to_string(), Value::String(channel.to_string()));
    if let Some(ref acct) = account_id {
        match_config.insert("accountId".to_string(), Value::String(acct.clone()));
    }

    new_binding.insert("match".to_string(), Value::Object(match_config));
    let binding_value = Value::Object(new_binding);

    let mut found = false;
    for binding in bindings_arr.iter_mut() {
        if let (Some(existing_agent), Some(existing_channel), Some(existing_match)) = (
            binding.get("agentId").and_then(|v| v.as_str()),
            binding.get("match").and_then(|m| m.get("channel")).and_then(|v| v.as_str()),
            binding.get("match"),
        ) {
            if existing_agent == agent_id && existing_channel == channel {
                let existing_account = existing_match.get("accountId").and_then(|v| v.as_str());
                if existing_account == account_id.as_deref() {
                    *binding = binding_value.clone();
                    found = true;
                    break;
                }
            }
        }
    }

    if !found {
        bindings_arr.push(binding_value);
    }

    Ok(())
}

/// 获取指定 Agent 的所有渠道绑定
/// 返回格式: { agentId, bindings: [{ channel, accountId, peer, ... }] }
#[tauri::command]
pub async fn get_agent_bindings(agent_id: String) -> Result<Value, String> {
    let cfg = super::config::load_openclaw_json()?;

    let bindings: Vec<Value> = cfg
        .get("bindings")
        .and_then(|b| b.as_array())
        .map(|arr| {
            arr.iter()
                .filter(|b| {
                    b.get("agentId")
                        .and_then(|v| v.as_str())
                        .map(|id| id == agent_id)
                        .unwrap_or(false)
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    Ok(crate::jv!({
        "agentId": agent_id,
        "bindings": bindings
    }))
}

/// 获取所有 Agent 的绑定列表（用于管理界面）
#[tauri::command]
pub async fn list_all_bindings() -> Result<Value, String> {
    let cfg = super::config::load_openclaw_json()?;
    let bindings: Vec<Value> = cfg.get("bindings").and_then(|b| b.as_array()).cloned().unwrap_or_default();

    Ok(crate::jv!({
        "bindings": bindings
    }))
}

/// 保存/更新 Agent 的渠道绑定
/// - agent_id: Agent ID
/// - channel: 渠道类型 (feishu/telegram/discord/qqbot/dingtalk)
/// - account_id: 可选，指定账号（多账号模式）
/// - binding_config: 绑定配置 { peer, match, ... }
#[tauri::command]
pub async fn save_agent_binding(
    agent_id: String,
    channel: String,
    account_id: Option<String>,
    binding_config: Value,
    app: tauri::AppHandle,
) -> Result<Value, String> {
    let mut cfg = super::config::load_openclaw_json()?;

    let mut warnings: Vec<String> = vec![];
    if let Some(ref acct) = account_id {
        if !acct.is_empty() {
            if let Some(ch) = cfg.get("channels").and_then(|c| c.get(channel.as_str())) {
                let has_account = ch
                    .get("accounts")
                    .and_then(|a| a.get(acct.as_str()))
                    .map(value_has_messaging_credential)
                    .unwrap_or(false);

                if !has_account {
                    let has_root = ch.as_object().map(channel_root_has_messaging_credential).unwrap_or(false);
                    if has_root {
                        warnings.push(format!(
                            "账号「{}」在 channels.{}.accounts 下未找到对应配置，\
                         当前凭证写在根级别（单账号旧格式）。\
                         建议将账号凭证移入 channels.{}.accounts.\"{}\" 下以支持多账号。",
                            acct, channel, channel, acct
                        ));
                    } else {
                        warnings.push(format!(
                            "账号「{}」在 channels.{}.accounts 下未找到对应配置，\
                         该绑定可能无法正常路由消息。\
                         请先在渠道列表中为账号「{}」接入对应渠道账号。",
                            acct, channel, acct
                        ));
                    }
                }
            } else {
                warnings.push(format!(
                    "渠道「{}」尚未接入（channels.{} 不存在），该绑定可能无法正常工作。",
                    channel, channel
                ));
            }
        }
    }

    let bindings = cfg
        .as_object_mut()
        .ok_or("配置格式错误")?
        .entry("bindings")
        .or_insert_with(|| crate::jv!([]));
    let bindings_arr = bindings.as_array_mut().ok_or("bindings 节点格式错误")?;

    let mut new_binding = Map::new();
    new_binding.insert("type".to_string(), Value::String("route".to_string()));
    new_binding.insert("agentId".to_string(), Value::String(agent_id.clone()));

    let target_match = build_binding_match(&channel, account_id.as_deref(), &binding_config);
    new_binding.insert("match".to_string(), target_match.clone());

    let binding_value = Value::Object(new_binding);

    let mut found = false;
    for binding in bindings_arr.iter_mut() {
        if binding_identity_matches(binding, &agent_id, &target_match) {
            *binding = binding_value.clone();
            found = true;
            break;
        }
    }

    if !found {
        bindings_arr.push(binding_value);
    }

    super::config::save_openclaw_json(&cfg)?;

    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = super::config::do_reload_gateway(&app2).await;
    });

    Ok(crate::jv!({
        "ok": true,
        "warnings": warnings
    }))
}

/// 删除 Agent 的渠道绑定
/// - agent_id: Agent ID
/// - channel: 渠道类型
/// - account_id: 指定子账号时仅删该条；为 None 时仅删除「无 accountId」的默认绑定（不会一次删掉同渠道下其它子账号）
#[tauri::command]
pub async fn delete_agent_binding(
    agent_id: String,
    channel: String,
    account_id: Option<String>,
    binding_config: Option<Value>,
    app: tauri::AppHandle,
) -> Result<Value, String> {
    let mut cfg = super::config::load_openclaw_json()?;
    let target_match = build_binding_match(&channel, account_id.as_deref(), binding_config.as_ref().unwrap_or(&Value::Null));

    let Some(bindings) = cfg.get_mut("bindings").and_then(|b| b.as_array_mut()) else {
        return Ok(crate::jv!({ "ok": true }));
    };

    let original_len = bindings.len();
    bindings.retain(|b| !binding_identity_matches(b, &agent_id, &target_match));

    let removed = original_len - bindings.len();
    if removed == 0 {
        return Err("未找到对应的绑定".to_string());
    }

    super::config::save_openclaw_json(&cfg)?;

    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = super::config::do_reload_gateway(&app2).await;
    });

    Ok(crate::jv!({
        "ok": true,
        "removed": removed
    }))
}

/// 删除指定 Agent 的所有绑定
#[tauri::command]
pub async fn delete_agent_all_bindings(agent_id: String, app: tauri::AppHandle) -> Result<Value, String> {
    let mut cfg = super::config::load_openclaw_json()?;

    let Some(bindings) = cfg.get_mut("bindings").and_then(|b| b.as_array_mut()) else {
        return Ok(crate::jv!({ "ok": true, "removed": 0 }));
    };

    let original_len = bindings.len();
    bindings.retain(|b| {
        b.get("agentId")
            .and_then(|v| v.as_str())
            .map(|id| id != agent_id)
            .unwrap_or(true)
    });

    let removed = original_len - bindings.len();

    super::config::save_openclaw_json(&cfg)?;

    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = super::config::do_reload_gateway(&app2).await;
    });

    Ok(crate::jv!({
        "ok": true,
        "removed": removed
    }))
}
