#[cfg(test)]
mod hermes_io_safety_config_tests {
    use super::{build_hermes_io_safety_config_values, merge_hermes_io_safety_config};

    #[test]
    fn io_safety_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_io_safety_config_values(&config);
        assert_eq!(values["fileReadMaxChars"], 100000);
        assert_eq!(values["toolOutputMaxBytes"], 50000);
        assert_eq!(values["toolOutputMaxLines"], 2000);
        assert_eq!(values["toolOutputMaxLineLength"], 2000);
    }

    #[test]
    fn io_safety_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
file_read_max_chars: 200000
tool_output:
  max_bytes: 150000
  max_lines: 5000
  max_line_length: 4000
"#,
        )
        .unwrap();
        let values = build_hermes_io_safety_config_values(&config);
        assert_eq!(values["fileReadMaxChars"], 200000);
        assert_eq!(values["toolOutputMaxBytes"], 150000);
        assert_eq!(values["toolOutputMaxLines"], 5000);
        assert_eq!(values["toolOutputMaxLineLength"], 4000);
    }

    #[test]
    fn merge_io_safety_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
file_read_max_chars: 100000
tool_output:
  max_bytes: 50000
  custom_flag: keep-output
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_io_safety_config(
            &mut config,
            &crate::jv!({
                "fileReadMaxChars": "120000",
                "toolOutputMaxBytes": "80000",
                "toolOutputMaxLines": "3000",
                "toolOutputMaxLineLength": "2500",
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["file_read_max_chars"].as_i64(), Some(120000));
        assert_eq!(config["tool_output"]["max_bytes"].as_i64(), Some(80000));
        assert_eq!(config["tool_output"]["max_lines"].as_i64(), Some(3000));
        assert_eq!(config["tool_output"]["max_line_length"].as_i64(), Some(2500));
        assert_eq!(config["tool_output"]["custom_flag"].as_str(), Some("keep-output"));
    }

    #[test]
    fn merge_io_safety_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_io_safety_config(&mut config, &crate::jv!({ "fileReadMaxChars": 999 })).unwrap_err();
        assert!(err.contains("file_read_max_chars"));
        let err = merge_hermes_io_safety_config(&mut config, &crate::jv!({ "toolOutputMaxBytes": 999 })).unwrap_err();
        assert!(err.contains("tool_output.max_bytes"));
        let err = merge_hermes_io_safety_config(&mut config, &crate::jv!({ "toolOutputMaxLines": 0 })).unwrap_err();
        assert!(err.contains("tool_output.max_lines"));
        let err = merge_hermes_io_safety_config(&mut config, &crate::jv!({ "toolOutputMaxLineLength": 0 })).unwrap_err();
        assert!(err.contains("tool_output.max_line_length"));
    }
}

#[cfg(test)]
mod hermes_privacy_config_tests {
    use super::{build_hermes_privacy_config_values, merge_hermes_privacy_config};

    #[test]
    fn privacy_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_privacy_config_values(&config);
        assert_eq!(values["redactPii"], false);
    }

    #[test]
    fn privacy_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
privacy:
  redact_pii: true
"#,
        )
        .unwrap();
        let values = build_hermes_privacy_config_values(&config);
        assert_eq!(values["redactPii"], true);
    }

    #[test]
    fn merge_privacy_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
privacy:
  redact_pii: false
  custom_flag: keep-privacy
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_privacy_config(
            &mut config,
            &crate::jv!({
                "redactPii": true,
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["privacy"]["redact_pii"].as_bool(), Some(true));
        assert_eq!(config["privacy"]["custom_flag"].as_str(), Some("keep-privacy"));
    }
}

#[cfg(test)]
mod hermes_browser_config_tests {
    use super::{build_hermes_browser_config_values, merge_hermes_browser_config};

