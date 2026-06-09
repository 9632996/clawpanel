fn insert_bool_as_string(form: &mut Map<String, Value>, source: &Value, key: &str) {
    if let Some(v) = source.get(key).and_then(|v| v.as_bool()) {
        form.insert(key.into(), Value::String(if v { "true" } else { "false" }.into()));
    }
}

fn insert_array_as_csv(form: &mut Map<String, Value>, source: &Value, key: &str) {
    if let Some(items) = source.get(key).and_then(|v| v.as_array()) {
        let joined = items
            .iter()
            .filter_map(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .collect::<Vec<_>>()
            .join(", ");
        if !joined.is_empty() {
            form.insert(key.into(), Value::String(joined));
        }
    }
}

fn insert_irc_groups_form_values(form: &mut Map<String, Value>, source: &Value) {
    let Some(groups) = source.get("groups").and_then(|v| v.as_object()) else {
        return;
    };
    let group_ids = groups
        .keys()
        .filter(|key| !key.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    if !group_ids.is_empty() {
        form.insert("groups".into(), Value::String(group_ids.join(", ")));
    }
    let mention_values = group_ids
        .iter()
        .filter_map(|group_id| {
            groups
                .get(group_id)
                .and_then(|group| group.get("requireMention"))
                .and_then(|v| v.as_bool())
        })
        .collect::<Vec<_>>();
    if let Some(first) = mention_values.first() {
        if mention_values.iter().all(|value| value == first) {
            form.insert("requireMention".into(), Value::String(if *first { "true" } else { "false" }.into()));
        }
    }
}

fn insert_number_as_string(form: &mut Map<String, Value>, source: &Value, key: &str) {
    if let Some(v) = source.get(key).and_then(|v| v.as_f64()) {
        form.insert(key.into(), Value::String(v.to_string()));
    }
}

fn insert_access_policy_form_values(form: &mut Map<String, Value>, source: &Value, telegram_compat: bool, mention_compat: bool) {
    insert_string_if_present(form, source, "dmPolicy");
    insert_string_if_present(form, source, "groupPolicy");
    if mention_compat
        && form.get("groupPolicy").and_then(|v| v.as_str()) == Some("open")
        && source.get("requireMention").and_then(|v| v.as_bool()) == Some(true)
    {
        form.insert("groupPolicy".into(), Value::String("mentioned".into()));
    }
    insert_array_as_csv(form, source, "allowFrom");
    if telegram_compat {
        if let Some(v) = form.get("allowFrom").cloned() {
            form.insert("allowedUsers".into(), v);
        }
    }
}

fn csv_to_json_array(raw: &str) -> Option<Value> {
    let items = raw
        .split(&[',', '\n', ';'][..])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| Value::String(s.to_string()))
        .collect::<Vec<_>>();
    if items.is_empty() {
        None
    } else {
        Some(Value::Array(items))
    }
}

fn json_array_from_csv_value(value: Option<&Value>) -> Vec<Value> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|v| {
                if let Some(s) = v.as_str() {
                    let trimmed = s.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(Value::String(trimmed.to_string()))
                    }
                } else if v.is_number() || v.is_boolean() {
                    Some(Value::String(v.to_string()))
                } else {
                    None
                }
            })
            .collect(),
        Some(Value::String(raw)) => csv_to_json_array(raw).and_then(|v| v.as_array().cloned()).unwrap_or_default(),
        _ => vec![],
    }
}

fn bool_from_form_value(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn put_string(entry: &mut Map<String, Value>, key: &str, value: String) {
    if !value.is_empty() {
        entry.insert(key.into(), Value::String(value));
    }
}

fn put_bool_from_form(entry: &mut Map<String, Value>, key: &str, raw: &str) {
    if let Some(v) = bool_from_form_value(raw) {
        entry.insert(key.into(), Value::Bool(v));
    }
}

fn put_number_from_form(entry: &mut Map<String, Value>, key: &str, raw: &str) {
    let value = raw.trim();
    if value.is_empty() {
        return;
    }
    if let Ok(number) = value.parse::<f64>() {
        if let Some(json_number) = serde_json::Number::from_f64(number) {
            entry.insert(key.into(), Value::Number(json_number));
        }
    }
}

fn put_number_value_if_present(entry: &mut Map<String, Value>, key: &str, value: Option<&Value>) {
    if let Some(number) = value.and_then(|v| v.as_f64()) {
        if let Some(json_number) = serde_json::Number::from_f64(number) {
            entry.insert(key.into(), Value::Number(json_number));
        }
        return;
    }
    put_number_from_form(entry, key, value.and_then(|v| v.as_str()).unwrap_or(""));
}

fn normalize_numeric_form_value(map: &mut Map<String, Value>, key: &str) {
    let Some(value) = map.get(key).cloned() else {
        return;
    };
    match value {
        Value::String(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                map.remove(key);
                return;
            }
            if let Ok(number) = trimmed.parse::<f64>() {
                if let Some(json_number) = serde_json::Number::from_f64(number) {
                    map.insert(key.into(), Value::Number(json_number));
                }
            }
        }
        Value::Null => {
            map.remove(key);
        }
        _ => {}
    }
}

