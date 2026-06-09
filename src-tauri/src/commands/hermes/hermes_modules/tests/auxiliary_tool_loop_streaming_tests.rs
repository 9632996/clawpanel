
#[cfg(test)]
mod hermes_auxiliary_config_tests {
    use super::{build_hermes_auxiliary_config_values, merge_hermes_auxiliary_config};

    #[test]
    fn auxiliary_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_auxiliary_config_values(&config);
        assert_eq!(values["auxiliaryVisionProvider"], "auto");
        assert_eq!(values["auxiliaryVisionModel"], "");
        assert_eq!(values["auxiliaryVisionTimeout"], 30);
        assert_eq!(values["auxiliaryVisionDownloadTimeout"], 30);
        assert_eq!(values["auxiliaryWebExtractProvider"], "auto");
        assert_eq!(values["auxiliaryWebExtractModel"], "");
        assert_eq!(values["auxiliarySessionSearchProvider"], "auto");
        assert_eq!(values["auxiliarySessionSearchModel"], "");
        assert_eq!(values["auxiliarySessionSearchTimeout"], 30);
        assert_eq!(values["auxiliarySessionSearchMaxConcurrency"], 3);
    }

    #[test]
    fn auxiliary_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
auxiliary:
  vision:
    provider: openrouter
    model: google/gemini-2.5-flash
    timeout: 45
    download_timeout: 60
  web_extract:
    provider: main
    model: local-summary
  session_search:
    provider: nous
    model: gemini-3-flash
    timeout: 50
    max_concurrency: 5
"#,
        )
        .unwrap();

        let values = build_hermes_auxiliary_config_values(&config);
        assert_eq!(values["auxiliaryVisionProvider"], "openrouter");
        assert_eq!(values["auxiliaryVisionModel"], "google/gemini-2.5-flash");
        assert_eq!(values["auxiliaryVisionTimeout"], 45);
        assert_eq!(values["auxiliaryVisionDownloadTimeout"], 60);
        assert_eq!(values["auxiliaryWebExtractProvider"], "main");
        assert_eq!(values["auxiliaryWebExtractModel"], "local-summary");
        assert_eq!(values["auxiliarySessionSearchProvider"], "nous");
        assert_eq!(values["auxiliarySessionSearchModel"], "gemini-3-flash");
        assert_eq!(values["auxiliarySessionSearchTimeout"], 50);
        assert_eq!(values["auxiliarySessionSearchMaxConcurrency"], 5);
    }

    #[test]
    fn merge_auxiliary_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