    #[test]
    fn browser_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_browser_config_values(&config);
        assert_eq!(values["browserInactivityTimeout"], 120);
        assert_eq!(values["browserCommandTimeout"], 30);
        assert_eq!(values["browserRecordSessions"], false);
        assert_eq!(values["browserEngine"], "auto");
        assert_eq!(values["browserAllowPrivateUrls"], false);
        assert_eq!(values["browserAutoLocalForPrivateUrls"], true);
        assert_eq!(values["browserCdpUrl"], "");
        assert_eq!(values["browserCamofoxManagedPersistence"], false);
        assert_eq!(values["browserCamofoxUserId"], "");
        assert_eq!(values["browserCamofoxSessionKey"], "");
        assert_eq!(values["browserCamofoxAdoptExistingTab"], false);
        assert_eq!(values["browserDialogPolicy"], "must_respond");
        assert_eq!(values["browserDialogTimeout"], 300);
    }

    #[test]
    fn browser_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
browser:
  inactivity_timeout: 300
  command_timeout: 45
  record_sessions: true
  engine: lightpanda
  allow_private_urls: true
  auto_local_for_private_urls: false
  cdp_url: ws://127.0.0.1:9222/devtools/browser/demo
  camofox:
    managed_persistence: true
    user_id: shared-camofox-user
    session_key: shared-session-key
    adopt_existing_tab: true
  dialog_policy: auto_accept
  dialog_timeout_s: 120
"#,
        )
        .unwrap();
        let values = build_hermes_browser_config_values(&config);
        assert_eq!(values["browserInactivityTimeout"], 300);
        assert_eq!(values["browserCommandTimeout"], 45);
        assert_eq!(values["browserRecordSessions"], true);
        assert_eq!(values["browserEngine"], "lightpanda");
        assert_eq!(values["browserAllowPrivateUrls"], true);
        assert_eq!(values["browserAutoLocalForPrivateUrls"], false);
        assert_eq!(values["browserCdpUrl"], "ws://127.0.0.1:9222/devtools/browser/demo");
        assert_eq!(values["browserCamofoxManagedPersistence"], true);
        assert_eq!(values["browserCamofoxUserId"], "shared-camofox-user");
        assert_eq!(values["browserCamofoxSessionKey"], "shared-session-key");
        assert_eq!(values["browserCamofoxAdoptExistingTab"], true);
        assert_eq!(values["browserDialogPolicy"], "auto_accept");
        assert_eq!(values["browserDialogTimeout"], 120);
    }

    #[test]
    fn merge_browser_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
browser:
  inactivity_timeout: 120
  command_timeout: 30
  record_sessions: false
  engine: auto
  cdp_url: ws://127.0.0.1:9222/devtools/browser/demo
  camofox:
    managed_persistence: false
    user_id: old-user
    session_key: old-session
    adopt_existing_tab: false
    custom_flag: keep-camofox
  custom_flag: keep-browser
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_browser_config(
            &mut config,
            &crate::jv!({
                "browserInactivityTimeout": "180",
                "browserCommandTimeout": "60",
                "browserRecordSessions": true,
                "browserEngine": "chrome",
                "browserAllowPrivateUrls": true,
                "browserAutoLocalForPrivateUrls": false,
                "browserCdpUrl": "http://127.0.0.1:9222",
                "browserCamofoxManagedPersistence": true,
                "browserCamofoxUserId": "shared-camofox-user",
                "browserCamofoxSessionKey": "shared-session-key",
                "browserCamofoxAdoptExistingTab": true,
                "browserDialogPolicy": "auto_dismiss",
                "browserDialogTimeout": "45",
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["browser"]["inactivity_timeout"].as_i64(), Some(180));
        assert_eq!(config["browser"]["command_timeout"].as_i64(), Some(60));
        assert_eq!(config["browser"]["record_sessions"].as_bool(), Some(true));
        assert_eq!(config["browser"]["engine"].as_str(), Some("chrome"));
        assert_eq!(config["browser"]["allow_private_urls"].as_bool(), Some(true));
        assert_eq!(config["browser"]["auto_local_for_private_urls"].as_bool(), Some(false));
        assert_eq!(config["browser"]["cdp_url"].as_str(), Some("http://127.0.0.1:9222"));
        assert_eq!(config["browser"]["dialog_policy"].as_str(), Some("auto_dismiss"));
        assert_eq!(config["browser"]["dialog_timeout_s"].as_i64(), Some(45));
        assert_eq!(config["browser"]["camofox"]["managed_persistence"].as_bool(), Some(true));
        assert_eq!(config["browser"]["camofox"]["user_id"].as_str(), Some("shared-camofox-user"));
        assert_eq!(config["browser"]["camofox"]["session_key"].as_str(), Some("shared-session-key"));
        assert_eq!(config["browser"]["camofox"]["adopt_existing_tab"].as_bool(), Some(true));
        assert_eq!(config["browser"]["camofox"]["custom_flag"].as_str(), Some("keep-camofox"));
        assert_eq!(config["browser"]["custom_flag"].as_str(), Some("keep-browser"));
    }

    #[test]
    fn merge_browser_config_removes_empty_camofox_identity_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
browser:
  camofox:
    managed_persistence: true
    user_id: old-user
    session_key: old-session
    adopt_existing_tab: true
    custom_flag: keep-camofox
  custom_flag: keep-browser
"#,
        )
        .unwrap();

        merge_hermes_browser_config(
            &mut config,
            &crate::jv!({
                "browserCamofoxManagedPersistence": false,
                "browserCamofoxUserId": "  ",
                "browserCamofoxSessionKey": "",
                "browserCamofoxAdoptExistingTab": false,
            }),
        )
        .unwrap();

        assert_eq!(config["browser"]["camofox"]["managed_persistence"].as_bool(), Some(false));
        assert!(config["browser"]["camofox"]["user_id"].is_null());
        assert!(config["browser"]["camofox"]["session_key"].is_null());
        assert_eq!(config["browser"]["camofox"]["adopt_existing_tab"].as_bool(), Some(false));
        assert_eq!(config["browser"]["camofox"]["custom_flag"].as_str(), Some("keep-camofox"));
        assert_eq!(config["browser"]["custom_flag"].as_str(), Some("keep-browser"));
    }

    #[test]
    fn merge_browser_config_removes_empty_cdp_url() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
