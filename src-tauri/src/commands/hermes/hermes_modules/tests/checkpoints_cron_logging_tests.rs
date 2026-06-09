
#[cfg(test)]
mod hermes_checkpoints_config_tests {
    use super::{build_hermes_checkpoints_config_values, merge_hermes_checkpoints_config};

    #[test]
    fn checkpoints_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_checkpoints_config_values(&config);
        assert_eq!(values["checkpointsEnabled"], false);
        assert_eq!(values["checkpointMaxSnapshots"], 20);
        assert_eq!(values["checkpointMaxTotalSizeMb"], 500);
        assert_eq!(values["checkpointMaxFileSizeMb"], 10);
        assert_eq!(values["checkpointAutoPrune"], true);
        assert_eq!(values["checkpointRetentionDays"], 7);
        assert_eq!(values["checkpointDeleteOrphans"], true);
        assert_eq!(values["checkpointMinIntervalHours"], 24);
    }

    #[test]
    fn checkpoints_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
checkpoints:
  enabled: true
  max_snapshots: 12
  max_total_size_mb: 900
  max_file_size_mb: 25
  auto_prune: false
  retention_days: 14
  delete_orphans: false
  min_interval_hours: 6
"#,
        )
        .unwrap();
        let values = build_hermes_checkpoints_config_values(&config);
        assert_eq!(values["checkpointsEnabled"], true);
        assert_eq!(values["checkpointMaxSnapshots"], 12);
        assert_eq!(values["checkpointMaxTotalSizeMb"], 900);
        assert_eq!(values["checkpointMaxFileSizeMb"], 25);
        assert_eq!(values["checkpointAutoPrune"], false);
        assert_eq!(values["checkpointRetentionDays"], 14);
        assert_eq!(values["checkpointDeleteOrphans"], false);
        assert_eq!(values["checkpointMinIntervalHours"], 6);
    }

    #[test]
    fn merge_checkpoints_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
checkpoints:
  enabled: true
  custom_flag: keep-checkpoints
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_checkpoints_config(
            &mut config,
            &crate::jv!({
                "checkpointsEnabled": false,
                "checkpointMaxSnapshots": "30",
                "checkpointMaxTotalSizeMb": "0",
                "checkpointMaxFileSizeMb": "0",
                "checkpointAutoPrune": true,
                "checkpointRetentionDays": "21",
                "checkpointDeleteOrphans": true,
                "checkpointMinIntervalHours": "12",
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["checkpoints"]["enabled"].as_bool(), Some(false));
        assert_eq!(config["checkpoints"]["max_snapshots"].as_i64(), Some(30));
        assert_eq!(config["checkpoints"]["max_total_size_mb"].as_i64(), Some(0));
        assert_eq!(config["checkpoints"]["max_file_size_mb"].as_i64(), Some(0));
        assert_eq!(config["checkpoints"]["auto_prune"].as_bool(), Some(true));
        assert_eq!(config["checkpoints"]["retention_days"].as_i64(), Some(21));
        assert_eq!(config["checkpoints"]["delete_orphans"].as_bool(), Some(true));
        assert_eq!(config["checkpoints"]["min_interval_hours"].as_i64(), Some(12));
        assert_eq!(config["checkpoints"]["custom_flag"].as_str(), Some("keep-checkpoints"));
    }

    #[test]
    fn merge_checkpoints_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_checkpoints_config(&mut config, &crate::jv!({ "checkpointMaxSnapshots": 0 })).unwrap_err();
        assert!(err.contains("checkpoints.max_snapshots"));
        let err = merge_hermes_checkpoints_config(&mut config, &crate::jv!({ "checkpointMaxTotalSizeMb": -1 })).unwrap_err();
        assert!(err.contains("checkpoints.max_total_size_mb"));
        let err = merge_hermes_checkpoints_config(&mut config, &crate::jv!({ "checkpointMaxFileSizeMb": -1 })).unwrap_err();
        assert!(err.contains("checkpoints.max_file_size_mb"));
        let err = merge_hermes_checkpoints_config(&mut config, &crate::jv!({ "checkpointRetentionDays": 0 })).unwrap_err();
        assert!(err.contains("checkpoints.retention_days"));
        let err = merge_hermes_checkpoints_config(&mut config, &crate::jv!({ "checkpointMinIntervalHours": -1 })).unwrap_err();
        assert!(err.contains("checkpoints.min_interval_hours"));
    }
}

