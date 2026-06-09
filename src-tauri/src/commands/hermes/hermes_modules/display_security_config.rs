
fn normalize_hermes_unauthorized_dm_behavior(value: Option<String>, strict: bool) -> Result<String, String> {
    let behavior = value.unwrap_or_default().trim().to_ascii_lowercase();
    if matches!(behavior.as_str(), "pair" | "ignore") {
        return Ok(behavior);
    }
    if strict {
        Err("unauthorized_dm_behavior 必须是 pair 或 ignore".to_string())
    } else {
        Ok("pair".to_string())
    }
}

fn build_hermes_unauthorized_dm_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let behavior = root
        .and_then(|map| yaml_string_field(map, "unauthorized_dm_behavior"))
        .and_then(|value| normalize_hermes_unauthorized_dm_behavior(Some(value), false).ok())
        .unwrap_or_else(|| "pair".to_string());

    crate::jv!({
        "unauthorizedDmBehavior": behavior,
    })
}

fn merge_hermes_unauthorized_dm_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_unauthorized_dm_config_values(config);
    let behavior = normalize_hermes_unauthorized_dm_behavior(
        form_string(form, "unauthorizedDmBehavior")
            .or_else(|| current["unauthorizedDmBehavior"].as_str().map(ToString::to_string)),
        true,
    )?;

    let root = ensure_yaml_object(config)?;
    root.insert(yaml_key("unauthorized_dm_behavior"), serde_yaml::Value::String(behavior));
    Ok(())
}

fn build_hermes_security_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let security = root.and_then(|map| yaml_get_mapping(map, "security"));

    let tirith_enabled = security
        .and_then(|map| yaml_bool_field(map, "tirith_enabled"))
        .unwrap_or(true);
    let tirith_path = security
        .and_then(|map| yaml_string_field(map, "tirith_path"))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "tirith".to_string());
    let tirith_timeout = security
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "tirith_timeout"), 5, 1, 300))
        .unwrap_or(5);
    let tirith_fail_open = security
        .and_then(|map| yaml_bool_field(map, "tirith_fail_open"))
        .unwrap_or(true);

    crate::jv!({
        "tirithEnabled": tirith_enabled,
        "tirithPath": tirith_path,
        "tirithTimeout": tirith_timeout,
        "tirithFailOpen": tirith_fail_open,
    })
}

fn merge_hermes_security_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_security_config_values(config);
    let tirith_path = form_string(form, "tirithPath")
        .or_else(|| current["tirithPath"].as_str().map(ToString::to_string))
        .unwrap_or_else(|| "tirith".to_string())
        .trim()
        .to_string();
    if tirith_path.is_empty() {
        return Err("security.tirith_path 不能为空".to_string());
    }

    let root = ensure_yaml_object(config)?;
    let tirith_timeout = validate_hermes_i64(
        if form.get("tirithTimeout").is_some() {
            form_i64(form, "tirithTimeout")
        } else {
            Some(current["tirithTimeout"].as_i64().unwrap_or(5))
        },
        "security.tirith_timeout",
        5,
        1,
        300,
    )?;
    let security = yaml_child_object(root, "security")?;
    security.insert(
        yaml_key("tirith_enabled"),
        serde_yaml::Value::Bool(
            form_bool(form, "tirithEnabled").unwrap_or_else(|| current["tirithEnabled"].as_bool().unwrap_or(true)),
        ),
    );
    security.insert(yaml_key("tirith_path"), serde_yaml::Value::String(tirith_path));
    security.insert(yaml_key("tirith_timeout"), serde_yaml::Value::Number(tirith_timeout.into()));
    security.insert(
        yaml_key("tirith_fail_open"),
        serde_yaml::Value::Bool(
            form_bool(form, "tirithFailOpen").unwrap_or_else(|| current["tirithFailOpen"].as_bool().unwrap_or(true)),
        ),
    );
    Ok(())
}

fn normalize_hermes_human_delay_mode(value: Option<String>, strict: bool) -> Result<String, String> {
    let mode = value.unwrap_or_default().trim().to_ascii_lowercase();
    let mode = if mode.is_empty() { "off".to_string() } else { mode };
    if matches!(mode.as_str(), "off" | "natural" | "custom") {
        return Ok(mode);
    }
    if strict {
        Err("human_delay.mode 必须是 off、natural 或 custom".to_string())
    } else {
        Ok("off".to_string())
    }
}

