
#[cfg(test)]
mod hermes_agent_runtime_config_tests {
    use super::{build_hermes_agent_runtime_config_values, merge_hermes_agent_runtime_config};
    use serde_json::Value;

    #[test]
    fn agent_runtime_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_agent_runtime_config_values(&config);
        assert_eq!(values["agentMaxTurns"], 90);
        assert_eq!(values["gatewayTimeout"], 1800);
        assert_eq!(values["restartDrainTimeout"], 180);
        assert_eq!(values["apiMaxRetries"], 3);
        assert_eq!(values["gatewayTimeoutWarning"], 900);
        assert_eq!(values["clarifyTimeout"], 600);
        assert_eq!(values["gatewayNotifyInterval"], 180);
        assert_eq!(values["gatewayAutoContinueFreshness"], 3600);
        assert_eq!(values["imageInputMode"], "auto");
        assert_eq!(values["agentVerbose"], false);
        assert_eq!(values["reasoningEffort"], "medium");
        assert_eq!(values["personalitiesJson"], "{}");
    }

    #[test]
    fn agent_runtime_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
agent:
  max_turns: 240
  gateway_timeout: 7200
  restart_drain_timeout: 600
  api_max_retries: 5
  gateway_timeout_warning: 1200
  clarify_timeout: 900
  gateway_notify_interval: 240
  gateway_auto_continue_freshness: 5400
  image_input_mode: native
  verbose: true
  reasoning_effort: high
  personalities:
    concise: Keep answers short.
    teacher: Explain with examples.
"#,
        )
        .unwrap();

        let values = build_hermes_agent_runtime_config_values(&config);
        assert_eq!(values["agentMaxTurns"], 240);
        assert_eq!(values["gatewayTimeout"], 7200);
        assert_eq!(values["restartDrainTimeout"], 600);
        assert_eq!(values["apiMaxRetries"], 5);
        assert_eq!(values["gatewayTimeoutWarning"], 1200);
        assert_eq!(values["clarifyTimeout"], 900);
        assert_eq!(values["gatewayNotifyInterval"], 240);
        assert_eq!(values["gatewayAutoContinueFreshness"], 5400);
        assert_eq!(values["imageInputMode"], "native");
        assert_eq!(values["agentVerbose"], true);
        assert_eq!(values["reasoningEffort"], "high");
        let personalities: Value = serde_json::from_str(values["personalitiesJson"].as_str().unwrap()).unwrap();
        assert_eq!(personalities["concise"], "Keep answers short.");
        assert_eq!(personalities["teacher"], "Explain with examples.");
    }

    #[test]
    fn merge_agent_runtime_config_preserves_unrelated_yaml() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
