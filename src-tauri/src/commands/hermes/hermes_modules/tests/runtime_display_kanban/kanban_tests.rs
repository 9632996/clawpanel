#[cfg(test)]
mod hermes_kanban_config_tests {
    use super::{build_hermes_kanban_config_values, merge_hermes_kanban_config};

    #[test]
    fn kanban_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_kanban_config_values(&config);
        assert_eq!(values["dispatchInGateway"], true);
        assert_eq!(values["dispatchIntervalSeconds"], 60);
        assert_eq!(values["maxSpawn"], 0);
        assert_eq!(values["maxInProgress"], 0);
        assert_eq!(values["failureLimit"], 2);
        assert_eq!(values["autoDecompose"], true);
        assert_eq!(values["autoDecomposePerTick"], 3);
        assert_eq!(values["workerLogRotateBytes"], 2097152);
        assert_eq!(values["workerLogBackupCount"], 1);
        assert_eq!(values["orchestratorProfile"], "");
        assert_eq!(values["defaultAssignee"], "");
        assert_eq!(values["dispatchStaleTimeoutSeconds"], 14400);
    }

    #[test]
    fn kanban_values_normalize_existing_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
kanban:
  dispatch_in_gateway: false
  dispatch_interval_seconds: "120"
  max_spawn: "4"
  max_in_progress: "6"
  failure_limit: "5"
  auto_decompose: false
  auto_decompose_per_tick: "7"
  worker_log_rotate_bytes: "4194304"
  worker_log_backup_count: "3"
  orchestrator_profile: triage
  default_assignee: builder
  dispatch_stale_timeout_seconds: "7200"
"#,
        )
        .unwrap();
        let values = build_hermes_kanban_config_values(&config);
        assert_eq!(values["dispatchInGateway"], false);
        assert_eq!(values["dispatchIntervalSeconds"], 120);
        assert_eq!(values["maxSpawn"], 4);
        assert_eq!(values["maxInProgress"], 6);
        assert_eq!(values["failureLimit"], 5);
        assert_eq!(values["autoDecompose"], false);
        assert_eq!(values["autoDecomposePerTick"], 7);
        assert_eq!(values["workerLogRotateBytes"], 4194304);
        assert_eq!(values["workerLogBackupCount"], 3);
        assert_eq!(values["orchestratorProfile"], "triage");
        assert_eq!(values["defaultAssignee"], "builder");
        assert_eq!(values["dispatchStaleTimeoutSeconds"], 7200);
    }

    #[test]
    fn merge_kanban_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
kanban:
  dispatch_interval_seconds: 30
  max_spawn: 9
  max_in_progress: 11
  custom_flag: keep-me
memory:
  memory_enabled: true
"#,
        )
        .unwrap();

        merge_hermes_kanban_config(
            &mut config,
            &crate::jv!({
                "dispatchInGateway": false,
                "dispatchIntervalSeconds": 15,
                "maxSpawn": 4,
                "maxInProgress": 6,
                "failureLimit": 4,
                "autoDecompose": false,
                "autoDecomposePerTick": 2,
                "workerLogRotateBytes": 1048576,
                "workerLogBackupCount": 0,
                "orchestratorProfile": "triage",
                "defaultAssignee": "builder",
                "dispatchStaleTimeoutSeconds": 0,
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["memory"]["memory_enabled"].as_bool(), Some(true));
        assert_eq!(config["kanban"]["custom_flag"].as_str(), Some("keep-me"));
        assert_eq!(config["kanban"]["dispatch_in_gateway"].as_bool(), Some(false));
        assert_eq!(config["kanban"]["dispatch_interval_seconds"].as_i64(), Some(15));
        assert_eq!(config["kanban"]["max_spawn"].as_i64(), Some(4));
        assert_eq!(config["kanban"]["max_in_progress"].as_i64(), Some(6));
        assert_eq!(config["kanban"]["failure_limit"].as_i64(), Some(4));
        assert_eq!(config["kanban"]["auto_decompose"].as_bool(), Some(false));
        assert_eq!(config["kanban"]["auto_decompose_per_tick"].as_i64(), Some(2));
        assert_eq!(config["kanban"]["worker_log_rotate_bytes"].as_i64(), Some(1048576));
        assert_eq!(config["kanban"]["worker_log_backup_count"].as_i64(), Some(0));
        assert_eq!(config["kanban"]["orchestrator_profile"].as_str(), Some("triage"));
        assert_eq!(config["kanban"]["default_assignee"].as_str(), Some("builder"));
        assert_eq!(config["kanban"]["dispatch_stale_timeout_seconds"].as_i64(), Some(0));
    }

    #[test]
    fn merge_kanban_config_removes_optional_profile_routes() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
kanban:
  orchestrator_profile: triage
  default_assignee: builder
  custom_flag: keep-me
"#,
        )
        .unwrap();

        merge_hermes_kanban_config(
            &mut config,
            &crate::jv!({
                "orchestratorProfile": "   ",
                "defaultAssignee": "",
            }),
        )
        .unwrap();

        assert_eq!(config["kanban"]["custom_flag"].as_str(), Some("keep-me"));
        assert!(config["kanban"].get("orchestrator_profile").is_none());
        assert!(config["kanban"].get("default_assignee").is_none());
    }

    #[test]
    fn merge_kanban_config_removes_optional_concurrency_limits() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
