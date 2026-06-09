
#[cfg(test)]
mod hermes_hooks_config_tests {
    use super::{build_hermes_hooks_config_values, merge_hermes_hooks_config};

    #[test]
    fn hooks_values_have_safe_defaults() {
        let config = serde_yaml::Value::Mapping(Default::default());
        let values = build_hermes_hooks_config_values(&config);

        assert_eq!(values["hooksAutoAccept"], false);
        assert_eq!(values["hooksJson"], "{}");
    }

    #[test]
    fn hooks_values_read_yaml_mapping() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
hooks_auto_accept: true
hooks:
  pre_tool_call:
    - matcher: terminal
      command: ~/.hermes/agent-hooks/block-rm-rf.sh
      timeout: 10
  pre_llm_call:
    - command: ~/.hermes/agent-hooks/inject-cwd-context.sh
"#,
        )
        .unwrap();

        let values = build_hermes_hooks_config_values(&config);
        let hooks: serde_json::Value = serde_json::from_str(values["hooksJson"].as_str().unwrap()).unwrap();

        assert_eq!(values["hooksAutoAccept"], true);
        assert_eq!(hooks["pre_tool_call"][0]["matcher"], "terminal");
        assert_eq!(hooks["pre_tool_call"][0]["command"], "~/.hermes/agent-hooks/block-rm-rf.sh");
        assert_eq!(hooks["pre_tool_call"][0]["timeout"], 10);
        assert_eq!(hooks["pre_llm_call"][0]["command"], "~/.hermes/agent-hooks/inject-cwd-context.sh");
    }

    #[test]
    fn merge_hooks_config_preserves_unknown_fields_and_unrelated_yaml() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: openrouter
hooks:
  pre_tool_call:
    - matcher: terminal
      command: old-hook.sh
      extra_flag: keep-old
memory:
  memory_enabled: true
"#,
        )
        .unwrap();

        merge_hermes_hooks_config(
            &mut config,
            &crate::jv!({
                "hooksAutoAccept": "true",
                "hooksJson": serde_json::to_string(&crate::jv!({
                    "pre_tool_call": [{
                        "matcher": "terminal",
                        "command": "~/.hermes/agent-hooks/block-rm-rf.sh",
                        "timeout": 10,
                        "extra_flag": "keep-hook"
                    }],
                    "post_tool_call": [{
                        "matcher": "write_file|patch",
                        "command": "~/.hermes/agent-hooks/auto-format.sh"
                    }]
                })).unwrap(),
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("openrouter"));
        assert_eq!(config["memory"]["memory_enabled"].as_bool(), Some(true));
        assert_eq!(config["hooks_auto_accept"].as_bool(), Some(true));
        assert_eq!(
            config["hooks"]["pre_tool_call"][0]["command"].as_str(),
            Some("~/.hermes/agent-hooks/block-rm-rf.sh")
        );
        assert_eq!(config["hooks"]["pre_tool_call"][0]["timeout"].as_i64(), Some(10));
        assert_eq!(config["hooks"]["pre_tool_call"][0]["extra_flag"].as_str(), Some("keep-hook"));
        assert_eq!(config["hooks"]["post_tool_call"][0]["matcher"].as_str(), Some("write_file|patch"));
    }

    #[test]
    fn merge_hooks_config_removes_empty_mapping_but_keeps_auto_accept() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
hooks_auto_accept: true
hooks:
  pre_tool_call:
    - command: old-hook.sh
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_hooks_config(&mut config, &crate::jv!({ "hooksAutoAccept": false, "hooksJson": "{}" })).unwrap();

        assert!(config["hooks"].is_null());
        assert_eq!(config["hooks_auto_accept"].as_bool(), Some(false));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
    }

    #[test]
    fn merge_hooks_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(Default::default());
        let err = merge_hermes_hooks_config(&mut config, &crate::jv!({ "hooksJson": "[" })).unwrap_err();
        assert!(err.contains("hooks JSON"));

        let err = merge_hermes_hooks_config(
            &mut config,
            &crate::jv!({ "hooksJson": serde_json::to_string(&crate::jv!({ "bad_event": [{ "command": "hook.sh" }] })).unwrap() }),
        )
        .unwrap_err();
        assert!(err.contains("hooks.bad_event"));

        let err = merge_hermes_hooks_config(
            &mut config,
            &crate::jv!({ "hooksJson": serde_json::to_string(&crate::jv!({ "pre_tool_call": { "command": "hook.sh" } })).unwrap() }),
        )
        .unwrap_err();
        assert!(err.contains("hooks.pre_tool_call"));

        let err = merge_hermes_hooks_config(
            &mut config,
            &crate::jv!({ "hooksJson": serde_json::to_string(&crate::jv!({ "pre_tool_call": ["hook.sh"] })).unwrap() }),
        )
        .unwrap_err();
        assert!(err.contains("hooks.pre_tool_call.0"));

        let err = merge_hermes_hooks_config(
            &mut config,
            &crate::jv!({ "hooksJson": serde_json::to_string(&crate::jv!({ "pre_tool_call": [{ "command": "" }] })).unwrap() }),
        )
        .unwrap_err();
        assert!(err.contains("hooks.pre_tool_call.0.command"));

        let err = merge_hermes_hooks_config(
            &mut config,
            &crate::jv!({ "hooksJson": serde_json::to_string(&crate::jv!({ "pre_tool_call": [{ "command": "hook.sh", "timeout": 0 }] })).unwrap() }),
        )
        .unwrap_err();
        assert!(err.contains("hooks.pre_tool_call.0.timeout"));
    }
}

