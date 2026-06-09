
#[cfg(test)]
mod hermes_web_config_tests {
    use super::{build_hermes_web_config_values, merge_hermes_web_config};

    #[test]
    fn web_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_web_config_values(&config);
        assert_eq!(values["webBackend"], "");
        assert_eq!(values["webSearchBackend"], "");
        assert_eq!(values["webExtractBackend"], "");
    }

    #[test]
    fn web_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
web:
  backend: tavily
  search_backend: searxng
  extract_backend: firecrawl
"#,
        )
        .unwrap();
        let values = build_hermes_web_config_values(&config);
        assert_eq!(values["webBackend"], "tavily");
        assert_eq!(values["webSearchBackend"], "searxng");
        assert_eq!(values["webExtractBackend"], "firecrawl");
    }

    #[test]
    fn merge_web_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
web:
  backend: tavily
  search_backend: searxng
  extract_backend: firecrawl
  custom_flag: keep-web
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_web_config(
            &mut config,
            &crate::jv!({
                "webBackend": "parallel",
                "webSearchBackend": "exa",
                "webExtractBackend": "native",
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["web"]["backend"].as_str(), Some("parallel"));
        assert_eq!(config["web"]["search_backend"].as_str(), Some("exa"));
        assert_eq!(config["web"]["extract_backend"].as_str(), Some("native"));
        assert_eq!(config["web"]["custom_flag"].as_str(), Some("keep-web"));
    }

    #[test]
    fn merge_web_config_removes_empty_optional_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
web:
  backend: tavily
  search_backend: searxng
  extract_backend: firecrawl
  custom_flag: keep-web
"#,
        )
        .unwrap();

        merge_hermes_web_config(
            &mut config,
            &crate::jv!({
                "webBackend": "   ",
                "webSearchBackend": "",
                "webExtractBackend": "  ",
            }),
        )
        .unwrap();

        assert_eq!(config["web"]["custom_flag"].as_str(), Some("keep-web"));
        assert!(config["web"].get("backend").is_none());
        assert!(config["web"].get("search_backend").is_none());
        assert!(config["web"].get("extract_backend").is_none());
    }

    #[test]
    fn merge_web_config_rejects_invalid_backends() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_web_config(&mut config, &crate::jv!({ "webBackend": "unsafe" })).unwrap_err();
        assert!(err.contains("web.backend"));
        let err = merge_hermes_web_config(&mut config, &crate::jv!({ "webSearchBackend": "unsafe" })).unwrap_err();
        assert!(err.contains("web.search_backend"));
        let err = merge_hermes_web_config(&mut config, &crate::jv!({ "webExtractBackend": "unsafe" })).unwrap_err();
        assert!(err.contains("web.extract_backend"));
    }
}

#[cfg(test)]
mod hermes_model_catalog_config_tests {
    use super::{build_hermes_model_catalog_config_values, merge_hermes_model_catalog_config, HERMES_MODEL_CATALOG_DEFAULT_URL};

    #[test]
    fn model_catalog_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_model_catalog_config_values(&config);
        assert_eq!(values["modelCatalogEnabled"], true);
        assert_eq!(values["modelCatalogUrl"], HERMES_MODEL_CATALOG_DEFAULT_URL);
        assert_eq!(values["modelCatalogTtlHours"], 24);
        assert_eq!(values["modelCatalogProvidersJson"], "{}");
    }

    #[test]
    fn model_catalog_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model_catalog:
  enabled: false
  url: https://example.com/catalog.json
  ttl_hours: 6
  providers:
    openrouter:
      url: https://mirror.example.com/openrouter.json
    nous:
      url: https://mirror.example.com/nous.json
"#,
        )
        .unwrap();
        let values = build_hermes_model_catalog_config_values(&config);
        assert_eq!(values["modelCatalogEnabled"], false);
        assert_eq!(values["modelCatalogUrl"], "https://example.com/catalog.json");
        assert_eq!(values["modelCatalogTtlHours"], 6);
        let providers: serde_json::Value = serde_json::from_str(values["modelCatalogProvidersJson"].as_str().unwrap()).unwrap();
        assert_eq!(providers["openrouter"]["url"], "https://mirror.example.com/openrouter.json");
        assert_eq!(providers["nous"]["url"], "https://mirror.example.com/nous.json");
    }

    #[test]
    fn merge_model_catalog_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: openrouter
