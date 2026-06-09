fn save_matrix_platform(
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
            let homeserver = form_string(form_obj, "homeserver");
            let access_token = form_string(form_obj, "accessToken");
            let user_id = form_string(form_obj, "userId");
            let password = form_string(form_obj, "password");

            if homeserver.is_empty() {
                return Err("Homeserver 不能为空".into());
            }
            if access_token.is_empty() && (user_id.is_empty() || password.is_empty()) {
                return Err("请至少填写 Access Token，或填写 User ID + Password".into());
            }

            let mut entry = Map::new();
            entry.insert("enabled".into(), Value::Bool(true));
            put_string(&mut entry, "homeserver", homeserver);
            put_string(&mut entry, "accessToken", access_token);
            put_string(&mut entry, "userId", user_id);
            put_string(&mut entry, "password", password);
            put_string(&mut entry, "deviceId", form_string(form_obj, "deviceId"));
            put_string(&mut entry, "dmPolicy", form_string(form_obj, "dmPolicy"));
            put_string(&mut entry, "groupPolicy", form_string(form_obj, "groupPolicy"));
            put_bool_from_form(&mut entry, "e2ee", &form_string(form_obj, "e2ee"));
            put_array_from_form_value(&mut entry, "allowFrom", form_obj.get("allowFrom"));
            preserve_messaging_credential_refs(&mut entry, form_obj, current_saved);
            merge_channel_entry_for_account(channels_map, storage_key, account_id, entry)?;
            ensure_plugin_allowed(cfg, "matrix")?;
    Ok(())
}

fn save_msteams_platform(
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
            let app_id = form_string(form_obj, "appId");
            let app_password = form_string(form_obj, "appPassword");
            let missing_credentials = msteams_credential_missing_labels(form_obj);
            if !missing_credentials.is_empty() {
                return Err(format!("缺少 {}", missing_credentials.join(" / ")));
            }

            let mut entry = Map::new();
            entry.insert("enabled".into(), Value::Bool(true));
            put_string(&mut entry, "appId", app_id);
            put_string(&mut entry, "appPassword", app_password);
            for key in [
                "tenantId",
                "authType",
                "certificatePath",
                "certificateThumbprint",
                "managedIdentityClientId",
                "replyStyle",
                "sharePointSiteId",
                "responsePrefix",
            ] {
                put_string(&mut entry, key, form_string(form_obj, key));
            }
            let mut webhook = current_saved
                .get("webhook")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();
            put_number_from_form(&mut webhook, "port", &form_string(form_obj, "webhookPort"));
            put_string(&mut webhook, "path", form_string(form_obj, "webhookPath"));
            if !webhook.is_empty() {
                entry.insert("webhook".into(), Value::Object(webhook));
            }
            put_string(&mut entry, "dmPolicy", form_string(form_obj, "dmPolicy"));
            put_string(&mut entry, "groupPolicy", form_string(form_obj, "groupPolicy"));
            put_array_from_form_value(&mut entry, "allowFrom", form_obj.get("allowFrom"));
            put_array_from_form_value(&mut entry, "groupAllowFrom", form_obj.get("groupAllowFrom"));
            for key in [
                "useManagedIdentity",
                "requireMention",
                "blockStreaming",
                "typingIndicator",
                "welcomeCard",
                "groupWelcomeCard",
                "feedbackEnabled",
                "feedbackReflection",
            ] {
                put_bool_value_if_present(&mut entry, key, form_obj.get(key));
            }
            for key in [
                "historyLimit",
                "dmHistoryLimit",
                "textChunkLimit",
                "mediaMaxMb",
                "feedbackReflectionCooldownMs",
            ] {
                put_number_from_form(&mut entry, key, &form_string(form_obj, key));
            }
            put_array_from_form_value(&mut entry, "promptStarters", form_obj.get("promptStarters"));
            let mut delegated_auth = current_saved
                .get("delegatedAuth")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();
            put_bool_value_if_present(&mut delegated_auth, "enabled", form_obj.get("delegatedAuthEnabled"));
            put_array_from_form_value(&mut delegated_auth, "scopes", form_obj.get("delegatedAuthScopes"));
            if !delegated_auth.is_empty() {
                entry.insert("delegatedAuth".into(), Value::Object(delegated_auth));
            }
            let mut sso = current_saved
                .get("sso")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();
            put_bool_value_if_present(&mut sso, "enabled", form_obj.get("ssoEnabled"));
            put_string(&mut sso, "connectionName", form_string(form_obj, "ssoConnectionName"));
            if !sso.is_empty() {
                entry.insert("sso".into(), Value::Object(sso));
            }
            preserve_messaging_credential_refs(&mut entry, form_obj, current_saved);
            merge_channel_entry_for_account(channels_map, storage_key, account_id, entry)?;
            ensure_plugin_allowed(cfg, "msteams")?;
    Ok(())
}

