#[cfg(test)]
mod hermes_openrouter_cache_config_tests {
    use super::{build_hermes_openrouter_cache_config_values, merge_hermes_openrouter_cache_config};

    #[test]
    fn openrouter_cache_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_openrouter_cache_config_values(&config);
        assert_eq!(values["openrouterResponseCache"], true);
        assert_eq!(values["openrouterResponseCacheTtl"], 300);
    }

    #[test]
    fn openrouter_cache_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
openrouter:
  response_cache: false
  response_cache_ttl: 900
"#,
        )
        .unwrap();

        let values = build_hermes_openrouter_cache_config_values(&config);
        assert_eq!(values["openrouterResponseCache"], false);
        assert_eq!(values["openrouterResponseCacheTtl"], 900);
    }

    #[test]
    fn merge_openrouter_cache_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: openrouter
openrouter:
  response_cache: false
  response_cache_ttl: 900
  custom_flag: keep-openrouter
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_openrouter_cache_config(
            &mut config,
            &crate::jv!({
                "openrouterResponseCache": true,
                "openrouterResponseCacheTtl": "600",
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("openrouter"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["openrouter"]["response_cache"].as_bool(), Some(true));
        assert_eq!(config["openrouter"]["response_cache_ttl"].as_i64(), Some(600));
        assert_eq!(config["openrouter"]["custom_flag"].as_str(), Some("keep-openrouter"));
    }

    #[test]
    fn merge_openrouter_cache_config_rejects_invalid_ttl() {
        let mut config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        for ttl in ["0", "86401", "1.5"] {
            let err = merge_hermes_openrouter_cache_config(&mut config, &crate::jv!({ "openrouterResponseCacheTtl": ttl }))
                .unwrap_err();
            assert!(err.contains("openrouter.response_cache_ttl"));
        }
    }
}

#[cfg(test)]
mod hermes_provider_routing_config_tests {
    use super::{build_hermes_provider_routing_config_values, merge_hermes_provider_routing_config};

    #[test]
    fn provider_routing_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_provider_routing_config_values(&config);
        assert_eq!(values["providerRoutingSort"], "price");
        assert_eq!(values["providerRoutingOnly"], "");
        assert_eq!(values["providerRoutingIgnore"], "");
        assert_eq!(values["providerRoutingOrder"], "");
        assert_eq!(values["providerRoutingRequireParameters"], false);
        assert_eq!(values["providerRoutingDataCollection"], "allow");
    }

    #[test]
    fn provider_routing_values_read_yaml_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
provider_routing:
  sort: throughput
  only:
    - anthropic
    - google
  ignore:
    - deepinfra
  order:
    - anthropic
    - google
    - together
  require_parameters: true
  data_collection: deny
"#,
        )
        .unwrap();

        let values = build_hermes_provider_routing_config_values(&config);
        assert_eq!(values["providerRoutingSort"], "throughput");
        assert_eq!(values["providerRoutingOnly"], "anthropic\ngoogle");
        assert_eq!(values["providerRoutingIgnore"], "deepinfra");
        assert_eq!(values["providerRoutingOrder"], "anthropic\ngoogle\ntogether");
        assert_eq!(values["providerRoutingRequireParameters"], true);
        assert_eq!(values["providerRoutingDataCollection"], "deny");
    }

    #[test]
    fn merge_provider_routing_config_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: openrouter
openrouter:
  response_cache: true
provider_routing:
  sort: price
  custom_flag: keep-routing
"#,
        )
        .unwrap();

        merge_hermes_provider_routing_config(
            &mut config,
            &crate::jv!({
                "providerRoutingSort": "latency",
                "providerRoutingOnly": " anthropic \n google \n anthropic ",
                "providerRoutingIgnore": "deepinfra\nfireworks",
                "providerRoutingOrder": "google\nanthropic",
                "providerRoutingRequireParameters": true,
                "providerRoutingDataCollection": "deny",
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("openrouter"));
        assert_eq!(config["openrouter"]["response_cache"].as_bool(), Some(true));
        assert_eq!(config["provider_routing"]["sort"].as_str(), Some("latency"));
        assert_eq!(
            config["provider_routing"]["only"].as_sequence().unwrap(),
            &vec![
                serde_yaml::Value::String("anthropic".to_string()),
                serde_yaml::Value::String("google".to_string()),
            ]
        );
        assert_eq!(
            config["provider_routing"]["ignore"].as_sequence().unwrap(),
            &vec![
                serde_yaml::Value::String("deepinfra".to_string()),
                serde_yaml::Value::String("fireworks".to_string()),
            ]
        );
        assert_eq!(
            config["provider_routing"]["order"].as_sequence().unwrap(),
            &vec![
                serde_yaml::Value::String("google".to_string()),
                serde_yaml::Value::String("anthropic".to_string()),
            ]
        );
        assert_eq!(config["provider_routing"]["require_parameters"].as_bool(), Some(true));
        assert_eq!(config["provider_routing"]["data_collection"].as_str(), Some("deny"));
        assert_eq!(config["provider_routing"]["custom_flag"].as_str(), Some("keep-routing"));
    }

    #[test]
    fn merge_provider_routing_config_removes_empty_lists() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
provider_routing:
  only:
    - anthropic
  ignore:
    - deepinfra
  order:
    - google
"#,
        )
        .unwrap();

        merge_hermes_provider_routing_config(
            &mut config,
            &crate::jv!({
                "providerRoutingOnly": "",
                "providerRoutingIgnore": "  \n ",
                "providerRoutingOrder": "",
                "providerRoutingRequireParameters": false,
                "providerRoutingDataCollection": "allow",
            }),
        )
        .unwrap();

        assert_eq!(config["provider_routing"]["sort"].as_str(), Some("price"));
        assert_eq!(config["provider_routing"]["require_parameters"].as_bool(), Some(false));
        assert_eq!(config["provider_routing"]["data_collection"].as_str(), Some("allow"));
        let provider_routing = config["provider_routing"].as_mapping().unwrap();
        assert!(!provider_routing.contains_key(super::yaml_key("only")));
        assert!(!provider_routing.contains_key(super::yaml_key("ignore")));
        assert!(!provider_routing.contains_key(super::yaml_key("order")));
    }

    #[test]
    fn merge_provider_routing_config_rejects_invalid_values() {
        for (form, expected) in [
            (crate::jv!({ "providerRoutingSort": "random" }), "provider_routing.sort"),
            (
                crate::jv!({ "providerRoutingDataCollection": "maybe" }),
                "provider_routing.data_collection",
            ),
            (crate::jv!({ "providerRoutingOnly": "bad provider" }), "provider_routing.only"),
            (crate::jv!({ "providerRoutingOrder": "../secret" }), "provider_routing.order"),
        ] {
            let mut config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
            let err = merge_hermes_provider_routing_config(&mut config, &form).unwrap_err();
            assert!(err.contains(expected), "{err}");
        }
    }
}