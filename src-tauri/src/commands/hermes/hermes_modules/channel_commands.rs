
fn csv_env_value(form: &Value, key: &str) -> String {
    form_string_array(form, key).unwrap_or_default().join(",")
}

fn bool_env_value(value: bool) -> String {
    if value { "true" } else { "false" }.to_string()
}

fn build_hermes_channel_env_updates(platform: &str, form: &Value) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut push = |key: &str, value: String| {
        let value = value.trim().to_string();
        if !value.is_empty() {
            pairs.push((key.to_string(), value));
        }
    };

    match platform {
        "telegram" => {
            push("TELEGRAM_BOT_TOKEN", form_string(form, "botToken").unwrap_or_default());
            push("TELEGRAM_ALLOWED_USERS", csv_env_value(form, "allowFrom"));
            push("TELEGRAM_GROUP_ALLOWED_USERS", csv_env_value(form, "groupAllowFrom"));
            if let Some(value) = form_bool(form, "requireMention") {
                push("TELEGRAM_REQUIRE_MENTION", bool_env_value(value));
            }
            push(
                "TELEGRAM_REPLY_TO_MODE",
                normalize_hermes_telegram_reply_to_mode(form_string(form, "replyToMode"), true)
                    .unwrap_or_else(|_| "first".to_string()),
            );
            if let Some(value) = form_bool(form, "guestMode") {
                push("TELEGRAM_GUEST_MODE", bool_env_value(value));
            }
            if let Some(value) = form_bool(form, "disableLinkPreviews") {
                push("TELEGRAM_DISABLE_LINK_PREVIEWS", bool_env_value(value));
            }
        }
        "discord" => {
            push("DISCORD_BOT_TOKEN", form_string(form, "token").unwrap_or_default());
            push("DISCORD_ALLOWED_USERS", csv_env_value(form, "allowFrom"));
            if let Some(value) = form_bool(form, "requireMention") {
                push("DISCORD_REQUIRE_MENTION", bool_env_value(value));
            }
            push("DISCORD_FREE_RESPONSE_CHANNELS", csv_env_value(form, "freeResponseChannels"));
            push("DISCORD_ALLOWED_CHANNELS", csv_env_value(form, "allowedChannels"));
            push("DISCORD_IGNORED_CHANNELS", csv_env_value(form, "ignoredChannels"));
            push("DISCORD_NO_THREAD_CHANNELS", csv_env_value(form, "noThreadChannels"));
            if let Some(value) = form_bool(form, "autoThread") {
                push("DISCORD_AUTO_THREAD", bool_env_value(value));
            }
            if let Some(value) = form_bool(form, "reactions") {
                push("DISCORD_REACTIONS", bool_env_value(value));
            }
            if let Some(value) = form_bool(form, "threadRequireMention") {
                push("DISCORD_THREAD_REQUIRE_MENTION", bool_env_value(value));
            }
            if let Some(value) = form_bool(form, "historyBackfill") {
                push("DISCORD_HISTORY_BACKFILL", bool_env_value(value));
            }
            push(
                "DISCORD_HISTORY_BACKFILL_LIMIT",
                form_string(form, "historyBackfillLimit").unwrap_or_default(),
            );
            push("DISCORD_REPLY_TO_MODE", form_string(form, "replyToMode").unwrap_or_default());
            push("DISCORD_HOME_CHANNEL", form_string(form, "homeChannel").unwrap_or_default());
            push("DISCORD_HOME_CHANNEL_NAME", form_string(form, "homeChannelName").unwrap_or_default());
        }
        "slack" => {
            push("SLACK_BOT_TOKEN", form_string(form, "botToken").unwrap_or_default());
            push("SLACK_APP_TOKEN", form_string(form, "appToken").unwrap_or_default());
            push("SLACK_ALLOWED_USERS", csv_env_value(form, "allowFrom"));
            if let Some(value) = form_bool(form, "requireMention") {
                push("SLACK_REQUIRE_MENTION", bool_env_value(value));
            }
        }
        "feishu" => {
            push("FEISHU_APP_ID", form_string(form, "appId").unwrap_or_default());
            push("FEISHU_APP_SECRET", form_string(form, "appSecret").unwrap_or_default());
            push("FEISHU_DOMAIN", form_string_or_default(form, "domain", "feishu"));
            push("FEISHU_CONNECTION_MODE", form_string_or_default(form, "connectionMode", "websocket"));
            push("FEISHU_WEBHOOK_PATH", form_string_or_default(form, "webhookPath", "/feishu/webhook"));
            push("FEISHU_ALLOWED_USERS", csv_env_value(form, "allowFrom"));
            push("FEISHU_GROUP_POLICY", normalize_hermes_group_policy(form_string(form, "groupPolicy")));
            push(
                "FEISHU_REQUIRE_MENTION",
                bool_env_value(form_bool(form, "requireMention").unwrap_or(true)),
            );
            let reactions = form_string(form, "reactionNotifications").unwrap_or_default();
            push("FEISHU_REACTIONS", if reactions.trim() == "off" { "false" } else { "true" }.to_string());
        }
        "dingtalk" => {
            push("DINGTALK_CLIENT_ID", form_string(form, "clientId").unwrap_or_default());
            push("DINGTALK_CLIENT_SECRET", form_string(form, "clientSecret").unwrap_or_default());
            push("DINGTALK_ALLOWED_USERS", csv_env_value(form, "allowFrom"));
            push("DINGTALK_ALLOWED_CHATS", csv_env_value(form, "groupAllowFrom"));
            if let Some(value) = form_bool(form, "requireMention") {
                push("DINGTALK_REQUIRE_MENTION", bool_env_value(value));
            }
        }
        "teams" => {
            push("TEAMS_CLIENT_ID", form_string(form, "clientId").unwrap_or_default());
            push("TEAMS_CLIENT_SECRET", form_string(form, "clientSecret").unwrap_or_default());
            push("TEAMS_TENANT_ID", form_string(form, "tenantId").unwrap_or_default());
            push("TEAMS_PORT", form_string(form, "port").unwrap_or_default());
            push("TEAMS_SERVICE_URL", form_string(form, "serviceUrl").unwrap_or_default());
            push("TEAMS_ALLOWED_USERS", csv_env_value(form, "allowFrom"));
            if let Some(value) = form_bool(form, "allowAllUsers") {
                push("TEAMS_ALLOW_ALL_USERS", bool_env_value(value));
            }
            push("TEAMS_HOME_CHANNEL", form_string(form, "homeChannel").unwrap_or_default());
            push("TEAMS_HOME_CHANNEL_NAME", form_string(form, "homeChannelName").unwrap_or_default());
        }
        "google_chat" => {
            push("GOOGLE_CHAT_PROJECT_ID", form_string(form, "projectId").unwrap_or_default());
            push("GOOGLE_CHAT_SUBSCRIPTION_NAME", form_string(form, "subscriptionName").unwrap_or_default());
            push(
                "GOOGLE_CHAT_SERVICE_ACCOUNT_JSON",
                form_string(form, "serviceAccountJson").unwrap_or_default(),
            );
            push("GOOGLE_CHAT_ALLOWED_USERS", csv_env_value(form, "allowFrom"));
            if let Some(value) = form_bool(form, "allowAllUsers") {
                push("GOOGLE_CHAT_ALLOW_ALL_USERS", bool_env_value(value));
            }
            push("GOOGLE_CHAT_HOME_CHANNEL", form_string(form, "homeChannel").unwrap_or_default());
            push("GOOGLE_CHAT_HOME_CHANNEL_NAME", form_string(form, "homeChannelName").unwrap_or_default());
        }
        "irc" => {
            push("IRC_SERVER", form_string(form, "server").unwrap_or_default());
            push("IRC_PORT", form_string(form, "port").unwrap_or_default());
            push("IRC_NICKNAME", form_string(form, "nickname").unwrap_or_default());
            push("IRC_CHANNEL", form_string(form, "channel").unwrap_or_default());
            if let Some(value) = form_bool(form, "useTls") {
                push("IRC_USE_TLS", bool_env_value(value));
            }
            push("IRC_SERVER_PASSWORD", form_string(form, "serverPassword").unwrap_or_default());
            push("IRC_NICKSERV_PASSWORD", form_string(form, "nickservPassword").unwrap_or_default());
            push("IRC_ALLOWED_USERS", csv_env_value(form, "allowFrom"));
            if let Some(value) = form_bool(form, "allowAllUsers") {
                push("IRC_ALLOW_ALL_USERS", bool_env_value(value));
            }
            push("IRC_HOME_CHANNEL", form_string(form, "homeChannel").unwrap_or_default());
            push("IRC_HOME_CHANNEL_NAME", form_string(form, "homeChannelName").unwrap_or_default());
        }
        "line" => {
            push("LINE_CHANNEL_ACCESS_TOKEN", form_string(form, "channelAccessToken").unwrap_or_default());
            push("LINE_CHANNEL_SECRET", form_string(form, "channelSecret").unwrap_or_default());
            push("LINE_PORT", form_string(form, "port").unwrap_or_default());
            push("LINE_HOST", form_string(form, "host").unwrap_or_default());
            push("LINE_PUBLIC_URL", form_string(form, "publicUrl").unwrap_or_default());
            push("LINE_ALLOWED_USERS", csv_env_value(form, "allowFrom"));
            push("LINE_ALLOWED_GROUPS", csv_env_value(form, "allowedGroups"));
            push("LINE_ALLOWED_ROOMS", csv_env_value(form, "allowedRooms"));
            if let Some(value) = form_bool(form, "allowAllUsers") {
                push("LINE_ALLOW_ALL_USERS", bool_env_value(value));
            }
            push("LINE_HOME_CHANNEL", form_string(form, "homeChannel").unwrap_or_default());
            push(
                "LINE_SLOW_RESPONSE_THRESHOLD",
                form_string(form, "slowResponseThreshold").unwrap_or_default(),
            );
        }
        "simplex" => {
            push("SIMPLEX_WS_URL", form_string(form, "wsUrl").unwrap_or_default());
            push("SIMPLEX_ALLOWED_USERS", csv_env_value(form, "allowFrom"));
            if let Some(value) = form_bool(form, "allowAllUsers") {
                push("SIMPLEX_ALLOW_ALL_USERS", bool_env_value(value));
            }
            push("SIMPLEX_HOME_CHANNEL", form_string(form, "homeChannel").unwrap_or_default());
            push("SIMPLEX_HOME_CHANNEL_NAME", form_string(form, "homeChannelName").unwrap_or_default());
        }
        _ => {}
    }

    pairs
}

