fn fill_extended_platform_config(
    platform: &str,
    form: &mut Map<String, Value>,
    saved: &Value,
    channel_root: Option<&Value>,
) -> bool {
    match platform {
        "line" => {
            for key in [
                "channelAccessToken",
                "tokenFile",
                "channelSecret",
                "secretFile",
                "webhookPath",
                "responsePrefix",
            ] {
                insert_secret_aware_form_value(form, saved, key);
            }
            insert_access_policy_form_values(form, saved, false, false);
            insert_array_as_csv(form, saved, "groupAllowFrom");
            if let Some(v) = saved.get("mediaMaxMb").and_then(|v| v.as_i64()) {
                form.insert("mediaMaxMb".into(), Value::String(v.to_string()));
            }
        }
        "mattermost" => {
            for key in ["botToken", "baseUrl", "name", "replyToMode", "responsePrefix"] {
                insert_secret_aware_form_value(form, saved, key);
            }
            insert_access_policy_form_values(form, saved, false, true);
            insert_array_as_csv(form, saved, "groupAllowFrom");
            insert_bool_as_string(form, saved, "dangerouslyAllowNameMatching");
            if let Some(network) = saved.get("network") {
                insert_bool_as_string(form, network, "dangerouslyAllowPrivateNetwork");
            }
            if let Some(commands) = saved.get("commands") {
                insert_string_if_present(form, commands, "callbackPath");
                insert_string_if_present(form, commands, "callbackUrl");
            }
        }
        "clickclack" => {
            for key in [
                "name",
                "baseUrl",
                "token",
                "workspace",
                "botUserId",
                "agentId",
                "replyMode",
                "model",
                "systemPrompt",
                "defaultTo",
            ] {
                insert_secret_aware_form_value(form, saved, key);
            }
            insert_bool_as_string(form, saved, "enabled");
            insert_bool_as_string(form, saved, "senderIsOwner");
            insert_array_as_csv(form, saved, "toolsAllow");
            insert_array_as_csv(form, saved, "allowFrom");
            insert_number_as_string(form, saved, "timeoutSeconds");
            insert_number_as_string(form, saved, "reconnectMs");
        }
        "nextcloud-talk" => {
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
                insert_secret_aware_form_value(form, saved, key);
            }
            insert_bool_as_string(form, saved, "enabled");
            insert_access_policy_form_values(form, saved, false, true);
            insert_array_as_csv(form, saved, "groupAllowFrom");
            insert_bool_as_string(form, saved, "blockStreaming");
            if let Some(network) = saved.get("network") {
                insert_bool_as_string(form, network, "dangerouslyAllowPrivateNetwork");
            }
            for key in [
                "webhookPort",
                "historyLimit",
                "dmHistoryLimit",
                "mediaMaxMb",
                "textChunkLimit",
            ] {
                insert_number_as_string(form, saved, key);
            }
        }
        "twitch" => {
            for key in [
                "username",
                "accessToken",
                "clientId",
                "channel",
                "responsePrefix",
                "clientSecret",
                "refreshToken",
            ] {
                insert_secret_aware_form_value(form, saved, key);
            }
            insert_bool_as_string(form, saved, "enabled");
            insert_array_as_csv(form, saved, "allowFrom");
            insert_array_as_csv(form, saved, "allowedRoles");
            insert_bool_as_string(form, saved, "requireMention");
            insert_number_as_string(form, saved, "expiresIn");
            insert_number_as_string(form, saved, "obtainmentTimestamp");
        }
        "nostr" => {
            insert_secret_aware_form_value(form, saved, "privateKey");
            for key in ["name", "defaultAccount", "dmPolicy"] {
                insert_string_if_present(form, saved, key);
            }
            insert_bool_as_string(form, saved, "enabled");
            insert_array_as_csv(form, saved, "relays");
            insert_array_as_csv(form, saved, "allowFrom");
            if let Some(profile) = saved.get("profile") {
                for (source_key, form_key) in [
                    ("name", "profileName"),
                    ("displayName", "profileDisplayName"),
                    ("about", "profileAbout"),
                    ("picture", "profilePicture"),
                    ("banner", "profileBanner"),
                    ("website", "profileWebsite"),
                    ("nip05", "profileNip05"),
                    ("lud16", "profileLud16"),
                ] {
                    if let Some(v) = profile.get(source_key).and_then(|v| v.as_str()) {
                        form.insert(form_key.into(), Value::String(v.into()));
                    }
                }
            }
        }
        "irc" => {
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
                insert_secret_aware_form_value(form, saved, key);
            }
            for key in ["enabled", "tls", "blockStreaming", "dangerouslyAllowNameMatching"] {
                insert_bool_as_string(form, saved, key);
            }
            insert_access_policy_form_values(form, saved, false, false);
            insert_array_as_csv(form, saved, "groupAllowFrom");
            insert_array_as_csv(form, saved, "channels");
            insert_array_as_csv(form, saved, "mentionPatterns");
            insert_irc_groups_form_values(form, saved);
            for key in ["port", "historyLimit", "dmHistoryLimit", "mediaMaxMb", "textChunkLimit"] {
                insert_number_as_string(form, saved, key);
            }
            if let Some(nickserv) = saved.get("nickserv") {
                if let Some(v) = nickserv.get("enabled").and_then(|v| v.as_bool()) {
                    form.insert("nickservEnabled".into(), Value::String(if v { "true" } else { "false" }.into()));
                }
                insert_secret_aware_form_alias(form, nickserv, "service", "nickservService");
                insert_secret_aware_form_alias(form, nickserv, "password", "nickservPassword");
                insert_secret_aware_form_alias(form, nickserv, "passwordFile", "nickservPasswordFile");
                if let Some(v) = nickserv.get("register").and_then(|v| v.as_bool()) {
                    form.insert("nickservRegister".into(), Value::String(if v { "true" } else { "false" }.into()));
                }
                if let Some(v) = nickserv.get("registerEmail").and_then(|v| v.as_str()) {
                    form.insert("nickservRegisterEmail".into(), Value::String(v.into()));
                }
            }
        }
        "tlon" => {
            let mut shared = channel_root.and_then(|root| root.as_object()).cloned().unwrap_or_default();
            if let Some(saved_obj) = saved.as_object() {
                for (key, value) in saved_obj {
                    shared.insert(key.clone(), value.clone());
                }
            }
            let shared = Value::Object(shared);
            for key in ["name", "ship", "url", "code", "responsePrefix", "ownerShip"] {
                insert_secret_aware_form_value(form, &shared, key);
            }
            insert_bool_as_string(form, &shared, "enabled");
            if let Some(network) = shared.get("network") {
                insert_bool_as_string(form, network, "dangerouslyAllowPrivateNetwork");
            }
            for key in [
                "groupChannels",
                "dmAllowlist",
                "groupInviteAllowlist",
                "defaultAuthorizedShips",
            ] {
                insert_array_as_csv(form, &shared, key);
            }
            for key in [
                "autoDiscoverChannels",
                "showModelSignature",
                "autoAcceptDmInvites",
                "autoAcceptGroupInvites",
            ] {
                insert_bool_as_string(form, &shared, key);
            }
        }
        "synology-chat" => {
            for key in ["token", "incomingUrl", "nasHost", "webhookPath", "botName"] {
                insert_secret_aware_form_value(form, saved, key);
            }
            insert_string_if_present(form, saved, "dmPolicy");
            insert_array_as_csv(form, saved, "allowedUserIds");
            if let Some(v) = saved.get("rateLimitPerMinute").and_then(|v| v.as_i64()) {
                form.insert("rateLimitPerMinute".into(), Value::String(v.to_string()));
            }
            insert_bool_as_string(form, saved, "dangerouslyAllowNameMatching");
            insert_bool_as_string(form, saved, "dangerouslyAllowInheritedWebhookPath");
            insert_bool_as_string(form, saved, "allowInsecureSsl");
        }
        "googlechat" => {
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
                insert_secret_aware_form_value(form, saved, key);
            }
            if let Some(dm) = saved.get("dm") {
                if let Some(policy) = dm.get("policy").and_then(|v| v.as_str()) {
                    form.insert("dmPolicy".into(), Value::String(policy.into()));
                }
                insert_array_as_csv(form, dm, "allowFrom");
            }
            insert_string_if_present(form, saved, "groupPolicy");
            insert_array_as_csv(form, saved, "groupAllowFrom");
            insert_bool_as_string(form, saved, "requireMention");
            insert_bool_as_string(form, saved, "dangerouslyAllowNameMatching");
            insert_bool_as_string(form, saved, "allowBots");
            insert_bool_as_string(form, saved, "blockStreaming");
            for key in ["historyLimit", "dmHistoryLimit", "textChunkLimit", "mediaMaxMb"] {
                if let Some(v) = saved.get(key).and_then(|v| v.as_f64()) {
                    form.insert(key.into(), Value::String(v.to_string()));
                }
            }
        }
        _ => return false,
    }
    true
}