auxiliary:
  vision:
    provider: auto
    custom_flag: keep-vision
  web_extract:
    custom_flag: keep-web
  session_search:
    extra_body:
      enable_thinking: false
    custom_flag: keep-search
  custom_task:
    provider: main
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_auxiliary_config(
            &mut config,
            &crate::jv!({
                "auxiliaryVisionProvider": "codex",
                "auxiliaryVisionModel": "gpt-5.3-codex",
                "auxiliaryVisionTimeout": "40",
                "auxiliaryVisionDownloadTimeout": "55",
                "auxiliaryWebExtractProvider": "gemini",
                "auxiliaryWebExtractModel": "gemini-3-flash",
                "auxiliarySessionSearchProvider": "ollama-cloud",
                "auxiliarySessionSearchModel": "gpt-oss:20b",
                "auxiliarySessionSearchTimeout": "70",
                "auxiliarySessionSearchMaxConcurrency": "6",
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["auxiliary"]["vision"]["provider"].as_str(), Some("codex"));
        assert_eq!(config["auxiliary"]["vision"]["model"].as_str(), Some("gpt-5.3-codex"));
        assert_eq!(config["auxiliary"]["vision"]["timeout"].as_i64(), Some(40));
        assert_eq!(config["auxiliary"]["vision"]["download_timeout"].as_i64(), Some(55));
        assert_eq!(config["auxiliary"]["vision"]["custom_flag"].as_str(), Some("keep-vision"));
        assert_eq!(config["auxiliary"]["web_extract"]["provider"].as_str(), Some("gemini"));
        assert_eq!(config["auxiliary"]["web_extract"]["model"].as_str(), Some("gemini-3-flash"));
        assert_eq!(config["auxiliary"]["web_extract"]["custom_flag"].as_str(), Some("keep-web"));
        assert_eq!(config["auxiliary"]["session_search"]["provider"].as_str(), Some("ollama-cloud"));
        assert_eq!(config["auxiliary"]["session_search"]["model"].as_str(), Some("gpt-oss:20b"));
        assert_eq!(config["auxiliary"]["session_search"]["timeout"].as_i64(), Some(70));
        assert_eq!(config["auxiliary"]["session_search"]["max_concurrency"].as_i64(), Some(6));
        assert_eq!(
            config["auxiliary"]["session_search"]["extra_body"]["enable_thinking"].as_bool(),
            Some(false)
        );
        assert_eq!(config["auxiliary"]["session_search"]["custom_flag"].as_str(), Some("keep-search"));
        assert_eq!(config["auxiliary"]["custom_task"]["provider"].as_str(), Some("main"));
    }

    #[test]
    fn merge_auxiliary_config_rejects_invalid_values() {
        let mut config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let err =
            merge_hermes_auxiliary_config(&mut config, &crate::jv!({ "auxiliaryVisionProvider": "bad-provider" })).unwrap_err();
        assert!(err.contains("auxiliary.vision.provider"));

        let err = merge_hermes_auxiliary_config(&mut config, &crate::jv!({ "auxiliaryVisionModel": "../secret" })).unwrap_err();
        assert!(err.contains("auxiliary.vision.model"));

        let err = merge_hermes_auxiliary_config(&mut config, &crate::jv!({ "auxiliaryVisionTimeout": 0 })).unwrap_err();
        assert!(err.contains("auxiliary.vision.timeout"));

        let err = merge_hermes_auxiliary_config(&mut config, &crate::jv!({ "auxiliaryVisionDownloadTimeout": 0 })).unwrap_err();
        assert!(err.contains("auxiliary.vision.download_timeout"));

        let err =
            merge_hermes_auxiliary_config(&mut config, &crate::jv!({ "auxiliarySessionSearchMaxConcurrency": 0 })).unwrap_err();
        assert!(err.contains("auxiliary.session_search.max_concurrency"));
    }
}

#[cfg(test)]
mod hermes_tool_loop_guardrails_config_tests {
    use super::{build_hermes_tool_loop_guardrails_config_values, merge_hermes_tool_loop_guardrails_config};

    #[test]
    fn tool_loop_guardrails_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_tool_loop_guardrails_config_values(&config);
        assert_eq!(values["warningsEnabled"], true);
        assert_eq!(values["hardStopEnabled"], false);
        assert_eq!(values["warnExactFailure"], 2);
        assert_eq!(values["warnSameToolFailure"], 3);
        assert_eq!(values["warnNoProgress"], 2);
        assert_eq!(values["hardStopExactFailure"], 5);
        assert_eq!(values["hardStopSameToolFailure"], 8);
        assert_eq!(values["hardStopNoProgress"], 5);
    }

    #[test]
    fn merge_tool_loop_guardrails_config_preserves_unrelated_yaml() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