fn put_bool_value_if_present(entry: &mut Map<String, Value>, key: &str, value: Option<&Value>) {
    match value {
        Some(Value::Bool(v)) => {
            entry.insert(key.into(), Value::Bool(*v));
        }
        Some(Value::String(raw)) => put_bool_from_form(entry, key, raw),
        _ => {}
    }
}

fn put_array_from_form_value(entry: &mut Map<String, Value>, key: &str, value: Option<&Value>) {
    let items = json_array_from_csv_value(value);
    if !items.is_empty() {
        entry.insert(key.into(), Value::Array(items));
    }
}

fn build_irc_groups_from_form(form_obj: &Map<String, Value>) -> Option<Value> {
    let group_ids = json_array_from_csv_value(form_obj.get("groups"));
    if group_ids.is_empty() {
        return None;
    }
    let require_mention = form_obj.get("requireMention").and_then(|v| v.as_bool());
    let mut groups = Map::new();
    for value in group_ids {
        let Some(group_id) = value.as_str().map(str::trim).filter(|s| !s.is_empty()) else {
            continue;
        };
        let mut group = Map::new();
        if let Some(require_mention) = require_mention {
            group.insert("requireMention".into(), Value::Bool(require_mention));
        }
        groups.insert(group_id.to_string(), Value::Object(group));
    }
    if groups.is_empty() {
        None
    } else {
        Some(Value::Object(groups))
    }
}

fn normalize_dm_policy_value(raw: Option<&Value>, fallback: &str) -> String {
    let value = raw.and_then(|v| v.as_str()).unwrap_or("").trim();
    match value {
        "" => fallback.to_string(),
        "allow" | "open" => "open".into(),
        "deny" | "disabled" => "disabled".into(),
        "pairing" => "pairing".into(),
        "allowlist" => "allowlist".into(),
        _ => fallback.to_string(),
    }
}

fn normalize_group_policy_value(raw: Option<&Value>, fallback: &str) -> String {
    let value = raw.and_then(|v| v.as_str()).unwrap_or("").trim();
    match value {
        "" => fallback.to_string(),
        "all" | "mentioned" | "open" => "open".into(),
        "deny" | "disabled" => "disabled".into(),
        "allowlist" => "allowlist".into(),
        _ => fallback.to_string(),
    }
}

fn platform_supports_top_level_require_mention(platform: &str) -> bool {
    matches!(
        platform_storage_key(platform),
        "feishu" | "slack" | "msteams" | "mattermost" | "googlechat" | "nextcloud-talk" | "twitch"
    )
}

