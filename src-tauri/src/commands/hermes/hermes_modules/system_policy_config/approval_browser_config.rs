fn build_hermes_approvals_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let approvals = root.and_then(|map| yaml_get_mapping(map, "approvals"));
    let approval_mode = normalize_hermes_approval_mode(approvals.and_then(|map| yaml_string_field(map, "mode")), false)
        .unwrap_or_else(|_| "manual".to_string());
    let approval_timeout = approvals
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "timeout"), 60, 1, 86400))
        .unwrap_or(60);
    let approval_cron_mode =
        normalize_hermes_approval_cron_mode(approvals.and_then(|map| yaml_string_field(map, "cron_mode")), false)
            .unwrap_or_else(|_| "deny".to_string());
    let approval_mcp_reload_confirm = approvals
        .and_then(|map| yaml_bool_field(map, "mcp_reload_confirm"))
        .unwrap_or(true);
    let approval_destructive_slash_confirm = approvals
        .and_then(|map| yaml_bool_field(map, "destructive_slash_confirm"))
        .unwrap_or(true);

    crate::jv!({
        "approvalMode": approval_mode,
        "approvalTimeout": approval_timeout,
        "approvalCronMode": approval_cron_mode,
        "approvalMcpReloadConfirm": approval_mcp_reload_confirm,
        "approvalDestructiveSlashConfirm": approval_destructive_slash_confirm,
    })
}

fn merge_hermes_approvals_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_approvals_config_values(config);
    let approval_mode = normalize_hermes_approval_mode(
        if form.get("approvalMode").is_some() {
            form_string(form, "approvalMode")
        } else {
            current["approvalMode"].as_str().map(ToString::to_string)
        },
        true,
    )?;
    let approval_timeout = validate_hermes_i64(
        if form.get("approvalTimeout").is_some() {
            form_i64(form, "approvalTimeout")
        } else {
            Some(current["approvalTimeout"].as_i64().unwrap_or(60))
        },
        "approvals.timeout",
        60,
        1,
        86400,
    )?;
    let approval_cron_mode = normalize_hermes_approval_cron_mode(
        if form.get("approvalCronMode").is_some() {
            form_string(form, "approvalCronMode")
        } else {
            current["approvalCronMode"].as_str().map(ToString::to_string)
        },
        true,
    )?;
    let approval_mcp_reload_confirm = form_bool(form, "approvalMcpReloadConfirm")
        .unwrap_or_else(|| current["approvalMcpReloadConfirm"].as_bool().unwrap_or(true));
    let approval_destructive_slash_confirm = form_bool(form, "approvalDestructiveSlashConfirm")
        .unwrap_or_else(|| current["approvalDestructiveSlashConfirm"].as_bool().unwrap_or(true));

    let root = ensure_yaml_object(config)?;
    let approvals = yaml_child_object(root, "approvals")?;
    approvals.insert(yaml_key("mode"), serde_yaml::Value::String(approval_mode));
    approvals.insert(yaml_key("timeout"), serde_yaml::Value::Number(approval_timeout.into()));
    approvals.insert(yaml_key("cron_mode"), serde_yaml::Value::String(approval_cron_mode));
    approvals.insert(yaml_key("mcp_reload_confirm"), serde_yaml::Value::Bool(approval_mcp_reload_confirm));
    approvals.insert(
        yaml_key("destructive_slash_confirm"),
        serde_yaml::Value::Bool(approval_destructive_slash_confirm),
    );
    Ok(())
}

fn build_hermes_privacy_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let privacy = root.and_then(|map| yaml_get_mapping(map, "privacy"));
    let redact_pii = privacy.and_then(|map| yaml_bool_field(map, "redact_pii")).unwrap_or(false);

    crate::jv!({
        "redactPii": redact_pii,
    })
}

fn merge_hermes_privacy_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_privacy_config_values(config);
    let redact_pii = form_bool(form, "redactPii").unwrap_or_else(|| current["redactPii"].as_bool().unwrap_or(false));

    let root = ensure_yaml_object(config)?;
    let privacy = yaml_child_object(root, "privacy")?;
    privacy.insert(yaml_key("redact_pii"), serde_yaml::Value::Bool(redact_pii));
    Ok(())
}

