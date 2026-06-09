
fn put_json_bool_from_env(
    form: &mut serde_json::Map<String, Value>,
    env_values: &std::collections::HashMap<String, String>,
    env_key: &str,
    json_key: &str,
) {
    if let Some(value) = hermes_env_value(env_values, env_key) {
        let enabled = matches!(value.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes" | "on");
        form.insert(json_key.to_string(), Value::Bool(enabled));
    }
}

fn insert_hermes_home_channel_if_present(form: &mut serde_json::Map<String, Value>, entry: &serde_yaml::Mapping) {
    let Some(home) = yaml_get_mapping(entry, "home_channel") else {
        return;
    };
    if let Some(value) = yaml_string_field(home, "chat_id") {
        form.insert("homeChannel".to_string(), Value::String(value));
    }
    if let Some(value) = yaml_string_field(home, "name") {
        form.insert("homeChannelName".to_string(), Value::String(value));
    }
}

fn insert_hermes_channel_display_fields(form: &mut serde_json::Map<String, Value>, config: &serde_yaml::Value, platform: &str) {
    let display = config.as_mapping().and_then(|map| yaml_get_mapping(map, "display"));
    let platform_display = display
        .and_then(|map| yaml_get_mapping(map, "platforms"))
        .and_then(|map| yaml_get_mapping(map, platform));
    let legacy_tool_progress = display
        .and_then(|map| yaml_get_mapping(map, "tool_progress_overrides"))
        .and_then(|map| yaml_string_field(map, platform));
    let tool_progress = normalize_hermes_display_tool_progress(
        platform_display
            .and_then(|map| yaml_string_field(map, "tool_progress"))
            .or(legacy_tool_progress)
            .or_else(|| display.and_then(|map| yaml_string_field(map, "tool_progress"))),
        false,
        "display.tool_progress",
    )
    .unwrap_or_else(|_| "all".to_string());
    let show_reasoning = platform_display
        .and_then(|map| yaml_bool_field(map, "show_reasoning"))
        .or_else(|| display.and_then(|map| yaml_bool_field(map, "show_reasoning")))
        .unwrap_or(false);
    let tool_preview_length = bounded_hermes_i64(
        platform_display
            .and_then(|map| yaml_i64_field(map, "tool_preview_length"))
            .or_else(|| display.and_then(|map| yaml_i64_field(map, "tool_preview_length"))),
        0,
        0,
        200000,
    );
    let streaming = if let Some(platform_display) = platform_display {
        if let Some(value) = yaml_get(platform_display, "streaming") {
            normalize_hermes_display_streaming_yaml(Some(value), false, "display.platforms.streaming")
                .unwrap_or_else(|_| "inherit".to_string())
        } else {
            "inherit".to_string()
        }
    } else {
        "inherit".to_string()
    };
    let cleanup_progress = platform_display
        .and_then(|map| yaml_bool_field(map, "cleanup_progress"))
        .or_else(|| display.and_then(|map| yaml_bool_field(map, "cleanup_progress")))
        .unwrap_or(false);

    form.insert("displayToolProgress".to_string(), Value::String(tool_progress));
    form.insert("displayShowReasoning".to_string(), Value::Bool(show_reasoning));
    form.insert("displayToolPreviewLength".to_string(), Value::Number(tool_preview_length.into()));
    form.insert("displayStreaming".to_string(), Value::String(streaming));
    form.insert("displayCleanupProgress".to_string(), Value::Bool(cleanup_progress));
}

fn build_hermes_channel_config_values(
    config: &serde_yaml::Value,
    env_values: &std::collections::HashMap<String, String>,
) -> Value {
    let mut values = serde_json::Map::new();
    let root = config.as_mapping();
    let platforms = root.and_then(|map| yaml_get_mapping(map, "platforms"));

    for platform in HERMES_CHANNEL_PLATFORMS {
        let entry = platforms
            .and_then(|map| yaml_get_mapping(map, platform))
            .cloned()
            .unwrap_or_default();
        let extra = yaml_get_mapping(&entry, "extra").cloned().unwrap_or_default();
        let mut form = serde_json::Map::new();
        form.insert("enabled".to_string(), Value::Bool(yaml_bool_field(&entry, "enabled").unwrap_or(false)));

        match platform {
            "telegram" => {
                let token = hermes_env_value(env_values, "TELEGRAM_BOT_TOKEN")
                    .or_else(|| yaml_string_field(&entry, "token"))
                    .unwrap_or_default();
                form.insert("botToken".to_string(), Value::String(token));
                let reply_to_mode = normalize_hermes_telegram_reply_to_mode(
                    hermes_env_value(env_values, "TELEGRAM_REPLY_TO_MODE").or_else(|| yaml_string_field(&extra, "reply_to_mode")),
                    false,
                )
                .unwrap_or_else(|_| "first".to_string());
                form.insert("replyToMode".to_string(), Value::String(reply_to_mode));
                insert_json_bool_if_present(&mut form, &extra, "guest_mode", "guestMode");
                insert_json_bool_if_present(&mut form, &extra, "disable_link_previews", "disableLinkPreviews");
                put_json_bool_from_env(&mut form, env_values, "TELEGRAM_GUEST_MODE", "guestMode");
                put_json_bool_from_env(&mut form, env_values, "TELEGRAM_DISABLE_LINK_PREVIEWS", "disableLinkPreviews");
            }
            "discord" => {
                let token = hermes_env_value(env_values, "DISCORD_BOT_TOKEN")
                    .or_else(|| yaml_string_field(&entry, "token"))
                    .unwrap_or_default();
                form.insert("token".to_string(), Value::String(token));
                for (yaml_key_name, json_key_name, env_key_name) in [
                    ("free_response_channels", "freeResponseChannels", "DISCORD_FREE_RESPONSE_CHANNELS"),
                    ("allowed_channels", "allowedChannels", "DISCORD_ALLOWED_CHANNELS"),
                    ("ignored_channels", "ignoredChannels", "DISCORD_IGNORED_CHANNELS"),
                    ("no_thread_channels", "noThreadChannels", "DISCORD_NO_THREAD_CHANNELS"),
                ] {
                    insert_json_csv_if_present(&mut form, &extra, yaml_key_name, json_key_name);
                    put_json_string_from_env(&mut form, env_values, env_key_name, json_key_name);
                }
                for (yaml_key_name, json_key_name, env_key_name) in [
                    ("auto_thread", "autoThread", "DISCORD_AUTO_THREAD"),
                    ("reactions", "reactions", "DISCORD_REACTIONS"),
                    ("thread_require_mention", "threadRequireMention", "DISCORD_THREAD_REQUIRE_MENTION"),
                    ("history_backfill", "historyBackfill", "DISCORD_HISTORY_BACKFILL"),
                ] {
                    insert_json_bool_if_present(&mut form, &extra, yaml_key_name, json_key_name);
                    put_json_bool_from_env(&mut form, env_values, env_key_name, json_key_name);
                }
                insert_json_string_if_present(&mut form, &extra, "history_backfill_limit", "historyBackfillLimit");
                put_json_string_from_env(&mut form, env_values, "DISCORD_HISTORY_BACKFILL_LIMIT", "historyBackfillLimit");
                insert_json_string_if_present(&mut form, &extra, "reply_to_mode", "replyToMode");
                put_json_string_from_env(&mut form, env_values, "DISCORD_REPLY_TO_MODE", "replyToMode");
                put_json_string_from_env(&mut form, env_values, "DISCORD_HOME_CHANNEL", "homeChannel");
                put_json_string_from_env(&mut form, env_values, "DISCORD_HOME_CHANNEL_NAME", "homeChannelName");
            }
            "slack" => {
                let bot_token = hermes_env_value(env_values, "SLACK_BOT_TOKEN")
                    .or_else(|| yaml_string_field(&entry, "token"))
                    .unwrap_or_default();
                form.insert("botToken".to_string(), Value::String(bot_token));
                insert_json_string_if_present(&mut form, &extra, "app_token", "appToken");
                let app_token = hermes_env_value(env_values, "SLACK_APP_TOKEN")
                    .or_else(|| json_form_string(&form, "appToken"))
                    .unwrap_or_default();
                form.insert("appToken".to_string(), Value::String(app_token));
                insert_json_string_if_present(&mut form, &extra, "signing_secret", "signingSecret");
                insert_json_string_if_present(&mut form, &extra, "webhook_path", "webhookPath");
            }
            "feishu" => {
                insert_json_string_if_present(&mut form, &extra, "app_id", "appId");
                insert_json_string_if_present(&mut form, &extra, "app_secret", "appSecret");
                insert_json_string_if_present(&mut form, &extra, "domain", "domain");
                insert_json_string_if_present(&mut form, &extra, "connection_mode", "connectionMode");
                insert_json_string_if_present(&mut form, &extra, "webhook_path", "webhookPath");
                insert_json_string_if_present(&mut form, &extra, "reaction_notifications", "reactionNotifications");
                put_json_string_from_env(&mut form, env_values, "FEISHU_APP_ID", "appId");
                put_json_string_from_env(&mut form, env_values, "FEISHU_APP_SECRET", "appSecret");
                put_json_string_from_env(&mut form, env_values, "FEISHU_DOMAIN", "domain");
                put_json_string_from_env(&mut form, env_values, "FEISHU_CONNECTION_MODE", "connectionMode");
                put_json_string_from_env(&mut form, env_values, "FEISHU_WEBHOOK_PATH", "webhookPath");
                insert_json_bool_if_present(&mut form, &extra, "typing_indicator", "typingIndicator");
                insert_json_bool_if_present(&mut form, &extra, "resolve_sender_names", "resolveSenderNames");
            }
            "dingtalk" => {
                insert_json_string_if_present(&mut form, &extra, "client_id", "clientId");
                insert_json_string_if_present(&mut form, &extra, "client_secret", "clientSecret");
                put_json_string_from_env(&mut form, env_values, "DINGTALK_CLIENT_ID", "clientId");
                put_json_string_from_env(&mut form, env_values, "DINGTALK_CLIENT_SECRET", "clientSecret");
            }
            "teams" => {
                for (yaml_key_name, json_key_name) in [
                    ("client_id", "clientId"),
                    ("client_secret", "clientSecret"),
                    ("tenant_id", "tenantId"),
                    ("service_url", "serviceUrl"),
                ] {
                    insert_json_string_if_present(&mut form, &extra, yaml_key_name, json_key_name);
                }
                insert_json_scalar_string_if_present(&mut form, &extra, "port", "port");
                insert_hermes_home_channel_if_present(&mut form, &entry);
                put_json_string_from_env(&mut form, env_values, "TEAMS_CLIENT_ID", "clientId");
                put_json_string_from_env(&mut form, env_values, "TEAMS_CLIENT_SECRET", "clientSecret");
                put_json_string_from_env(&mut form, env_values, "TEAMS_TENANT_ID", "tenantId");
                put_json_string_from_env(&mut form, env_values, "TEAMS_PORT", "port");
                put_json_string_from_env(&mut form, env_values, "TEAMS_SERVICE_URL", "serviceUrl");
                put_json_string_from_env(&mut form, env_values, "TEAMS_ALLOWED_USERS", "allowFrom");
                put_json_bool_from_env(&mut form, env_values, "TEAMS_ALLOW_ALL_USERS", "allowAllUsers");
                put_json_string_from_env(&mut form, env_values, "TEAMS_HOME_CHANNEL", "homeChannel");
                put_json_string_from_env(&mut form, env_values, "TEAMS_HOME_CHANNEL_NAME", "homeChannelName");
            }
            "google_chat" => {
                for (yaml_key_name, json_key_name) in [
                    ("project_id", "projectId"),
                    ("subscription_name", "subscriptionName"),
                    ("service_account_json", "serviceAccountJson"),
                ] {
                    insert_json_string_if_present(&mut form, &extra, yaml_key_name, json_key_name);
                }
                insert_hermes_home_channel_if_present(&mut form, &entry);
                if let Some(value) = hermes_env_value(env_values, "GOOGLE_CHAT_PROJECT_ID")
                    .or_else(|| hermes_env_value(env_values, "GOOGLE_CLOUD_PROJECT"))
                {
                    form.insert("projectId".to_string(), Value::String(value));
                }
                if let Some(value) = hermes_env_value(env_values, "GOOGLE_CHAT_SUBSCRIPTION_NAME")
                    .or_else(|| hermes_env_value(env_values, "GOOGLE_CHAT_SUBSCRIPTION"))
                {
                    form.insert("subscriptionName".to_string(), Value::String(value));
                }
                if let Some(value) = hermes_env_value(env_values, "GOOGLE_CHAT_SERVICE_ACCOUNT_JSON")
                    .or_else(|| hermes_env_value(env_values, "GOOGLE_APPLICATION_CREDENTIALS"))
                {
                    form.insert("serviceAccountJson".to_string(), Value::String(value));
                }
                put_json_string_from_env(&mut form, env_values, "GOOGLE_CHAT_ALLOWED_USERS", "allowFrom");
                put_json_bool_from_env(&mut form, env_values, "GOOGLE_CHAT_ALLOW_ALL_USERS", "allowAllUsers");
                put_json_string_from_env(&mut form, env_values, "GOOGLE_CHAT_HOME_CHANNEL", "homeChannel");
                put_json_string_from_env(&mut form, env_values, "GOOGLE_CHAT_HOME_CHANNEL_NAME", "homeChannelName");
            }
            "irc" => {
                for (yaml_key_name, json_key_name) in [
                    ("server", "server"),
                    ("channel", "channel"),
                    ("nickname", "nickname"),
                    ("server_password", "serverPassword"),
                    ("nickserv_password", "nickservPassword"),
                ] {
                    insert_json_string_if_present(&mut form, &extra, yaml_key_name, json_key_name);
                }
                insert_json_scalar_string_if_present(&mut form, &extra, "port", "port");
                insert_json_bool_if_present(&mut form, &extra, "use_tls", "useTls");
                insert_json_csv_if_present(&mut form, &extra, "allowed_users", "allowFrom");
                insert_hermes_home_channel_if_present(&mut form, &entry);
                put_json_string_from_env(&mut form, env_values, "IRC_SERVER", "server");
                put_json_string_from_env(&mut form, env_values, "IRC_CHANNEL", "channel");
                put_json_string_from_env(&mut form, env_values, "IRC_NICKNAME", "nickname");
                put_json_string_from_env(&mut form, env_values, "IRC_PORT", "port");
                put_json_bool_from_env(&mut form, env_values, "IRC_USE_TLS", "useTls");
                put_json_string_from_env(&mut form, env_values, "IRC_SERVER_PASSWORD", "serverPassword");
                put_json_string_from_env(&mut form, env_values, "IRC_NICKSERV_PASSWORD", "nickservPassword");
                put_json_string_from_env(&mut form, env_values, "IRC_ALLOWED_USERS", "allowFrom");
                put_json_bool_from_env(&mut form, env_values, "IRC_ALLOW_ALL_USERS", "allowAllUsers");
                put_json_string_from_env(&mut form, env_values, "IRC_HOME_CHANNEL", "homeChannel");
                put_json_string_from_env(&mut form, env_values, "IRC_HOME_CHANNEL_NAME", "homeChannelName");
            }
            "line" => {
                for (yaml_key_name, json_key_name) in [
                    ("channel_access_token", "channelAccessToken"),
                    ("channel_secret", "channelSecret"),
                    ("host", "host"),
                    ("public_url", "publicUrl"),
                    ("slow_response_threshold", "slowResponseThreshold"),
                ] {
                    insert_json_string_if_present(&mut form, &extra, yaml_key_name, json_key_name);
                }
                insert_json_scalar_string_if_present(&mut form, &extra, "port", "port");
                insert_json_csv_if_present(&mut form, &extra, "allowed_users", "allowFrom");
                insert_json_csv_if_present(&mut form, &extra, "allowed_groups", "allowedGroups");
                insert_json_csv_if_present(&mut form, &extra, "allowed_rooms", "allowedRooms");
                insert_hermes_home_channel_if_present(&mut form, &entry);
                put_json_string_from_env(&mut form, env_values, "LINE_CHANNEL_ACCESS_TOKEN", "channelAccessToken");
                put_json_string_from_env(&mut form, env_values, "LINE_CHANNEL_SECRET", "channelSecret");
                put_json_string_from_env(&mut form, env_values, "LINE_PORT", "port");
                put_json_string_from_env(&mut form, env_values, "LINE_HOST", "host");
                put_json_string_from_env(&mut form, env_values, "LINE_PUBLIC_URL", "publicUrl");
                put_json_string_from_env(&mut form, env_values, "LINE_ALLOWED_USERS", "allowFrom");
                put_json_string_from_env(&mut form, env_values, "LINE_ALLOWED_GROUPS", "allowedGroups");
                put_json_string_from_env(&mut form, env_values, "LINE_ALLOWED_ROOMS", "allowedRooms");
                put_json_bool_from_env(&mut form, env_values, "LINE_ALLOW_ALL_USERS", "allowAllUsers");
                put_json_string_from_env(&mut form, env_values, "LINE_HOME_CHANNEL", "homeChannel");
                put_json_string_from_env(&mut form, env_values, "LINE_SLOW_RESPONSE_THRESHOLD", "slowResponseThreshold");
            }
            "simplex" => {
                insert_json_string_if_present(&mut form, &extra, "ws_url", "wsUrl");
                insert_json_csv_if_present(&mut form, &extra, "allowed_users", "allowFrom");
                insert_hermes_home_channel_if_present(&mut form, &entry);
                put_json_string_from_env(&mut form, env_values, "SIMPLEX_WS_URL", "wsUrl");
                put_json_string_from_env(&mut form, env_values, "SIMPLEX_ALLOWED_USERS", "allowFrom");
                put_json_bool_from_env(&mut form, env_values, "SIMPLEX_ALLOW_ALL_USERS", "allowAllUsers");
                put_json_string_from_env(&mut form, env_values, "SIMPLEX_HOME_CHANNEL", "homeChannel");
                put_json_string_from_env(&mut form, env_values, "SIMPLEX_HOME_CHANNEL_NAME", "homeChannelName");
            }
            _ => {}
        }

        insert_json_string_if_present(&mut form, &extra, "dm_policy", "dmPolicy");
        insert_json_string_if_present(&mut form, &extra, "group_policy", "groupPolicy");
        insert_json_bool_if_present(&mut form, &extra, "require_mention", "requireMention");
        if platform == "dingtalk" {
            insert_json_csv_if_present(&mut form, &extra, "allowed_users", "allowFrom");
            insert_json_csv_if_present(&mut form, &extra, "allowed_chats", "groupAllowFrom");
        } else if ["irc", "line", "simplex"].contains(&platform) {
            insert_json_csv_if_present(&mut form, &extra, "allowed_users", "allowFrom");
        } else {
            insert_json_csv_if_present(&mut form, &extra, "allow_from", "allowFrom");
            insert_json_csv_if_present(&mut form, &extra, "group_allow_from", "groupAllowFrom");
        }
        insert_hermes_channel_display_fields(&mut form, config, platform);
        values.insert(platform.to_string(), Value::Object(form));
    }

    Value::Object(values)
}

fn ensure_yaml_object(value: &mut serde_yaml::Value) -> Result<&mut serde_yaml::Mapping, String> {
    if value.is_null() {
        *value = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
    }
    value.as_mapping_mut().ok_or_else(|| "config.yaml 顶层必须是对象".to_string())
}

fn yaml_child_object<'a>(parent: &'a mut serde_yaml::Mapping, key: &str) -> Result<&'a mut serde_yaml::Mapping, String> {
    let key_value = yaml_key(key);
    if !parent.get(&key_value).map(|value| value.is_mapping()).unwrap_or(false) {
        parent.insert(key_value.clone(), serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    }
    parent
        .get_mut(&key_value)
        .and_then(|value| value.as_mapping_mut())
        .ok_or_else(|| format!("{key} 必须是对象"))
}

