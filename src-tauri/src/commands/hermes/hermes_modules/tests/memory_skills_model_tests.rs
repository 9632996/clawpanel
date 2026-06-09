
#[cfg(test)]
mod hermes_memory_config_tests {
    use super::{build_hermes_memory_config_values, merge_hermes_memory_config};

    #[test]
    fn memory_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_memory_config_values(&config);
        assert_eq!(values["memoryEnabled"], true);
        assert_eq!(values["userProfileEnabled"], true);
        assert_eq!(values["memoryCharLimit"], 2200);
        assert_eq!(values["userCharLimit"], 1375);
        assert_eq!(values["nudgeInterval"], 10);
        assert_eq!(values["flushMinTurns"], 6);
    }

    #[test]
    fn merge_memory_config_preserves_unrelated_yaml() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
memory:
  memory_enabled: true
  provider: honcho
  custom_flag: keep-me
  flush_min_turns: 9
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_memory_config(
            &mut config,
            &crate::jv!({
                "memoryEnabled": false,
                "userProfileEnabled": false,
                "memoryCharLimit": "2600",
                "userCharLimit": "1500",
                "nudgeInterval": "0",
                "flushMinTurns": "7",
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["memory"]["memory_enabled"].as_bool(), Some(false));
        assert_eq!(config["memory"]["user_profile_enabled"].as_bool(), Some(false));
        assert_eq!(config["memory"]["memory_char_limit"].as_i64(), Some(2600));
        assert_eq!(config["memory"]["user_char_limit"].as_i64(), Some(1500));
        assert_eq!(config["memory"]["nudge_interval"].as_i64(), Some(0));
        assert_eq!(config["memory"]["flush_min_turns"].as_i64(), Some(7));
        assert_eq!(config["memory"]["provider"].as_str(), Some("honcho"));
        assert_eq!(config["memory"]["custom_flag"].as_str(), Some("keep-me"));
    }

    #[test]
    fn merge_memory_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_memory_config(&mut config, &crate::jv!({ "memoryCharLimit": 99 })).unwrap_err();
        assert!(err.contains("memory.memory_char_limit"));
        let err = merge_hermes_memory_config(&mut config, &crate::jv!({ "userCharLimit": 200001 })).unwrap_err();
        assert!(err.contains("memory.user_char_limit"));
        let err = merge_hermes_memory_config(&mut config, &crate::jv!({ "nudgeInterval": -1 })).unwrap_err();
        assert!(err.contains("memory.nudge_interval"));
        let err = merge_hermes_memory_config(&mut config, &crate::jv!({ "nudgeInterval": 1001 })).unwrap_err();
        assert!(err.contains("memory.nudge_interval"));
        let err = merge_hermes_memory_config(&mut config, &crate::jv!({ "flushMinTurns": -1 })).unwrap_err();
        assert!(err.contains("memory.flush_min_turns"));
        let err = merge_hermes_memory_config(&mut config, &crate::jv!({ "flushMinTurns": 1001 })).unwrap_err();
        assert!(err.contains("memory.flush_min_turns"));
    }
}

#[cfg(test)]
mod hermes_skills_config_tests {
    use super::{build_hermes_skills_config_values, merge_hermes_skills_config};

    #[test]
    fn skills_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_skills_config_values(&config);
        assert_eq!(values["creationNudgeInterval"], 15);
        assert_eq!(values["externalDirs"], "");
        assert_eq!(values["templateVars"], true);
        assert_eq!(values["inlineShell"], false);
        assert_eq!(values["inlineShellTimeout"], 10);
        assert_eq!(values["guardAgentCreated"], false);
    }

    #[test]
    fn skills_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
skills:
  creation_nudge_interval: 30
  external_dirs:
    - ~/.agents/skills
    - /home/shared/team-skills
  template_vars: false
  inline_shell: true
  inline_shell_timeout: 25
  guard_agent_created: true
"#,
        )
        .unwrap();

        let values = build_hermes_skills_config_values(&config);
        assert_eq!(values["creationNudgeInterval"], 30);
        assert_eq!(values["externalDirs"], "~/.agents/skills\n/home/shared/team-skills");
        assert_eq!(values["templateVars"], false);
        assert_eq!(values["inlineShell"], true);
        assert_eq!(values["inlineShellTimeout"], 25);
        assert_eq!(values["guardAgentCreated"], true);
    }

    #[test]
    fn merge_skills_config_preserves_unrelated_yaml() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
skills:
  creation_nudge_interval: 15
  disabled:
    - legacy-skill
  custom_flag: keep-skills
memory:
  memory_enabled: true