fn build_hermes_browser_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let browser = root.and_then(|map| yaml_get_mapping(map, "browser"));
    let browser_inactivity_timeout = browser
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "inactivity_timeout"), 120, 1, 86400))
        .unwrap_or(120);
    let browser_command_timeout = browser
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "command_timeout"), 30, 5, 3600))
        .unwrap_or(30);
    let browser_record_sessions = browser
        .and_then(|map| yaml_bool_field(map, "record_sessions"))
        .unwrap_or(false);
    let browser_engine = normalize_hermes_browser_engine(browser.and_then(|map| yaml_string_field(map, "engine")), false)
        .unwrap_or_else(|_| "auto".to_string());
    let browser_allow_private_urls = browser
        .and_then(|map| yaml_bool_field(map, "allow_private_urls"))
        .unwrap_or(false);
    let browser_auto_local_for_private_urls = browser
        .and_then(|map| yaml_bool_field(map, "auto_local_for_private_urls"))
        .unwrap_or(true);
    let browser_cdp_url = browser.and_then(|map| yaml_string_field(map, "cdp_url")).unwrap_or_default();
    let camofox = browser.and_then(|map| yaml_get_mapping(map, "camofox"));
    let browser_camofox_managed_persistence = camofox
        .and_then(|map| yaml_bool_field(map, "managed_persistence"))
        .unwrap_or(false);
    let browser_camofox_user_id =
        normalize_hermes_camofox_identity(camofox.and_then(|map| yaml_string_field(map, "user_id")), "browser.camofox.user_id")
            .unwrap_or_default();
    let browser_camofox_session_key = normalize_hermes_camofox_identity(
        camofox.and_then(|map| yaml_string_field(map, "session_key")),
        "browser.camofox.session_key",
    )
    .unwrap_or_default();
    let browser_camofox_adopt_existing_tab = camofox
        .and_then(|map| yaml_bool_field(map, "adopt_existing_tab"))
        .unwrap_or(false);
    let browser_dialog_policy =
        normalize_hermes_browser_dialog_policy(browser.and_then(|map| yaml_string_field(map, "dialog_policy")), false)
            .unwrap_or_else(|_| "must_respond".to_string());
    let browser_dialog_timeout = browser
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "dialog_timeout_s"), 300, 1, 86400))
        .unwrap_or(300);

    crate::jv!({
        "browserInactivityTimeout": browser_inactivity_timeout,
        "browserCommandTimeout": browser_command_timeout,
        "browserRecordSessions": browser_record_sessions,
        "browserEngine": browser_engine,
        "browserAllowPrivateUrls": browser_allow_private_urls,
        "browserAutoLocalForPrivateUrls": browser_auto_local_for_private_urls,
        "browserCdpUrl": browser_cdp_url,
        "browserCamofoxManagedPersistence": browser_camofox_managed_persistence,
        "browserCamofoxUserId": browser_camofox_user_id,
        "browserCamofoxSessionKey": browser_camofox_session_key,
        "browserCamofoxAdoptExistingTab": browser_camofox_adopt_existing_tab,
        "browserDialogPolicy": browser_dialog_policy,
        "browserDialogTimeout": browser_dialog_timeout,
    })
}