fn set_extra_string_if_present(entry: &mut serde_yaml::Mapping, key: &str, value: Option<String>) {
    if let Some(value) = value.map(|v| v.trim().to_string()).filter(|v| !v.is_empty()) {
        if let Ok(extra) = yaml_child_object(entry, "extra") {
            extra.insert(yaml_key(key), serde_yaml::Value::String(value));
        }
    }
}

fn set_extra_integer_if_present(entry: &mut serde_yaml::Mapping, key: &str, value: Option<i64>) {
    if let Some(value) = value {
        if let Ok(extra) = yaml_child_object(entry, "extra") {
            extra.insert(yaml_key(key), serde_yaml::Value::Number(value.into()));
        }
    }
}

fn delete_yaml_key(entry: &mut serde_yaml::Mapping, key: &str) {
    entry.remove(yaml_key(key));
}

fn delete_extra_key(entry: &mut serde_yaml::Mapping, key: &str) {
    if let Some(extra) = entry.get_mut(yaml_key("extra")).and_then(|value| value.as_mapping_mut()) {
        extra.remove(yaml_key(key));
    }
}

fn set_extra_bool(entry: &mut serde_yaml::Mapping, key: &str, value: bool) {
    if let Ok(extra) = yaml_child_object(entry, "extra") {
        extra.insert(yaml_key(key), serde_yaml::Value::Bool(value));
    }
}

