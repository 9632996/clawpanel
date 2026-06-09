#[cfg(test)]
mod hermes_model_config_tests {
    use super::{build_hermes_model_config_values, merge_hermes_model_config};

    #[test]
    fn model_values_have_defaults_and_read_legacy_model_key() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_model_config_values(&config);
        assert_eq!(values["modelDefault"], "");
        assert_eq!(values["modelProvider"], "auto");
        assert_eq!(values["modelBaseUrl"], "");
        assert_eq!(values["modelContextLength"], "");
        assert_eq!(values["modelMaxTokens"], "");

        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  model: anthropic/claude-sonnet-4-6
  provider: openrouter
  base_url: https://openrouter.ai/api/v1
  context_length: 131072
  max_tokens: 8192
"#,
        )
        .unwrap();
        let values = build_hermes_model_config_values(&config);
        assert_eq!(values["modelDefault"], "anthropic/claude-sonnet-4-6");
        assert_eq!(values["modelProvider"], "openrouter");
        assert_eq!(values["modelBaseUrl"], "https://openrouter.ai/api/v1");
        assert_eq!(values["modelContextLength"], "131072");
        assert_eq!(values["modelMaxTokens"], "8192");
    }

    #[test]
    fn merge_model_preserves_unknown_fields_and_writes_base_url() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  default: old-model
  provider: auto
  base_url: https://old.example/v1
  auth_mode: env
  context_length: 200000
memory:
  memory_enabled: true
"#,
        )
        .unwrap();

        merge_hermes_model_config(
            &mut config,
            &crate::jv!({
                "modelDefault": "anthropic/claude-opus-4.6",
                "modelProvider": "openrouter",
                "modelBaseUrl": "https://openrouter.ai/api/v1",
                "modelContextLength": "262144",
                "modelMaxTokens": "16384",
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["default"].as_str(), Some("anthropic/claude-opus-4.6"));
        assert_eq!(config["model"]["provider"].as_str(), Some("openrouter"));
        assert_eq!(config["model"]["base_url"].as_str(), Some("https://openrouter.ai/api/v1"));
        assert_eq!(config["model"]["context_length"].as_i64(), Some(262144));
        assert_eq!(config["model"]["max_tokens"].as_i64(), Some(16384));
        assert_eq!(config["model"]["auth_mode"].as_str(), Some("env"));
        assert_eq!(config["memory"]["memory_enabled"].as_bool(), Some(true));
    }

    #[test]
    fn merge_model_empty_base_url_removes_field_and_legacy_model_key() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  model: old-model
  provider: custom
  base_url: https://old.example/v1
  max_tokens: 8192
display:
  language: zh
"#,
        )
        .unwrap();

        merge_hermes_model_config(
            &mut config,
            &crate::jv!({
                "modelDefault": "google/gemini-3-flash-preview",
                "modelProvider": "auto",
                "modelBaseUrl": "  ",
                "modelContextLength": "",
                "modelMaxTokens": " ",
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["default"].as_str(), Some("google/gemini-3-flash-preview"));
        assert_eq!(config["model"]["provider"].as_str(), Some("auto"));
        assert!(config["model"]["base_url"].is_null());
        assert!(config["model"]["model"].is_null());
        assert!(config["model"]["context_length"].is_null());
        assert!(config["model"]["max_tokens"].is_null());
        assert_eq!(config["display"]["language"].as_str(), Some("zh"));
    }

    #[test]
    fn merge_model_rejects_empty_model() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_model_config(
            &mut config,
            &crate::jv!({
                "modelDefault": " ",
                "modelProvider": "auto",
            }),
        )
        .unwrap_err();
        assert!(err.contains("model.default"));
    }

    #[test]
    fn merge_model_rejects_non_string_form_values() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  default: gpt-5
  provider: auto
"#,
        )
        .unwrap();
        let err = merge_hermes_model_config(
            &mut config,
            &crate::jv!({
                "modelDefault": "gpt-5",
                "modelProvider": 123,
            }),
        )
        .unwrap_err();
        assert!(err.contains("model.provider"));

        let err = merge_hermes_model_config(
            &mut config,
            &crate::jv!({
                "modelDefault": "gpt-5",
                "modelProvider": "auto",
                "modelBaseUrl": 123,
            }),
        )
        .unwrap_err();
        assert!(err.contains("model.base_url"));

        let err = merge_hermes_model_config(
            &mut config,
            &crate::jv!({
                "modelDefault": "gpt-5",
                "modelProvider": "auto",
                "modelContextLength": "0",
            }),
        )
        .unwrap_err();
        assert!(err.contains("model.context_length"));

        let err = merge_hermes_model_config(
            &mut config,
            &crate::jv!({
                "modelDefault": "gpt-5",
                "modelProvider": "auto",
                "modelMaxTokens": "1.5",
            }),
        )
        .unwrap_err();
        assert!(err.contains("model.max_tokens"));
    }
}