tool_loop_guardrails:
  warnings_enabled: true
  custom_flag: keep-me
  warn_after:
    exact_failure: 2
    custom_warn: 99
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_tool_loop_guardrails_config(
            &mut config,
            &crate::jv!({
                "warningsEnabled": false,
                "hardStopEnabled": true,
                "warnExactFailure": "3",
                "warnSameToolFailure": "4",
                "warnNoProgress": "5",
                "hardStopExactFailure": "6",
                "hardStopSameToolFailure": "7",
                "hardStopNoProgress": "8",
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["tool_loop_guardrails"]["warnings_enabled"].as_bool(), Some(false));
        assert_eq!(config["tool_loop_guardrails"]["hard_stop_enabled"].as_bool(), Some(true));
        assert_eq!(config["tool_loop_guardrails"]["custom_flag"].as_str(), Some("keep-me"));
        assert_eq!(config["tool_loop_guardrails"]["warn_after"]["exact_failure"].as_i64(), Some(3));
        assert_eq!(config["tool_loop_guardrails"]["warn_after"]["same_tool_failure"].as_i64(), Some(4));
        assert_eq!(config["tool_loop_guardrails"]["warn_after"]["idempotent_no_progress"].as_i64(), Some(5));
        assert_eq!(config["tool_loop_guardrails"]["warn_after"]["custom_warn"].as_i64(), Some(99));
        assert_eq!(config["tool_loop_guardrails"]["hard_stop_after"]["exact_failure"].as_i64(), Some(6));
        assert_eq!(config["tool_loop_guardrails"]["hard_stop_after"]["same_tool_failure"].as_i64(), Some(7));
        assert_eq!(
            config["tool_loop_guardrails"]["hard_stop_after"]["idempotent_no_progress"].as_i64(),
            Some(8)
        );
    }

    #[test]
    fn merge_tool_loop_guardrails_config_rejects_invalid_values() {
        let mut config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let err = merge_hermes_tool_loop_guardrails_config(&mut config, &crate::jv!({ "warnExactFailure": 0 })).unwrap_err();
        assert!(err.contains("tool_loop_guardrails.warn_after.exact_failure"));
        let err = merge_hermes_tool_loop_guardrails_config(&mut config, &crate::jv!({ "warnSameToolFailure": 101 })).unwrap_err();
        assert!(err.contains("tool_loop_guardrails.warn_after.same_tool_failure"));
        let err = merge_hermes_tool_loop_guardrails_config(&mut config, &crate::jv!({ "hardStopExactFailure": 0 })).unwrap_err();
        assert!(err.contains("tool_loop_guardrails.hard_stop_after.exact_failure"));
        let err = merge_hermes_tool_loop_guardrails_config(&mut config, &crate::jv!({ "hardStopNoProgress": 101 })).unwrap_err();
        assert!(err.contains("tool_loop_guardrails.hard_stop_after.idempotent_no_progress"));
    }
}

#[cfg(test)]
mod hermes_streaming_config_tests {
    use super::{build_hermes_streaming_config_values, merge_hermes_streaming_config};

    #[test]
    fn streaming_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_streaming_config_values(&config);
        assert_eq!(values["enabled"], false);
        assert_eq!(values["transport"], "edit");
        assert_eq!(values["editInterval"], 0.8);
        assert_eq!(values["bufferThreshold"], 24);
        assert_eq!(values["cursor"], " ▉");
        assert_eq!(values["freshFinalAfterSeconds"], 60.0);
    }

    #[test]
    fn streaming_values_prefer_top_level_and_fallback_to_gateway() {
        let fallback: serde_yaml::Value = serde_yaml::from_str(
            r#"
gateway:
  streaming:
    enabled: true
    transport: draft
    edit_interval: 0.25
    buffer_threshold: 11
    cursor: "..."
    fresh_final_after_seconds: 0
"#,
        )
        .unwrap();
        let values = build_hermes_streaming_config_values(&fallback);
        assert_eq!(values["enabled"], true);
        assert_eq!(values["transport"], "draft");
        assert_eq!(values["editInterval"], 0.25);
        assert_eq!(values["bufferThreshold"], 11);
        assert_eq!(values["cursor"], "...");
        assert_eq!(values["freshFinalAfterSeconds"], 0.0);

        let top_level: serde_yaml::Value = serde_yaml::from_str(
            r#"
streaming:
  enabled: false
  transport: auto
  edit_interval: 0.5
  buffer_threshold: 40
  cursor: ">"
  fresh_final_after_seconds: 120
gateway:
  streaming:
    enabled: true
    transport: draft
"#,
        )
        .unwrap();
        let values = build_hermes_streaming_config_values(&top_level);
        assert_eq!(values["enabled"], false);
        assert_eq!(values["transport"], "auto");
        assert_eq!(values["editInterval"], 0.5);
        assert_eq!(values["bufferThreshold"], 40);
        assert_eq!(values["cursor"], ">");
        assert_eq!(values["freshFinalAfterSeconds"], 120.0);
    }

    #[test]
    fn merge_streaming_config_preserves_unrelated_yaml() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
streaming:
  enabled: false
  custom_flag: keep-me
