#[cfg(test)]
mod hermes_provider_overrides_config_tests {
    use super::{build_hermes_provider_overrides_config_values, merge_hermes_provider_overrides_config};

    #[test]
    fn provider_overrides_values_have_empty_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_provider_overrides_config_values(&config);
        assert_eq!(values["providerOverridesJson"], "{}");
    }

    #[test]
    fn provider_overrides_values_read_yaml_mapping() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
providers:
  ollama-local:
    request_timeout_seconds: 300
    stale_timeout_seconds: 900
  anthropic:
    request_timeout_seconds: 30
    models:
      claude-opus-4.6:
        timeout_seconds: 600
"#,
        )
        .unwrap();

        let values = build_hermes_provider_overrides_config_values(&config);
        let mapping: serde_json::Value = serde_json::from_str(values["providerOverridesJson"].as_str().unwrap()).unwrap();
        assert_eq!(mapping["ollama-local"]["request_timeout_seconds"].as_i64(), Some(300));
        assert_eq!(mapping["ollama-local"]["stale_timeout_seconds"].as_i64(), Some(900));
        assert_eq!(mapping["anthropic"]["models"]["claude-opus-4.6"]["timeout_seconds"].as_i64(), Some(600));
    }

    #[test]
    fn merge_provider_overrides_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: openrouter
providers:
  anthropic:
    request_timeout_seconds: 30
    custom_flag: keep-provider
    models:
      claude-opus-4.6:
        timeout_seconds: 600
        custom_flag: keep-model
openrouter:
  response_cache: true
"#,
        )
        .unwrap();

        merge_hermes_provider_overrides_config(
            &mut config,
            &crate::jv!({
                "providerOverridesJson": r#"{
                  "anthropic": {
                    "request_timeout_seconds": 45,
                    "stale_timeout_seconds": 300,
                    "custom_flag": "keep-provider",
                    "models": {
                      "claude-opus-4.6": {
                        "timeout_seconds": 900,
                        "stale_timeout_seconds": 1200,
                        "custom_flag": "keep-model"
                      }
                    }
                  },
                  "openai-codex": {
                    "models": {
                      "gpt-5.4": {
                        "stale_timeout_seconds": 1800
                      }
                    }
                  }
                }"#,
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("openrouter"));
        assert_eq!(config["openrouter"]["response_cache"].as_bool(), Some(true));
        assert_eq!(config["providers"]["anthropic"]["request_timeout_seconds"].as_i64(), Some(45));
        assert_eq!(config["providers"]["anthropic"]["stale_timeout_seconds"].as_i64(), Some(300));
        assert_eq!(config["providers"]["anthropic"]["custom_flag"].as_str(), Some("keep-provider"));
        assert_eq!(
            config["providers"]["anthropic"]["models"]["claude-opus-4.6"]["timeout_seconds"].as_i64(),
            Some(900)
        );
        assert_eq!(
            config["providers"]["anthropic"]["models"]["claude-opus-4.6"]["stale_timeout_seconds"].as_i64(),
            Some(1200)
        );
        assert_eq!(
            config["providers"]["anthropic"]["models"]["claude-opus-4.6"]["custom_flag"].as_str(),
            Some("keep-model")
        );
        assert_eq!(
            config["providers"]["openai-codex"]["models"]["gpt-5.4"]["stale_timeout_seconds"].as_i64(),
            Some(1800)
        );
    }

    #[test]
    fn merge_provider_overrides_config_removes_empty_mapping() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