#[cfg(test)]
mod hermes_mcp_servers_config_tests {
    use super::{build_hermes_mcp_servers_config_values, merge_hermes_mcp_servers_config};

    #[test]
    fn mcp_servers_values_have_empty_defaults() {
        let config = serde_yaml::Value::Mapping(Default::default());
        let values = build_hermes_mcp_servers_config_values(&config);

        assert_eq!(values["mcpServersJson"], "{}");
    }

    #[test]
    fn mcp_servers_values_read_yaml_mapping() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
mcp_servers:
  time:
    command: uvx
    args:
      - mcp-server-time
  notion:
    url: https://mcp.notion.com/mcp
    connect_timeout: 30
"#,
        )
        .unwrap();

        let values = build_hermes_mcp_servers_config_values(&config);
        let mapping: serde_json::Value = serde_json::from_str(values["mcpServersJson"].as_str().unwrap()).unwrap();

        assert_eq!(mapping["time"]["command"], "uvx");
        assert_eq!(mapping["time"]["args"][0], "mcp-server-time");
        assert_eq!(mapping["notion"]["url"], "https://mcp.notion.com/mcp");
        assert_eq!(mapping["notion"]["connect_timeout"], 30);
    }

    #[test]
    fn merge_mcp_servers_config_preserves_unknown_fields_and_unrelated_yaml() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: openrouter
mcp_servers:
  time:
    command: uvx
    args:
      - old-server
    sampling:
      enabled: true
      model: gemini-3-flash
memory:
  memory_enabled: true