model_catalog:
  enabled: false
  url: https://old.example.com/catalog.json
  ttl_hours: 12
  providers:
    openrouter:
      url: https://old.example.com/openrouter.json
  custom_flag: keep-catalog
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_model_catalog_config(
            &mut config,
            &crate::jv!({
                "modelCatalogEnabled": true,
                "modelCatalogUrl": "https://catalog.example.com/model-catalog.json",
                "modelCatalogTtlHours": 48,
                "modelCatalogProvidersJson": serde_json::to_string(&crate::jv!({
                    "openrouter": { "url": "https://catalog.example.com/openrouter.json" },
                    "nous": { "url": "https://catalog.example.com/nous.json" },
                })).unwrap(),
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("openrouter"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["model_catalog"]["enabled"].as_bool(), Some(true));
        assert_eq!(
            config["model_catalog"]["url"].as_str(),
            Some("https://catalog.example.com/model-catalog.json")
        );
        assert_eq!(config["model_catalog"]["ttl_hours"].as_i64(), Some(48));
        assert_eq!(
            config["model_catalog"]["providers"]["openrouter"]["url"].as_str(),
            Some("https://catalog.example.com/openrouter.json")
        );
        assert_eq!(
            config["model_catalog"]["providers"]["nous"]["url"].as_str(),
            Some("https://catalog.example.com/nous.json")
        );
        assert_eq!(config["model_catalog"]["custom_flag"].as_str(), Some("keep-catalog"));
    }

    #[test]
    fn merge_model_catalog_config_removes_empty_providers() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model_catalog:
  providers:
    openrouter:
      url: https://old.example.com/openrouter.json
  custom_flag: keep-catalog
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_model_catalog_config(
            &mut config,
            &crate::jv!({
                "modelCatalogProvidersJson": "{}",
            }),
        )
        .unwrap();

        assert_eq!(config["model_catalog"]["custom_flag"].as_str(), Some("keep-catalog"));
        assert!(config["model_catalog"].get("providers").is_none());
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
    }

    #[test]
    fn merge_model_catalog_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err =
            merge_hermes_model_catalog_config(&mut config, &crate::jv!({ "modelCatalogUrl": "ftp://example.com/catalog.json" }))
                .unwrap_err();
        assert!(err.contains("model_catalog.url"));
        let err = merge_hermes_model_catalog_config(&mut config, &crate::jv!({ "modelCatalogTtlHours": 0 })).unwrap_err();
        assert!(err.contains("model_catalog.ttl_hours"));
        let err = merge_hermes_model_catalog_config(&mut config, &crate::jv!({ "modelCatalogProvidersJson": "[" })).unwrap_err();
        assert!(err.contains("model_catalog.providers"));
        let err = merge_hermes_model_catalog_config(
            &mut config,
            &crate::jv!({ "modelCatalogProvidersJson": serde_json::to_string(&crate::jv!({
                "bad provider": { "url": "https://example.com/catalog.json" }
            })).unwrap() }),
        )
        .unwrap_err();
        assert!(err.contains("model_catalog.providers.bad provider"));
        let err = merge_hermes_model_catalog_config(
            &mut config,
            &crate::jv!({ "modelCatalogProvidersJson": serde_json::to_string(&crate::jv!({
                "openrouter": { "url": "file:///tmp/catalog.json" }
            })).unwrap() }),
        )
        .unwrap_err();
        assert!(err.contains("model_catalog.providers.openrouter.url"));
    }
}

#[cfg(test)]
mod hermes_x_search_config_tests {
    use super::{build_hermes_x_search_config_values, merge_hermes_x_search_config};

    #[test]
    fn x_search_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_x_search_config_values(&config);
        assert_eq!(values["xSearchModel"], "grok-4.20-reasoning");
        assert_eq!(values["xSearchTimeoutSeconds"], 180);
        assert_eq!(values["xSearchRetries"], 2);
    }

    #[test]
    fn x_search_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
x_search:
  model: grok-4.20-fast
  timeout_seconds: 90
  retries: 4
"#,
        )
        .unwrap();
        let values = build_hermes_x_search_config_values(&config);
        assert_eq!(values["xSearchModel"], "grok-4.20-fast");
        assert_eq!(values["xSearchTimeoutSeconds"], 90);
        assert_eq!(values["xSearchRetries"], 4);
    }

    #[test]
    fn merge_x_search_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: xai