gateway:
  streaming:
    enabled: false
    legacy_flag: keep-nested
display:
  streaming: true
"#,
        )
        .unwrap();

        merge_hermes_streaming_config(
            &mut config,
            &crate::jv!({
                "enabled": true,
                "transport": "draft",
                "editInterval": "0.35",
                "bufferThreshold": "48",
                "cursor": "",
                "freshFinalAfterSeconds": "0",
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["display"]["streaming"].as_bool(), Some(true));
        assert_eq!(config["gateway"]["streaming"]["legacy_flag"].as_str(), Some("keep-nested"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["streaming"]["transport"].as_str(), Some("draft"));
        assert_eq!(config["streaming"]["edit_interval"].as_f64(), Some(0.35));
        assert_eq!(config["streaming"]["buffer_threshold"].as_i64(), Some(48));
        assert_eq!(config["streaming"]["cursor"].as_str(), Some(""));
        assert_eq!(config["streaming"]["fresh_final_after_seconds"].as_f64(), Some(0.0));
        assert_eq!(config["streaming"]["custom_flag"].as_str(), Some("keep-me"));
    }

    #[test]
    fn merge_streaming_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_streaming_config(&mut config, &crate::jv!({ "transport": "invalid" })).unwrap_err();
        assert!(err.contains("streaming.transport"));
        let err = merge_hermes_streaming_config(&mut config, &crate::jv!({ "editInterval": 0.01 })).unwrap_err();
        assert!(err.contains("streaming.edit_interval"));
        let err = merge_hermes_streaming_config(&mut config, &crate::jv!({ "bufferThreshold": 0 })).unwrap_err();
        assert!(err.contains("streaming.buffer_threshold"));
        let err = merge_hermes_streaming_config(&mut config, &crate::jv!({ "freshFinalAfterSeconds": -1 })).unwrap_err();
        assert!(err.contains("streaming.fresh_final_after_seconds"));
    }
}

#[cfg(test)]
mod hermes_execution_limits_config_tests {
    use super::{build_hermes_execution_limits_config_values, merge_hermes_execution_limits_config};

    #[test]
    fn execution_limits_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_execution_limits_config_values(&config);
        assert_eq!(values["codeExecutionMode"], "project");
        assert_eq!(values["codeExecutionTimeout"], 300);
        assert_eq!(values["codeExecutionMaxToolCalls"], 50);
        assert_eq!(values["delegationMaxIterations"], 50);
        assert_eq!(values["delegationChildTimeoutSeconds"], 600);
        assert_eq!(values["delegationMaxConcurrentChildren"], 3);
        assert_eq!(values["delegationMaxSpawnDepth"], 1);
        assert_eq!(values["delegationOrchestratorEnabled"], true);
        assert_eq!(values["delegationSubagentAutoApprove"], false);
        assert_eq!(values["delegationInheritMcpToolsets"], true);
        assert_eq!(values["delegationModel"], "");
        assert_eq!(values["delegationProvider"], "");
    }

    #[test]
    fn execution_limits_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
code_execution:
  mode: strict
  timeout: 120
  max_tool_calls: 12
delegation:
  max_iterations: 30
  child_timeout_seconds: 900
  max_concurrent_children: 5
  max_spawn_depth: 2
  orchestrator_enabled: false
  subagent_auto_approve: true
  inherit_mcp_toolsets: false
  model: google/gemini-3-flash-preview
  provider: openrouter
"#,
        )
        .unwrap();
        let values = build_hermes_execution_limits_config_values(&config);
        assert_eq!(values["codeExecutionMode"], "strict");
        assert_eq!(values["codeExecutionTimeout"], 120);
        assert_eq!(values["codeExecutionMaxToolCalls"], 12);
        assert_eq!(values["delegationMaxIterations"], 30);
        assert_eq!(values["delegationChildTimeoutSeconds"], 900);
        assert_eq!(values["delegationMaxConcurrentChildren"], 5);
        assert_eq!(values["delegationMaxSpawnDepth"], 2);
        assert_eq!(values["delegationOrchestratorEnabled"], false);
        assert_eq!(values["delegationSubagentAutoApprove"], true);
        assert_eq!(values["delegationInheritMcpToolsets"], false);
        assert_eq!(values["delegationModel"], "google/gemini-3-flash-preview");
        assert_eq!(values["delegationProvider"], "openrouter");
    }

    #[test]
    fn merge_execution_limits_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