fn normalize_messaging_platform_form(platform: &str, form: &Map<String, Value>) -> Map<String, Value> {
    let storage_key = platform_storage_key(platform);
    let mut normalized = form.clone();

    if !normalized.contains_key("allowFrom") {
        if let Some(v) = normalized.get("allowedUsers").cloned() {
            normalized.insert("allowFrom".into(), v);
        }
    }

    let needs_access_defaults = matches!(
        storage_key,
        "telegram"
            | "discord"
            | "feishu"
            | "slack"
            | "signal"
            | "msteams"
            | "whatsapp"
            | "zalo"
            | "zalouser"
            | "line"
            | "mattermost"
            | "googlechat"
            | "nextcloud-talk"
            | "imessage"
            | "irc"
    );
    let has_dm_field = normalized.contains_key("dmPolicy") || needs_access_defaults;
    let has_group_field = normalized.contains_key("groupPolicy") || needs_access_defaults;

    if has_dm_field {
        let dm_policy = normalize_dm_policy_value(normalized.get("dmPolicy"), "pairing");
        normalized.insert("dmPolicy".into(), Value::String(dm_policy.clone()));
        if normalized.contains_key("allowFrom") {
            let items = json_array_from_csv_value(normalized.get("allowFrom"));
            normalized.insert("allowFrom".into(), Value::Array(items));
        }
        if dm_policy == "open" {
            let mut items = json_array_from_csv_value(normalized.get("allowFrom"));
            if !items.iter().any(|v| v.as_str() == Some("*")) {
                items.push(Value::String("*".into()));
            }
            normalized.insert("allowFrom".into(), Value::Array(items));
        }
    } else if normalized.contains_key("allowFrom") {
        let items = json_array_from_csv_value(normalized.get("allowFrom"));
        normalized.insert("allowFrom".into(), Value::Array(items));
    }

    if has_group_field {
        let requested_group_policy = normalized
            .get("groupPolicy")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let group_policy = normalize_group_policy_value(normalized.get("groupPolicy"), "allowlist");
        normalized.insert("groupPolicy".into(), Value::String(group_policy));
        if requested_group_policy == "mentioned" && platform_supports_top_level_require_mention(storage_key) {
            normalized.insert("requireMention".into(), Value::Bool(true));
        } else if requested_group_policy != "mentioned" {
            if platform_supports_top_level_require_mention(storage_key) {
                normalized.insert("requireMention".into(), Value::Bool(false));
            } else if normalized.contains_key("requireMention") {
                let value = match normalized.get("requireMention") {
                    Some(Value::Bool(v)) => *v,
                    Some(Value::String(s)) => bool_from_form_value(s).unwrap_or(false),
                    _ => false,
                };
                normalized.insert("requireMention".into(), Value::Bool(value));
            }
        }
    }

    if normalized.contains_key("groupAllowFrom") {
        let items = json_array_from_csv_value(normalized.get("groupAllowFrom"));
        normalized.insert("groupAllowFrom".into(), Value::Array(items));
    }

    if normalized.contains_key("allowedUserIds") {
        let items = json_array_from_csv_value(normalized.get("allowedUserIds"));
        normalized.insert("allowedUserIds".into(), Value::Array(items));
    }

    normalize_numeric_form_value(&mut normalized, "mediaMaxMb");
    normalize_numeric_form_value(&mut normalized, "historyLimit");
    normalize_numeric_form_value(&mut normalized, "dmHistoryLimit");
    normalize_numeric_form_value(&mut normalized, "textChunkLimit");
    normalize_numeric_form_value(&mut normalized, "probeTimeoutMs");
    normalize_numeric_form_value(&mut normalized, "debounceMs");
    normalize_numeric_form_value(&mut normalized, "rateLimitPerMinute");
    normalize_numeric_form_value(&mut normalized, "httpPort");
    normalize_numeric_form_value(&mut normalized, "webhookPort");
    normalize_numeric_form_value(&mut normalized, "feedbackReflectionCooldownMs");
    normalize_numeric_form_value(&mut normalized, "timeoutSeconds");
    normalize_numeric_form_value(&mut normalized, "reconnectMs");
    normalize_numeric_form_value(&mut normalized, "expiresIn");
    normalize_numeric_form_value(&mut normalized, "obtainmentTimestamp");
    normalize_numeric_form_value(&mut normalized, "port");

    for key in [
        "promptStarters",
        "delegatedAuthScopes",
        "attachmentRoots",
        "remoteAttachmentRoots",
        "toolsAllow",
        "allowedRoles",
        "relays",
        "channels",
        "groups",
        "mentionPatterns",
        "groupChannels",
        "dmAllowlist",
        "groupInviteAllowlist",
        "defaultAuthorizedShips",
    ] {
        if normalized.contains_key(key) {
            let items = json_array_from_csv_value(normalized.get(key));
            normalized.insert(key.into(), Value::Array(items));
        }
    }

    for key in [
        "dangerouslyAllowNameMatching",
        "dangerouslyAllowPrivateNetwork",
        "dangerouslyAllowInheritedWebhookPath",
        "allowInsecureSsl",
        "enabled",
        "allowBots",
        "blockStreaming",
        "useManagedIdentity",
        "typingIndicator",
        "welcomeCard",
        "groupWelcomeCard",
        "feedbackEnabled",
        "feedbackReflection",
        "delegatedAuthEnabled",
        "ssoEnabled",
        "configWrites",
        "includeAttachments",
        "sendReadReceipts",
        "coalesceSameSenderDms",
        "selfChatMode",
        "ackDirect",
        "senderIsOwner",
        "requireMention",
        "tls",
        "nickservEnabled",
        "nickservRegister",
        "autoDiscoverChannels",
        "showModelSignature",
        "autoAcceptDmInvites",
        "autoAcceptGroupInvites",
    ] {
        if normalized.contains_key(key) {
            let value = match normalized.get(key) {
                Some(Value::Bool(v)) => Some(*v),
                Some(Value::String(raw)) => {
                    let trimmed = raw.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(bool_from_form_value(trimmed).unwrap_or(false))
                    }
                }
                _ => None,
            };
            if let Some(v) = value {
                normalized.insert(key.into(), Value::Bool(v));
            } else {
                normalized.remove(key);
            }
        }
    }

    if storage_key == "feishu" {
        let domain = normalized.get("domain").and_then(|v| v.as_str()).unwrap_or("").trim();
        normalized.insert("domain".into(), Value::String(if domain.is_empty() { "feishu" } else { domain }.into()));
        normalized
            .entry("connectionMode")
            .or_insert(Value::String("websocket".into()));
        normalized
            .entry("webhookPath")
            .or_insert(Value::String("/feishu/events".into()));
        normalized
            .entry("reactionNotifications")
            .or_insert(Value::String("off".into()));
        normalized.entry("typingIndicator").or_insert(Value::Bool(true));
        normalized.entry("resolveSenderNames").or_insert(Value::Bool(true));
    }

    if storage_key == "slack" {
        normalized.entry("mode").or_insert(Value::String("socket".into()));
        normalized
            .entry("webhookPath")
            .or_insert(Value::String("/slack/events".into()));
        normalized.entry("userTokenReadOnly").or_insert(Value::Bool(false));
    }

    normalized
}