fn write_hermes_channel_env(platform: &str, form: &Value) -> Result<(), String> {
    let env_path = hermes_home().join(".env");
    if let Some(parent) = env_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建 Hermes 配置目录失败: {e}"))?;
    }
    let raw = std::fs::read_to_string(&env_path).unwrap_or_default();
    let managed_keys: Vec<&str> = match platform {
        "telegram" => vec![
            "TELEGRAM_BOT_TOKEN",
            "TELEGRAM_ALLOWED_USERS",
            "TELEGRAM_GROUP_ALLOWED_USERS",
            "TELEGRAM_REQUIRE_MENTION",
        ],
        "discord" => vec![
            "DISCORD_BOT_TOKEN",
            "DISCORD_ALLOWED_USERS",
            "DISCORD_REQUIRE_MENTION",
            "DISCORD_FREE_RESPONSE_CHANNELS",
            "DISCORD_ALLOWED_CHANNELS",
            "DISCORD_IGNORED_CHANNELS",
            "DISCORD_NO_THREAD_CHANNELS",
            "DISCORD_AUTO_THREAD",
            "DISCORD_REACTIONS",
            "DISCORD_THREAD_REQUIRE_MENTION",
            "DISCORD_HISTORY_BACKFILL",
            "DISCORD_HISTORY_BACKFILL_LIMIT",
            "DISCORD_REPLY_TO_MODE",
            "DISCORD_HOME_CHANNEL",
            "DISCORD_HOME_CHANNEL_NAME",
        ],
        "slack" => vec![
            "SLACK_BOT_TOKEN",
            "SLACK_APP_TOKEN",
            "SLACK_ALLOWED_USERS",
            "SLACK_REQUIRE_MENTION",
        ],
        "feishu" => vec![
            "FEISHU_APP_ID",
            "FEISHU_APP_SECRET",
            "FEISHU_DOMAIN",
            "FEISHU_CONNECTION_MODE",
            "FEISHU_WEBHOOK_PATH",
            "FEISHU_ALLOWED_USERS",
            "FEISHU_GROUP_POLICY",
            "FEISHU_REQUIRE_MENTION",
            "FEISHU_REACTIONS",
        ],
        "dingtalk" => vec![
            "DINGTALK_CLIENT_ID",
            "DINGTALK_CLIENT_SECRET",
            "DINGTALK_ALLOWED_USERS",
            "DINGTALK_ALLOWED_CHATS",
            "DINGTALK_REQUIRE_MENTION",
        ],
        "teams" => vec![
            "TEAMS_CLIENT_ID",
            "TEAMS_CLIENT_SECRET",
            "TEAMS_TENANT_ID",
            "TEAMS_PORT",
            "TEAMS_SERVICE_URL",
            "TEAMS_ALLOWED_USERS",
            "TEAMS_ALLOW_ALL_USERS",
            "TEAMS_HOME_CHANNEL",
            "TEAMS_HOME_CHANNEL_NAME",
        ],
        "google_chat" => vec![
            "GOOGLE_CHAT_PROJECT_ID",
            "GOOGLE_CHAT_SUBSCRIPTION_NAME",
            "GOOGLE_CHAT_SERVICE_ACCOUNT_JSON",
            "GOOGLE_CHAT_ALLOWED_USERS",
            "GOOGLE_CHAT_ALLOW_ALL_USERS",
            "GOOGLE_CHAT_HOME_CHANNEL",
            "GOOGLE_CHAT_HOME_CHANNEL_NAME",
        ],
        "irc" => vec![
            "IRC_SERVER",
            "IRC_PORT",
            "IRC_NICKNAME",
            "IRC_CHANNEL",
            "IRC_USE_TLS",
            "IRC_SERVER_PASSWORD",
            "IRC_NICKSERV_PASSWORD",
            "IRC_ALLOWED_USERS",
            "IRC_ALLOW_ALL_USERS",
            "IRC_HOME_CHANNEL",
            "IRC_HOME_CHANNEL_NAME",
        ],
        "line" => vec![
            "LINE_CHANNEL_ACCESS_TOKEN",
            "LINE_CHANNEL_SECRET",
            "LINE_PORT",
            "LINE_HOST",
            "LINE_PUBLIC_URL",
            "LINE_ALLOWED_USERS",
            "LINE_ALLOWED_GROUPS",
            "LINE_ALLOWED_ROOMS",
            "LINE_ALLOW_ALL_USERS",
            "LINE_HOME_CHANNEL",
            "LINE_SLOW_RESPONSE_THRESHOLD",
        ],
        "simplex" => vec![
            "SIMPLEX_WS_URL",
            "SIMPLEX_ALLOWED_USERS",
            "SIMPLEX_ALLOW_ALL_USERS",
            "SIMPLEX_HOME_CHANNEL",
            "SIMPLEX_HOME_CHANNEL_NAME",
        ],
        _ => Vec::new(),
    };
    let pairs = build_hermes_channel_env_updates(platform, form);
    let content = merge_env_file(&raw, &managed_keys, &pairs);
    std::fs::write(&env_path, content).map_err(|e| format!("写入 .env 失败: {e}"))
}