fn set_extra_string_array(entry: &mut serde_yaml::Mapping, key: &str, values: Vec<String>) {
    if let Ok(extra) = yaml_child_object(entry, "extra") {
        extra.insert(
            yaml_key(key),
            serde_yaml::Value::Sequence(values.into_iter().map(serde_yaml::Value::String).collect::<Vec<_>>()),
        );
    }
}

fn form_string(form: &Value, key: &str) -> Option<String> {
    form.get(key).and_then(|v| v.as_str()).map(|v| v.to_string())
}

fn form_i64(form: &Value, key: &str) -> Option<i64> {
    let value = form.get(key)?;
    if let Some(value) = value.as_i64() {
        Some(value)
    } else if let Some(value) = value.as_u64() {
        i64::try_from(value).ok()
    } else if let Some(value) = value.as_f64() {
        if value.is_finite() {
            Some(value as i64)
        } else {
            None
        }
    } else {
        value.as_str().and_then(|value| value.trim().parse::<i64>().ok())
    }
}

fn form_f64(form: &Value, key: &str) -> Option<f64> {
    let value = form.get(key)?;
    if let Some(value) = value.as_f64() {
        value.is_finite().then_some(value)
    } else {
        value
            .as_str()
            .and_then(|value| value.trim().parse::<f64>().ok())
            .filter(|value| value.is_finite())
    }
}