"#,
        )
        .unwrap();

        merge_hermes_mcp_servers_config(
            &mut config,
            &crate::jv!({
                "mcpServersJson": serde_json::to_string(&crate::jv!({
                    "time": {
                        "command": "uvx",
                        "args": ["mcp-server-time"],
                        "timeout": 120,
                        "sampling": {
                            "enabled": true,
                            "model": "gemini-3-flash",
                            "max_tokens_cap": 4096,
                            "timeout": 30,
                            "max_rpm": 10,
                            "allowed_models": ["gemini-3-flash", "gpt-5-mini"],
                            "max_tool_rounds": 5,
                            "log_level": "info",
                            "custom_flag": "keep-sampling"
                        }
                    },
                    "notion": {
                        "url": "https://mcp.notion.com/mcp",
                        "headers": {
                            "Authorization": "Bearer token"
                        },
                        "connect_timeout": 30
                    }
                })).unwrap(),
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("openrouter"));
        assert_eq!(config["memory"]["memory_enabled"].as_bool(), Some(true));
        assert_eq!(config["mcp_servers"]["time"]["command"].as_str(), Some("uvx"));
        assert_eq!(config["mcp_servers"]["time"]["args"][0].as_str(), Some("mcp-server-time"));
        assert_eq!(config["mcp_servers"]["time"]["timeout"].as_i64(), Some(120));
        assert_eq!(config["mcp_servers"]["time"]["sampling"]["model"].as_str(), Some("gemini-3-flash"));
        assert_eq!(config["mcp_servers"]["time"]["sampling"]["max_tokens_cap"].as_i64(), Some(4096));
        assert_eq!(config["mcp_servers"]["time"]["sampling"]["timeout"].as_i64(), Some(30));
        assert_eq!(config["mcp_servers"]["time"]["sampling"]["max_rpm"].as_i64(), Some(10));
        assert_eq!(
            config["mcp_servers"]["time"]["sampling"]["allowed_models"][1].as_str(),
            Some("gpt-5-mini")
        );
        assert_eq!(config["mcp_servers"]["time"]["sampling"]["max_tool_rounds"].as_i64(), Some(5));
        assert_eq!(config["mcp_servers"]["time"]["sampling"]["log_level"].as_str(), Some("info"));
        assert_eq!(config["mcp_servers"]["time"]["sampling"]["custom_flag"].as_str(), Some("keep-sampling"));
        assert_eq!(config["mcp_servers"]["notion"]["headers"]["Authorization"].as_str(), Some("Bearer token"));
        assert_eq!(config["mcp_servers"]["notion"]["connect_timeout"].as_i64(), Some(30));
    }

    #[test]
    fn merge_mcp_servers_config_removes_empty_mapping() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
mcp_servers:
  time:
    command: uvx
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_mcp_servers_config(&mut config, &crate::jv!({ "mcpServersJson": "{}" })).unwrap();

        assert!(config["mcp_servers"].is_null());
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
    }

    #[test]
    fn merge_mcp_servers_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(Default::default());
        let err = merge_hermes_mcp_servers_config(&mut config, &crate::jv!({ "mcpServersJson": "[" })).unwrap_err();
        assert!(err.contains("mcp_servers JSON"));

        let err = merge_hermes_mcp_servers_config(
            &mut config,
            &crate::jv!({ "mcpServersJson": serde_json::to_string(&crate::jv!({ "bad server": { "command": "uvx" } })).unwrap() }),
        )
        .unwrap_err();
        assert!(err.contains("mcp_servers.bad server"));

        let err = merge_hermes_mcp_servers_config(
            &mut config,
            &crate::jv!({ "mcpServersJson": serde_json::to_string(&crate::jv!({ "time": "uvx" })).unwrap() }),
        )
        .unwrap_err();
        assert!(err.contains("mcp_servers.time"));

        let err = merge_hermes_mcp_servers_config(
            &mut config,
            &crate::jv!({ "mcpServersJson": serde_json::to_string(&crate::jv!({ "time": { "command": "" } })).unwrap() }),
        )
        .unwrap_err();
        assert!(err.contains("mcp_servers.time.command"));

        let err = merge_hermes_mcp_servers_config(
            &mut config,
            &crate::jv!({ "mcpServersJson": serde_json::to_string(&crate::jv!({ "notion": { "url": "ftp://example.com/mcp" } })).unwrap() }),
        )
        .unwrap_err();
        assert!(err.contains("mcp_servers.notion.url"));

        let err = merge_hermes_mcp_servers_config(
            &mut config,
            &crate::jv!({ "mcpServersJson": serde_json::to_string(&crate::jv!({ "time": { "command": "uvx", "args": "mcp-server-time" } })).unwrap() }),
        )
        .unwrap_err();
        assert!(err.contains("mcp_servers.time.args"));

        let err = merge_hermes_mcp_servers_config(
            &mut config,
            &crate::jv!({ "mcpServersJson": serde_json::to_string(&crate::jv!({ "time": { "command": "uvx", "timeout": 0 } })).unwrap() }),
        )
        .unwrap_err();
        assert!(err.contains("mcp_servers.time.timeout"));

        let err = merge_hermes_mcp_servers_config(
            &mut config,
            &crate::jv!({ "mcpServersJson": serde_json::to_string(&crate::jv!({ "time": { "command": "uvx", "sampling": [] } })).unwrap() }),
        )
        .unwrap_err();
        assert!(err.contains("mcp_servers.time.sampling"));

        let err = merge_hermes_mcp_servers_config(
            &mut config,
            &crate::jv!({ "mcpServersJson": serde_json::to_string(&crate::jv!({ "time": { "command": "uvx", "sampling": { "enabled": "yes" } } })).unwrap() }),
        )
        .unwrap_err();
        assert!(err.contains("mcp_servers.time.sampling.enabled"));

        let err = merge_hermes_mcp_servers_config(
            &mut config,
            &crate::jv!({ "mcpServersJson": serde_json::to_string(&crate::jv!({ "time": { "command": "uvx", "sampling": { "allowed_models": "gpt-5" } } })).unwrap() }),
        )
        .unwrap_err();
        assert!(err.contains("mcp_servers.time.sampling.allowed_models"));

        let err = merge_hermes_mcp_servers_config(
            &mut config,
            &crate::jv!({ "mcpServersJson": serde_json::to_string(&crate::jv!({ "time": { "command": "uvx", "sampling": { "max_tool_rounds": -1 } } })).unwrap() }),
        )
        .unwrap_err();
        assert!(err.contains("mcp_servers.time.sampling.max_tool_rounds"));

        let err = merge_hermes_mcp_servers_config(
            &mut config,
            &crate::jv!({ "mcpServersJson": serde_json::to_string(&crate::jv!({ "time": { "command": "uvx", "sampling": { "log_level": "trace" } } })).unwrap() }),
        )
        .unwrap_err();
        assert!(err.contains("mcp_servers.time.sampling.log_level"));
    }
}

include!("hooks_mcp_toolsets/toolsets_tests.rs");