/// 读取指定平台的当前配置（从 openclaw.json 中提取表单可用的值）
/// account_id: 可选，指定时读取 channels.<platform>.accounts.<account_id>（多账号模式）
#[tauri::command]
pub async fn read_platform_config(platform: String, account_id: Option<String>) -> Result<Value, String> {
    let mut cfg = super::config::load_openclaw_json()?;
    let storage_key = platform_storage_key(&platform);

    let mut form = Map::new();

    // 多账号模式：读凭证位置
    // 飞书：credentials 可写在 root 或 accounts.<id> 下，优先找非空那个
    let channel_root = cfg.get("channels").and_then(|c| c.get(storage_key));
    let saved = resolve_platform_config_entry(channel_root, &platform, account_id.as_deref()).unwrap_or(Value::Null);

    let exists = !saved.is_null();

    match platform.as_str() {
        "discord" => {
            if saved.is_null() {
                return Ok(crate::jv!({ "exists": false }));
            }
            // Discord 配置在 openclaw.json 中是展开的 guilds 结构
            // 需要反向提取成表单字段：token, guildId, channelId
            insert_secret_aware_form_value(&mut form, &saved, "token");
            insert_string_if_present(&mut form, &saved, "applicationId");
            insert_access_policy_form_values(&mut form, &saved, false, false);
            if let Some(guilds) = saved.get("guilds").and_then(|v| v.as_object()) {
                if let Some(gid) = guilds.keys().next() {
                    form.insert("guildId".into(), Value::String(gid.clone()));
                    if let Some(channels) = guilds[gid].get("channels").and_then(|v| v.as_object()) {
                        let cids: Vec<&String> = channels.keys().filter(|k| k.as_str() != "*").collect();
                        if let Some(cid) = cids.first() {
                            form.insert("channelId".into(), Value::String((*cid).clone()));
                        }
                    }
                }
            }
        }
        "telegram" => {
            if saved.is_null() {
                return Ok(crate::jv!({ "exists": false }));
            }
            // Telegram: botToken 直接保存, allowFrom 数组需要拼回逗号字符串
            insert_secret_aware_form_value(&mut form, &saved, "botToken");
            insert_access_policy_form_values(&mut form, &saved, true, false);
        }
        "qqbot" => {
            // 多账号：读 accounts.<account_id>；单账号：先读 qqbot 根节点，若无凭证再读 accounts.default（与官方 CLI 一致）
            let qqbot_val: &Value = match (&account_id, channel_root) {
                (Some(acct), Some(ch)) if !acct.is_empty() => ch
                    .get("accounts")
                    .and_then(|a| a.get(acct.as_str()))
                    .filter(|v| !v.is_null())
                    .unwrap_or(&Value::Null),
                (_, Some(ch)) => {
                    if qqbot_channel_has_credentials(ch) {
                        ch
                    } else {
                        ch.get("accounts")
                            .and_then(|a| a.get(QQBOT_DEFAULT_ACCOUNT_ID))
                            .filter(|v| !v.is_null())
                            .unwrap_or(ch)
                    }
                }
                _ => &Value::Null,
            };

            let mut needs_migrate = false;
            let mut app_id_val: Option<&str> = None;
            let mut client_secret_val: Option<&str> = None;

            // 优先读新格式 appId + clientSecret
            if let Some(v) = qqbot_val.get("appId").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                app_id_val = Some(v);
            }
            if let Some(v) = qqbot_val
                .get("clientSecret")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                client_secret_val = Some(v);
            }

            // 旧格式兼容：token = "AppID:ClientSecret"
            // 若新格式缺失，尝试从 token 拆分（仅读，不写回）
            if app_id_val.is_none() || client_secret_val.is_none() {
                if let Some(t) = qqbot_val.get("token").and_then(|v| v.as_str()) {
                    if let Some((aid, csec)) = t.split_once(':') {
                        if app_id_val.is_none() {
                            app_id_val = Some(aid.trim());
                        }
                        if client_secret_val.is_none() {
                            client_secret_val = Some(csec.trim());
                        }
                        needs_migrate = app_id_val.is_some() && client_secret_val.is_some();
                    }
                }
            }

            if app_id_val.is_none() && client_secret_val.is_none() {
                return Ok(crate::jv!({ "exists": false }));
            }

            // 写入表单字段（前端 UI 用 clientSecret）
            if let Some(v) = app_id_val {
                form.insert("appId".into(), Value::String(v.into()));
            }
            if let Some(v) = client_secret_val {
                form.insert("clientSecret".into(), Value::String(v.into()));
            }

            // 旧格式迁移：仅有 token 字符串时，折叠为 accounts.* 下的 appId + clientSecret + token（与官方 CLI 结构一致）
            let migrate_app_id = app_id_val.map(|s| s.to_string());
            let migrate_secret = client_secret_val.map(|s| s.to_string());
            if needs_migrate {
                let acct_key = account_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .unwrap_or(QQBOT_DEFAULT_ACCOUNT_ID);
                let channels = cfg.as_object_mut().ok_or("配置格式错误")?;
                let qqbot_node = channels.entry("qqbot").or_insert_with(|| crate::jv!({ "enabled": true }));
                let qqbot_obj = qqbot_node.as_object_mut().ok_or("qqbot 节点格式错误")?;
                qqbot_obj.insert("enabled".into(), Value::Bool(true));
                qqbot_obj.remove("appId");
                qqbot_obj.remove("clientSecret");
                qqbot_obj.remove("appSecret");
                qqbot_obj.remove("token");
                let accounts = qqbot_obj.entry("accounts").or_insert_with(|| crate::jv!({}));
                let accounts_obj = accounts.as_object_mut().ok_or("accounts 格式错误")?;
                let target = accounts_obj.entry(acct_key.to_string()).or_insert_with(|| crate::jv!({}));
                if let Some(obj) = target.as_object_mut() {
                    if let (Some(aid), Some(sec)) = (&migrate_app_id, &migrate_secret) {
                        obj.insert("appId".into(), Value::String(aid.clone()));
                        obj.insert("clientSecret".into(), Value::String(sec.clone()));
                        obj.insert("token".into(), Value::String(format!("{}:{}", aid, sec)));
                    }
                    obj.insert("enabled".into(), Value::Bool(true));
                }
                super::config::save_openclaw_json(&cfg)?;
            }

            return Ok(crate::jv!({ "exists": true, "values": Value::Object(form) }));
        }
        "feishu" => {
            if saved.is_null() {
                return Ok(crate::jv!({ "exists": false }));
            }
            // 飞书凭证：优先从 accounts.<id> 读（多账号），否则从 root 读
            insert_secret_aware_form_value(&mut form, &saved, "appId");
            insert_secret_aware_form_value(&mut form, &saved, "appSecret");
            // 读 shared fields：优先从 channel root 读（多账号模式下 credentials 在 accounts 下，shared fields 在 root）
            if let Some(ref acct) = account_id {
                if !acct.is_empty() {
                    // 从 channel root 补 shared fields
                    let mut shared_source = saved.clone();
                    if let Some(ch_root) = channel_root {
                        if let (Some(target), Some(root)) = (shared_source.as_object_mut(), ch_root.as_object()) {
                            for key in &[
                                "domain",
                                "connectionMode",
                                "webhookPath",
                                "dmPolicy",
                                "groupPolicy",
                                "allowFrom",
                                "reactionNotifications",
                                "typingIndicator",
                                "resolveSenderNames",
                                "requireMention",
                                "textChunkLimit",
                                "mediaMaxMb",
                            ] {
                                if let Some(v) = root.get(*key) {
                                    target.insert(key.to_string(), v.clone());
                                }
                            }
                        }
                    }
                    {
                        for key in &[
                            "domain",
                            "connectionMode",
                            "webhookPath",
                            "groupAllowFrom",
                            "groups",
                            "reactionNotifications",
                            "streaming",
                            "blockStreaming",
                            "textChunkLimit",
                            "mediaMaxMb",
                        ] {
                            if let Some(v) = shared_source.get(*key) {
                                if !v.is_null() {
                                    form.insert(key.to_string(), v.clone());
                                }
                            }
                        }
                        insert_access_policy_form_values(&mut form, &shared_source, false, true);
                        insert_bool_as_string(&mut form, &shared_source, "typingIndicator");
                        insert_bool_as_string(&mut form, &shared_source, "resolveSenderNames");
                        insert_bool_as_string(&mut form, &shared_source, "requireMention");
                    }
                }
            } else {
                // 无账号：直接从 root 读 shared fields
                for key in &[
                    "domain",
                    "connectionMode",
                    "webhookPath",
                    "reactionNotifications",
                    "textChunkLimit",
                    "mediaMaxMb",
                ] {
                    insert_string_if_present(&mut form, &saved, key);
                }
                insert_access_policy_form_values(&mut form, &saved, false, true);
                insert_bool_as_string(&mut form, &saved, "typingIndicator");
                insert_bool_as_string(&mut form, &saved, "resolveSenderNames");
                insert_bool_as_string(&mut form, &saved, "requireMention");
            }
        }
        "dingtalk" | "dingtalk-connector" => {
            insert_secret_aware_form_value(&mut form, &saved, "clientId");
            insert_secret_aware_form_value(&mut form, &saved, "clientSecret");
            insert_secret_aware_form_value(&mut form, &saved, "gatewayToken");
            insert_secret_aware_form_value(&mut form, &saved, "gatewayPassword");
            match gateway_auth_mode(&cfg) {
                Some("token") => {
                    if let Some(v) = gateway_auth_value(&cfg, "token") {
                        form.insert("gatewayToken".into(), Value::String(v));
                    }
                    form.remove("gatewayPassword");
                }
                Some("password") => {
                    if let Some(v) = gateway_auth_value(&cfg, "password") {
                        form.insert("gatewayPassword".into(), Value::String(v));
                    }
                    form.remove("gatewayToken");
                }
                _ => {}
            }
        }
        "slack" => {
            insert_string_if_present(&mut form, &saved, "mode");
            insert_secret_aware_form_value(&mut form, &saved, "botToken");
            insert_secret_aware_form_value(&mut form, &saved, "appToken");
            insert_secret_aware_form_value(&mut form, &saved, "signingSecret");
            insert_string_if_present(&mut form, &saved, "webhookPath");
            insert_string_if_present(&mut form, &saved, "teamId");
            insert_string_if_present(&mut form, &saved, "appId");
            insert_string_if_present(&mut form, &saved, "socketMode");
            insert_access_policy_form_values(&mut form, &saved, false, true);
            insert_bool_as_string(&mut form, &saved, "userTokenReadOnly");
            insert_bool_as_string(&mut form, &saved, "requireMention");
        }
        "whatsapp" => {
            insert_access_policy_form_values(&mut form, &saved, false, false);
            insert_array_as_csv(&mut form, &saved, "groupAllowFrom");
            insert_bool_as_string(&mut form, &saved, "enabled");
            for key in ["configWrites", "sendReadReceipts", "selfChatMode", "blockStreaming"] {
                insert_bool_as_string(&mut form, &saved, key);
            }
            for key in [
                "defaultTo",
                "contextVisibility",
                "chunkMode",
                "reactionLevel",
                "replyToMode",
                "messagePrefix",
                "responsePrefix",
            ] {
                insert_string_if_present(&mut form, &saved, key);
            }
            for key in ["historyLimit", "dmHistoryLimit", "mediaMaxMb", "debounceMs", "textChunkLimit"] {
                insert_number_as_string(&mut form, &saved, key);
            }
            if let Some(ack_reaction) = saved.get("ackReaction") {
                if let Some(v) = ack_reaction.get("emoji").and_then(|v| v.as_str()) {
                    form.insert("ackEmoji".into(), Value::String(v.into()));
                }
                if let Some(v) = ack_reaction.get("direct").and_then(|v| v.as_bool()) {
                    form.insert("ackDirect".into(), Value::String(if v { "true" } else { "false" }.into()));
                }
                if let Some(v) = ack_reaction.get("group").and_then(|v| v.as_str()) {
                    form.insert("ackGroup".into(), Value::String(v.into()));
                }
            }
        }
        "signal" => {
            insert_string_if_present(&mut form, &saved, "account");
            insert_string_if_present(&mut form, &saved, "cliPath");
            insert_string_if_present(&mut form, &saved, "httpUrl");
            insert_string_if_present(&mut form, &saved, "httpHost");
            insert_number_as_string(&mut form, &saved, "httpPort");
            insert_string_if_present(&mut form, &saved, "responsePrefix");
            insert_access_policy_form_values(&mut form, &saved, false, false);
            insert_array_as_csv(&mut form, &saved, "groupAllowFrom");
            insert_bool_as_string(&mut form, &saved, "blockStreaming");
            for key in ["historyLimit", "dmHistoryLimit", "textChunkLimit", "mediaMaxMb"] {
                insert_number_as_string(&mut form, &saved, key);
            }
        }
        "imessage" => {
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
                insert_string_if_present(&mut form, &saved, key);
            }
            insert_access_policy_form_values(&mut form, &saved, false, false);
            insert_array_as_csv(&mut form, &saved, "groupAllowFrom");
            insert_array_as_csv(&mut form, &saved, "attachmentRoots");
            insert_array_as_csv(&mut form, &saved, "remoteAttachmentRoots");
            for key in [
                "configWrites",
                "includeAttachments",
                "blockStreaming",
                "sendReadReceipts",
                "coalesceSameSenderDms",
            ] {
                insert_bool_as_string(&mut form, &saved, key);
            }
            for key in [
                "historyLimit",
                "dmHistoryLimit",
                "mediaMaxMb",
                "probeTimeoutMs",
                "textChunkLimit",
            ] {
                insert_number_as_string(&mut form, &saved, key);
            }
        }
        "matrix" => {
            insert_string_if_present(&mut form, &saved, "homeserver");
            insert_secret_aware_form_value(&mut form, &saved, "accessToken");
            insert_string_if_present(&mut form, &saved, "userId");
            insert_secret_aware_form_value(&mut form, &saved, "password");
            insert_string_if_present(&mut form, &saved, "deviceId");
            insert_access_policy_form_values(&mut form, &saved, false, false);
            insert_bool_as_string(&mut form, &saved, "e2ee");
            if saved.get("accessToken").and_then(|v| v.as_str()).is_some() {
                form.insert("authMode".into(), Value::String("token".into()));
            } else if saved.get("userId").and_then(|v| v.as_str()).is_some()
                || saved.get("password").and_then(|v| v.as_str()).is_some()
            {
                form.insert("authMode".into(), Value::String("password".into()));
            }
        }
        "msteams" => {
            insert_secret_aware_form_value(&mut form, &saved, "appId");
            insert_secret_aware_form_value(&mut form, &saved, "appPassword");
            for key in [
                "tenantId",
                "authType",
                "certificatePath",
                "certificateThumbprint",
                "managedIdentityClientId",
                "botEndpoint",
                "replyStyle",
                "sharePointSiteId",
                "responsePrefix",
            ] {
                insert_string_if_present(&mut form, &saved, key);
            }
            if let Some(webhook) = saved.get("webhook") {
                insert_string_if_present(&mut form, webhook, "path");
                if let Some(v) = form.remove("path") {
                    form.insert("webhookPath".into(), v);
                }
                insert_number_as_string(&mut form, webhook, "port");
                if let Some(v) = form.remove("port") {
                    form.insert("webhookPort".into(), v);
                }
            } else {
                insert_string_if_present(&mut form, &saved, "webhookPath");
            }
            insert_access_policy_form_values(&mut form, &saved, false, true);
            insert_array_as_csv(&mut form, &saved, "groupAllowFrom");
            insert_bool_as_string(&mut form, &saved, "requireMention");
            for key in [
                "useManagedIdentity",
                "blockStreaming",
                "typingIndicator",
                "welcomeCard",
                "groupWelcomeCard",
                "feedbackEnabled",
                "feedbackReflection",
            ] {
                insert_bool_as_string(&mut form, &saved, key);
            }
            for key in [
                "historyLimit",
                "dmHistoryLimit",
                "textChunkLimit",
                "mediaMaxMb",
                "feedbackReflectionCooldownMs",
            ] {
                insert_number_as_string(&mut form, &saved, key);
            }
            insert_array_as_csv(&mut form, &saved, "promptStarters");
            if let Some(delegated_auth) = saved.get("delegatedAuth") {
                insert_bool_as_string(&mut form, delegated_auth, "enabled");
                if let Some(v) = form.remove("enabled") {
                    form.insert("delegatedAuthEnabled".into(), v);
                }
                insert_array_as_csv(&mut form, delegated_auth, "scopes");
                if let Some(v) = form.remove("scopes") {
                    form.insert("delegatedAuthScopes".into(), v);
                }
            }
            if let Some(sso) = saved.get("sso") {
                insert_bool_as_string(&mut form, sso, "enabled");
                if let Some(v) = form.remove("enabled") {
                    form.insert("ssoEnabled".into(), v);
                }
                insert_string_if_present(&mut form, sso, "connectionName");
                if let Some(v) = form.remove("connectionName") {
                    form.insert("ssoConnectionName".into(), v);
                }
            }
        }
        "line" | "mattermost" | "clickclack" | "nextcloud-talk" | "twitch" | "nostr" | "irc" | "tlon"
        | "synology-chat" | "googlechat" => {
            fill_extended_platform_config(platform.as_str(), &mut form, &saved, channel_root);
        }        _ => {
            if saved.is_null() {
                return Ok(crate::jv!({ "exists": false }));
            }
            // 通用：原样返回字符串 / 数组 / 布尔字段
            if let Some(obj) = saved.as_object() {
                for (k, v) in obj {
                    if k == "enabled" {
                        continue;
                    }
                    if secret_ref_placeholder(v).is_some() {
                        insert_secret_aware_form_value(&mut form, &saved, k);
                    } else if let Some(s) = v.as_str() {
                        form.insert(k.clone(), Value::String(s.into()));
                    } else if v.is_array() {
                        insert_array_as_csv(&mut form, &saved, k);
                    } else if let Some(b) = v.as_bool() {
                        form.insert(k.clone(), Value::String(if b { "true" } else { "false" }.into()));
                    } else if v.is_number() {
                        form.insert(k.clone(), Value::String(v.to_string()));
                    }
                }
            }
        }
    }

    Ok(crate::jv!({ "exists": exists, "values": Value::Object(form) }))
}

include!("read_platform_config/extended_platforms.rs");