code_execution:
  mode: project
  custom_flag: keep-code
delegation:
  model: child-model
  provider: openrouter
  custom_flag: keep-delegation
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_execution_limits_config(
            &mut config,
            &crate::jv!({
                "codeExecutionMode": "strict",
                "codeExecutionTimeout": "180",
                "codeExecutionMaxToolCalls": "25",
                "delegationMaxIterations": "40",
                "delegationChildTimeoutSeconds": "1200",
                "delegationMaxConcurrentChildren": "4",
                "delegationMaxSpawnDepth": "2",
                "delegationOrchestratorEnabled": false,
                "delegationSubagentAutoApprove": true,
                "delegationInheritMcpToolsets": false,
                "delegationModel": "anthropic/claude-haiku-4.6",
                "delegationProvider": "anthropic",
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["code_execution"]["mode"].as_str(), Some("strict"));
        assert_eq!(config["code_execution"]["timeout"].as_i64(), Some(180));
        assert_eq!(config["code_execution"]["max_tool_calls"].as_i64(), Some(25));
        assert_eq!(config["code_execution"]["custom_flag"].as_str(), Some("keep-code"));
        assert_eq!(config["delegation"]["max_iterations"].as_i64(), Some(40));
        assert_eq!(config["delegation"]["child_timeout_seconds"].as_i64(), Some(1200));
        assert_eq!(config["delegation"]["max_concurrent_children"].as_i64(), Some(4));
        assert_eq!(config["delegation"]["max_spawn_depth"].as_i64(), Some(2));
        assert_eq!(config["delegation"]["orchestrator_enabled"].as_bool(), Some(false));
        assert_eq!(config["delegation"]["subagent_auto_approve"].as_bool(), Some(true));
        assert_eq!(config["delegation"]["inherit_mcp_toolsets"].as_bool(), Some(false));
        assert_eq!(config["delegation"]["model"].as_str(), Some("anthropic/claude-haiku-4.6"));
        assert_eq!(config["delegation"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["delegation"]["custom_flag"].as_str(), Some("keep-delegation"));
    }

    #[test]
    fn merge_execution_limits_config_removes_empty_child_model_overrides() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
delegation:
  model: child-model
  provider: openrouter
  custom_flag: keep-delegation
"#,
        )
        .unwrap();

        merge_hermes_execution_limits_config(
            &mut config,
            &crate::jv!({
                "delegationModel": "  ",
                "delegationProvider": "",
            }),
        )
        .unwrap();

        assert!(config["delegation"]["model"].is_null());
        assert!(config["delegation"]["provider"].is_null());
        assert_eq!(config["delegation"]["custom_flag"].as_str(), Some("keep-delegation"));
    }

    #[test]
    fn merge_execution_limits_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_execution_limits_config(&mut config, &crate::jv!({ "codeExecutionMode": "unsafe" })).unwrap_err();
        assert!(err.contains("code_execution.mode"));
        let err = merge_hermes_execution_limits_config(&mut config, &crate::jv!({ "codeExecutionTimeout": 0 })).unwrap_err();
        assert!(err.contains("code_execution.timeout"));
        let err =
            merge_hermes_execution_limits_config(&mut config, &crate::jv!({ "delegationMaxConcurrentChildren": 0 })).unwrap_err();
        assert!(err.contains("delegation.max_concurrent_children"));
        let err = merge_hermes_execution_limits_config(&mut config, &crate::jv!({ "delegationMaxSpawnDepth": 4 })).unwrap_err();
        assert!(err.contains("delegation.max_spawn_depth"));
        let err =
            merge_hermes_execution_limits_config(&mut config, &crate::jv!({ "delegationChildTimeoutSeconds": 29 })).unwrap_err();
        assert!(err.contains("delegation.child_timeout_seconds"));
    }
}

include!("auxiliary_tool_loop_streaming/browser_tests.rs");