#[cfg(test)]
mod hermes_cron_config_tests {
    use super::{build_hermes_cron_config_values, merge_hermes_cron_config};

    #[test]
    fn cron_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_cron_config_values(&config);
        assert_eq!(values["cronWrapResponse"], true);
        assert_eq!(values["cronMaxParallelJobs"], 0);
    }

    #[test]
    fn cron_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
cron:
  wrap_response: false
  max_parallel_jobs: 4
"#,
        )
        .unwrap();
        let values = build_hermes_cron_config_values(&config);
        assert_eq!(values["cronWrapResponse"], false);
        assert_eq!(values["cronMaxParallelJobs"], 4);
    }

    #[test]
    fn merge_cron_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
cron:
  wrap_response: true
  custom_flag: keep-cron
approvals:
  cron_mode: deny
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_cron_config(
            &mut config,
            &crate::jv!({
                "cronWrapResponse": false,
                "cronMaxParallelJobs": "3",
            }),
        )
        .unwrap();

        assert_eq!(config["approvals"]["cron_mode"].as_str(), Some("deny"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["cron"]["wrap_response"].as_bool(), Some(false));
        assert_eq!(config["cron"]["max_parallel_jobs"].as_i64(), Some(3));
        assert_eq!(config["cron"]["custom_flag"].as_str(), Some("keep-cron"));
    }

    #[test]
    fn merge_cron_config_writes_unbounded_null_and_rejects_invalid_values() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
cron:
  max_parallel_jobs: 8
"#,
        )
        .unwrap();

        merge_hermes_cron_config(
            &mut config,
            &crate::jv!({
                "cronMaxParallelJobs": "0",
            }),
        )
        .unwrap();
        assert_eq!(config["cron"]["max_parallel_jobs"], serde_yaml::Value::Null);

        let err = merge_hermes_cron_config(&mut config, &crate::jv!({ "cronMaxParallelJobs": -1 })).unwrap_err();
        assert!(err.contains("cron.max_parallel_jobs"));
        let err = merge_hermes_cron_config(&mut config, &crate::jv!({ "cronMaxParallelJobs": 10001 })).unwrap_err();
        assert!(err.contains("cron.max_parallel_jobs"));
    }
}

#[cfg(test)]
mod hermes_sessions_maintenance_config_tests {
    use super::{build_hermes_sessions_maintenance_config_values, merge_hermes_sessions_maintenance_config};

    #[test]
    fn sessions_maintenance_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_sessions_maintenance_config_values(&config);
        assert_eq!(values["sessionsAutoPrune"], false);
        assert_eq!(values["sessionsRetentionDays"], 90);
        assert_eq!(values["sessionsVacuumAfterPrune"], true);
        assert_eq!(values["sessionsMinIntervalHours"], 24);
        assert_eq!(values["sessionsWriteJsonSnapshots"], false);
    }

    #[test]
    fn sessions_maintenance_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
sessions:
  auto_prune: true
  retention_days: 14
  vacuum_after_prune: false
  min_interval_hours: 6
  write_json_snapshots: true
"#,
        )
        .unwrap();
        let values = build_hermes_sessions_maintenance_config_values(&config);
        assert_eq!(values["sessionsAutoPrune"], true);
        assert_eq!(values["sessionsRetentionDays"], 14);
        assert_eq!(values["sessionsVacuumAfterPrune"], false);
        assert_eq!(values["sessionsMinIntervalHours"], 6);
        assert_eq!(values["sessionsWriteJsonSnapshots"], true);
    }

    #[test]
    fn merge_sessions_maintenance_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
sessions:
  auto_prune: false
  custom_flag: keep-sessions
model:
  provider: anthropic
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_sessions_maintenance_config(
            &mut config,
            &crate::jv!({
                "sessionsAutoPrune": true,
                "sessionsRetentionDays": "30",
                "sessionsVacuumAfterPrune": false,
                "sessionsMinIntervalHours": "12",
                "sessionsWriteJsonSnapshots": true,
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["sessions"]["auto_prune"].as_bool(), Some(true));
        assert_eq!(config["sessions"]["retention_days"].as_i64(), Some(30));
        assert_eq!(config["sessions"]["vacuum_after_prune"].as_bool(), Some(false));
        assert_eq!(config["sessions"]["min_interval_hours"].as_i64(), Some(12));
        assert_eq!(config["sessions"]["write_json_snapshots"].as_bool(), Some(true));
        assert_eq!(config["sessions"]["custom_flag"].as_str(), Some("keep-sessions"));
    }

    #[test]
    fn merge_sessions_maintenance_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_sessions_maintenance_config(&mut config, &crate::jv!({ "sessionsRetentionDays": 0 })).unwrap_err();
        assert!(err.contains("sessions.retention_days"));
        let err =
            merge_hermes_sessions_maintenance_config(&mut config, &crate::jv!({ "sessionsRetentionDays": 36501 })).unwrap_err();
        assert!(err.contains("sessions.retention_days"));
        let err =
            merge_hermes_sessions_maintenance_config(&mut config, &crate::jv!({ "sessionsMinIntervalHours": -1 })).unwrap_err();
        assert!(err.contains("sessions.min_interval_hours"));
        let err = merge_hermes_sessions_maintenance_config(&mut config, &crate::jv!({ "sessionsMinIntervalHours": 87601 }))
            .unwrap_err();
        assert!(err.contains("sessions.min_interval_hours"));
    }
}