providers:
  anthropic:
    request_timeout_seconds: 30
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_provider_overrides_config(&mut config, &crate::jv!({ "providerOverridesJson": "{}" })).unwrap();

        assert!(config["providers"].is_null());
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
    }

    #[test]
    fn merge_provider_overrides_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_provider_overrides_config(&mut config, &crate::jv!({ "providerOverridesJson": "[" })).unwrap_err();
        assert!(err.contains("providers JSON"));
        let err = merge_hermes_provider_overrides_config(
            &mut config,
            &crate::jv!({ "providerOverridesJson": r#"{ "bad provider": { "request_timeout_seconds": 30 } }"# }),
        )
        .unwrap_err();
        assert!(err.contains("providers.bad provider"));
        let err = merge_hermes_provider_overrides_config(
            &mut config,
            &crate::jv!({ "providerOverridesJson": r#"{ "anthropic": { "request_timeout_seconds": 0 } }"# }),
        )
        .unwrap_err();
        assert!(err.contains("providers.anthropic.request_timeout_seconds"));
        let err = merge_hermes_provider_overrides_config(
            &mut config,
            &crate::jv!({ "providerOverridesJson": r#"{ "anthropic": { "models": { "../secret": { "timeout_seconds": 30 } } } }"# }),
        )
        .unwrap_err();
        assert!(err.contains("providers.anthropic.models.../secret"));
        let err = merge_hermes_provider_overrides_config(
            &mut config,
            &crate::jv!({ "providerOverridesJson": r#"{ "anthropic": { "models": { "opus": { "timeout_seconds": "slow" } } } }"# }),
        )
        .unwrap_err();
        assert!(err.contains("providers.anthropic.models.opus.timeout_seconds"));
    }
}

#[cfg(test)]
mod hermes_agent_toolsets_config_tests {
    use super::{build_hermes_agent_toolsets_config_values, merge_hermes_agent_toolsets_config};

    #[test]
    fn agent_toolsets_values_have_empty_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_agent_toolsets_config_values(&config);
        assert_eq!(values["disabledToolsets"], "");
    }

    #[test]
    fn agent_toolsets_values_read_yaml_sequence() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
agent:
  disabled_toolsets:
    - memory
    - web
    - browser
"#,
        )
        .unwrap();

        let values = build_hermes_agent_toolsets_config_values(&config);
        assert_eq!(values["disabledToolsets"], "memory\nweb\nbrowser");
    }

    #[test]
    fn merge_agent_toolsets_config_preserves_unrelated_yaml() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
agent:
  disabled_toolsets:
    - memory
  max_turns: 80
  custom_flag: keep-agent
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_agent_toolsets_config(
            &mut config,
            &crate::jv!({
                "disabledToolsets": " terminal \n browser \n\n memory\nbrowser ",
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["agent"]["disabled_toolsets"][0].as_str(), Some("terminal"));
        assert_eq!(config["agent"]["disabled_toolsets"][1].as_str(), Some("browser"));
        assert_eq!(config["agent"]["disabled_toolsets"][2].as_str(), Some("memory"));
        assert_eq!(config["agent"]["max_turns"].as_i64(), Some(80));
        assert_eq!(config["agent"]["custom_flag"].as_str(), Some("keep-agent"));
    }

    #[test]
    fn merge_agent_toolsets_config_writes_empty_sequence() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
agent:
  disabled_toolsets:
    - memory
  custom_flag: keep-agent
"#,
        )
        .unwrap();

        merge_hermes_agent_toolsets_config(&mut config, &crate::jv!({ "disabledToolsets": "  \n " })).unwrap();

        assert!(config["agent"]["disabled_toolsets"].as_sequence().unwrap().is_empty());
        assert_eq!(config["agent"]["custom_flag"].as_str(), Some("keep-agent"));
    }

    #[test]
    fn merge_agent_toolsets_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_agent_toolsets_config(&mut config, &crate::jv!({ "disabledToolsets": "bad tool" })).unwrap_err();
        assert!(err.contains("agent.disabled_toolsets"));
        let err = merge_hermes_agent_toolsets_config(&mut config, &crate::jv!({ "disabledToolsets": "../secret" })).unwrap_err();
        assert!(err.contains("agent.disabled_toolsets"));
    }
}

#[cfg(test)]
mod hermes_platform_toolsets_config_tests {
    use super::{build_hermes_platform_toolsets_config_values, merge_hermes_platform_toolsets_config};

    #[test]
    fn platform_toolsets_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_platform_toolsets_config_values(&config);
        let mapping: serde_json::Value = serde_json::from_str(values["platformToolsetsJson"].as_str().unwrap()).unwrap();

        assert_eq!(mapping["cli"][0].as_str(), Some("hermes-cli"));
        assert_eq!(mapping["telegram"][0].as_str(), Some("hermes-telegram"));
        assert_eq!(mapping["discord"][0].as_str(), Some("hermes-discord"));
        assert_eq!(mapping["whatsapp"][0].as_str(), Some("hermes-whatsapp"));
        assert_eq!(mapping["google_chat"][0].as_str(), Some("hermes-google_chat"));
    }

    #[test]
    fn platform_toolsets_values_read_yaml_mapping() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
platform_toolsets:
  cli:
    - web
    - terminal
    - file
  telegram:
    - hermes-telegram
  custom_platform:
    - safe
"#,
        )
        .unwrap();
        let values = build_hermes_platform_toolsets_config_values(&config);
        let mapping: serde_json::Value = serde_json::from_str(values["platformToolsetsJson"].as_str().unwrap()).unwrap();

        assert_eq!(mapping["cli"][0].as_str(), Some("web"));
        assert_eq!(mapping["cli"][1].as_str(), Some("terminal"));
        assert_eq!(mapping["cli"][2].as_str(), Some("file"));
        assert_eq!(mapping["telegram"][0].as_str(), Some("hermes-telegram"));
        assert_eq!(mapping["custom_platform"][0].as_str(), Some("safe"));
    }

    #[test]
    fn merge_platform_toolsets_config_preserves_unrelated_yaml() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