/// 合并渠道配置：将新的表单字段覆盖到现有配置上，保留用户通过 CLI 或手动编辑的自定义字段。
/// 例如用户手动添加的 streaming / retry / dmPolicy 等不会被丢弃。
fn merge_channel_entry(channels_map: &mut Map<String, Value>, key: &str, new_entry: Map<String, Value>) {
    let merged = if let Some(Value::Object(existing)) = channels_map.get(key) {
        let mut m = existing.clone();
        for (k, v) in new_entry {
            m.insert(k, v);
        }
        m
    } else {
        new_entry
    };
    channels_map.insert(key.to_string(), Value::Object(merged));
}

/// 合并账号级渠道配置：保留渠道根节点和账号已有自定义字段，只覆盖本次表单字段。
fn merge_account_channel_entry(
    channels_map: &mut Map<String, Value>,
    key: &str,
    account_id: &str,
    new_entry: Map<String, Value>,
) -> Result<(), String> {
    let channel = channels_map
        .entry(key.to_string())
        .or_insert_with(|| crate::jv!({ "enabled": true }));
    let channel_obj = channel.as_object_mut().ok_or(format!("{} 节点格式错误", key))?;
    let accounts_before = channel_obj
        .get("accounts")
        .and_then(|value| value.as_object())
        .map(|accounts| accounts.keys().filter(|id| !id.is_empty()).count())
        .unwrap_or(0);
    let should_set_default_account = channel_obj
        .get("defaultAccount")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
        && !channel_root_has_messaging_credential(channel_obj)
        && accounts_before == 0;
    channel_obj.insert("enabled".into(), Value::Bool(true));
    let accounts = channel_obj.entry("accounts").or_insert_with(|| crate::jv!({}));
    let accounts_obj = accounts.as_object_mut().ok_or("accounts 格式错误")?;
    let merged = if let Some(Value::Object(existing)) = accounts_obj.get(account_id) {
        let mut m = existing.clone();
        for (k, v) in new_entry {
            m.insert(k, v);
        }
        m
    } else {
        new_entry
    };
    accounts_obj.insert(account_id.to_string(), Value::Object(merged));
    if should_set_default_account {
        channel_obj.insert("defaultAccount".into(), Value::String(account_id.to_string()));
    }
    Ok(())
}

fn merge_channel_entry_for_account(
    channels_map: &mut Map<String, Value>,
    key: &str,
    account_id: Option<&str>,
    new_entry: Map<String, Value>,
) -> Result<(), String> {
    if let Some(acct) = account_id.map(str::trim).filter(|s| !s.is_empty()) {
        merge_account_channel_entry(channels_map, key, acct, new_entry)
    } else {
        merge_channel_entry(channels_map, key, new_entry);
        Ok(())
    }
}

fn gateway_auth_mode(cfg: &Value) -> Option<&str> {
    cfg.get("gateway")
        .and_then(|g| g.get("auth"))
        .and_then(|a| a.get("mode"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
}

fn gateway_auth_value(cfg: &Value, key: &str) -> Option<String> {
    cfg.get("gateway")
        .and_then(|g| g.get("auth"))
        .and_then(|a| a.get(key))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
}

fn resolve_platform_config_entry(channel_root: Option<&Value>, platform: &str, account_id: Option<&str>) -> Option<Value> {
    let root = channel_root?;
    let account = account_id.map(str::trim).filter(|s| !s.is_empty());
    if let Some(acct) = account {
        if platform_storage_key(platform) == "tlon" && acct == QQBOT_DEFAULT_ACCOUNT_ID {
            return Some(root.clone());
        }
        if let Some(value) = root.get("accounts").and_then(|a| a.get(acct)) {
            return Some(value.clone());
        }
        if platform_storage_key(platform) == "qqbot" && !qqbot_channel_has_credentials(root) {
            return None;
        }
        return Some(root.clone());
    }

    if platform_storage_key(platform) == "qqbot" && !qqbot_channel_has_credentials(root) {
        return root
            .get("accounts")
            .and_then(|a| a.get(QQBOT_DEFAULT_ACCOUNT_ID))
            .cloned()
            .or_else(|| Some(root.clone()));
    }

    Some(root.clone())
}