const HERMES_DISPLAY_LANGUAGE_VALUES: &[&str] = &[
    "en", "zh", "zh-hant", "ja", "de", "es", "fr", "tr", "uk", "af", "ko", "it", "ga", "pt", "ru", "hu",
];

const HERMES_DISPLAY_BUSY_INPUT_MODES: &[&str] = &["interrupt", "queue", "steer"];
const HERMES_DISPLAY_BACKGROUND_PROCESS_NOTIFICATIONS: &[&str] = &["off", "result", "error", "all"];
const HERMES_DISPLAY_FINAL_RESPONSE_MARKDOWN_VALUES: &[&str] = &["render", "strip", "raw"];
const HERMES_TUI_STATUS_INDICATORS: &[&str] = &["kaomoji", "emoji", "unicode", "ascii"];
const HERMES_COPY_SHORTCUTS: &[&str] = &["auto", "ctrl_c", "ctrl_shift_c", "disabled"];
const HERMES_DISPLAY_SKINS: &[&str] = &[
    "default",
    "ares",
    "mono",
    "slate",
    "daylight",
    "warm-lightmode",
    "poseidon",
    "sisyphus",
    "charizard",
];

const HERMES_RUNTIME_FOOTER_FIELDS: &[&str] = &["model", "context_pct", "cwd", "duration", "tokens", "cost"];

fn normalize_hermes_display_language(value: Option<String>, strict: bool) -> Result<String, String> {
    let language = value.unwrap_or_default().trim().to_ascii_lowercase();
    let language = if language.is_empty() { "en".to_string() } else { language };
    if HERMES_DISPLAY_LANGUAGE_VALUES.contains(&language.as_str()) {
        Ok(language)
    } else if strict {
        Err("display.language 不在支持列表中".to_string())
    } else {
        Ok("en".to_string())
    }
}

fn normalize_hermes_display_skin(value: Option<String>, strict: bool) -> Result<String, String> {
    let skin = value.unwrap_or_default().trim().to_ascii_lowercase();
    let skin = if skin.is_empty() { "default".to_string() } else { skin };
    if HERMES_DISPLAY_SKINS.contains(&skin.as_str()) {
        Ok(skin)
    } else if strict {
        Err(
            "display.skin 必须是内置皮肤 default、ares、mono、slate、daylight、warm-lightmode、poseidon、sisyphus 或 charizard"
                .to_string(),
        )
    } else {
        Ok("default".to_string())
    }
}

fn normalize_hermes_display_resume(value: Option<String>, strict: bool) -> Result<String, String> {
    let mode = value.unwrap_or_default().trim().to_ascii_lowercase();
    let mode = if mode.is_empty() { "full".to_string() } else { mode };
    if matches!(mode.as_str(), "full" | "minimal") {
        Ok(mode)
    } else if strict {
        Err("display.resume_display 必须是 full 或 minimal".to_string())
    } else {
        Ok("full".to_string())
    }
}

fn normalize_hermes_display_busy_input_mode(value: Option<String>, strict: bool) -> Result<String, String> {
    let mode = value.unwrap_or_default().trim().to_ascii_lowercase();
    let mode = if mode.is_empty() { "interrupt".to_string() } else { mode };
    if HERMES_DISPLAY_BUSY_INPUT_MODES.contains(&mode.as_str()) {
        Ok(mode)
    } else if strict {
        Err("display.busy_input_mode 必须是 interrupt、queue 或 steer".to_string())
    } else {
        Ok("interrupt".to_string())
    }
}

fn normalize_hermes_display_background_process_notifications(value: Option<String>, strict: bool) -> Result<String, String> {
    let mode = value.unwrap_or_default().trim().to_ascii_lowercase();
    let mode = if mode.is_empty() { "all".to_string() } else { mode };
    if HERMES_DISPLAY_BACKGROUND_PROCESS_NOTIFICATIONS.contains(&mode.as_str()) {
        Ok(mode)
    } else if strict {
        Err("display.background_process_notifications 必须是 off、result、error 或 all".to_string())
    } else {
        Ok("all".to_string())
    }
}

fn normalize_hermes_display_final_response_markdown(value: Option<String>, strict: bool) -> Result<String, String> {
    let mode = value.unwrap_or_default().trim().to_ascii_lowercase();
    let mode = if mode.is_empty() { "strip".to_string() } else { mode };
    if HERMES_DISPLAY_FINAL_RESPONSE_MARKDOWN_VALUES.contains(&mode.as_str()) {
        Ok(mode)
    } else if strict {
        Err("display.final_response_markdown 必须是 render、strip 或 raw".to_string())
    } else {
        Ok("strip".to_string())
    }
}