agent:
  max_turns: 90
  disabled_toolsets:
    - terminal
  custom_flag: keep-agent
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_agent_runtime_config(
            &mut config,
            &crate::jv!({
                "agentMaxTurns": "180",
                "gatewayTimeout": "3600",
                "restartDrainTimeout": "300",
                "apiMaxRetries": "2",
                "gatewayTimeoutWarning": "600",
                "clarifyTimeout": "300",
                "gatewayNotifyInterval": "120",
                "gatewayAutoContinueFreshness": "1800",
                "imageInputMode": "text",
                "agentVerbose": true,
                "reasoningEffort": "low",
                "personalitiesJson": r#"{"concise":" Keep replies brief. ","ops":"Focus on operational risk."}"#,
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["agent"]["max_turns"].as_i64(), Some(180));
        assert_eq!(config["agent"]["gateway_timeout"].as_i64(), Some(3600));
        assert_eq!(config["agent"]["restart_drain_timeout"].as_i64(), Some(300));
        assert_eq!(config["agent"]["api_max_retries"].as_i64(), Some(2));
        assert_eq!(config["agent"]["gateway_timeout_warning"].as_i64(), Some(600));
        assert_eq!(config["agent"]["clarify_timeout"].as_i64(), Some(300));
        assert_eq!(config["agent"]["gateway_notify_interval"].as_i64(), Some(120));
        assert_eq!(config["agent"]["gateway_auto_continue_freshness"].as_i64(), Some(1800));
        assert_eq!(config["agent"]["image_input_mode"].as_str(), Some("text"));
        assert_eq!(config["agent"]["verbose"].as_bool(), Some(true));
        assert_eq!(config["agent"]["reasoning_effort"].as_str(), Some("low"));
        assert_eq!(config["agent"]["personalities"]["concise"].as_str(), Some("Keep replies brief."));
        assert_eq!(config["agent"]["personalities"]["ops"].as_str(), Some("Focus on operational risk."));
        assert_eq!(config["agent"]["disabled_toolsets"][0].as_str(), Some("terminal"));
        assert_eq!(config["agent"]["custom_flag"].as_str(), Some("keep-agent"));
    }

    #[test]
    fn merge_agent_runtime_config_removes_empty_personalities() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
agent:
  personalities:
    concise: Keep answers short.
  custom_flag: keep-agent
"#,
        )
        .unwrap();

        merge_hermes_agent_runtime_config(
            &mut config,
            &crate::jv!({
                "personalitiesJson": "{}",
            }),
        )
        .unwrap();

        assert!(config["agent"].get("personalities").is_none());
        assert_eq!(config["agent"]["custom_flag"].as_str(), Some("keep-agent"));
    }

    #[test]
    fn merge_agent_runtime_config_allows_zero_disable_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        merge_hermes_agent_runtime_config(
            &mut config,
            &crate::jv!({
                "gatewayTimeout": "0",
                "restartDrainTimeout": "0",
                "gatewayTimeoutWarning": "0",
                "gatewayNotifyInterval": "0",
                "gatewayAutoContinueFreshness": "0",
            }),
        )
        .unwrap();

        assert_eq!(config["agent"]["gateway_timeout"].as_i64(), Some(0));
        assert_eq!(config["agent"]["restart_drain_timeout"].as_i64(), Some(0));
        assert_eq!(config["agent"]["gateway_timeout_warning"].as_i64(), Some(0));
        assert_eq!(config["agent"]["gateway_notify_interval"].as_i64(), Some(0));
        assert_eq!(config["agent"]["gateway_auto_continue_freshness"].as_i64(), Some(0));
    }

    #[test]
    fn merge_agent_runtime_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_agent_runtime_config(&mut config, &crate::jv!({ "imageInputMode": "pixel" })).unwrap_err();
        assert!(err.contains("agent.image_input_mode"));
        let err = merge_hermes_agent_runtime_config(&mut config, &crate::jv!({ "agentMaxTurns": "0" })).unwrap_err();
        assert!(err.contains("agent.max_turns"));
        let err = merge_hermes_agent_runtime_config(&mut config, &crate::jv!({ "apiMaxRetries": "0" })).unwrap_err();
        assert!(err.contains("agent.api_max_retries"));
        let err = merge_hermes_agent_runtime_config(&mut config, &crate::jv!({ "clarifyTimeout": "-1" })).unwrap_err();
        assert!(err.contains("agent.clarify_timeout"));
        let err = merge_hermes_agent_runtime_config(&mut config, &crate::jv!({ "reasoningEffort": "max" })).unwrap_err();
        assert!(err.contains("agent.reasoning_effort"));
        let err = merge_hermes_agent_runtime_config(&mut config, &crate::jv!({ "personalitiesJson": r#"{"bad name":"x"}"# }))
            .unwrap_err();
        assert!(err.contains("agent.personalities.bad name"));
        let err = merge_hermes_agent_runtime_config(&mut config, &crate::jv!({ "personalitiesJson": r#"{"concise":123}"# }))
            .unwrap_err();
        assert!(err.contains("agent.personalities.concise"));
    }
}

#[cfg(test)]
mod hermes_unauthorized_dm_config_tests {
    use super::{build_hermes_unauthorized_dm_config_values, merge_hermes_unauthorized_dm_config};

    #[test]
    fn unauthorized_dm_values_have_pair_default() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_unauthorized_dm_config_values(&config);
        assert_eq!(values["unauthorizedDmBehavior"], "pair");
    }

    #[test]
    fn unauthorized_dm_values_normalize_existing_behavior() {
        let config: serde_yaml::Value = serde_yaml::from_str("unauthorized_dm_behavior: IGNORE").unwrap();
        let values = build_hermes_unauthorized_dm_config_values(&config);
        assert_eq!(values["unauthorizedDmBehavior"], "ignore");

        let config: serde_yaml::Value = serde_yaml::from_str("unauthorized_dm_behavior: silent").unwrap();
        let values = build_hermes_unauthorized_dm_config_values(&config);
        assert_eq!(values["unauthorizedDmBehavior"], "pair");
    }

    #[test]
    fn merge_unauthorized_dm_config_preserves_unrelated_yaml() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
unauthorized_dm_behavior: pair
platforms:
  telegram:
    enabled: true
    custom_flag: keep-platform
memory:
  memory_enabled: true
"#,
        )
        .unwrap();

        merge_hermes_unauthorized_dm_config(&mut config, &crate::jv!({ "unauthorizedDmBehavior": "ignore" })).unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["memory"]["memory_enabled"].as_bool(), Some(true));
        assert_eq!(config["platforms"]["telegram"]["custom_flag"].as_str(), Some("keep-platform"));
        assert_eq!(config["unauthorized_dm_behavior"].as_str(), Some("ignore"));
    }

    #[test]
    fn merge_unauthorized_dm_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err =
            merge_hermes_unauthorized_dm_config(&mut config, &crate::jv!({ "unauthorizedDmBehavior": "silent" })).unwrap_err();
        assert!(err.contains("unauthorized_dm_behavior"));
    }
}