fn save_line_platform(
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
            let channel_access_token = form_string(form_obj, "channelAccessToken");
            let token_file = form_string(form_obj, "tokenFile");
            let channel_secret = form_string(form_obj, "channelSecret");
            let secret_file = form_string(form_obj, "secretFile");
            if channel_access_token.is_empty() && token_file.is_empty() {
                return Err("Channel Access Token 或 Token File 至少填写一项".into());
            }
            if channel_secret.is_empty() && secret_file.is_empty() {
                return Err("Channel Secret 或 Secret File 至少填写一项".into());
            }

            let mut entry = Map::new();
            entry.insert("enabled".into(), Value::Bool(true));
            put_string(&mut entry, "channelAccessToken", channel_access_token);
            put_string(&mut entry, "tokenFile", token_file);
            put_string(&mut entry, "channelSecret", channel_secret);
            put_string(&mut entry, "secretFile", secret_file);
            put_string(&mut entry, "webhookPath", form_string(form_obj, "webhookPath"));
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
            ensure_plugin_allowed(cfg, "line")?;
    Ok(())
}

fn save_mattermost_platform(
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
            let base_url = form_string(form_obj, "baseUrl");
            if bot_token.is_empty() {
                return Err("Mattermost Bot Token 不能为空".into());
            }
            if base_url.is_empty() {
                return Err("Mattermost Base URL 不能为空".into());
            }

            let mut entry = Map::new();
            entry.insert("enabled".into(), Value::Bool(true));
            put_string(&mut entry, "botToken", bot_token);
            put_string(&mut entry, "baseUrl", base_url);
            put_string(&mut entry, "name", form_string(form_obj, "name"));
            put_string(&mut entry, "replyToMode", form_string(form_obj, "replyToMode"));
            put_string(&mut entry, "responsePrefix", form_string(form_obj, "responsePrefix"));
            put_string(&mut entry, "dmPolicy", form_string(form_obj, "dmPolicy"));
            put_string(&mut entry, "groupPolicy", form_string(form_obj, "groupPolicy"));
            put_bool_value_if_present(&mut entry, "requireMention", form_obj.get("requireMention"));
            put_array_from_form_value(&mut entry, "allowFrom", form_obj.get("allowFrom"));
            put_array_from_form_value(&mut entry, "groupAllowFrom", form_obj.get("groupAllowFrom"));
            put_bool_value_if_present(&mut entry, "dangerouslyAllowNameMatching", form_obj.get("dangerouslyAllowNameMatching"));

            if form_obj.contains_key("dangerouslyAllowPrivateNetwork") {
                let mut network = current_saved
                    .get("network")
                    .and_then(|v| v.as_object())
                    .cloned()
                    .unwrap_or_default();
                match form_obj.get("dangerouslyAllowPrivateNetwork") {
                    Some(Value::Bool(v)) => {
                        network.insert("dangerouslyAllowPrivateNetwork".into(), Value::Bool(*v));
                    }
                    Some(Value::String(raw)) => {
                        if let Some(v) = bool_from_form_value(raw) {
                            network.insert("dangerouslyAllowPrivateNetwork".into(), Value::Bool(v));
                        }
                    }
                    _ => {}
                }
                if !network.is_empty() {
                    entry.insert("network".into(), Value::Object(network));
                }
            }

            let mut commands = current_saved
                .get("commands")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();
            put_string(&mut commands, "callbackPath", form_string(form_obj, "callbackPath"));
            put_string(&mut commands, "callbackUrl", form_string(form_obj, "callbackUrl"));
            if !commands.is_empty() {
                entry.insert("commands".into(), Value::Object(commands));
            }

            preserve_messaging_credential_refs(&mut entry, form_obj, current_saved);
            merge_channel_entry_for_account(channels_map, storage_key, account_id, entry)?;
            ensure_plugin_allowed(cfg, "mattermost")?;
    Ok(())
}

fn save_clickclack_platform(
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
            let token = form_string(form_obj, "token");
            let workspace = form_string(form_obj, "workspace");
            if base_url.is_empty() {
                return Err("ClickClack Base URL 不能为空".into());
            }
            if token.is_empty() {
                return Err("ClickClack Token 不能为空".into());
            }
            if workspace.is_empty() {
                return Err("ClickClack Workspace 不能为空".into());
            }

            let mut entry = Map::new();
            entry.insert("enabled".into(), Value::Bool(true));
            put_bool_value_if_present(&mut entry, "enabled", form_obj.get("enabled"));
            put_string(&mut entry, "baseUrl", base_url);
            put_string(&mut entry, "token", token);
            put_string(&mut entry, "workspace", workspace);
            for key in [
                "name",
                "botUserId",
                "agentId",
                "replyMode",
                "model",
                "systemPrompt",
                "defaultTo",
            ] {
                put_string(&mut entry, key, form_string(form_obj, key));
            }
            put_array_from_form_value(&mut entry, "toolsAllow", form_obj.get("toolsAllow"));
            put_array_from_form_value(&mut entry, "allowFrom", form_obj.get("allowFrom"));
            put_bool_value_if_present(&mut entry, "senderIsOwner", form_obj.get("senderIsOwner"));
            put_number_value_if_present(&mut entry, "timeoutSeconds", form_obj.get("timeoutSeconds"));
            put_number_value_if_present(&mut entry, "reconnectMs", form_obj.get("reconnectMs"));
            preserve_messaging_credential_refs(&mut entry, form_obj, current_saved);
            merge_channel_entry_for_account(channels_map, storage_key, account_id, entry)?;
            ensure_plugin_allowed(cfg, "clickclack")?;
    Ok(())
}