fn form_string_or_default(form: &Value, key: &str, default_value: &str) -> String {
    form_string(form, key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default_value.to_string())
}

fn form_bool(form: &Value, key: &str) -> Option<bool> {
    form.get(key).and_then(|value| {
        if let Some(b) = value.as_bool() {
            Some(b)
        } else {
            value
                .as_str()
                .map(|s| matches!(s.trim().to_ascii_lowercase().as_str(), "true" | "on" | "1" | "yes"))
        }
    })
}

fn form_string_array(form: &Value, key: &str) -> Option<Vec<String>> {
    let value = form.get(key)?;
    let items = if let Some(values) = value.as_array() {
        values
            .iter()
            .filter_map(|item| item.as_str())
            .flat_map(split_csv_items)
            .collect()
    } else if let Some(value) = value.as_str() {
        split_csv_items(value)
    } else {
        Vec::new()
    };
    Some(items)
}

fn set_hermes_home_channel(entry: &mut serde_yaml::Mapping, form: &Value) {
    if form.get("homeChannel").is_none() {
        return;
    }
    let chat_id = form_string(form, "homeChannel")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let Some(chat_id) = chat_id else {
        delete_yaml_key(entry, "home_channel");
        return;
    };
    let name = form_string(form, "homeChannelName")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| chat_id.clone());
    let mut home = serde_yaml::Mapping::new();
    home.insert(yaml_key("chat_id"), serde_yaml::Value::String(chat_id));
    home.insert(yaml_key("name"), serde_yaml::Value::String(name));
    entry.insert(yaml_key("home_channel"), serde_yaml::Value::Mapping(home));
}

