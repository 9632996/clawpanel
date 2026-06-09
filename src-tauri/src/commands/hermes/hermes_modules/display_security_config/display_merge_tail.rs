fn merge_hermes_display_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_display_config_values(config);
    let tool_progress = normalize_hermes_display_tool_progress(
        form_string(form, "displayToolProgress").or_else(|| current["displayToolProgress"].as_str().map(ToString::to_string)),
        true,
        "display.tool_progress",
    )?;
    let runtime_footer_fields = normalize_hermes_runtime_footer_fields_text(
        form.get("displayRuntimeFooterFields")
            .and_then(|value| value.as_str().map(ToString::to_string))
            .or_else(|| current["displayRuntimeFooterFields"].as_str().map(ToString::to_string)),
        true,
    )?;
    let final_response_markdown = normalize_hermes_display_final_response_markdown(
        form_string(form, "displayFinalResponseMarkdown")
            .or_else(|| current["displayFinalResponseMarkdown"].as_str().map(ToString::to_string)),
        true,
    )?;
    let persistent_output_max_lines = validate_hermes_i64(
        form_i64(form, "displayPersistentOutputMaxLines").or_else(|| current["displayPersistentOutputMaxLines"].as_i64()),
        "display.persistent_output_max_lines",
        200,
        0,
        100000,
    )?;
    let user_message_preview_first_lines = validate_hermes_i64(
        form_i64(form, "displayUserMessagePreviewFirstLines").or_else(|| current["displayUserMessagePreviewFirstLines"].as_i64()),
        "display.user_message_preview.first_lines",
        2,
        1,
        100,
    )?;
    let user_message_preview_last_lines = validate_hermes_i64(
        form_i64(form, "displayUserMessagePreviewLastLines").or_else(|| current["displayUserMessagePreviewLastLines"].as_i64()),
        "display.user_message_preview.last_lines",
        2,
        0,
        100,
    )?;
    let ephemeral_system_ttl = validate_hermes_i64(
        form_i64(form, "displayEphemeralSystemTtl").or_else(|| current["displayEphemeralSystemTtl"].as_i64()),
        "display.ephemeral_system_ttl",
        0,
        0,
        86400,
    )?;
    let tool_preview_length = validate_hermes_i64(
        form_i64(form, "displayToolPreviewLength").or_else(|| current["displayToolPreviewLength"].as_i64()),
        "display.tool_preview_length",
        0,
        0,
        200000,
    )?;

    let display = yaml_child_object(ensure_yaml_object(config)?, "display")?;
    display.insert(
        yaml_key("compact"),
        serde_yaml::Value::Bool(
            form_bool(form, "displayCompact").unwrap_or_else(|| current["displayCompact"].as_bool().unwrap_or(false)),
        ),
    );
    display.insert(
        yaml_key("skin"),
        serde_yaml::Value::String(normalize_hermes_display_skin(
            form_string(form, "displaySkin").or_else(|| current["displaySkin"].as_str().map(ToString::to_string)),
            true,
        )?),
    );
    display.insert(
        yaml_key("tool_prefix"),
        serde_yaml::Value::String(normalize_hermes_display_tool_prefix(
            form_string(form, "displayToolPrefix").or_else(|| current["displayToolPrefix"].as_str().map(ToString::to_string)),
            true,
        )?),
    );
    display.insert(yaml_key("tool_progress"), serde_yaml::Value::String(tool_progress));
    display.insert(
        yaml_key("show_reasoning"),
        serde_yaml::Value::Bool(
            form_bool(form, "displayShowReasoning").unwrap_or_else(|| current["displayShowReasoning"].as_bool().unwrap_or(false)),
        ),
    );
    display.insert(
        yaml_key("tool_preview_length"),
        serde_yaml::Value::Number(serde_yaml::Number::from(tool_preview_length)),
    );
    display.insert(
        yaml_key("cleanup_progress"),
        serde_yaml::Value::Bool(
            form_bool(form, "displayCleanupProgress")
                .unwrap_or_else(|| current["displayCleanupProgress"].as_bool().unwrap_or(false)),
        ),
    );
    display.insert(
        yaml_key("tool_progress_command"),
        serde_yaml::Value::Bool(
            form_bool(form, "displayToolProgressCommand")
                .unwrap_or_else(|| current["displayToolProgressCommand"].as_bool().unwrap_or(false)),
        ),
    );
    display.insert(
        yaml_key("interim_assistant_messages"),
        serde_yaml::Value::Bool(
            form_bool(form, "displayInterimAssistantMessages")
                .unwrap_or_else(|| current["displayInterimAssistantMessages"].as_bool().unwrap_or(true)),
        ),
    );
    display.insert(
        yaml_key("file_mutation_verifier"),
        serde_yaml::Value::Bool(
            form_bool(form, "displayFileMutationVerifier")
                .unwrap_or_else(|| current["displayFileMutationVerifier"].as_bool().unwrap_or(true)),
        ),
    );
    display.insert(
        yaml_key("show_cost"),
        serde_yaml::Value::Bool(
            form_bool(form, "displayShowCost").unwrap_or_else(|| current["displayShowCost"].as_bool().unwrap_or(false)),
        ),
    );
    display.insert(
        yaml_key("language"),
        serde_yaml::Value::String(normalize_hermes_display_language(
            form_string(form, "displayLanguage").or_else(|| current["displayLanguage"].as_str().map(ToString::to_string)),
            true,
        )?),
    );
    display.insert(
        yaml_key("resume_display"),
        serde_yaml::Value::String(normalize_hermes_display_resume(
            form_string(form, "displayResumeDisplay")
                .or_else(|| current["displayResumeDisplay"].as_str().map(ToString::to_string)),
            true,
        )?),
    );
    display.insert(
        yaml_key("busy_input_mode"),
        serde_yaml::Value::String(normalize_hermes_display_busy_input_mode(
            form_string(form, "displayBusyInputMode")
                .or_else(|| current["displayBusyInputMode"].as_str().map(ToString::to_string)),
            true,
        )?),
    );
    display.insert(
        yaml_key("background_process_notifications"),
        serde_yaml::Value::String(normalize_hermes_display_background_process_notifications(
            form_string(form, "displayBackgroundProcessNotifications").or_else(|| {
                current["displayBackgroundProcessNotifications"]
                    .as_str()
                    .map(ToString::to_string)
            }),
            true,
        )?),
    );
    display.insert(yaml_key("final_response_markdown"), serde_yaml::Value::String(final_response_markdown));
    display.insert(
        yaml_key("timestamps"),
        serde_yaml::Value::Bool(
            form_bool(form, "displayTimestamps").unwrap_or_else(|| current["displayTimestamps"].as_bool().unwrap_or(false)),
        ),
    );
    display.insert(
        yaml_key("bell_on_complete"),
        serde_yaml::Value::Bool(
            form_bool(form, "displayBellOnComplete")
                .unwrap_or_else(|| current["displayBellOnComplete"].as_bool().unwrap_or(false)),
        ),
    );
    display.insert(
        yaml_key("persistent_output"),
        serde_yaml::Value::Bool(
            form_bool(form, "displayPersistentOutput")
                .unwrap_or_else(|| current["displayPersistentOutput"].as_bool().unwrap_or(true)),
        ),
    );
    display.insert(
        yaml_key("persistent_output_max_lines"),
        serde_yaml::Value::Number(serde_yaml::Number::from(persistent_output_max_lines)),
    );
    display.insert(
        yaml_key("inline_diffs"),
        serde_yaml::Value::Bool(
            form_bool(form, "displayInlineDiffs").unwrap_or_else(|| current["displayInlineDiffs"].as_bool().unwrap_or(true)),
        ),
    );
    display.insert(
        yaml_key("tui_auto_resume_recent"),
        serde_yaml::Value::Bool(
            form_bool(form, "displayTuiAutoResumeRecent")
                .unwrap_or_else(|| current["displayTuiAutoResumeRecent"].as_bool().unwrap_or(false)),
        ),
    );
    display.insert(
        yaml_key("tui_status_indicator"),
        serde_yaml::Value::String(normalize_hermes_tui_status_indicator(
            form_string(form, "displayTuiStatusIndicator")
                .or_else(|| current["displayTuiStatusIndicator"].as_str().map(ToString::to_string)),
            true,
        )?),
    );
    display.insert(
        yaml_key("ephemeral_system_ttl"),
        serde_yaml::Value::Number(serde_yaml::Number::from(ephemeral_system_ttl)),
    );
    display.insert(
        yaml_key("copy_shortcut"),
        serde_yaml::Value::String(normalize_hermes_copy_shortcut(
            form_string(form, "displayCopyShortcut").or_else(|| current["displayCopyShortcut"].as_str().map(ToString::to_string)),
            true,
        )?),
    );
    let user_message_preview = yaml_child_object(display, "user_message_preview")?;
    user_message_preview.insert(
        yaml_key("first_lines"),
        serde_yaml::Value::Number(serde_yaml::Number::from(user_message_preview_first_lines)),
    );
    user_message_preview.insert(
        yaml_key("last_lines"),
        serde_yaml::Value::Number(serde_yaml::Number::from(user_message_preview_last_lines)),
    );
    let runtime_footer = yaml_child_object(display, "runtime_footer")?;
    runtime_footer.insert(
        yaml_key("enabled"),
        serde_yaml::Value::Bool(
            form_bool(form, "displayRuntimeFooterEnabled")
                .unwrap_or_else(|| current["displayRuntimeFooterEnabled"].as_bool().unwrap_or(false)),
        ),
    );
    runtime_footer.insert(
        yaml_key("fields"),
        serde_yaml::Value::Sequence(runtime_footer_fields.into_iter().map(serde_yaml::Value::String).collect()),
    );
    let dashboard = yaml_child_object(ensure_yaml_object(config)?, "dashboard")?;
    dashboard.insert(
        yaml_key("show_token_analytics"),
        serde_yaml::Value::Bool(
            form_bool(form, "dashboardShowTokenAnalytics")
                .unwrap_or_else(|| current["dashboardShowTokenAnalytics"].as_bool().unwrap_or(false)),
        ),
    );
    Ok(())
}