#[tauri::command]
pub fn hermes_channel_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    let env_values = read_hermes_channel_env_values();
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_channel_config_values(&config, &env_values),
    }))
}

#[tauri::command]
pub fn hermes_channel_config_save(platform: String, form: Value) -> Result<Value, String> {
    let platform =
        normalize_hermes_channel_platform(&platform).ok_or_else(|| format!("不支持的 Hermes 渠道: {}", platform.trim()))?;
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_channel_config(&mut config, platform, &form)?;
    write_hermes_yaml_config(&config_path, &config)?;
    write_hermes_channel_env(platform, &form)?;
    let mut env_values = read_hermes_channel_env_values();
    for (key, value) in build_hermes_channel_env_updates(platform, &form) {
        env_values.insert(key, value);
    }
    let values = build_hermes_channel_config_values(&config, &env_values);
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "values": values.get(platform).cloned().unwrap_or(Value::Null),
    }))
}

#[tauri::command]
pub fn hermes_session_runtime_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_session_runtime_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_session_runtime_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_session_runtime_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_session_runtime_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_compression_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_compression_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_compression_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_compression_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_compression_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_prompt_caching_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_prompt_caching_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_prompt_caching_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_prompt_caching_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_prompt_caching_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_openrouter_cache_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_openrouter_cache_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_openrouter_cache_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_openrouter_cache_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_openrouter_cache_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_provider_routing_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_provider_routing_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_provider_routing_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_provider_routing_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_provider_routing_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_auxiliary_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_auxiliary_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_auxiliary_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_auxiliary_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_auxiliary_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_tool_loop_guardrails_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_tool_loop_guardrails_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_tool_loop_guardrails_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_tool_loop_guardrails_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_tool_loop_guardrails_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_memory_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_memory_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_memory_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_memory_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_memory_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_skills_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_skills_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_skills_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_skills_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_skills_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_curator_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_curator_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_curator_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_curator_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_curator_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_quick_commands_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_quick_commands_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_quick_commands_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_quick_commands_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_quick_commands_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_model_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_model_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_model_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_model_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_model_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_model_aliases_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_model_aliases_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_model_aliases_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_model_aliases_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_model_aliases_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_hooks_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_hooks_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_hooks_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_hooks_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_hooks_config_values(&config),
    }))
}