fn normalize_hermes_tui_status_indicator(value: Option<String>, strict: bool) -> Result<String, String> {
    let mode = value.unwrap_or_default().trim().to_ascii_lowercase();
    let mode = if mode.is_empty() { "kaomoji".to_string() } else { mode };
    if HERMES_TUI_STATUS_INDICATORS.contains(&mode.as_str()) {
        Ok(mode)
    } else if strict {
        Err("display.tui_status_indicator 必须是 kaomoji、emoji、unicode 或 ascii".to_string())
    } else {
        Ok("kaomoji".to_string())
    }
}

fn normalize_hermes_copy_shortcut(value: Option<String>, strict: bool) -> Result<String, String> {
    let mode = value.unwrap_or_default().trim().to_ascii_lowercase();
    let mode = if mode.is_empty() { "auto".to_string() } else { mode };
    if HERMES_COPY_SHORTCUTS.contains(&mode.as_str()) {
        Ok(mode)
    } else if strict {
        Err("display.copy_shortcut 必须是 auto、ctrl_c、ctrl_shift_c 或 disabled".to_string())
    } else {
        Ok("auto".to_string())
    }
}

fn normalize_hermes_runtime_footer_fields_text(value: Option<String>, strict: bool) -> Result<Vec<String>, String> {
    let fields = match value {
        Some(value) => {
            let text = value.trim().to_string();
            if text.contains('\n') || text.contains(',') {
                text.split(['\n', ','])
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            } else if text.is_empty() {
                Vec::new()
            } else {
                vec![text]
            }
        }
        None => Vec::new(),
    };
    let fields = if fields.is_empty() {
        vec!["model".to_string(), "context_pct".to_string(), "cwd".to_string()]
    } else {
        fields
    };
    if let Some(invalid) = fields
        .iter()
        .find(|item| !HERMES_RUNTIME_FOOTER_FIELDS.contains(&item.as_str()))
    {
        if strict {
            return Err(format!("display.runtime_footer.fields 包含不支持的字段: {invalid}"));
        }
        return Ok(vec!["model".to_string(), "context_pct".to_string(), "cwd".to_string()]);
    }
    Ok(fields)
}