fn merge_hermes_channel_display_config(root: &mut serde_yaml::Mapping, platform: &str, form: &Value) -> Result<(), String> {
    let has_display_fields = [
        "displayToolProgress",
        "displayShowReasoning",
        "displayToolPreviewLength",
        "displayStreaming",
        "displayCleanupProgress",
    ]
    .iter()
    .any(|key| form.get(*key).is_some());
    if !has_display_fields {
        return Ok(());
    }

    let tool_progress = if form.get("displayToolProgress").is_some() {
        Some(normalize_hermes_display_tool_progress(
            form_string(form, "displayToolProgress"),
            true,
            &format!("display.platforms.{platform}.tool_progress"),
        )?)
    } else {
        None
    };
    let show_reasoning = if form.get("displayShowReasoning").is_some() {
        Some(form_bool(form, "displayShowReasoning").unwrap_or(false))
    } else {
        None
    };
    let tool_preview_length = if form.get("displayToolPreviewLength").is_some() {
        Some(validate_hermes_i64(
            form_i64(form, "displayToolPreviewLength"),
            &format!("display.platforms.{platform}.tool_preview_length"),
            0,
            0,
            200000,
        )?)
    } else {
        None
    };
    let streaming = if form.get("displayStreaming").is_some() {
        Some(normalize_hermes_display_streaming_json(
            form.get("displayStreaming"),
            true,
            &format!("display.platforms.{platform}.streaming"),
        )?)
    } else {
        None
    };
    let cleanup_progress = if form.get("displayCleanupProgress").is_some() {
        Some(form_bool(form, "displayCleanupProgress").unwrap_or(false))
    } else {
        None
    };

    let display = yaml_child_object(root, "display")?;
    let platforms = yaml_child_object(display, "platforms")?;
    let platform_display = yaml_child_object(platforms, platform)?;
    if let Some(value) = tool_progress {
        platform_display.insert(yaml_key("tool_progress"), serde_yaml::Value::String(value));
    }
    if let Some(value) = show_reasoning {
        platform_display.insert(yaml_key("show_reasoning"), serde_yaml::Value::Bool(value));
    }
    if let Some(value) = tool_preview_length {
        platform_display.insert(yaml_key("tool_preview_length"), serde_yaml::Value::Number(value.into()));
    }
    if let Some(value) = streaming {
        if value == "inherit" {
            platform_display.remove(yaml_key("streaming"));
        } else {
            platform_display.insert(yaml_key("streaming"), serde_yaml::Value::Bool(value == "true"));
        }
    }
    if let Some(value) = cleanup_progress {
        platform_display.insert(yaml_key("cleanup_progress"), serde_yaml::Value::Bool(value));
    }
    Ok(())
}

