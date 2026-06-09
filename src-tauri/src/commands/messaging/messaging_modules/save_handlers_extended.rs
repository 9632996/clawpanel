fn save_nextcloud_talk_platform(
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
            let base_url = form_string(form_obj, "baseUrl");
            let bot_secret = form_string(form_obj, "botSecret");
            let bot_secret_file = form_string(form_obj, "botSecretFile");
            if base_url.is_empty() {
                return Err("Nextcloud Talk Base URL 不能为空".into());
            }
            if bot_secret.is_empty()
                && bot_secret_file.is_empty()
                && !has_configured_messaging_value(form_obj.get("botSecret"))
                && !has_configured_messaging_value(form_obj.get("botSecretFile"))
            {
                return Err("Nextcloud Talk Bot Secret 或 Secret File 至少填写一项".into());
            }

            let mut entry = Map::new();
            entry.insert("enabled".into(), Value::Bool(true));
            put_bool_value_if_present(&mut entry, "enabled", form_obj.get("enabled"));
            for key in [
                "name",
                "baseUrl",
                "botSecret",
                "botSecretFile",
                "apiUser",
                "apiPassword",
                "apiPasswordFile",
                "webhookHost",
                "webhookPath",
                "webhookPublicUrl",
                "chunkMode",
                "responsePrefix",
            ] {
                put_string(&mut entry, key, form_string(form_obj, key));
            }
            put_string(&mut entry, "dmPolicy", form_string(form_obj, "dmPolicy"));
            put_string(&mut entry, "groupPolicy", form_string(form_obj, "groupPolicy"));
            put_bool_value_if_present(&mut entry, "requireMention", form_obj.get("requireMention"));
            put_array_from_form_value(&mut entry, "allowFrom", form_obj.get("allowFrom"));
            put_array_from_form_value(&mut entry, "groupAllowFrom", form_obj.get("groupAllowFrom"));
            put_bool_value_if_present(&mut entry, "blockStreaming", form_obj.get("blockStreaming"));
            for key in [
                "webhookPort",
                "historyLimit",
                "dmHistoryLimit",
                "mediaMaxMb",
                "textChunkLimit",
            ] {
                put_number_value_if_present(&mut entry, key, form_obj.get(key));
            }
            if form_obj.contains_key("dangerouslyAllowPrivateNetwork") {
                let mut network = current_saved
                    .get("network")
                    .and_then(|v| v.as_object())
                    .cloned()
                    .unwrap_or_default();
                put_bool_value_if_present(
                    &mut network,
                    "dangerouslyAllowPrivateNetwork",
                    form_obj.get("dangerouslyAllowPrivateNetwork"),
                );
                if !network.is_empty() {
                    entry.insert("network".into(), Value::Object(network));
                }
            }
            preserve_messaging_credential_refs(&mut entry, form_obj, current_saved);
            merge_channel_entry_for_account(channels_map, storage_key, account_id, entry)?;
            ensure_plugin_allowed(cfg, "nextcloud-talk")?;
    Ok(())
}

fn save_twitch_platform(
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
            let username = form_string(form_obj, "username");
            let access_token = form_string(form_obj, "accessToken");
            let client_id = form_string(form_obj, "clientId");
            let channel = form_string(form_obj, "channel");
            if username.is_empty() {
                return Err("Twitch Username 不能为空".into());
            }
            if access_token.is_empty() && !has_configured_messaging_value(form_obj.get("accessToken")) {
                return Err("Twitch Access Token 不能为空".into());
            }
            if client_id.is_empty() {
                return Err("Twitch Client ID 不能为空".into());
            }
            if channel.is_empty() {
                return Err("Twitch Channel 不能为空".into());
            }

            let mut entry = Map::new();
            entry.insert("enabled".into(), Value::Bool(true));
            put_bool_value_if_present(&mut entry, "enabled", form_obj.get("enabled"));
            for key in [
                "username",
                "accessToken",
                "clientId",
                "channel",
                "responsePrefix",
                "clientSecret",
                "refreshToken",
            ] {
                put_string(&mut entry, key, form_string(form_obj, key));
            }
            put_array_from_form_value(&mut entry, "allowFrom", form_obj.get("allowFrom"));
            put_array_from_form_value(&mut entry, "allowedRoles", form_obj.get("allowedRoles"));
            put_bool_value_if_present(&mut entry, "requireMention", form_obj.get("requireMention"));
            put_number_value_if_present(&mut entry, "expiresIn", form_obj.get("expiresIn"));
            put_number_value_if_present(&mut entry, "obtainmentTimestamp", form_obj.get("obtainmentTimestamp"));
            preserve_messaging_credential_refs(&mut entry, form_obj, current_saved);
            merge_channel_entry_for_account(channels_map, storage_key, account_id, entry)?;
            ensure_plugin_allowed(cfg, "twitch")?;
    Ok(())
}