#[cfg(test)]
mod hermes_human_delay_config_tests {
    use super::{build_hermes_human_delay_config_values, merge_hermes_human_delay_config};

    #[test]
    fn human_delay_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_human_delay_config_values(&config);
        assert_eq!(values["humanDelayMode"], "off");
        assert_eq!(values["humanDelayMinMs"], 800);
        assert_eq!(values["humanDelayMaxMs"], 2500);
    }

    #[test]
    fn human_delay_values_normalize_existing_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
human_delay:
  mode: CUSTOM
  min_ms: 1200
  max_ms: 3600
"#,
        )
        .unwrap();
        let values = build_hermes_human_delay_config_values(&config);
        assert_eq!(values["humanDelayMode"], "custom");
        assert_eq!(values["humanDelayMinMs"], 1200);
        assert_eq!(values["humanDelayMaxMs"], 3600);
    }

    #[test]
    fn merge_human_delay_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
human_delay:
  mode: off
  custom_flag: keep-delay
streaming:
  enabled: true
memory:
  memory_enabled: true
"#,
        )
        .unwrap();

        merge_hermes_human_delay_config(
            &mut config,
            &crate::jv!({
                "humanDelayMode": "custom",
                "humanDelayMinMs": "900",
                "humanDelayMaxMs": "2400",
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["memory"]["memory_enabled"].as_bool(), Some(true));
        assert_eq!(config["human_delay"]["custom_flag"].as_str(), Some("keep-delay"));
        assert_eq!(config["human_delay"]["mode"].as_str(), Some("custom"));
        assert_eq!(config["human_delay"]["min_ms"].as_i64(), Some(900));
        assert_eq!(config["human_delay"]["max_ms"].as_i64(), Some(2400));
    }

    #[test]
    fn merge_human_delay_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_human_delay_config(&mut config, &crate::jv!({ "humanDelayMode": "slow" })).unwrap_err();
        assert!(err.contains("human_delay.mode"));

        let err = merge_hermes_human_delay_config(
            &mut config,
            &crate::jv!({
                "humanDelayMode": "custom",
                "humanDelayMinMs": 3000,
                "humanDelayMaxMs": 1000,
            }),
        )
        .unwrap_err();
        assert!(err.contains("human_delay.max_ms"));
    }
}

#[cfg(test)]
mod hermes_display_config_tests {
    use super::{build_hermes_display_config_values, merge_hermes_display_config};