kanban:
  max_spawn: 4
  max_in_progress: 6
  custom_flag: keep-me
"#,
        )
        .unwrap();

        merge_hermes_kanban_config(
            &mut config,
            &crate::jv!({
                "maxSpawn": 0,
                "maxInProgress": 0,
            }),
        )
        .unwrap();

        assert_eq!(config["kanban"]["custom_flag"].as_str(), Some("keep-me"));
        assert!(config["kanban"].get("max_spawn").is_none());
        assert!(config["kanban"].get("max_in_progress").is_none());
    }

    #[test]
    fn merge_kanban_config_rejects_invalid_timeout() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_kanban_config(&mut config, &crate::jv!({ "dispatchIntervalSeconds": 0 })).unwrap_err();
        assert!(err.contains("kanban.dispatch_interval_seconds"));

        let err = merge_hermes_kanban_config(&mut config, &crate::jv!({ "maxSpawn": -1 })).unwrap_err();
        assert!(err.contains("kanban.max_spawn"));

        let err = merge_hermes_kanban_config(&mut config, &crate::jv!({ "maxInProgress": -1 })).unwrap_err();
        assert!(err.contains("kanban.max_in_progress"));

        let err = merge_hermes_kanban_config(&mut config, &crate::jv!({ "failureLimit": 0 })).unwrap_err();
        assert!(err.contains("kanban.failure_limit"));

        let err = merge_hermes_kanban_config(&mut config, &crate::jv!({ "autoDecomposePerTick": 0 })).unwrap_err();
        assert!(err.contains("kanban.auto_decompose_per_tick"));

        let err = merge_hermes_kanban_config(&mut config, &crate::jv!({ "workerLogRotateBytes": 0 })).unwrap_err();
        assert!(err.contains("kanban.worker_log_rotate_bytes"));

        let err = merge_hermes_kanban_config(&mut config, &crate::jv!({ "workerLogBackupCount": -1 })).unwrap_err();
        assert!(err.contains("kanban.worker_log_backup_count"));

        let err = merge_hermes_kanban_config(&mut config, &crate::jv!({ "orchestratorProfile": 123 })).unwrap_err();
        assert!(err.contains("kanban.orchestrator_profile"));

        let err = merge_hermes_kanban_config(&mut config, &crate::jv!({ "defaultAssignee": false })).unwrap_err();
        assert!(err.contains("kanban.default_assignee"));

        let err = merge_hermes_kanban_config(&mut config, &crate::jv!({ "dispatchStaleTimeoutSeconds": -1 })).unwrap_err();
        assert!(err.contains("kanban.dispatch_stale_timeout_seconds"));

        let err = merge_hermes_kanban_config(&mut config, &crate::jv!({ "dispatchStaleTimeoutSeconds": 604801 })).unwrap_err();
        assert!(err.contains("kanban.dispatch_stale_timeout_seconds"));
    }
}