fn save_nostr_platform(
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
            let private_key = form_string(form_obj, "privateKey");
            if private_key.is_empty() && !has_configured_messaging_value(form_obj.get("privateKey")) {
                return Err("Nostr Private Key 不能为空".into());
            }

            let root_saved = channels_map.get(storage_key).cloned().unwrap_or(Value::Null);
            let mut entry = Map::new();
            entry.insert("enabled".into(), Value::Bool(true));
            put_bool_value_if_present(&mut entry, "enabled", form_obj.get("enabled"));
            for key in ["name", "defaultAccount", "privateKey", "dmPolicy"] {
                put_string(&mut entry, key, form_string(form_obj, key));
            }
            put_array_from_form_value(&mut entry, "relays", form_obj.get("relays"));
            put_array_from_form_value(&mut entry, "allowFrom", form_obj.get("allowFrom"));

            let mut profile = Map::new();
            for (form_key, target_key) in [
                ("profileName", "name"),
                ("profileDisplayName", "displayName"),
                ("profileAbout", "about"),
                ("profilePicture", "picture"),
                ("profileBanner", "banner"),
                ("profileWebsite", "website"),
                ("profileNip05", "nip05"),
                ("profileLud16", "lud16"),
            ] {
                put_string(&mut profile, target_key, form_string(form_obj, form_key));
            }
            if !profile.is_empty() {
                entry.insert("profile".into(), Value::Object(profile));
            }

            preserve_messaging_credential_refs(&mut entry, form_obj, &root_saved);
            merge_channel_entry_for_account(channels_map, storage_key, None, entry)?;
            ensure_plugin_allowed(cfg, "nostr")?;
    Ok(())
}

fn save_irc_platform(
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
            let host = form_string(form_obj, "host");
            let nick = form_string(form_obj, "nick");
            if host.is_empty() {
                return Err("IRC Host 不能为空".into());
            }
            if nick.is_empty() {
                return Err("IRC Nick 不能为空".into());
            }

            let mut entry = Map::new();
            entry.insert("enabled".into(), Value::Bool(true));
            put_bool_value_if_present(&mut entry, "enabled", form_obj.get("enabled"));
            for key in [
                "name",
                "host",
                "nick",
                "username",
                "realname",
                "password",
                "passwordFile",
                "defaultTo",
                "chunkMode",
                "responsePrefix",
            ] {
                put_string(&mut entry, key, form_string(form_obj, key));
            }
            put_string(&mut entry, "dmPolicy", form_string(form_obj, "dmPolicy"));
            put_string(&mut entry, "groupPolicy", form_string(form_obj, "groupPolicy"));
            put_array_from_form_value(&mut entry, "allowFrom", form_obj.get("allowFrom"));
            put_array_from_form_value(&mut entry, "groupAllowFrom", form_obj.get("groupAllowFrom"));
            put_array_from_form_value(&mut entry, "channels", form_obj.get("channels"));
            put_array_from_form_value(&mut entry, "mentionPatterns", form_obj.get("mentionPatterns"));
            if let Some(groups) = build_irc_groups_from_form(form_obj) {
                entry.insert("groups".into(), groups);
            }
            for key in ["tls", "blockStreaming", "dangerouslyAllowNameMatching"] {
                put_bool_value_if_present(&mut entry, key, form_obj.get(key));
            }
            for key in ["port", "historyLimit", "dmHistoryLimit", "mediaMaxMb", "textChunkLimit"] {
                put_number_value_if_present(&mut entry, key, form_obj.get(key));
            }

            let mut nickserv = current_saved
                .get("nickserv")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();
            put_bool_value_if_present(&mut nickserv, "enabled", form_obj.get("nickservEnabled"));
            put_string(&mut nickserv, "service", form_string(form_obj, "nickservService"));
            match resolve_messaging_credential_value_for_save_alias(
                form_obj,
                current_saved.get("nickserv").unwrap_or(&Value::Null),
                "nickservPassword",
                "password",
            ) {
                Some(value) => {
                    nickserv.insert("password".into(), value);
                }
                None => {
                    nickserv.remove("password");
                }
            }
            match resolve_messaging_credential_value_for_save_alias(
                form_obj,
                current_saved.get("nickserv").unwrap_or(&Value::Null),
                "nickservPasswordFile",
                "passwordFile",
            ) {
                Some(value) => {
                    nickserv.insert("passwordFile".into(), value);
                }
                None => {
                    nickserv.remove("passwordFile");
                }
            }
            put_bool_value_if_present(&mut nickserv, "register", form_obj.get("nickservRegister"));
            put_string(&mut nickserv, "registerEmail", form_string(form_obj, "nickservRegisterEmail"));
            if !nickserv.is_empty() {
                entry.insert("nickserv".into(), Value::Object(nickserv));
            }

            preserve_messaging_credential_refs(&mut entry, form_obj, current_saved);
            merge_channel_entry_for_account(channels_map, storage_key, account_id, entry)?;
            ensure_plugin_allowed(cfg, "irc")?;
    Ok(())
}