    #[test]
    fn display_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_display_config_values(&config);
        assert_eq!(values["displayToolProgress"], "all");
        assert_eq!(values["displayCompact"], false);
        assert_eq!(values["displaySkin"], "default");
        assert_eq!(values["displayToolPrefix"], "┊");
        assert_eq!(values["displayShowReasoning"], false);
        assert_eq!(values["displayToolPreviewLength"], 0);
        assert_eq!(values["displayCleanupProgress"], false);
        assert_eq!(values["displayToolProgressCommand"], false);
        assert_eq!(values["displayInterimAssistantMessages"], true);
        assert_eq!(values["displayRuntimeFooterEnabled"], false);
        assert_eq!(values["displayRuntimeFooterFields"], "model\ncontext_pct\ncwd");
        assert_eq!(values["displayFileMutationVerifier"], true);
        assert_eq!(values["displayShowCost"], false);
        assert_eq!(values["dashboardShowTokenAnalytics"], false);
        assert_eq!(values["displayLanguage"], "en");
        assert_eq!(values["displayResumeDisplay"], "full");
        assert_eq!(values["displayBusyInputMode"], "interrupt");
        assert_eq!(values["displayBackgroundProcessNotifications"], "all");
        assert_eq!(values["displayFinalResponseMarkdown"], "strip");
        assert_eq!(values["displayTimestamps"], false);
        assert_eq!(values["displayBellOnComplete"], false);
        assert_eq!(values["displayPersistentOutput"], true);
        assert_eq!(values["displayPersistentOutputMaxLines"], 200);
        assert_eq!(values["displayInlineDiffs"], true);
        assert_eq!(values["displayTuiAutoResumeRecent"], false);
        assert_eq!(values["displayTuiStatusIndicator"], "kaomoji");
        assert_eq!(values["displayUserMessagePreviewFirstLines"], 2);
        assert_eq!(values["displayUserMessagePreviewLastLines"], 2);
        assert_eq!(values["displayEphemeralSystemTtl"], 0);
        assert_eq!(values["displayCopyShortcut"], "auto");
    }

    #[test]
    fn display_values_normalize_existing_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
display:
  tool_progress: VERBOSE
  compact: true
  skin: MONO
  tool_prefix: "╎"
  show_reasoning: true
  tool_preview_length: 80
  cleanup_progress: true
  tool_progress_command: true
  interim_assistant_messages: false
  runtime_footer:
    enabled: true
    fields:
      - model
      - duration
      - cost
  file_mutation_verifier: false
  show_cost: true
  language: ZH
  resume_display: minimal
  busy_input_mode: QUEUE
  background_process_notifications: ERROR
  final_response_markdown: RAW
  timestamps: true
  bell_on_complete: true
  persistent_output: false
  persistent_output_max_lines: 80
  inline_diffs: false
  tui_auto_resume_recent: true
  tui_status_indicator: EMOJI
  user_message_preview:
    first_lines: 3
    last_lines: 1
  ephemeral_system_ttl: 120
  copy_shortcut: CTRL_SHIFT_C
dashboard:
  show_token_analytics: true
"#,
        )
        .unwrap();
        let values = build_hermes_display_config_values(&config);
        assert_eq!(values["displayToolProgress"], "verbose");
        assert_eq!(values["displayCompact"], true);
        assert_eq!(values["displaySkin"], "mono");
        assert_eq!(values["displayToolPrefix"], "╎");
        assert_eq!(values["displayShowReasoning"], true);
        assert_eq!(values["displayToolPreviewLength"], 80);
        assert_eq!(values["displayCleanupProgress"], true);
        assert_eq!(values["displayToolProgressCommand"], true);
        assert_eq!(values["displayInterimAssistantMessages"], false);
        assert_eq!(values["displayRuntimeFooterEnabled"], true);
        assert_eq!(values["displayRuntimeFooterFields"], "model\nduration\ncost");
        assert_eq!(values["displayFileMutationVerifier"], false);
        assert_eq!(values["displayShowCost"], true);
        assert_eq!(values["dashboardShowTokenAnalytics"], true);
        assert_eq!(values["displayLanguage"], "zh");
        assert_eq!(values["displayResumeDisplay"], "minimal");
        assert_eq!(values["displayBusyInputMode"], "queue");
        assert_eq!(values["displayBackgroundProcessNotifications"], "error");
        assert_eq!(values["displayFinalResponseMarkdown"], "raw");
        assert_eq!(values["displayTimestamps"], true);
        assert_eq!(values["displayBellOnComplete"], true);
        assert_eq!(values["displayPersistentOutput"], false);
        assert_eq!(values["displayPersistentOutputMaxLines"], 80);
        assert_eq!(values["displayInlineDiffs"], false);
        assert_eq!(values["displayTuiAutoResumeRecent"], true);
        assert_eq!(values["displayTuiStatusIndicator"], "emoji");
        assert_eq!(values["displayUserMessagePreviewFirstLines"], 3);
        assert_eq!(values["displayUserMessagePreviewLastLines"], 1);
        assert_eq!(values["displayEphemeralSystemTtl"], 120);
        assert_eq!(values["displayCopyShortcut"], "ctrl_shift_c");
    }

    #[test]
    fn merge_display_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