"#,
        )
        .unwrap();

        merge_hermes_skills_config(
            &mut config,
            &crate::jv!({
                "creationNudgeInterval": "0",
                "externalDirs": " ~/.agents/skills \n\n /home/shared/team-skills ",
                "templateVars": false,
                "inlineShell": true,
                "inlineShellTimeout": "30",
                "guardAgentCreated": true,
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["memory"]["memory_enabled"].as_bool(), Some(true));
        assert_eq!(config["skills"]["creation_nudge_interval"].as_i64(), Some(0));
        assert_eq!(config["skills"]["external_dirs"][0].as_str(), Some("~/.agents/skills"));
        assert_eq!(config["skills"]["external_dirs"][1].as_str(), Some("/home/shared/team-skills"));
        assert_eq!(config["skills"]["template_vars"].as_bool(), Some(false));
        assert_eq!(config["skills"]["inline_shell"].as_bool(), Some(true));
        assert_eq!(config["skills"]["inline_shell_timeout"].as_i64(), Some(30));
        assert_eq!(config["skills"]["guard_agent_created"].as_bool(), Some(true));
        assert_eq!(config["skills"]["disabled"][0].as_str(), Some("legacy-skill"));
        assert_eq!(config["skills"]["custom_flag"].as_str(), Some("keep-skills"));
    }

    #[test]
    fn merge_skills_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_skills_config(&mut config, &crate::jv!({ "creationNudgeInterval": -1 })).unwrap_err();
        assert!(err.contains("skills.creation_nudge_interval"));
        let err = merge_hermes_skills_config(&mut config, &crate::jv!({ "creationNudgeInterval": 10001 })).unwrap_err();
        assert!(err.contains("skills.creation_nudge_interval"));
        let err = merge_hermes_skills_config(&mut config, &crate::jv!({ "inlineShellTimeout": 0 })).unwrap_err();
        assert!(err.contains("skills.inline_shell_timeout"));
        let err = merge_hermes_skills_config(&mut config, &crate::jv!({ "inlineShellTimeout": 86401 })).unwrap_err();
        assert!(err.contains("skills.inline_shell_timeout"));
    }
}

#[cfg(test)]
mod hermes_curator_config_tests {
    use super::{build_hermes_curator_config_values, merge_hermes_curator_config};

    #[test]
    fn curator_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_curator_config_values(&config);
        assert_eq!(values["curatorEnabled"], true);
        assert_eq!(values["curatorIntervalHours"], 168);
        assert_eq!(values["curatorMinIdleHours"], 2);
        assert_eq!(values["curatorStaleAfterDays"], 30);
        assert_eq!(values["curatorArchiveAfterDays"], 90);
        assert_eq!(values["curatorBackupEnabled"], true);
        assert_eq!(values["curatorBackupKeep"], 5);
    }

    #[test]
    fn curator_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
curator:
  enabled: false
  interval_hours: 24
  min_idle_hours: 6
  stale_after_days: 14
  archive_after_days: 45
  backup:
    enabled: false
    keep: 9
"#,
        )
        .unwrap();

        let values = build_hermes_curator_config_values(&config);
        assert_eq!(values["curatorEnabled"], false);
        assert_eq!(values["curatorIntervalHours"], 24);
        assert_eq!(values["curatorMinIdleHours"], 6);
        assert_eq!(values["curatorStaleAfterDays"], 14);
        assert_eq!(values["curatorArchiveAfterDays"], 45);
        assert_eq!(values["curatorBackupEnabled"], false);
        assert_eq!(values["curatorBackupKeep"], 9);
    }

    #[test]
    fn merge_curator_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
curator:
  enabled: true
  backup:
    enabled: true
    custom_flag: keep-backup
  custom_flag: keep-curator
skills:
  external_dirs:
    - ~/.agents/skills
model:
  provider: anthropic