fn split_csv_items(value: &str) -> Vec<String> {
    value
        .split([',', ';', '\n'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn normalize_hermes_dm_policy(value: Option<String>) -> String {
    let value = value.unwrap_or_default().trim().to_ascii_lowercase();
    match value.as_str() {
        "pairing" => "pair".to_string(),
        "allow" => "open".to_string(),
        "deny" => "disabled".to_string(),
        "pair" | "open" | "allowlist" | "disabled" => value,
        _ => "pair".to_string(),
    }
}

fn normalize_hermes_group_policy(value: Option<String>) -> String {
    let value = value.unwrap_or_default().trim().to_ascii_lowercase();
    match value.as_str() {
        "all" | "mentioned" => "open".to_string(),
        "deny" => "disabled".to_string(),
        "open" | "allowlist" | "disabled" => value,
        _ => "allowlist".to_string(),
    }
}

fn yaml_i64_field(map: &serde_yaml::Mapping, key: &str) -> Option<i64> {
    let value = yaml_get(map, key)?;
    if let Some(value) = value.as_i64() {
        Some(value)
    } else if let Some(value) = value.as_u64() {
        i64::try_from(value).ok()
    } else if let Some(value) = value.as_f64() {
        if value.is_finite() {
            Some(value as i64)
        } else {
            None
        }
    } else {
        value.as_str().and_then(|value| value.trim().parse::<i64>().ok())
    }
}

fn yaml_f64_field(map: &serde_yaml::Mapping, key: &str) -> Option<f64> {
    let value = yaml_get(map, key)?;
    if let Some(value) = value.as_f64() {
        value.is_finite().then_some(value)
    } else {
        value
            .as_str()
            .and_then(|value| value.trim().parse::<f64>().ok())
            .filter(|value| value.is_finite())
    }
}

fn bounded_hermes_i64(value: Option<i64>, fallback: i64, min: i64, max: i64) -> i64 {
    value.filter(|value| *value >= min && *value <= max).unwrap_or(fallback)
}

fn bounded_hermes_f64(value: Option<f64>, fallback: f64, min: f64, max: f64) -> f64 {
    value
        .filter(|value| value.is_finite() && *value >= min && *value <= max)
        .unwrap_or(fallback)
}

fn validate_hermes_i64(value: Option<i64>, key: &str, fallback: i64, min: i64, max: i64) -> Result<i64, String> {
    let value = value.unwrap_or(fallback);
    if value < min || value > max {
        return Err(format!("{key} 必须在 {min}-{max} 范围内"));
    }
    Ok(value)
}

fn validate_hermes_f64(value: Option<f64>, key: &str, fallback: f64, min: f64, max: f64) -> Result<f64, String> {
    let value = value.unwrap_or(fallback);
    if !value.is_finite() || value < min || value > max {
        return Err(format!("{key} 必须在 {min}-{max} 范围内"));
    }
    Ok((value * 10_000.0).round() / 10_000.0)
}

const HERMES_MODEL_CATALOG_DEFAULT_URL: &str = "https://hermes-agent.nousresearch.com/docs/api/model-catalog.json";

fn normalize_hermes_http_url(value: Option<String>, key: &str, fallback: &str, strict: bool) -> Result<String, String> {
    let raw = value.unwrap_or_default().trim().to_string();
    if raw.is_empty() {
        if strict && fallback.is_empty() {
            return Err(format!("{key} 不能为空"));
        }
        return Ok(fallback.to_string());
    }
    if raw.starts_with("http://") || raw.starts_with("https://") {
        return Ok(raw);
    }
    if strict {
        return Err(format!("{key} 必须是 http:// 或 https:// URL"));
    }
    Ok(fallback.to_string())
}

fn validate_hermes_model_catalog_providers(value: &Value) -> Result<serde_json::Map<String, Value>, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "model_catalog.providers 必须是 JSON object".to_string())?;
    let mut normalized = serde_json::Map::new();
    for (provider, raw_entry) in object {
        if provider.is_empty()
            || !provider
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
        {
            return Err(format!("model_catalog.providers.{provider} 名称只能包含字母、数字、下划线、点和短横线"));
        }
        let mut entry = raw_entry
            .as_object()
            .cloned()
            .ok_or_else(|| format!("model_catalog.providers.{provider} 必须是 object"))?;
        if entry.contains_key("url") {
            let url = normalize_hermes_http_url(
                entry.get("url").and_then(|value| value.as_str()).map(ToString::to_string),
                &format!("model_catalog.providers.{provider}.url"),
                "",
                true,
            )?;
            if url.is_empty() {
                entry.remove("url");
            } else {
                entry.insert("url".to_string(), Value::String(url));
            }
        }
        normalized.insert(provider.to_string(), Value::Object(entry));
    }
    Ok(normalized)
}