#[cfg(test)]
mod hermes_updates_config_tests {
    use super::{build_hermes_updates_config_values, merge_hermes_updates_config};

    #[test]
    fn updates_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_updates_config_values(&config);
        assert_eq!(values["updatesPreUpdateBackup"], false);
        assert_eq!(values["updatesBackupKeep"], 5);
    }

    #[test]
    fn updates_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
updates:
  pre_update_backup: true
  backup_keep: 9
"#,
        )
        .unwrap();
        let values = build_hermes_updates_config_values(&config);
        assert_eq!(values["updatesPreUpdateBackup"], true);
        assert_eq!(values["updatesBackupKeep"], 9);
    }

    #[test]
    fn merge_updates_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
updates:
  pre_update_backup: false
  custom_flag: keep-updates
sessions:
  auto_prune: true
model:
  provider: anthropic
"#,
        )
        .unwrap();

        merge_hermes_updates_config(
            &mut config,
            &crate::jv!({
                "updatesPreUpdateBackup": true,
                "updatesBackupKeep": "7",
            }),
        )
        .unwrap();

        assert_eq!(config["sessions"]["auto_prune"].as_bool(), Some(true));
        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["updates"]["pre_update_backup"].as_bool(), Some(true));
        assert_eq!(config["updates"]["backup_keep"].as_i64(), Some(7));
        assert_eq!(config["updates"]["custom_flag"].as_str(), Some("keep-updates"));
    }

    #[test]
    fn merge_updates_config_rejects_invalid_backup_keep() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_updates_config(&mut config, &crate::jv!({ "updatesBackupKeep": 0 })).unwrap_err();
        assert!(err.contains("updates.backup_keep"));
        let err = merge_hermes_updates_config(&mut config, &crate::jv!({ "updatesBackupKeep": 1001 })).unwrap_err();
        assert!(err.contains("updates.backup_keep"));
    }
}

#[cfg(test)]
mod hermes_logging_config_tests {
    use super::{build_hermes_logging_config_values, merge_hermes_logging_config};

    #[test]
    fn logging_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_logging_config_values(&config);
        assert_eq!(values["loggingLevel"], "INFO");
        assert_eq!(values["loggingMaxSizeMb"], 5);
        assert_eq!(values["loggingBackupCount"], 3);
        assert_eq!(values["loggingMemoryMonitorEnabled"], true);
        assert_eq!(values["loggingMemoryMonitorIntervalSeconds"], 300);
    }

    #[test]
    fn logging_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
logging:
  level: DEBUG
  max_size_mb: 12
  backup_count: 7
  memory_monitor:
    enabled: false
    interval_seconds: 120
"#,
        )
        .unwrap();
        let values = build_hermes_logging_config_values(&config);
        assert_eq!(values["loggingLevel"], "DEBUG");
        assert_eq!(values["loggingMaxSizeMb"], 12);
        assert_eq!(values["loggingBackupCount"], 7);
        assert_eq!(values["loggingMemoryMonitorEnabled"], false);
        assert_eq!(values["loggingMemoryMonitorIntervalSeconds"], 120);
    }

    #[test]
    fn merge_logging_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
logging:
  level: INFO
  custom_flag: keep-logging
  memory_monitor:
    custom_flag: keep-memory-monitor