x_search:
  model: old-grok
  timeout_seconds: 60
  retries: 1
  custom_flag: keep-x-search
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_x_search_config(
            &mut config,
            &crate::jv!({
                "xSearchModel": "grok-4.20-reasoning",
                "xSearchTimeoutSeconds": 240,
                "xSearchRetries": 3,
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("xai"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["x_search"]["model"].as_str(), Some("grok-4.20-reasoning"));
        assert_eq!(config["x_search"]["timeout_seconds"].as_i64(), Some(240));
        assert_eq!(config["x_search"]["retries"].as_i64(), Some(3));
        assert_eq!(config["x_search"]["custom_flag"].as_str(), Some("keep-x-search"));
    }

    #[test]
    fn merge_x_search_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_x_search_config(&mut config, &crate::jv!({ "xSearchModel": "" })).unwrap_err();
        assert!(err.contains("x_search.model"));
        let err = merge_hermes_x_search_config(&mut config, &crate::jv!({ "xSearchModel": "bad model" })).unwrap_err();
        assert!(err.contains("x_search.model"));
        let err = merge_hermes_x_search_config(&mut config, &crate::jv!({ "xSearchTimeoutSeconds": 29 })).unwrap_err();
        assert!(err.contains("x_search.timeout_seconds"));
        let err = merge_hermes_x_search_config(&mut config, &crate::jv!({ "xSearchRetries": -1 })).unwrap_err();
        assert!(err.contains("x_search.retries"));
        let err = merge_hermes_x_search_config(&mut config, &crate::jv!({ "xSearchRetries": 21 })).unwrap_err();
        assert!(err.contains("x_search.retries"));
    }
}

#[cfg(test)]
mod hermes_context_config_tests {
    use super::{build_hermes_context_config_values, merge_hermes_context_config};

    #[test]
    fn context_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_context_config_values(&config);
        assert_eq!(values["contextEngine"], "compressor");
    }

    #[test]
    fn context_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
context:
  engine: lcm
"#,
        )
        .unwrap();
        let values = build_hermes_context_config_values(&config);
        assert_eq!(values["contextEngine"], "lcm");
    }

    #[test]
    fn merge_context_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
context:
  engine: compressor
  custom_flag: keep-context
model:
  provider: anthropic
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_context_config(&mut config, &crate::jv!({ "contextEngine": "lcm" })).unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["context"]["engine"].as_str(), Some("lcm"));
        assert_eq!(config["context"]["custom_flag"].as_str(), Some("keep-context"));
    }

    #[test]
    fn merge_context_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_context_config(&mut config, &crate::jv!({ "contextEngine": "" })).unwrap_err();
        assert!(err.contains("context.engine"));
        let err = merge_hermes_context_config(&mut config, &crate::jv!({ "contextEngine": "bad engine" })).unwrap_err();
        assert!(err.contains("context.engine"));
        let err = merge_hermes_context_config(&mut config, &crate::jv!({ "contextEngine": "中文" })).unwrap_err();
        assert!(err.contains("context.engine"));
    }
}

#[cfg(test)]
mod hermes_lsp_config_tests {
    use super::{build_hermes_lsp_config_values, merge_hermes_lsp_config};

    #[test]
    fn lsp_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_lsp_config_values(&config);
        assert_eq!(values["lspEnabled"], true);
        assert_eq!(values["lspWaitMode"], "document");
        assert_eq!(values["lspWaitTimeout"], 5.0);
        assert_eq!(values["lspInstallStrategy"], "auto");
    }

    #[test]
    fn lsp_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
lsp:
  enabled: false
  wait_mode: full
  wait_timeout: 12.5
  install_strategy: manual
  servers:
    pyright:
      disabled: true
"#,
        )
        .unwrap();
        let values = build_hermes_lsp_config_values(&config);
        assert_eq!(values["lspEnabled"], false);
        assert_eq!(values["lspWaitMode"], "full");
        assert_eq!(values["lspWaitTimeout"], 12.5);
        assert_eq!(values["lspInstallStrategy"], "manual");
    }

    #[test]
    fn merge_lsp_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
lsp:
  enabled: false
  wait_mode: full
  wait_timeout: 12.5
  install_strategy: manual
  servers:
    pyright:
      disabled: true
  custom_flag: keep-lsp
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_lsp_config(
            &mut config,
            &crate::jv!({
                "lspEnabled": true,
                "lspWaitMode": "document",
                "lspWaitTimeout": 7.5,
                "lspInstallStrategy": "off",
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["lsp"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["lsp"]["wait_mode"].as_str(), Some("document"));
        assert_eq!(config["lsp"]["wait_timeout"].as_f64(), Some(7.5));
        assert_eq!(config["lsp"]["install_strategy"].as_str(), Some("off"));
        assert_eq!(config["lsp"]["servers"]["pyright"]["disabled"].as_bool(), Some(true));
        assert_eq!(config["lsp"]["custom_flag"].as_str(), Some("keep-lsp"));
    }

    #[test]
    fn merge_lsp_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_lsp_config(&mut config, &crate::jv!({ "lspWaitMode": "workspace" })).unwrap_err();
        assert!(err.contains("lsp.wait_mode"));
        let err = merge_hermes_lsp_config(&mut config, &crate::jv!({ "lspInstallStrategy": "unsafe" })).unwrap_err();
        assert!(err.contains("lsp.install_strategy"));
        let err = merge_hermes_lsp_config(&mut config, &crate::jv!({ "lspWaitTimeout": 0 })).unwrap_err();
        assert!(err.contains("lsp.wait_timeout"));
        let err = merge_hermes_lsp_config(&mut config, &crate::jv!({ "lspWaitTimeout": 120.5 })).unwrap_err();
        assert!(err.contains("lsp.wait_timeout"));
    }
}