fn normalize_hermes_runtime_footer_fields(value: Option<&serde_yaml::Value>, strict: bool) -> Result<Vec<String>, String> {
    let fields = match value {
        Some(serde_yaml::Value::Sequence(items)) => items
            .iter()
            .filter_map(|item| item.as_str().map(str::trim))
            .filter(|item| !item.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        Some(serde_yaml::Value::String(text)) => text
            .split(['\n', ','])
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    normalize_hermes_runtime_footer_fields_text(if fields.is_empty() { None } else { Some(fields.join("\n")) }, strict)
}

fn build_hermes_display_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let display = root.and_then(|map| yaml_get_mapping(map, "display"));
    let dashboard = root.and_then(|map| yaml_get_mapping(map, "dashboard"));
    let runtime_footer = display.and_then(|map| yaml_get_mapping(map, "runtime_footer"));
    let user_message_preview = display.and_then(|map| yaml_get_mapping(map, "user_message_preview"));
    let runtime_footer_fields =
        normalize_hermes_runtime_footer_fields(runtime_footer.and_then(|map| yaml_get(map, "fields")), false)
            .unwrap_or_else(|_| vec!["model".to_string(), "context_pct".to_string(), "cwd".to_string()]);

    crate::jv!({
        "displayCompact": display.and_then(|map| yaml_bool_field(map, "compact")).unwrap_or(false),
        "displaySkin": normalize_hermes_display_skin(
            display.and_then(|map| yaml_string_field(map, "skin")),
            false,
        ).unwrap_or_else(|_| "default".to_string()),
        "displayToolPrefix": normalize_hermes_display_tool_prefix(
            display.and_then(|map| yaml_string_field(map, "tool_prefix")),
            false,
        ).unwrap_or_else(|_| "┊".to_string()),
        "displayToolProgress": normalize_hermes_display_tool_progress(
            display.and_then(|map| yaml_string_field(map, "tool_progress")),
            false,
            "display.tool_progress",
        ).unwrap_or_else(|_| "all".to_string()),
        "displayShowReasoning": display.and_then(|map| yaml_bool_field(map, "show_reasoning")).unwrap_or(false),
        "displayToolPreviewLength": display
            .map(|map| bounded_hermes_i64(yaml_i64_field(map, "tool_preview_length"), 0, 0, 200000))
            .unwrap_or(0),
        "displayCleanupProgress": display.and_then(|map| yaml_bool_field(map, "cleanup_progress")).unwrap_or(false),
        "displayToolProgressCommand": display.and_then(|map| yaml_bool_field(map, "tool_progress_command")).unwrap_or(false),
        "displayInterimAssistantMessages": display.and_then(|map| yaml_bool_field(map, "interim_assistant_messages")).unwrap_or(true),
        "displayRuntimeFooterEnabled": runtime_footer.and_then(|map| yaml_bool_field(map, "enabled")).unwrap_or(false),
        "displayRuntimeFooterFields": runtime_footer_fields.join("\n"),
        "displayFileMutationVerifier": display.and_then(|map| yaml_bool_field(map, "file_mutation_verifier")).unwrap_or(true),
        "displayShowCost": display.and_then(|map| yaml_bool_field(map, "show_cost")).unwrap_or(false),
        "dashboardShowTokenAnalytics": dashboard.and_then(|map| yaml_bool_field(map, "show_token_analytics")).unwrap_or(false),
        "displayLanguage": normalize_hermes_display_language(
            display.and_then(|map| yaml_string_field(map, "language")),
            false,
        ).unwrap_or_else(|_| "en".to_string()),
        "displayResumeDisplay": normalize_hermes_display_resume(
            display.and_then(|map| yaml_string_field(map, "resume_display")),
            false,
        ).unwrap_or_else(|_| "full".to_string()),
        "displayBusyInputMode": normalize_hermes_display_busy_input_mode(
            display.and_then(|map| yaml_string_field(map, "busy_input_mode")),
            false,
        ).unwrap_or_else(|_| "interrupt".to_string()),
        "displayBackgroundProcessNotifications": normalize_hermes_display_background_process_notifications(
            display.and_then(|map| yaml_string_field(map, "background_process_notifications")),
            false,
        ).unwrap_or_else(|_| "all".to_string()),
        "displayFinalResponseMarkdown": normalize_hermes_display_final_response_markdown(
            display.and_then(|map| yaml_string_field(map, "final_response_markdown")),
            false,
        ).unwrap_or_else(|_| "strip".to_string()),
        "displayTimestamps": display.and_then(|map| yaml_bool_field(map, "timestamps")).unwrap_or(false),
        "displayBellOnComplete": display.and_then(|map| yaml_bool_field(map, "bell_on_complete")).unwrap_or(false),
        "displayPersistentOutput": display.and_then(|map| yaml_bool_field(map, "persistent_output")).unwrap_or(true),
        "displayPersistentOutputMaxLines": display
            .map(|map| bounded_hermes_i64(yaml_i64_field(map, "persistent_output_max_lines"), 200, 0, 100000))
            .unwrap_or(200),
        "displayInlineDiffs": display.and_then(|map| yaml_bool_field(map, "inline_diffs")).unwrap_or(true),
        "displayTuiAutoResumeRecent": display.and_then(|map| yaml_bool_field(map, "tui_auto_resume_recent")).unwrap_or(false),
        "displayTuiStatusIndicator": normalize_hermes_tui_status_indicator(
            display.and_then(|map| yaml_string_field(map, "tui_status_indicator")),
            false,
        ).unwrap_or_else(|_| "kaomoji".to_string()),
        "displayUserMessagePreviewFirstLines": user_message_preview
            .map(|map| bounded_hermes_i64(yaml_i64_field(map, "first_lines"), 2, 1, 100))
            .unwrap_or(2),
        "displayUserMessagePreviewLastLines": user_message_preview
            .map(|map| bounded_hermes_i64(yaml_i64_field(map, "last_lines"), 2, 0, 100))
            .unwrap_or(2),
        "displayEphemeralSystemTtl": display
            .map(|map| bounded_hermes_i64(yaml_i64_field(map, "ephemeral_system_ttl"), 0, 0, 86400))
            .unwrap_or(0),
        "displayCopyShortcut": normalize_hermes_copy_shortcut(
            display.and_then(|map| yaml_string_field(map, "copy_shortcut")),
            false,
        ).unwrap_or_else(|_| "auto".to_string()),
    })
}

include!("display_security_config/display_merge_tail.rs");