fn merge_hermes_browser_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_browser_config_values(config);
    let browser_inactivity_timeout = validate_hermes_i64(
        if form.get("browserInactivityTimeout").is_some() {
            form_i64(form, "browserInactivityTimeout")
        } else {
            Some(current["browserInactivityTimeout"].as_i64().unwrap_or(120))
        },
        "browser.inactivity_timeout",
        120,
        1,
        86400,
    )?;
    let browser_command_timeout = validate_hermes_i64(
        if form.get("browserCommandTimeout").is_some() {
            form_i64(form, "browserCommandTimeout")
        } else {
            Some(current["browserCommandTimeout"].as_i64().unwrap_or(30))
        },
        "browser.command_timeout",
        30,
        5,
        3600,
    )?;
    let browser_record_sessions =
        form_bool(form, "browserRecordSessions").unwrap_or_else(|| current["browserRecordSessions"].as_bool().unwrap_or(false));
    let browser_engine = normalize_hermes_browser_engine(
        if form.get("browserEngine").is_some() {
            form_string(form, "browserEngine")
        } else {
            current["browserEngine"].as_str().map(ToString::to_string)
        },
        true,
    )?;
    let browser_allow_private_urls = form_bool(form, "browserAllowPrivateUrls")
        .unwrap_or_else(|| current["browserAllowPrivateUrls"].as_bool().unwrap_or(false));
    let browser_auto_local_for_private_urls = form_bool(form, "browserAutoLocalForPrivateUrls")
        .unwrap_or_else(|| current["browserAutoLocalForPrivateUrls"].as_bool().unwrap_or(true));
    let browser_cdp_url = if form.get("browserCdpUrl").is_some() {
        form_string(form, "browserCdpUrl")
            .ok_or_else(|| "browser.cdp_url 必须是字符串".to_string())?
            .trim()
            .to_string()
    } else {
        current["browserCdpUrl"].as_str().unwrap_or_default().trim().to_string()
    };
    let browser_camofox_managed_persistence = form_bool(form, "browserCamofoxManagedPersistence")
        .unwrap_or_else(|| current["browserCamofoxManagedPersistence"].as_bool().unwrap_or(false));
    let browser_camofox_user_id = normalize_hermes_camofox_identity(
        if form.get("browserCamofoxUserId").is_some() {
            Some(form_string(form, "browserCamofoxUserId").ok_or_else(|| "browser.camofox.user_id 必须是字符串".to_string())?)
        } else {
            current["browserCamofoxUserId"].as_str().map(ToString::to_string)
        },
        "browser.camofox.user_id",
    )?;
    let browser_camofox_session_key = normalize_hermes_camofox_identity(
        if form.get("browserCamofoxSessionKey").is_some() {
            Some(
                form_string(form, "browserCamofoxSessionKey")
                    .ok_or_else(|| "browser.camofox.session_key 必须是字符串".to_string())?,
            )
        } else {
            current["browserCamofoxSessionKey"].as_str().map(ToString::to_string)
        },
        "browser.camofox.session_key",
    )?;
    let browser_camofox_adopt_existing_tab = form_bool(form, "browserCamofoxAdoptExistingTab")
        .unwrap_or_else(|| current["browserCamofoxAdoptExistingTab"].as_bool().unwrap_or(false));
    let browser_dialog_policy = normalize_hermes_browser_dialog_policy(
        if form.get("browserDialogPolicy").is_some() {
            form_string(form, "browserDialogPolicy")
        } else {
            current["browserDialogPolicy"].as_str().map(ToString::to_string)
        },
        true,
    )?;
    let browser_dialog_timeout = validate_hermes_i64(
        if form.get("browserDialogTimeout").is_some() {
            form_i64(form, "browserDialogTimeout")
        } else {
            Some(current["browserDialogTimeout"].as_i64().unwrap_or(300))
        },
        "browser.dialog_timeout_s",
        300,
        1,
        86400,
    )?;

    let root = ensure_yaml_object(config)?;
    let browser = yaml_child_object(root, "browser")?;
    browser.insert(
        yaml_key("inactivity_timeout"),
        serde_yaml::Value::Number(browser_inactivity_timeout.into()),
    );
    browser.insert(yaml_key("command_timeout"), serde_yaml::Value::Number(browser_command_timeout.into()));
    browser.insert(yaml_key("record_sessions"), serde_yaml::Value::Bool(browser_record_sessions));
    browser.insert(yaml_key("engine"), serde_yaml::Value::String(browser_engine));
    browser.insert(yaml_key("allow_private_urls"), serde_yaml::Value::Bool(browser_allow_private_urls));
    browser.insert(
        yaml_key("auto_local_for_private_urls"),
        serde_yaml::Value::Bool(browser_auto_local_for_private_urls),
    );
    set_optional_yaml_string(browser, "cdp_url", browser_cdp_url);
    let camofox = yaml_child_object(browser, "camofox")?;
    camofox.insert(
        yaml_key("managed_persistence"),
        serde_yaml::Value::Bool(browser_camofox_managed_persistence),
    );
    set_optional_yaml_string(camofox, "user_id", browser_camofox_user_id);
    set_optional_yaml_string(camofox, "session_key", browser_camofox_session_key);
    camofox.insert(
        yaml_key("adopt_existing_tab"),
        serde_yaml::Value::Bool(browser_camofox_adopt_existing_tab),
    );
    browser.insert(yaml_key("dialog_policy"), serde_yaml::Value::String(browser_dialog_policy));
    browser.insert(yaml_key("dialog_timeout_s"), serde_yaml::Value::Number(browser_dialog_timeout.into()));
    Ok(())
}