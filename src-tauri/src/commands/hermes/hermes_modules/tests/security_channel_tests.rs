
#[cfg(test)]
mod hermes_security_config_tests {
    use super::{build_hermes_security_config_values, merge_hermes_security_config};

    #[test]
    fn security_values_have_tirith_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_security_config_values(&config);
        assert_eq!(values["tirithEnabled"], true);
        assert_eq!(values["tirithPath"], "tirith");
        assert_eq!(values["tirithTimeout"], 5);
        assert_eq!(values["tirithFailOpen"], true);
    }

    #[test]
    fn security_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
security:
  tirith_enabled: false
  tirith_path: C:/tools/tirith.exe
  tirith_timeout: 12
  tirith_fail_open: false
"#,
        )
        .unwrap();
        let values = build_hermes_security_config_values(&config);
        assert_eq!(values["tirithEnabled"], false);
        assert_eq!(values["tirithPath"], "C:/tools/tirith.exe");
        assert_eq!(values["tirithTimeout"], 12);
        assert_eq!(values["tirithFailOpen"], false);
    }

    #[test]
    fn merge_security_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
security:
  allow_private_urls: false
  website_blocklist:
    enabled: true
    domains:
      - example.com
  custom_flag: keep-security
terminal:
  backend: docker
"#,
        )
        .unwrap();

        merge_hermes_security_config(
            &mut config,
            &crate::jv!({
                "tirithEnabled": false,
                "tirithPath": "~/bin/tirith",
                "tirithTimeout": 9,
                "tirithFailOpen": false,
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["terminal"]["backend"].as_str(), Some("docker"));
        assert_eq!(config["security"]["custom_flag"].as_str(), Some("keep-security"));
        assert_eq!(config["security"]["tirith_enabled"].as_bool(), Some(false));
        assert_eq!(config["security"]["tirith_path"].as_str(), Some("~/bin/tirith"));
        assert_eq!(config["security"]["tirith_timeout"].as_i64(), Some(9));
        assert_eq!(config["security"]["tirith_fail_open"].as_bool(), Some(false));
    }

    #[test]
    fn merge_security_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_security_config(&mut config, &crate::jv!({ "tirithTimeout": 0 })).unwrap_err();
        assert!(err.contains("security.tirith_timeout"));

        let err = merge_hermes_security_config(&mut config, &crate::jv!({ "tirithPath": "" })).unwrap_err();
        assert!(err.contains("security.tirith_path"));
    }
}

#[cfg(test)]
#[cfg(test)]
mod hermes_channel_tests {
    use super::{build_hermes_channel_config_values, build_hermes_channel_env_updates, merge_hermes_channel_config};
    use std::collections::HashMap;

    include!("security_channel/channel_core_tests.rs");
    include!("security_channel/channel_display_tests.rs");
}