#[cfg(test)]
mod hermes_stt_config_tests {
    use super::{build_hermes_stt_config_values, merge_hermes_stt_config};

    #[test]
    fn stt_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_stt_config_values(&config);
        assert_eq!(values["sttEnabled"], true);
        assert_eq!(values["sttProvider"], "auto");
        assert_eq!(values["sttLocalModel"], "base");
        assert_eq!(values["sttLocalLanguage"], "");
        assert_eq!(values["sttOpenaiModel"], "whisper-1");
        assert_eq!(values["sttMistralModel"], "voxtral-mini-latest");
    }

    #[test]
    fn stt_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
stt:
  enabled: false
  provider: openai
  local:
    model: small
    language: zh
  openai:
    model: gpt-4o-mini-transcribe
  mistral:
    model: voxtral-mini-2602
"#,
        )
        .unwrap();
        let values = build_hermes_stt_config_values(&config);
        assert_eq!(values["sttEnabled"], false);
        assert_eq!(values["sttProvider"], "openai");
        assert_eq!(values["sttLocalModel"], "small");
        assert_eq!(values["sttLocalLanguage"], "zh");
        assert_eq!(values["sttOpenaiModel"], "gpt-4o-mini-transcribe");
        assert_eq!(values["sttMistralModel"], "voxtral-mini-2602");
    }

    #[test]
    fn merge_stt_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
stt:
  enabled: true
  provider: auto
  custom_flag: keep-stt
  local:
    model: base
    custom_flag: keep-local
memory:
  memory_enabled: true
"#,
        )
        .unwrap();

        merge_hermes_stt_config(
            &mut config,
            &crate::jv!({
                "sttEnabled": false,
                "sttProvider": "openai",
                "sttLocalModel": "small",
                "sttLocalLanguage": "zh",
                "sttOpenaiModel": "gpt-4o-mini-transcribe",
                "sttMistralModel": "voxtral-mini-2602",
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["memory"]["memory_enabled"].as_bool(), Some(true));
        assert_eq!(config["stt"]["enabled"].as_bool(), Some(false));
        assert_eq!(config["stt"]["provider"].as_str(), Some("openai"));
        assert_eq!(config["stt"]["local"]["model"].as_str(), Some("small"));
        assert_eq!(config["stt"]["local"]["language"].as_str(), Some("zh"));
        assert_eq!(config["stt"]["openai"]["model"].as_str(), Some("gpt-4o-mini-transcribe"));
        assert_eq!(config["stt"]["mistral"]["model"].as_str(), Some("voxtral-mini-2602"));
        assert_eq!(config["stt"]["custom_flag"].as_str(), Some("keep-stt"));
        assert_eq!(config["stt"]["local"]["custom_flag"].as_str(), Some("keep-local"));
    }

    #[test]
    fn merge_stt_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_stt_config(&mut config, &crate::jv!({ "sttProvider": "bad" })).unwrap_err();
        assert!(err.contains("stt.provider"));
        let err = merge_hermes_stt_config(&mut config, &crate::jv!({ "sttLocalModel": "giant" })).unwrap_err();
        assert!(err.contains("stt.local.model"));
        let err = merge_hermes_stt_config(&mut config, &crate::jv!({ "sttOpenaiModel": "gpt-4.1" })).unwrap_err();
        assert!(err.contains("stt.openai.model"));
        let err = merge_hermes_stt_config(&mut config, &crate::jv!({ "sttMistralModel": "voxtral-large" })).unwrap_err();
        assert!(err.contains("stt.mistral.model"));
        let err = merge_hermes_stt_config(&mut config, &crate::jv!({ "sttLocalLanguage": "中文" })).unwrap_err();
        assert!(err.contains("stt.local.language"));
    }
}

include!("web_model_catalog_context/tts_voice_tests.rs");