fn build_hermes_human_delay_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let human_delay = root.and_then(|map| yaml_get_mapping(map, "human_delay"));
    let mode = human_delay
        .and_then(|map| yaml_string_field(map, "mode"))
        .and_then(|value| normalize_hermes_human_delay_mode(Some(value), false).ok())
        .unwrap_or_else(|| "off".to_string());
    let min_ms = human_delay
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "min_ms"), 800, 0, 60000))
        .unwrap_or(800);
    let max_ms = human_delay
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "max_ms"), 2500, 0, 60000))
        .unwrap_or(2500)
        .max(min_ms);

    crate::jv!({
        "humanDelayMode": mode,
        "humanDelayMinMs": min_ms,
        "humanDelayMaxMs": max_ms,
    })
}

fn build_hermes_kanban_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let kanban = root.and_then(|map| yaml_get_mapping(map, "kanban"));
    crate::jv!({
        "dispatchInGateway": kanban
            .and_then(|map| yaml_bool_field(map, "dispatch_in_gateway"))
            .unwrap_or(true),
        "dispatchIntervalSeconds": kanban
            .map(|map| bounded_hermes_i64(
                yaml_i64_field(map, "dispatch_interval_seconds"),
                60,
                1,
                86400,
            ))
            .unwrap_or(60),
        "maxSpawn": kanban
            .map(|map| bounded_hermes_i64(
                yaml_i64_field(map, "max_spawn"),
                0,
                0,
                1000,
            ))
            .unwrap_or(0),
        "maxInProgress": kanban
            .map(|map| bounded_hermes_i64(
                yaml_i64_field(map, "max_in_progress"),
                0,
                0,
                1000,
            ))
            .unwrap_or(0),
        "failureLimit": kanban
            .map(|map| bounded_hermes_i64(
                yaml_i64_field(map, "failure_limit"),
                2,
                1,
                100,
            ))
            .unwrap_or(2),
        "autoDecompose": kanban
            .and_then(|map| yaml_bool_field(map, "auto_decompose"))
            .unwrap_or(true),
        "autoDecomposePerTick": kanban
            .map(|map| bounded_hermes_i64(
                yaml_i64_field(map, "auto_decompose_per_tick"),
                3,
                1,
                1000,
            ))
            .unwrap_or(3),
        "workerLogRotateBytes": kanban
            .map(|map| bounded_hermes_i64(
                yaml_i64_field(map, "worker_log_rotate_bytes"),
                2097152,
                1,
                1073741824,
            ))
            .unwrap_or(2097152),
        "workerLogBackupCount": kanban
            .map(|map| bounded_hermes_i64(
                yaml_i64_field(map, "worker_log_backup_count"),
                1,
                0,
                100,
            ))
            .unwrap_or(1),
        "orchestratorProfile": kanban
            .and_then(|map| yaml_string_field(map, "orchestrator_profile"))
            .unwrap_or_default(),
        "defaultAssignee": kanban
            .and_then(|map| yaml_string_field(map, "default_assignee"))
            .unwrap_or_default(),
        "dispatchStaleTimeoutSeconds": kanban
            .map(|map| bounded_hermes_i64(
                yaml_i64_field(map, "dispatch_stale_timeout_seconds"),
                14400,
                0,
                604800,
            ))
            .unwrap_or(14400),
    })
}