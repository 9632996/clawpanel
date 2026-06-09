fn save_discord_platform(
    cfg: &mut Value,
    storage_key: &str,
    form_obj: &Map<String, Value>,
    current_saved: &Value,
    account_id: Option<&str>,
) -> Result<(), String> {
    let channels_map = messaging_channels_map(cfg)?;
    let _ = storage_key;
    let _ = current_saved;
    let _ = account_id;
            let mut entry = Map::new();

            // Bot Token
            if let Some(t) = form_obj.get("token").and_then(|v| v.as_str()) {
                entry.insert("token".into(), Value::String(t.trim().into()));
            }
            put_string(&mut entry, "applicationId", form_string(form_obj, "applicationId"));
            entry.insert("enabled".into(), Value::Bool(true));
            put_string(&mut entry, "dmPolicy", form_string(form_obj, "dmPolicy"));
            put_string(&mut entry, "groupPolicy", form_string(form_obj, "groupPolicy"));
            put_array_from_form_value(&mut entry, "allowFrom", form_obj.get("allowFrom"));

            // guildId + channelId 展开为 guilds 嵌套结构
            let guild_id = form_obj
                .get("guildId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if !guild_id.is_empty() {
                let channel_id = form_obj
                    .get("channelId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let channel_key = if channel_id.is_empty() { "*".to_string() } else { channel_id };
                let mut channels = Map::new();
                channels.insert(channel_key, crate::jv!({ "allow": true, "requireMention": true }));
                let mut guild = Map::new();
                guild.insert("users".into(), crate::jv!(["*"]));
                guild.insert("requireMention".into(), Value::Bool(true));
                guild.insert("channels".into(), Value::Object(channels));
                let mut guilds = Map::new();
                guilds.insert(guild_id, Value::Object(guild));
                entry.insert("guilds".into(), Value::Object(guilds));
            }

            // 合并到现有配置，保留用户通过 CLI 设置的 streaming / retry / dmPolicy 等
            preserve_messaging_credential_refs(&mut entry, form_obj, current_saved);
            merge_channel_entry_for_account(channels_map, "discord", account_id, entry)?;
            // 仅在首次创建时设置默认值，不覆盖用户已有的设置
            if let Some(Value::Object(d)) = channels_map.get_mut("discord") {
                d.entry("groupPolicy").or_insert(Value::String("allowlist".into()));
                d.entry("dm").or_insert(crate::jv!({ "enabled": false }));
                d.entry("retry").or_insert(crate::jv!({
                    "attempts": 3,
                    "minDelayMs": 500,
                    "maxDelayMs": 30000,
                    "jitter": 0.1
                }));
            }
    Ok(())
}

fn save_telegram_platform(
    cfg: &mut Value,
    storage_key: &str,
    form_obj: &Map<String, Value>,
    current_saved: &Value,
    account_id: Option<&str>,
) -> Result<(), String> {
    let channels_map = messaging_channels_map(cfg)?;
    let _ = storage_key;
    let _ = current_saved;
    let _ = account_id;
            let mut entry = Map::new();

            if let Some(t) = form_obj.get("botToken").and_then(|v| v.as_str()) {
                entry.insert("botToken".into(), Value::String(t.trim().into()));
            }
            entry.insert("enabled".into(), Value::Bool(true));
            put_string(&mut entry, "dmPolicy", form_string(form_obj, "dmPolicy"));
            put_string(&mut entry, "groupPolicy", form_string(form_obj, "groupPolicy"));
            put_array_from_form_value(&mut entry, "allowFrom", form_obj.get("allowFrom"));

            preserve_messaging_credential_refs(&mut entry, form_obj, current_saved);
            merge_channel_entry_for_account(channels_map, "telegram", account_id, entry)?;
    Ok(())
}

fn save_zalo_platform(
    cfg: &mut Value,
    storage_key: &str,
    form_obj: &Map<String, Value>,
    current_saved: &Value,
    account_id: Option<&str>,
) -> Result<(), String> {
    let channels_map = messaging_channels_map(cfg)?;
    let _ = storage_key;
    let _ = current_saved;
    let _ = account_id;
            let bot_token = form_string(form_obj, "botToken");
            let token_file = form_string(form_obj, "tokenFile");
            if bot_token.is_empty() && token_file.is_empty() {
                return Err("Bot Token 或 Token File 至少填写一项".into());
            }

            let mut entry = Map::new();
            entry.insert("enabled".into(), Value::Bool(true));
            put_string(&mut entry, "botToken", bot_token);
            put_string(&mut entry, "tokenFile", token_file);
            put_string(&mut entry, "webhookUrl", form_string(form_obj, "webhookUrl"));
            put_string(&mut entry, "webhookSecret", form_string(form_obj, "webhookSecret"));
            put_string(&mut entry, "webhookPath", form_string(form_obj, "webhookPath"));
            put_string(&mut entry, "proxy", form_string(form_obj, "proxy"));
            put_string(&mut entry, "responsePrefix", form_string(form_obj, "responsePrefix"));
            put_string(&mut entry, "dmPolicy", form_string(form_obj, "dmPolicy"));
            put_string(&mut entry, "groupPolicy", form_string(form_obj, "groupPolicy"));
            put_array_from_form_value(&mut entry, "allowFrom", form_obj.get("allowFrom"));
            put_array_from_form_value(&mut entry, "groupAllowFrom", form_obj.get("groupAllowFrom"));
            if let Some(value) = form_obj.get("mediaMaxMb").and_then(|v| v.as_f64()) {
                if let Some(number) = serde_json::Number::from_f64(value) {
                    entry.insert("mediaMaxMb".into(), Value::Number(number));
                }
            } else {
                put_number_from_form(&mut entry, "mediaMaxMb", &form_string(form_obj, "mediaMaxMb"));
            }
            preserve_messaging_credential_refs(&mut entry, form_obj, current_saved);
            merge_channel_entry_for_account(channels_map, storage_key, account_id, entry)?;
            ensure_plugin_allowed(cfg, "zalo")?;
    Ok(())
}

fn save_zalouser_platform(
    cfg: &mut Value,
    storage_key: &str,
    form_obj: &Map<String, Value>,
    current_saved: &Value,
    account_id: Option<&str>,
) -> Result<(), String> {
    let channels_map = messaging_channels_map(cfg)?;
    let _ = storage_key;
    let _ = current_saved;
    let _ = account_id;
            let mut entry = Map::new();
            entry.insert("enabled".into(), Value::Bool(true));
            put_string(&mut entry, "profile", form_string(form_obj, "profile"));
            put_string(&mut entry, "messagePrefix", form_string(form_obj, "messagePrefix"));
            put_string(&mut entry, "responsePrefix", form_string(form_obj, "responsePrefix"));
            put_string(&mut entry, "dmPolicy", form_string(form_obj, "dmPolicy"));
            put_string(&mut entry, "groupPolicy", form_string(form_obj, "groupPolicy"));
            put_array_from_form_value(&mut entry, "allowFrom", form_obj.get("allowFrom"));
            put_array_from_form_value(&mut entry, "groupAllowFrom", form_obj.get("groupAllowFrom"));
            put_bool_value_if_present(&mut entry, "dangerouslyAllowNameMatching", form_obj.get("dangerouslyAllowNameMatching"));
            if let Some(value) = form_obj.get("historyLimit").and_then(|v| v.as_f64()) {
                if let Some(number) = serde_json::Number::from_f64(value) {
                    entry.insert("historyLimit".into(), Value::Number(number));
                }
            } else {
                put_number_from_form(&mut entry, "historyLimit", &form_string(form_obj, "historyLimit"));
            }
            merge_channel_entry_for_account(channels_map, storage_key, account_id, entry)?;
            ensure_plugin_allowed(cfg, "zalouser")?;
    Ok(())
}

fn save_qqbot_platform(
    cfg: &mut Value,
    storage_key: &str,
    form_obj: &Map<String, Value>,
    current_saved: &Value,
    account_id: Option<&str>,
) -> Result<(), String> {
    let channels_map = messaging_channels_map(cfg)?;
    let _ = storage_key;
    let _ = current_saved;
    let _ = account_id;
            let app_id = form_obj
                .get("appId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            // 优先取 clientSecret（腾讯官方插件字段名）
            // 也兼容前端 UI 传 appSecret（旧字段名）
            let client_secret = form_obj
                .get("clientSecret")
                .or_else(|| form_obj.get("appSecret"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();

            if app_id.is_empty() {
                return Err("AppID 不能为空".into());
            }
            if client_secret.is_empty() {
                return Err("ClientSecret 不能为空".into());
            }

            // 与 `openclaw channels add --channel qqbot --token "AppID:Secret"` 一致：凭证写在 accounts.<id> 下，并保留组合 token
            let acct_key = account_id.map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(QQBOT_DEFAULT_ACCOUNT_ID);
            let token_combo = format!("{}:{}", app_id, client_secret);

            let qqbot_node = channels_map.entry("qqbot").or_insert_with(|| crate::jv!({ "enabled": true }));
            let qqbot_obj = qqbot_node.as_object_mut().ok_or("qqbot 节点格式错误")?;
            qqbot_obj.insert("enabled".into(), Value::Bool(true));
            // 清除写在根上的旧字段，避免官方插件只认 accounts.* 时读不到账号
            qqbot_obj.remove("appId");
            qqbot_obj.remove("clientSecret");
            qqbot_obj.remove("appSecret");
            qqbot_obj.remove("token");

            let accounts = qqbot_obj.entry("accounts").or_insert_with(|| crate::jv!({}));
            let accounts_obj = accounts.as_object_mut().ok_or("accounts 格式错误")?;
            let mut entry = Map::new();
            entry.insert("appId".into(), Value::String(app_id));
            entry.insert("clientSecret".into(), Value::String(client_secret));
            entry.insert("token".into(), Value::String(token_combo));
            entry.insert("enabled".into(), Value::Bool(true));
            accounts_obj.insert(acct_key.to_string(), Value::Object(entry));

            ensure_openclaw_qqbot_plugin(cfg)?;
            ensure_chat_completions_enabled(cfg)?;
            let _ = cleanup_legacy_plugin_backup_dir("qqbot");
    Ok(())
}

fn save_feishu_platform(
    cfg: &mut Value,
    storage_key: &str,
    form_obj: &Map<String, Value>,
    current_saved: &Value,
    account_id: Option<&str>,
) -> Result<(), String> {
    let channels_map = messaging_channels_map(cfg)?;
    let _ = storage_key;
    let _ = current_saved;
    let _ = account_id;
            let app_id = form_obj
                .get("appId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let app_secret = form_obj
                .get("appSecret")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();

            if app_id.is_empty() || app_secret.is_empty() {
                return Err("App ID 和 App Secret 不能为空".into());
            }

            let mut entry = Map::new();
            entry.insert("appId".into(), Value::String(app_id));
            entry.insert("appSecret".into(), Value::String(app_secret));
            entry.insert("enabled".into(), Value::Bool(true));
            put_string(&mut entry, "connectionMode", form_string(form_obj, "connectionMode"));
            put_string(&mut entry, "domain", form_string(form_obj, "domain"));
            put_string(&mut entry, "webhookPath", form_string(form_obj, "webhookPath"));
            put_string(&mut entry, "dmPolicy", form_string(form_obj, "dmPolicy"));
            put_string(&mut entry, "groupPolicy", form_string(form_obj, "groupPolicy"));
            put_string(&mut entry, "reactionNotifications", form_string(form_obj, "reactionNotifications"));
            put_array_from_form_value(&mut entry, "allowFrom", form_obj.get("allowFrom"));
            put_bool_value_if_present(&mut entry, "typingIndicator", form_obj.get("typingIndicator"));
            put_bool_value_if_present(&mut entry, "resolveSenderNames", form_obj.get("resolveSenderNames"));
            put_bool_value_if_present(&mut entry, "requireMention", form_obj.get("requireMention"));
            preserve_messaging_credential_refs(&mut entry, form_obj, current_saved);

            merge_channel_entry_for_account(channels_map, storage_key, account_id, entry)?;
            ensure_plugin_allowed(cfg, "openclaw-lark")?;
            // 禁用旧版 feishu 插件，防止新旧插件同时运行冲突
            disable_legacy_plugin(cfg, "feishu");
            let _ = cleanup_legacy_plugin_backup_dir("feishu");
            let _ = cleanup_legacy_plugin_backup_dir("openclaw-lark");
    Ok(())
}

fn save_dingtalk_platform(
    cfg: &mut Value,
    storage_key: &str,
    form_obj: &Map<String, Value>,
    current_saved: &Value,
    account_id: Option<&str>,
) -> Result<(), String> {
    let channels_map = messaging_channels_map(cfg)?;
    let _ = storage_key;
    let _ = current_saved;
    let _ = account_id;
            let client_id = form_obj
                .get("clientId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let client_secret = form_obj
                .get("clientSecret")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();

            if client_id.is_empty() || client_secret.is_empty() {
                return Err("Client ID 和 Client Secret 不能为空".into());
            }

            let mut entry = Map::new();
            entry.insert("clientId".into(), Value::String(client_id));
            entry.insert("clientSecret".into(), Value::String(client_secret));
            entry.insert("enabled".into(), Value::Bool(true));

            let gateway_token = form_obj.get("gatewayToken").and_then(|v| v.as_str()).unwrap_or("").trim();
            if !gateway_token.is_empty() {
                entry.insert("gatewayToken".into(), Value::String(gateway_token.into()));
            }

            let gateway_password = form_obj.get("gatewayPassword").and_then(|v| v.as_str()).unwrap_or("").trim();
            if !gateway_password.is_empty() {
                entry.insert("gatewayPassword".into(), Value::String(gateway_password.into()));
            }

            preserve_messaging_credential_refs(&mut entry, form_obj, current_saved);
            merge_channel_entry_for_account(channels_map, storage_key, account_id, entry)?;
            ensure_plugin_allowed(cfg, "dingtalk-connector")?;
            ensure_chat_completions_enabled(cfg)?;
            let _ = cleanup_legacy_plugin_backup_dir("dingtalk-connector");
    Ok(())
}

fn save_slack_platform(
    cfg: &mut Value,
    storage_key: &str,
    form_obj: &Map<String, Value>,
    current_saved: &Value,
    account_id: Option<&str>,
) -> Result<(), String> {
    let channels_map = messaging_channels_map(cfg)?;
    let _ = storage_key;
    let _ = current_saved;
    let _ = account_id;
            let mode = form_string(form_obj, "mode");
            let bot_token = form_string(form_obj, "botToken");
            let app_token = form_string(form_obj, "appToken");
            let signing_secret = form_string(form_obj, "signingSecret");

            if bot_token.is_empty() {
                return Err("Slack Bot Token 不能为空".into());
            }
            if mode == "http" && signing_secret.is_empty() {
                return Err("HTTP 模式下 Signing Secret 不能为空".into());
            }
            if mode != "http" && app_token.is_empty() {
                return Err("Socket 模式下 App Token 不能为空".into());
            }

            let mut entry = Map::new();
            entry.insert("enabled".into(), Value::Bool(true));
            put_string(&mut entry, "mode", if mode.is_empty() { "socket".into() } else { mode });
            put_string(&mut entry, "botToken", bot_token);
            put_string(&mut entry, "appToken", app_token);
            put_string(&mut entry, "signingSecret", signing_secret);
            put_string(&mut entry, "webhookPath", form_string(form_obj, "webhookPath"));
            put_string(&mut entry, "teamId", form_string(form_obj, "teamId"));
            put_string(&mut entry, "appId", form_string(form_obj, "appId"));
            put_bool_value_if_present(&mut entry, "userTokenReadOnly", form_obj.get("userTokenReadOnly"));
            put_bool_value_if_present(&mut entry, "requireMention", form_obj.get("requireMention"));
            put_string(&mut entry, "dmPolicy", form_string(form_obj, "dmPolicy"));
            put_string(&mut entry, "groupPolicy", form_string(form_obj, "groupPolicy"));
            put_array_from_form_value(&mut entry, "allowFrom", form_obj.get("allowFrom"));
            preserve_messaging_credential_refs(&mut entry, form_obj, current_saved);
            merge_channel_entry_for_account(channels_map, storage_key, account_id, entry)?;
    Ok(())
}

fn save_whatsapp_platform(
    cfg: &mut Value,
    storage_key: &str,
    form_obj: &Map<String, Value>,
    current_saved: &Value,
    account_id: Option<&str>,
) -> Result<(), String> {
    let channels_map = messaging_channels_map(cfg)?;
    let _ = storage_key;
    let _ = current_saved;
    let _ = account_id;
            let mut entry = Map::new();
            entry.insert("enabled".into(), Value::Bool(true));
            put_bool_value_if_present(&mut entry, "enabled", form_obj.get("enabled"));
            for key in [
                "defaultTo",
                "contextVisibility",
                "chunkMode",
                "reactionLevel",
                "replyToMode",
                "messagePrefix",
                "responsePrefix",
            ] {
                put_string(&mut entry, key, form_string(form_obj, key));
            }
            put_string(&mut entry, "dmPolicy", form_string(form_obj, "dmPolicy"));
            put_string(&mut entry, "groupPolicy", form_string(form_obj, "groupPolicy"));
            put_array_from_form_value(&mut entry, "allowFrom", form_obj.get("allowFrom"));
            put_array_from_form_value(&mut entry, "groupAllowFrom", form_obj.get("groupAllowFrom"));
            for key in ["configWrites", "sendReadReceipts", "selfChatMode", "blockStreaming"] {
                put_bool_value_if_present(&mut entry, key, form_obj.get(key));
            }
            for key in ["historyLimit", "dmHistoryLimit", "mediaMaxMb", "debounceMs", "textChunkLimit"] {
                put_number_value_if_present(&mut entry, key, form_obj.get(key));
            }
            let mut ack_reaction = current_saved
                .get("ackReaction")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();
            put_string(&mut ack_reaction, "emoji", form_string(form_obj, "ackEmoji"));
            put_bool_value_if_present(&mut ack_reaction, "direct", form_obj.get("ackDirect"));
            put_string(&mut ack_reaction, "group", form_string(form_obj, "ackGroup"));
            if !ack_reaction.is_empty() {
                entry.insert("ackReaction".into(), Value::Object(ack_reaction));
            }
            merge_channel_entry_for_account(channels_map, storage_key, account_id, entry)?;
            ensure_plugin_allowed(cfg, "whatsapp")?;
    Ok(())
}

fn save_signal_platform(
    cfg: &mut Value,
    storage_key: &str,
    form_obj: &Map<String, Value>,
    current_saved: &Value,
    account_id: Option<&str>,
) -> Result<(), String> {
    let channels_map = messaging_channels_map(cfg)?;
    let _ = storage_key;
    let _ = current_saved;
    let _ = account_id;
            let account = form_string(form_obj, "account");
            if account.is_empty() {
                return Err("Signal 号码不能为空".into());
            }

            let mut entry = Map::new();
            entry.insert("enabled".into(), Value::Bool(true));
            put_string(&mut entry, "account", account);
            put_string(&mut entry, "cliPath", form_string(form_obj, "cliPath"));
            put_string(&mut entry, "httpUrl", form_string(form_obj, "httpUrl"));
            put_string(&mut entry, "httpHost", form_string(form_obj, "httpHost"));
            put_number_from_form(&mut entry, "httpPort", &form_string(form_obj, "httpPort"));
            put_string(&mut entry, "responsePrefix", form_string(form_obj, "responsePrefix"));
            put_string(&mut entry, "dmPolicy", form_string(form_obj, "dmPolicy"));
            put_string(&mut entry, "groupPolicy", form_string(form_obj, "groupPolicy"));
            put_array_from_form_value(&mut entry, "allowFrom", form_obj.get("allowFrom"));
            put_array_from_form_value(&mut entry, "groupAllowFrom", form_obj.get("groupAllowFrom"));
            put_bool_value_if_present(&mut entry, "blockStreaming", form_obj.get("blockStreaming"));
            for key in ["historyLimit", "dmHistoryLimit", "textChunkLimit", "mediaMaxMb"] {
                put_number_from_form(&mut entry, key, &form_string(form_obj, key));
            }
            preserve_messaging_credential_refs(&mut entry, form_obj, current_saved);
            merge_channel_entry_for_account(channels_map, storage_key, account_id, entry)?;
    Ok(())
}

fn save_imessage_platform(
    cfg: &mut Value,
    storage_key: &str,
    form_obj: &Map<String, Value>,
    current_saved: &Value,
    account_id: Option<&str>,
) -> Result<(), String> {
    let channels_map = messaging_channels_map(cfg)?;
    let _ = storage_key;
    let _ = current_saved;
    let _ = account_id;
            let mut entry = Map::new();
            entry.insert("enabled".into(), Value::Bool(true));
            for key in [
                "cliPath",
                "dbPath",
                "remoteHost",
                "service",
                "region",
                "defaultTo",
                "contextVisibility",
                "chunkMode",
                "reactionNotifications",
                "responsePrefix",
            ] {
                put_string(&mut entry, key, form_string(form_obj, key));
            }
            put_string(&mut entry, "dmPolicy", form_string(form_obj, "dmPolicy"));
            put_string(&mut entry, "groupPolicy", form_string(form_obj, "groupPolicy"));
            put_array_from_form_value(&mut entry, "allowFrom", form_obj.get("allowFrom"));
            put_array_from_form_value(&mut entry, "groupAllowFrom", form_obj.get("groupAllowFrom"));
            put_array_from_form_value(&mut entry, "attachmentRoots", form_obj.get("attachmentRoots"));
            put_array_from_form_value(&mut entry, "remoteAttachmentRoots", form_obj.get("remoteAttachmentRoots"));
            for key in [
                "configWrites",
                "includeAttachments",
                "blockStreaming",
                "sendReadReceipts",
                "coalesceSameSenderDms",
            ] {
                put_bool_value_if_present(&mut entry, key, form_obj.get(key));
            }
            for key in [
                "historyLimit",
                "dmHistoryLimit",
                "mediaMaxMb",
                "probeTimeoutMs",
                "textChunkLimit",
            ] {
                put_number_value_if_present(&mut entry, key, form_obj.get(key));
            }
            merge_channel_entry_for_account(channels_map, storage_key, account_id, entry)?;
            ensure_plugin_allowed(cfg, "imessage")?;
    Ok(())
}

include!("save_handlers_primary/extra_handlers.rs");