browser:
  cdp_url: ws://127.0.0.1:9222/devtools/browser/demo
  custom_flag: keep-browser
"#,
        )
        .unwrap();

        merge_hermes_browser_config(&mut config, &crate::jv!({ "browserCdpUrl": "   " })).unwrap();

        assert_eq!(config["browser"]["custom_flag"].as_str(), Some("keep-browser"));
        assert!(config["browser"]["cdp_url"].is_null());
    }

    #[test]
    fn merge_browser_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_browser_config(&mut config, &crate::jv!({ "browserEngine": "firefox" })).unwrap_err();
        assert!(err.contains("browser.engine"));
        let err = merge_hermes_browser_config(&mut config, &crate::jv!({ "browserInactivityTimeout": 0 })).unwrap_err();
        assert!(err.contains("browser.inactivity_timeout"));
        let err = merge_hermes_browser_config(&mut config, &crate::jv!({ "browserCommandTimeout": 4 })).unwrap_err();
        assert!(err.contains("browser.command_timeout"));
        let err = merge_hermes_browser_config(&mut config, &crate::jv!({ "browserDialogPolicy": "ignore" })).unwrap_err();
        assert!(err.contains("browser.dialog_policy"));
        let err = merge_hermes_browser_config(&mut config, &crate::jv!({ "browserDialogTimeout": 0 })).unwrap_err();
        assert!(err.contains("browser.dialog_timeout_s"));
        let err = merge_hermes_browser_config(&mut config, &crate::jv!({ "browserCdpUrl": 123 })).unwrap_err();
        assert!(err.contains("browser.cdp_url"));
        let err = merge_hermes_browser_config(&mut config, &crate::jv!({ "browserCamofoxUserId": 123 })).unwrap_err();
        assert!(err.contains("browser.camofox.user_id"));
        let err = merge_hermes_browser_config(&mut config, &crate::jv!({ "browserCamofoxUserId": "bad user" })).unwrap_err();
        assert!(err.contains("browser.camofox.user_id"));
        let err =
            merge_hermes_browser_config(&mut config, &crate::jv!({ "browserCamofoxSessionKey": "bad session" })).unwrap_err();
        assert!(err.contains("browser.camofox.session_key"));
    }
}