fn save_tlon_platform(
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
            let ship = form_string(form_obj, "ship");
            let url = form_string(form_obj, "url");
            let code = form_string(form_obj, "code");
            if ship.is_empty() {
                return Err("Tlon Ship 不能为空".into());
            }
            if url.is_empty() {
                return Err("Tlon URL 不能为空".into());
            }
            if code.is_empty() && !has_configured_messaging_value(form_obj.get("code")) {
                return Err("Tlon Code 不能为空".into());
            }

            let mut entry = Map::new();
            entry.insert("enabled".into(), Value::Bool(true));
            put_bool_value_if_present(&mut entry, "enabled", form_obj.get("enabled"));
            for key in ["name", "ship", "url", "responsePrefix", "ownerShip"] {
                put_string(&mut entry, key, form_string(form_obj, key));
            }
            match resolve_messaging_credential_value_for_save(form_obj, current_saved, "code") {
                Some(value) => {
                    entry.insert("code".into(), value);
                }
                None => {
                    entry.remove("code");
                }
            }
            for key in [
                "groupChannels",
                "dmAllowlist",
                "groupInviteAllowlist",
                "defaultAuthorizedShips",
            ] {
                put_array_from_form_value(&mut entry, key, form_obj.get(key));
            }
            for key in [
                "autoDiscoverChannels",
                "showModelSignature",
                "autoAcceptDmInvites",
                "autoAcceptGroupInvites",
            ] {
                put_bool_value_if_present(&mut entry, key, form_obj.get(key));
            }
            if form_obj.contains_key("dangerouslyAllowPrivateNetwork") {
                let mut network = current_saved
                    .get("network")
                    .and_then(|v| v.as_object())
                    .cloned()
                    .unwrap_or_default();
                put_bool_value_if_present(
                    &mut network,
                    "dangerouslyAllowPrivateNetwork",
                    form_obj.get("dangerouslyAllowPrivateNetwork"),
                );
                if !network.is_empty() {
                    entry.insert("network".into(), Value::Object(network));
                }
            }
            preserve_messaging_credential_refs(&mut entry, form_obj, current_saved);
            let target_account_id = if account_id.map(str::trim) == Some(QQBOT_DEFAULT_ACCOUNT_ID) {
                None
            } else {
                account_id
            };
            merge_channel_entry_for_account(channels_map, storage_key, target_account_id, entry)?;
            ensure_plugin_allowed(cfg, "tlon")?;
    Ok(())
}