"#,
        )
        .unwrap();

        merge_hermes_curator_config(
            &mut config,
            &crate::jv!({
                "curatorEnabled": false,
                "curatorIntervalHours": "48",
                "curatorMinIdleHours": "4",
                "curatorStaleAfterDays": "21",
                "curatorArchiveAfterDays": "60",
                "curatorBackupEnabled": false,
                "curatorBackupKeep": "3",
            }),
        )
        .unwrap();

        assert_eq!(config["skills"]["external_dirs"][0].as_str(), Some("~/.agents/skills"));
        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["curator"]["enabled"].as_bool(), Some(false));
        assert_eq!(config["curator"]["interval_hours"].as_i64(), Some(48));
        assert_eq!(config["curator"]["min_idle_hours"].as_i64(), Some(4));
        assert_eq!(config["curator"]["stale_after_days"].as_i64(), Some(21));
        assert_eq!(config["curator"]["archive_after_days"].as_i64(), Some(60));
        assert_eq!(config["curator"]["backup"]["enabled"].as_bool(), Some(false));
        assert_eq!(config["curator"]["backup"]["keep"].as_i64(), Some(3));
        assert_eq!(config["curator"]["backup"]["custom_flag"].as_str(), Some("keep-backup"));
        assert_eq!(config["curator"]["custom_flag"].as_str(), Some("keep-curator"));
    }

    #[test]
    fn merge_curator_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_curator_config(&mut config, &crate::jv!({ "curatorIntervalHours": 0 })).unwrap_err();
        assert!(err.contains("curator.interval_hours"));
        let err = merge_hermes_curator_config(&mut config, &crate::jv!({ "curatorMinIdleHours": -1 })).unwrap_err();
        assert!(err.contains("curator.min_idle_hours"));
        let err = merge_hermes_curator_config(&mut config, &crate::jv!({ "curatorBackupKeep": 1001 })).unwrap_err();
        assert!(err.contains("curator.backup.keep"));
        let err = merge_hermes_curator_config(
            &mut config,
            &crate::jv!({
                "curatorStaleAfterDays": 90,
                "curatorArchiveAfterDays": 30,
            }),
        )
        .unwrap_err();
        assert!(err.contains("curator.archive_after_days"));
    }
}

#[cfg(test)]
mod hermes_quick_commands_config_tests {
    use super::{build_hermes_quick_commands_config_values, merge_hermes_quick_commands_config};

    #[test]
    fn quick_commands_values_have_empty_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_quick_commands_config_values(&config);
        assert_eq!(values["quickCommandsJson"], "{}");
    }

    #[test]
    fn quick_commands_values_read_yaml_mapping() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
quick_commands:
  status:
    type: exec
    command: systemctl status hermes-agent
  restart:
    type: alias
    target: /gateway restart
"#,
        )
        .unwrap();

        let values = build_hermes_quick_commands_config_values(&config);
        let parsed: serde_json::Value = serde_json::from_str(values["quickCommandsJson"].as_str().unwrap()).unwrap();
        assert_eq!(parsed["status"]["command"], "systemctl status hermes-agent");
        assert_eq!(parsed["restart"]["target"], "/gateway restart");
    }

    #[test]
    fn merge_quick_commands_config_preserves_unrelated_yaml() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
quick_commands:
  old:
    type: exec
    command: uptime
memory:
  memory_enabled: true
"#,
        )
        .unwrap();

        merge_hermes_quick_commands_config(
            &mut config,
            &crate::jv!({
                "quickCommandsJson": r#"{
                  "status": { "type": "exec", "command": "systemctl status hermes-agent", "timeout": 10 },
                  "restart": { "type": "alias", "target": "/gateway restart" }
                }"#,
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["memory"]["memory_enabled"].as_bool(), Some(true));
        assert_eq!(
            config["quick_commands"]["status"]["command"].as_str(),
            Some("systemctl status hermes-agent")
        );
        assert_eq!(config["quick_commands"]["status"]["timeout"].as_i64(), Some(10));
        assert_eq!(config["quick_commands"]["restart"]["target"].as_str(), Some("/gateway restart"));
        assert!(config["quick_commands"]["old"].is_null());
    }

    #[test]
    fn merge_quick_commands_config_removes_empty_mapping() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
quick_commands:
  status:
    type: exec
    command: uptime
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_quick_commands_config(&mut config, &crate::jv!({ "quickCommandsJson": "{}" })).unwrap();

        assert!(config["quick_commands"].is_null());
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
    }

    #[test]
    fn merge_quick_commands_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_quick_commands_config(&mut config, &crate::jv!({ "quickCommandsJson": "[" })).unwrap_err();
        assert!(err.contains("quick_commands"));
        let err = merge_hermes_quick_commands_config(&mut config, &crate::jv!({ "quickCommandsJson": "[]" })).unwrap_err();
        assert!(err.contains("quick_commands"));
        let err = merge_hermes_quick_commands_config(&mut config, &crate::jv!({ "quickCommandsJson": r#"{ "bad": "uptime" }"# }))
            .unwrap_err();
        assert!(err.contains("quick_commands.bad"));
        let err = merge_hermes_quick_commands_config(
            &mut config,
            &crate::jv!({ "quickCommandsJson": r#"{ "status": { "type": "exec", "command": "" } }"# }),
        )
        .unwrap_err();
        assert!(err.contains("quick_commands.status.command"));
        let err = merge_hermes_quick_commands_config(
            &mut config,
            &crate::jv!({ "quickCommandsJson": r#"{ "restart": { "type": "alias", "target": "gateway restart" } }"# }),
        )
        .unwrap_err();
        assert!(err.contains("quick_commands.restart.target"));
    }
}

include!("memory_skills_model/model_tests.rs");