display:
  skin: midnight
  runtime_footer:
    enabled: false
    custom_flag: keep-footer
  user_message_preview:
    custom_flag: keep-preview
  platforms:
    telegram:
      tool_progress: new
  custom_flag: keep-display
dashboard:
  custom_flag: keep-dashboard
memory:
  memory_enabled: true
"#,
        )
        .unwrap();

        merge_hermes_display_config(
            &mut config,
            &crate::jv!({
                "displayToolProgress": "off",
                "displayCompact": true,
                "displaySkin": "slate",
                "displayToolPrefix": "│",
                "displayShowReasoning": true,
                "displayToolPreviewLength": 120,
                "displayCleanupProgress": true,
                "displayToolProgressCommand": true,
                "displayInterimAssistantMessages": false,
                "displayRuntimeFooterEnabled": true,
                "displayRuntimeFooterFields": "model\ncontext_pct\nduration",
                "displayFileMutationVerifier": true,
                "displayShowCost": true,
                "dashboardShowTokenAnalytics": true,
                "displayLanguage": "zh-hant",
                "displayResumeDisplay": "minimal",
                "displayBusyInputMode": "steer",
                "displayBackgroundProcessNotifications": "result",
                "displayFinalResponseMarkdown": "render",
                "displayTimestamps": true,
                "displayBellOnComplete": true,
                "displayPersistentOutput": false,
                "displayPersistentOutputMaxLines": 120,
                "displayInlineDiffs": false,
                "displayTuiAutoResumeRecent": true,
                "displayTuiStatusIndicator": "ascii",
                "displayUserMessagePreviewFirstLines": 4,
                "displayUserMessagePreviewLastLines": 0,
                "displayEphemeralSystemTtl": 360,
                "displayCopyShortcut": "disabled",
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["memory"]["memory_enabled"].as_bool(), Some(true));
        assert_eq!(config["dashboard"]["custom_flag"].as_str(), Some("keep-dashboard"));
        assert_eq!(config["dashboard"]["show_token_analytics"].as_bool(), Some(true));
        assert_eq!(config["display"]["compact"].as_bool(), Some(true));
        assert_eq!(config["display"]["skin"].as_str(), Some("slate"));
        assert_eq!(config["display"]["tool_prefix"].as_str(), Some("│"));
        assert_eq!(config["display"]["show_reasoning"].as_bool(), Some(true));
        assert_eq!(config["display"]["tool_preview_length"].as_i64(), Some(120));
        assert_eq!(config["display"]["cleanup_progress"].as_bool(), Some(true));
        assert_eq!(config["display"]["platforms"]["telegram"]["tool_progress"].as_str(), Some("new"));
        assert_eq!(config["display"]["tool_progress"].as_str(), Some("off"));
        assert_eq!(config["display"]["tool_progress_command"].as_bool(), Some(true));
        assert_eq!(config["display"]["interim_assistant_messages"].as_bool(), Some(false));
        assert_eq!(config["display"]["runtime_footer"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["display"]["runtime_footer"]["custom_flag"].as_str(), Some("keep-footer"));
        assert_eq!(
            config["display"]["runtime_footer"]["fields"]
                .as_sequence()
                .unwrap()
                .iter()
                .filter_map(|item| item.as_str())
                .collect::<Vec<_>>(),
            vec!["model", "context_pct", "duration"]
        );
        assert_eq!(config["display"]["file_mutation_verifier"].as_bool(), Some(true));
        assert_eq!(config["display"]["show_cost"].as_bool(), Some(true));
        assert_eq!(config["display"]["language"].as_str(), Some("zh-hant"));
        assert_eq!(config["display"]["resume_display"].as_str(), Some("minimal"));
        assert_eq!(config["display"]["busy_input_mode"].as_str(), Some("steer"));
        assert_eq!(config["display"]["background_process_notifications"].as_str(), Some("result"));
        assert_eq!(config["display"]["final_response_markdown"].as_str(), Some("render"));
        assert_eq!(config["display"]["timestamps"].as_bool(), Some(true));
        assert_eq!(config["display"]["bell_on_complete"].as_bool(), Some(true));
        assert_eq!(config["display"]["persistent_output"].as_bool(), Some(false));
        assert_eq!(config["display"]["persistent_output_max_lines"].as_i64(), Some(120));
        assert_eq!(config["display"]["inline_diffs"].as_bool(), Some(false));
        assert_eq!(config["display"]["tui_auto_resume_recent"].as_bool(), Some(true));
        assert_eq!(config["display"]["tui_status_indicator"].as_str(), Some("ascii"));
        assert_eq!(config["display"]["user_message_preview"]["first_lines"].as_i64(), Some(4));
        assert_eq!(config["display"]["user_message_preview"]["last_lines"].as_i64(), Some(0));
        assert_eq!(config["display"]["user_message_preview"]["custom_flag"].as_str(), Some("keep-preview"));
        assert_eq!(config["display"]["ephemeral_system_ttl"].as_i64(), Some(360));
        assert_eq!(config["display"]["copy_shortcut"].as_str(), Some("disabled"));
        assert_eq!(config["display"]["custom_flag"].as_str(), Some("keep-display"));
    }

    #[test]
    fn merge_display_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_display_config(&mut config, &crate::jv!({ "displayToolProgress": "everything" })).unwrap_err();
        assert!(err.contains("display.tool_progress"));

        let err = merge_hermes_display_config(&mut config, &crate::jv!({ "displaySkin": "unknown" })).unwrap_err();
        assert!(err.contains("display.skin"));

        let err = merge_hermes_display_config(&mut config, &crate::jv!({ "displayToolPrefix": "too-long-prefix" })).unwrap_err();
        assert!(err.contains("display.tool_prefix"));

        let err = merge_hermes_display_config(&mut config, &crate::jv!({ "displayResumeDisplay": "compact" })).unwrap_err();
        assert!(err.contains("display.resume_display"));

        let err = merge_hermes_display_config(&mut config, &crate::jv!({ "displayLanguage": "cn" })).unwrap_err();
        assert!(err.contains("display.language"));

        let err = merge_hermes_display_config(&mut config, &crate::jv!({ "displayRuntimeFooterFields": "model\npassword" }))
            .unwrap_err();
        assert!(err.contains("display.runtime_footer.fields"));

        let err = merge_hermes_display_config(&mut config, &crate::jv!({ "displayBusyInputMode": "replace" })).unwrap_err();
        assert!(err.contains("display.busy_input_mode"));

        let err = merge_hermes_display_config(&mut config, &crate::jv!({ "displayBackgroundProcessNotifications": "silent" }))
            .unwrap_err();
        assert!(err.contains("display.background_process_notifications"));

        let err = merge_hermes_display_config(&mut config, &crate::jv!({ "displayFinalResponseMarkdown": "html" })).unwrap_err();
        assert!(err.contains("display.final_response_markdown"));

        let err = merge_hermes_display_config(&mut config, &crate::jv!({ "displayPersistentOutputMaxLines": -1 })).unwrap_err();
        assert!(err.contains("display.persistent_output_max_lines"));

        let err = merge_hermes_display_config(&mut config, &crate::jv!({ "displayToolPreviewLength": 200001 })).unwrap_err();
        assert!(err.contains("display.tool_preview_length"));

        let err = merge_hermes_display_config(&mut config, &crate::jv!({ "displayTuiStatusIndicator": "rainbow" })).unwrap_err();
        assert!(err.contains("display.tui_status_indicator"));

        let err = merge_hermes_display_config(&mut config, &crate::jv!({ "displayCopyShortcut": "cmd_c" })).unwrap_err();
        assert!(err.contains("display.copy_shortcut"));

        let err =
            merge_hermes_display_config(&mut config, &crate::jv!({ "displayUserMessagePreviewFirstLines": 0 })).unwrap_err();
        assert!(err.contains("display.user_message_preview.first_lines"));

        let err =
            merge_hermes_display_config(&mut config, &crate::jv!({ "displayUserMessagePreviewLastLines": 101 })).unwrap_err();
        assert!(err.contains("display.user_message_preview.last_lines"));

        let err = merge_hermes_display_config(&mut config, &crate::jv!({ "displayEphemeralSystemTtl": 86401 })).unwrap_err();
        assert!(err.contains("display.ephemeral_system_ttl"));
    }
}

include!("runtime_display_kanban/kanban_tests.rs");