fn save_synology_chat_platform(
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
            let token = form_string(form_obj, "token");
            let incoming_url = form_string(form_obj, "incomingUrl");
            if token.is_empty() {
                return Err("Synology Chat Token 不能为空".into());
            }
            if incoming_url.is_empty() {
                return Err("Synology Chat Incoming URL 不能为空".into());
            }

            let mut entry = Map::new();
            entry.insert("enabled".into(), Value::Bool(true));
            put_string(&mut entry, "token", token);
            put_string(&mut entry, "incomingUrl", incoming_url);
            put_string(&mut entry, "nasHost", form_string(form_obj, "nasHost"));
            put_string(&mut entry, "webhookPath", form_string(form_obj, "webhookPath"));
            put_string(&mut entry, "botName", form_string(form_obj, "botName"));
            put_string(&mut entry, "dmPolicy", form_string(form_obj, "dmPolicy"));
            put_array_from_form_value(&mut entry, "allowedUserIds", form_obj.get("allowedUserIds"));
            if let Some(value) = form_obj.get("rateLimitPerMinute").and_then(|v| v.as_f64()) {
                if let Some(number) = serde_json::Number::from_f64(value) {
                    entry.insert("rateLimitPerMinute".into(), Value::Number(number));
                }
            } else {
                put_number_from_form(&mut entry, "rateLimitPerMinute", &form_string(form_obj, "rateLimitPerMinute"));
            }
            put_bool_value_if_present(&mut entry, "dangerouslyAllowNameMatching", form_obj.get("dangerouslyAllowNameMatching"));
            put_bool_value_if_present(
                &mut entry,
                "dangerouslyAllowInheritedWebhookPath",
                form_obj.get("dangerouslyAllowInheritedWebhookPath"),
            );
            put_bool_value_if_present(&mut entry, "allowInsecureSsl", form_obj.get("allowInsecureSsl"));
            preserve_messaging_credential_refs(&mut entry, form_obj, current_saved);
            merge_channel_entry_for_account(channels_map, storage_key, account_id, entry)?;
            ensure_plugin_allowed(cfg, "synology-chat")?;
    Ok(())
}

fn save_googlechat_platform(
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
            let has_service_account = has_configured_messaging_value(form_obj.get("serviceAccount"))
                || has_configured_messaging_value(form_obj.get("serviceAccountFile"))
                || has_configured_messaging_value(form_obj.get("serviceAccountRef"));
            if !has_service_account {
                return Err("Google Chat 需要填写 Service Account JSON、Service Account File 或 SecretRef".into());
            }

            let mut entry = Map::new();
            entry.insert("enabled".into(), Value::Bool(true));
            for key in [
                "serviceAccount",
                "serviceAccountFile",
                "serviceAccountRef",
                "audienceType",
                "audience",
                "appPrincipal",
                "webhookPath",
                "webhookUrl",
                "botUser",
                "chunkMode",
                "replyToMode",
                "typingIndicator",
                "responsePrefix",
            ] {
                put_string(&mut entry, key, form_string(form_obj, key));
            }

            let mut dm = current_saved
                .get("dm")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();
            put_string(&mut dm, "policy", form_string(form_obj, "dmPolicy"));
            let allow_from = json_array_from_csv_value(form_obj.get("allowFrom"));
            if !allow_from.is_empty() {
                dm.insert("allowFrom".into(), Value::Array(allow_from));
            }
            if !dm.is_empty() {
                entry.insert("dm".into(), Value::Object(dm));
            }

            put_string(&mut entry, "groupPolicy", form_string(form_obj, "groupPolicy"));
            put_array_from_form_value(&mut entry, "groupAllowFrom", form_obj.get("groupAllowFrom"));
            for key in [
                "dangerouslyAllowNameMatching",
                "requireMention",
                "allowBots",
                "blockStreaming",
            ] {
                put_bool_value_if_present(&mut entry, key, form_obj.get(key));
            }
            for key in ["historyLimit", "dmHistoryLimit", "textChunkLimit", "mediaMaxMb"] {
                if let Some(value) = form_obj.get(key).and_then(|v| v.as_f64()) {
                    if let Some(number) = serde_json::Number::from_f64(value) {
                        entry.insert(key.into(), Value::Number(number));
                    }
                } else {
                    put_number_from_form(&mut entry, key, &form_string(form_obj, key));
                }
            }

            preserve_messaging_credential_refs(&mut entry, form_obj, current_saved);
            merge_channel_entry_for_account(channels_map, storage_key, account_id, entry)?;
            ensure_plugin_allowed(cfg, "googlechat")?;
    Ok(())
}

fn save_generic_messaging_platform(
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
            // 通用平台：直接保存表单字段
            let mut entry = Map::new();
            for (k, v) in form_obj {
                entry.insert(k.clone(), v.clone());
            }
            entry.insert("enabled".into(), Value::Bool(true));
            preserve_messaging_credential_refs(&mut entry, form_obj, current_saved);
            merge_channel_entry_for_account(channels_map, storage_key, account_id, entry)?;
    Ok(())
}