cron:
  wrap_response: true
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_logging_config(
            &mut config,
            &crate::jv!({
                "loggingLevel": "WARNING",
                "loggingMaxSizeMb": "20",
                "loggingBackupCount": "5",
                "loggingMemoryMonitorEnabled": true,
                "loggingMemoryMonitorIntervalSeconds": "180",
            }),
        )
        .unwrap();

        assert_eq!(config["cron"]["wrap_response"].as_bool(), Some(true));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["logging"]["level"].as_str(), Some("WARNING"));
        assert_eq!(config["logging"]["max_size_mb"].as_i64(), Some(20));
        assert_eq!(config["logging"]["backup_count"].as_i64(), Some(5));
        assert_eq!(config["logging"]["memory_monitor"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["logging"]["memory_monitor"]["interval_seconds"].as_i64(), Some(180));
        assert_eq!(config["logging"]["custom_flag"].as_str(), Some("keep-logging"));
        assert_eq!(config["logging"]["memory_monitor"]["custom_flag"].as_str(), Some("keep-memory-monitor"));
    }

    #[test]
    fn merge_logging_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_logging_config(&mut config, &crate::jv!({ "loggingLevel": "TRACE" })).unwrap_err();
        assert!(err.contains("logging.level"));
        let err = merge_hermes_logging_config(&mut config, &crate::jv!({ "loggingMaxSizeMb": 0 })).unwrap_err();
        assert!(err.contains("logging.max_size_mb"));
        let err = merge_hermes_logging_config(&mut config, &crate::jv!({ "loggingBackupCount": -1 })).unwrap_err();
        assert!(err.contains("logging.backup_count"));
        let err =
            merge_hermes_logging_config(&mut config, &crate::jv!({ "loggingMemoryMonitorIntervalSeconds": 0 })).unwrap_err();
        assert!(err.contains("logging.memory_monitor.interval_seconds"));
    }
}

#[cfg(test)]
mod hermes_approvals_config_tests {
    use super::{build_hermes_approvals_config_values, merge_hermes_approvals_config};

    #[test]
    fn approvals_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_approvals_config_values(&config);
        assert_eq!(values["approvalMode"], "manual");
        assert_eq!(values["approvalTimeout"], 60);
        assert_eq!(values["approvalCronMode"], "deny");
        assert_eq!(values["approvalMcpReloadConfirm"], true);
        assert_eq!(values["approvalDestructiveSlashConfirm"], true);
    }

    #[test]
    fn approvals_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
approvals:
  mode: smart
  timeout: 120
  cron_mode: approve
  mcp_reload_confirm: false
  destructive_slash_confirm: false
"#,
        )
        .unwrap();
        let values = build_hermes_approvals_config_values(&config);
        assert_eq!(values["approvalMode"], "smart");
        assert_eq!(values["approvalTimeout"], 120);
        assert_eq!(values["approvalCronMode"], "approve");
        assert_eq!(values["approvalMcpReloadConfirm"], false);
        assert_eq!(values["approvalDestructiveSlashConfirm"], false);
    }

    #[test]
    fn merge_approvals_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
approvals:
  mode: manual
  custom_flag: keep-approvals
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_approvals_config(
            &mut config,
            &crate::jv!({
                "approvalMode": "off",
                "approvalTimeout": "15",
                "approvalCronMode": "approve",
                "approvalMcpReloadConfirm": false,
                "approvalDestructiveSlashConfirm": false,
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["approvals"]["mode"].as_str(), Some("off"));
        assert_eq!(config["approvals"]["timeout"].as_i64(), Some(15));
        assert_eq!(config["approvals"]["cron_mode"].as_str(), Some("approve"));
        assert_eq!(config["approvals"]["mcp_reload_confirm"].as_bool(), Some(false));
        assert_eq!(config["approvals"]["destructive_slash_confirm"].as_bool(), Some(false));
        assert_eq!(config["approvals"]["custom_flag"].as_str(), Some("keep-approvals"));
    }

    #[test]
    fn merge_approvals_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_approvals_config(&mut config, &crate::jv!({ "approvalMode": "always" })).unwrap_err();
        assert!(err.contains("approvals.mode"));
        let err = merge_hermes_approvals_config(&mut config, &crate::jv!({ "approvalCronMode": "prompt" })).unwrap_err();
        assert!(err.contains("approvals.cron_mode"));
        let err = merge_hermes_approvals_config(&mut config, &crate::jv!({ "approvalTimeout": 0 })).unwrap_err();
        assert!(err.contains("approvals.timeout"));
        let err = merge_hermes_approvals_config(&mut config, &crate::jv!({ "approvalTimeout": 86401 })).unwrap_err();
        assert!(err.contains("approvals.timeout"));
    }
}

include!("checkpoints_cron_logging/terminal_tests.rs");