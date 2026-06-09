    #[test]
    fn channel_display_values_read_platform_overrides_and_legacy_fallback() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
display:
  tool_progress: all
  show_reasoning: false
  cleanup_progress: false
  tool_progress_overrides:
    discord: off
  platforms:
    telegram:
      tool_progress: new
      show_reasoning: true
      tool_preview_length: 80
      streaming: false
      cleanup_progress: true
      custom_flag: keep-me
"#,
        )
        .unwrap();

        let values = build_hermes_channel_config_values(&config, &HashMap::new());

        assert_eq!(values["telegram"]["displayToolProgress"], "new");
        assert_eq!(values["telegram"]["displayShowReasoning"], true);
        assert_eq!(values["telegram"]["displayToolPreviewLength"], 80);
        assert_eq!(values["telegram"]["displayStreaming"], "false");
        assert_eq!(values["telegram"]["displayCleanupProgress"], true);
        assert_eq!(values["discord"]["displayToolProgress"], "off");
        assert_eq!(values["discord"]["displayStreaming"], "inherit");
    }

    #[test]
    fn merge_channel_display_writes_platform_overrides_and_preserves_unknown_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
display:
  tool_progress: all
  tool_progress_overrides:
    telegram: off
  platforms:
    telegram:
      tool_progress: new
      streaming: false
      custom_flag: keep-me
      runtime_footer:
        enabled: true
platforms:
  telegram:
    enabled: true
    extra:
      unknown_option: keep-platform
"#,
        )
        .unwrap();

        merge_hermes_channel_config(
            &mut config,
            "telegram",
            &crate::jv!({
                "enabled": true,
                "botToken": "",
                "displayToolProgress": "verbose",
                "displayShowReasoning": false,
                "displayToolPreviewLength": "120",
                "displayStreaming": "inherit",
                "displayCleanupProgress": false,
            }),
        )
        .unwrap();

        assert_eq!(config["display"]["tool_progress"].as_str(), Some("all"));
        assert_eq!(config["display"]["tool_progress_overrides"]["telegram"].as_str(), Some("off"));
        assert_eq!(config["display"]["platforms"]["telegram"]["tool_progress"].as_str(), Some("verbose"));
        assert_eq!(config["display"]["platforms"]["telegram"]["show_reasoning"].as_bool(), Some(false));
        assert_eq!(config["display"]["platforms"]["telegram"]["tool_preview_length"].as_i64(), Some(120));
        assert_eq!(config["display"]["platforms"]["telegram"]["streaming"], serde_yaml::Value::Null);
        assert_eq!(config["display"]["platforms"]["telegram"]["cleanup_progress"].as_bool(), Some(false));
        assert_eq!(config["display"]["platforms"]["telegram"]["custom_flag"].as_str(), Some("keep-me"));
        assert_eq!(
            config["display"]["platforms"]["telegram"]["runtime_footer"]["enabled"].as_bool(),
            Some(true)
        );
        assert_eq!(config["platforms"]["telegram"]["extra"]["unknown_option"].as_str(), Some("keep-platform"));
    }

    #[test]
    fn merge_channel_display_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_channel_config(
            &mut config,
            "telegram",
            &crate::jv!({
                "enabled": true,
                "displayToolProgress": "everything",
                "displayToolPreviewLength": 80,
                "displayStreaming": "inherit",
            }),
        )
        .unwrap_err();
        assert!(err.contains("display.platforms.telegram.tool_progress"));

        let err = merge_hermes_channel_config(
            &mut config,
            "telegram",
            &crate::jv!({
                "enabled": true,
                "displayToolProgress": "all",
                "displayToolPreviewLength": 200001,
                "displayStreaming": "inherit",
            }),
        )
        .unwrap_err();
        assert!(err.contains("display.platforms.telegram.tool_preview_length"));

        let err = merge_hermes_channel_config(
            &mut config,
            "telegram",
            &crate::jv!({
                "enabled": true,
                "displayToolProgress": "all",
                "displayToolPreviewLength": 80,
                "displayStreaming": "global",
            }),
        )
        .unwrap_err();
        assert!(err.contains("display.platforms.telegram.streaming"));
    }