platform_toolsets:
  cli:
    - hermes-cli
agent:
  max_turns: 80
"#,
        )
        .unwrap();

        merge_hermes_platform_toolsets_config(
            &mut config,
            &crate::jv!({
                "platformToolsetsJson": serde_json::to_string(&crate::jv!({
                    "cli": ["web", "terminal", "file", "web"],
                    "telegram": ["hermes-telegram"],
                    "custom_platform": ["safe"]
                })).unwrap()
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["agent"]["max_turns"].as_i64(), Some(80));
        assert_eq!(config["platform_toolsets"]["cli"][0].as_str(), Some("web"));
        assert_eq!(config["platform_toolsets"]["cli"][1].as_str(), Some("terminal"));
        assert_eq!(config["platform_toolsets"]["cli"][2].as_str(), Some("file"));
        assert_eq!(config["platform_toolsets"]["telegram"][0].as_str(), Some("hermes-telegram"));
        assert_eq!(config["platform_toolsets"]["custom_platform"][0].as_str(), Some("safe"));
    }

    #[test]
    fn merge_platform_toolsets_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_platform_toolsets_config(&mut config, &crate::jv!({ "platformToolsetsJson": "[" })).unwrap_err();
        assert!(err.contains("platform_toolsets JSON"));

        let err = merge_hermes_platform_toolsets_config(
            &mut config,
            &crate::jv!({ "platformToolsetsJson": r#"{"bad platform":["web"]}"# }),
        )
        .unwrap_err();
        assert!(err.contains("platform_toolsets.bad platform"));

        let err = merge_hermes_platform_toolsets_config(
            &mut config,
            &crate::jv!({ "platformToolsetsJson": r#"{"cli":["bad tool"]}"# }),
        )
        .unwrap_err();
        assert!(err.contains("platform_toolsets.cli"));

        let err = merge_hermes_platform_toolsets_config(&mut config, &crate::jv!({ "platformToolsetsJson": r#"{"cli":[]}"# }))
            .unwrap_err();
        assert!(err.contains("platform_toolsets.cli"));
    }
}