#[cfg(test)]
mod hermes_model_aliases_config_tests {
    use super::{build_hermes_model_aliases_config_values, merge_hermes_model_aliases_config};

    #[test]
    fn model_aliases_values_have_empty_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_model_aliases_config_values(&config);
        assert_eq!(values["modelAliasesJson"], "{}");
    }

    #[test]
    fn model_aliases_values_read_yaml_mapping() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model_aliases:
  opus:
    model: claude-opus-4-6
    provider: anthropic
  qwen:
    model: "qwen3.5:397b"
    provider: custom
    base_url: https://ollama.com/v1
"#,
        )
        .unwrap();

        let values = build_hermes_model_aliases_config_values(&config);
        let parsed: serde_json::Value = serde_json::from_str(values["modelAliasesJson"].as_str().unwrap()).unwrap();
        assert_eq!(parsed["opus"]["model"], "claude-opus-4-6");
        assert_eq!(parsed["opus"]["provider"], "anthropic");
        assert_eq!(parsed["qwen"]["model"], "qwen3.5:397b");
        assert_eq!(parsed["qwen"]["base_url"], "https://ollama.com/v1");
    }

    #[test]
    fn merge_model_aliases_config_preserves_unknown_fields_and_unrelated_yaml() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: openrouter
model_aliases:
  opus:
    model: old-opus
    provider: anthropic
    custom_flag: drop-with-replace
memory:
  memory_enabled: true
"#,
        )
        .unwrap();

        merge_hermes_model_aliases_config(
            &mut config,
            &crate::jv!({
                "modelAliasesJson": r#"{
                  "opus": {
                    "model": "claude-opus-4-6",
                    "provider": "anthropic",
                    "custom_flag": "keep-alias"
                  },
                  "qwen": {
                    "model": "qwen3.5:397b",
                    "provider": "custom",
                    "base_url": "https://ollama.com/v1"
                  }
                }"#,
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("openrouter"));
        assert_eq!(config["memory"]["memory_enabled"].as_bool(), Some(true));
        assert_eq!(config["model_aliases"]["opus"]["model"].as_str(), Some("claude-opus-4-6"));
        assert_eq!(config["model_aliases"]["opus"]["custom_flag"].as_str(), Some("keep-alias"));
        assert_eq!(config["model_aliases"]["qwen"]["provider"].as_str(), Some("custom"));
        assert_eq!(config["model_aliases"]["qwen"]["base_url"].as_str(), Some("https://ollama.com/v1"));
    }

    #[test]
    fn merge_model_aliases_config_removes_empty_mapping() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model_aliases:
  opus:
    model: claude-opus-4-6
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_model_aliases_config(&mut config, &crate::jv!({ "modelAliasesJson": "{}" })).unwrap();

        assert!(config["model_aliases"].is_null());
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
    }

    #[test]
    fn merge_model_aliases_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_model_aliases_config(&mut config, &crate::jv!({ "modelAliasesJson": "[" })).unwrap_err();
        assert!(err.contains("model_aliases JSON"));
        let err = merge_hermes_model_aliases_config(&mut config, &crate::jv!({ "modelAliasesJson": "[]" })).unwrap_err();
        assert!(err.contains("model_aliases"));
        let err = merge_hermes_model_aliases_config(
            &mut config,
            &crate::jv!({ "modelAliasesJson": r#"{ "bad alias": { "model": "m", "provider": "p" } }"# }),
        )
        .unwrap_err();
        assert!(err.contains("model_aliases.bad alias"));
        let err = merge_hermes_model_aliases_config(
            &mut config,
            &crate::jv!({ "modelAliasesJson": r#"{ "opus": "claude-opus-4-6" }"# }),
        )
        .unwrap_err();
        assert!(err.contains("model_aliases.opus"));
        let err = merge_hermes_model_aliases_config(
            &mut config,
            &crate::jv!({ "modelAliasesJson": r#"{ "opus": { "provider": "anthropic" } }"# }),
        )
        .unwrap_err();
        assert!(err.contains("model_aliases.opus.model"));
        let err = merge_hermes_model_aliases_config(
            &mut config,
            &crate::jv!({ "modelAliasesJson": r#"{ "opus": { "model": "claude-opus-4-6", "provider": 123 } }"# }),
        )
        .unwrap_err();
        assert!(err.contains("model_aliases.opus.provider"));
        let err = merge_hermes_model_aliases_config(
            &mut config,
            &crate::jv!({ "modelAliasesJson": r#"{ "qwen": { "model": "qwen3.5:397b", "base_url": 123 } }"# }),
        )
        .unwrap_err();
        assert!(err.contains("model_aliases.qwen.base_url"));
    }
}