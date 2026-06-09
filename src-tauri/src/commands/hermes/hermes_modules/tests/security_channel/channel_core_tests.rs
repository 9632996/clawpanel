
    #[test]
    fn merge_telegram_channel_keeps_unknown_extra_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
  default: claude-sonnet-4-6
platforms:
  telegram:
    enabled: false
    token: old
    extra:
      unknown_option: keep-me
"#,
        )
        .unwrap();

        merge_hermes_channel_config(
            &mut config,
            "telegram",
            &crate::jv!({
                "enabled": true,
                "botToken": "123:token",
                "dmPolicy": "pair",
                "groupPolicy": "allowlist",
                "allowFrom": "1001, 1002",
                "requireMention": true,
                "replyToMode": "off",
                "guestMode": true,
                "disableLinkPreviews": true,
            }),
        )
        .unwrap();

        let values = build_hermes_channel_config_values(&config, &HashMap::new());
        assert_eq!(values["telegram"]["enabled"], true);
        assert_eq!(values["telegram"]["botToken"], "");
        assert_eq!(values["telegram"]["allowFrom"], "1001, 1002");
        assert_eq!(config["platforms"]["telegram"]["token"], serde_yaml::Value::Null);
        assert_eq!(config["platforms"]["telegram"]["extra"]["unknown_option"].as_str(), Some("keep-me"));
        assert_eq!(config["platforms"]["telegram"]["extra"]["reply_to_mode"].as_str(), Some("off"));
        assert_eq!(config["platforms"]["telegram"]["extra"]["guest_mode"].as_bool(), Some(true));
        assert_eq!(config["platforms"]["telegram"]["extra"]["disable_link_previews"].as_bool(), Some(true));
        assert_eq!(values["telegram"]["replyToMode"], "off");
        assert_eq!(values["telegram"]["guestMode"], true);
        assert_eq!(values["telegram"]["disableLinkPreviews"], true);
        let env = build_hermes_channel_env_updates(
            "telegram",
            &crate::jv!({
                "botToken": "123:token",
                "allowFrom": "1001, 1002",
                "requireMention": true,
                "replyToMode": "off",
                "guestMode": true,
                "disableLinkPreviews": true,
            }),
        );
        assert!(env.contains(&("TELEGRAM_BOT_TOKEN".to_string(), "123:token".to_string())));
        assert!(env.contains(&("TELEGRAM_REPLY_TO_MODE".to_string(), "off".to_string())));
        assert!(env.contains(&("TELEGRAM_GUEST_MODE".to_string(), "true".to_string())));
        assert!(env.contains(&("TELEGRAM_DISABLE_LINK_PREVIEWS".to_string(), "true".to_string())));
    }

    #[test]
    fn build_channel_values_prefers_runtime_env_credentials() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
platforms:
  telegram:
    enabled: true
    token: yaml-token
    extra:
      allow_from: ["1001"]
  feishu:
    enabled: true
    extra:
      app_id: yaml-app-id
      app_secret: yaml-secret
      domain: lark
      connection_mode: webhook
  dingtalk:
    enabled: true
    extra:
      client_id: yaml-client-id
      client_secret: yaml-client-secret
      allowed_users: ["staff-1"]
      allowed_chats: ["cid-1"]
"#,
        )
        .unwrap();
        let mut env = HashMap::new();
        env.insert("TELEGRAM_BOT_TOKEN".to_string(), "env-token".to_string());
        env.insert("FEISHU_APP_ID".to_string(), "env-app-id".to_string());
        env.insert("FEISHU_APP_SECRET".to_string(), "env-secret".to_string());
        env.insert("FEISHU_DOMAIN".to_string(), "feishu".to_string());
        env.insert("FEISHU_CONNECTION_MODE".to_string(), "websocket".to_string());
        env.insert("DINGTALK_CLIENT_ID".to_string(), "env-client-id".to_string());
        env.insert("DINGTALK_CLIENT_SECRET".to_string(), "env-client-secret".to_string());

        let values = build_hermes_channel_config_values(&config, &env);

        assert_eq!(values["telegram"]["botToken"], "env-token");
        assert_eq!(values["telegram"]["allowFrom"], "1001");
        assert_eq!(values["feishu"]["appId"], "env-app-id");
        assert_eq!(values["feishu"]["appSecret"], "env-secret");
        assert_eq!(values["feishu"]["domain"], "feishu");
        assert_eq!(values["feishu"]["connectionMode"], "websocket");
        assert_eq!(values["dingtalk"]["clientId"], "env-client-id");
        assert_eq!(values["dingtalk"]["clientSecret"], "env-client-secret");
        assert_eq!(values["dingtalk"]["allowFrom"], "staff-1");
        assert_eq!(values["dingtalk"]["groupAllowFrom"], "cid-1");
    }

    #[test]
    fn merge_feishu_channel_fills_runtime_defaults() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());

        merge_hermes_channel_config(
            &mut config,
            "feishu",
            &crate::jv!({
                "enabled": true,
                "appId": "cli_xxx",
                "appSecret": "secret",
                "domain": "",
                "connectionMode": "",
                "webhookPath": "",
                "reactionNotifications": "",
                "typingIndicator": true,
                "resolveSenderNames": true,
            }),
        )
        .unwrap();

        assert_eq!(config["platforms"]["feishu"]["extra"]["app_id"], serde_yaml::Value::Null);
        assert_eq!(config["platforms"]["feishu"]["extra"]["app_secret"], serde_yaml::Value::Null);
        assert_eq!(config["platforms"]["feishu"]["extra"]["domain"].as_str(), Some("feishu"));
        assert_eq!(config["platforms"]["feishu"]["extra"]["connection_mode"].as_str(), Some("websocket"));
        assert_eq!(config["platforms"]["feishu"]["extra"]["webhook_path"].as_str(), Some("/feishu/webhook"));
        assert_eq!(config["platforms"]["feishu"]["extra"]["reaction_notifications"].as_str(), Some("off"));

        let env = build_hermes_channel_env_updates(
            "feishu",
            &crate::jv!({
                "appId": "cli_xxx",
                "appSecret": "secret",
                "domain": "",
                "connectionMode": "",
                "webhookPath": "",
                "groupPolicy": "allowlist",
            }),
        );
        assert!(env.contains(&("FEISHU_DOMAIN".to_string(), "feishu".to_string())));
        assert!(env.contains(&("FEISHU_CONNECTION_MODE".to_string(), "websocket".to_string())));
        assert!(env.contains(&("FEISHU_WEBHOOK_PATH".to_string(), "/feishu/webhook".to_string())));
    }

    #[test]
    fn discord_channel_supports_plugin_runtime_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
platforms:
  discord:
    enabled: true
    token: old-token
    extra:
      unknown_option: keep-me
      free_response_channels: ["yaml-free"]
      auto_thread: true
"#,
        )
        .unwrap();
        let mut env = HashMap::new();
        env.insert("DISCORD_BOT_TOKEN".to_string(), "env-discord-token".to_string());
        env.insert("DISCORD_FREE_RESPONSE_CHANNELS".to_string(), "env-free".to_string());
        env.insert("DISCORD_AUTO_THREAD".to_string(), "false".to_string());
        env.insert("DISCORD_HOME_CHANNEL".to_string(), "home-1".to_string());

        let values = build_hermes_channel_config_values(&config, &env);
        assert_eq!(values["discord"]["token"], "env-discord-token");
        assert_eq!(values["discord"]["freeResponseChannels"], "env-free");
        assert_eq!(values["discord"]["autoThread"], false);
        assert_eq!(values["discord"]["homeChannel"], "home-1");

        merge_hermes_channel_config(
            &mut config,
            "discord",
            &crate::jv!({
                "enabled": true,
                "token": "discord-token",
                "allowFrom": "1001, 1002",
                "requireMention": true,
                "freeResponseChannels": "free-a\nfree-b",
                "allowedChannels": "allow-a",
                "ignoredChannels": "ignore-a",
                "noThreadChannels": "plain-a",
                "autoThread": false,
                "reactions": true,
                "threadRequireMention": true,
                "historyBackfill": true,
                "historyBackfillLimit": "12",
                "replyToMode": "off",
                "homeChannel": "home-1",
                "homeChannelName": "ops-home",
            }),
        )
        .unwrap();

        assert_eq!(config["platforms"]["discord"]["token"], serde_yaml::Value::Null);
        assert_eq!(
            config["platforms"]["discord"]["extra"]["free_response_channels"]
                .as_sequence()
                .unwrap()
                .iter()
                .filter_map(|item| item.as_str())
                .collect::<Vec<_>>(),
            vec!["free-a", "free-b"]
        );
        assert_eq!(
            config["platforms"]["discord"]["extra"]["allowed_channels"]
                .as_sequence()
                .unwrap()
                .iter()
                .filter_map(|item| item.as_str())
                .collect::<Vec<_>>(),
            vec!["allow-a"]
        );
        assert_eq!(config["platforms"]["discord"]["extra"]["auto_thread"].as_bool(), Some(false));
        assert_eq!(config["platforms"]["discord"]["extra"]["reactions"].as_bool(), Some(true));
        assert_eq!(config["platforms"]["discord"]["extra"]["thread_require_mention"].as_bool(), Some(true));
        assert_eq!(config["platforms"]["discord"]["extra"]["history_backfill"].as_bool(), Some(true));
        assert_eq!(config["platforms"]["discord"]["extra"]["history_backfill_limit"].as_str(), Some("12"));
        assert_eq!(config["platforms"]["discord"]["extra"]["reply_to_mode"].as_str(), Some("off"));
        assert_eq!(config["platforms"]["discord"]["extra"]["unknown_option"].as_str(), Some("keep-me"));

        let env_updates = build_hermes_channel_env_updates(
            "discord",
            &crate::jv!({
                "token": "discord-token",
                "allowFrom": "1001, 1002",
                "requireMention": true,
                "freeResponseChannels": "free-a\nfree-b",
                "allowedChannels": "allow-a",
                "ignoredChannels": "ignore-a",
                "noThreadChannels": "plain-a",
                "autoThread": false,
                "reactions": true,
                "threadRequireMention": true,
                "historyBackfill": true,
                "historyBackfillLimit": "12",
                "replyToMode": "off",
                "homeChannel": "home-1",
                "homeChannelName": "ops-home",
            }),
        );

        assert!(env_updates.contains(&("DISCORD_BOT_TOKEN".to_string(), "discord-token".to_string())));
        assert!(env_updates.contains(&("DISCORD_FREE_RESPONSE_CHANNELS".to_string(), "free-a,free-b".to_string())));
        assert!(env_updates.contains(&("DISCORD_AUTO_THREAD".to_string(), "false".to_string())));
        assert!(env_updates.contains(&("DISCORD_THREAD_REQUIRE_MENTION".to_string(), "true".to_string())));
        assert!(env_updates.contains(&("DISCORD_HOME_CHANNEL".to_string(), "home-1".to_string())));
    }

    #[test]
    fn merge_dingtalk_channel_uses_runtime_fields() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
platforms:
  dingtalk:
    enabled: true
    extra:
      client_id: old-client-id
      client_secret: old-client-secret
      group_allow_from: ["legacy-chat"]
      unknown_option: keep-me
"#,
        )
        .unwrap();

        merge_hermes_channel_config(
            &mut config,
            "dingtalk",
            &crate::jv!({
                "enabled": true,
                "clientId": "ding-app-key",
                "clientSecret": "ding-secret",
                "allowFrom": "staff-1, staff-2",
                "groupAllowFrom": "cid-1\ncid-2",
                "requireMention": true,
            }),
        )
        .unwrap();

        assert_eq!(config["platforms"]["dingtalk"]["enabled"], true);
        assert_eq!(config["platforms"]["dingtalk"]["extra"]["client_id"], serde_yaml::Value::Null);
        assert_eq!(config["platforms"]["dingtalk"]["extra"]["client_secret"], serde_yaml::Value::Null);
        assert_eq!(config["platforms"]["dingtalk"]["extra"]["group_allow_from"], serde_yaml::Value::Null);
        assert_eq!(
            config["platforms"]["dingtalk"]["extra"]["allowed_users"]
                .as_sequence()
                .unwrap()
                .iter()
                .filter_map(|item| item.as_str())
                .collect::<Vec<_>>(),
            vec!["staff-1", "staff-2"]
        );
        assert_eq!(
            config["platforms"]["dingtalk"]["extra"]["allowed_chats"]
                .as_sequence()
                .unwrap()
                .iter()
                .filter_map(|item| item.as_str())
                .collect::<Vec<_>>(),
            vec!["cid-1", "cid-2"]
        );
        assert_eq!(config["platforms"]["dingtalk"]["extra"]["require_mention"].as_bool(), Some(true));
        assert_eq!(config["platforms"]["dingtalk"]["extra"]["unknown_option"].as_str(), Some("keep-me"));

        let env = build_hermes_channel_env_updates(
            "dingtalk",
            &crate::jv!({
                "clientId": "ding-app-key",
                "clientSecret": "ding-secret",
                "allowFrom": "staff-1, staff-2",
                "groupAllowFrom": "cid-1\ncid-2",
                "requireMention": true,
            }),
        );

        assert!(env.contains(&("DINGTALK_CLIENT_ID".to_string(), "ding-app-key".to_string())));
        assert!(env.contains(&("DINGTALK_CLIENT_SECRET".to_string(), "ding-secret".to_string())));
        assert!(env.contains(&("DINGTALK_ALLOWED_USERS".to_string(), "staff-1,staff-2".to_string())));
        assert!(env.contains(&("DINGTALK_ALLOWED_CHATS".to_string(), "cid-1,cid-2".to_string())));
        assert!(env.contains(&("DINGTALK_REQUIRE_MENTION".to_string(), "true".to_string())));
    }

    #[test]
    fn merge_channel_config_removes_yaml_secrets() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
platforms:
  slack:
    enabled: true
    token: old-bot-token
    extra:
      app_token: old-app-token
      signing_secret: old-signing-secret
      webhook_path: /old/events
      unknown_option: keep-me
"#,
        )
        .unwrap();

        merge_hermes_channel_config(
            &mut config,
            "slack",
            &crate::jv!({
                "enabled": true,
                "botToken": "xoxb-new",
                "appToken": "xapp-new",
                "signingSecret": "new-signing-secret",
                "webhookPath": "/slack/events",
            }),
        )
        .unwrap();

        assert_eq!(config["platforms"]["slack"]["token"], serde_yaml::Value::Null);
        assert_eq!(config["platforms"]["slack"]["extra"]["app_token"], serde_yaml::Value::Null);
        assert_eq!(config["platforms"]["slack"]["extra"]["signing_secret"], serde_yaml::Value::Null);
        assert_eq!(config["platforms"]["slack"]["extra"]["webhook_path"].as_str(), Some("/slack/events"));
        assert_eq!(config["platforms"]["slack"]["extra"]["unknown_option"].as_str(), Some("keep-me"));
    }

    #[test]
    fn plugin_platform_values_prefer_env_and_preserve_yaml_runtime_fields() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r##"
platforms:
  teams:
    enabled: true
    extra:
      client_id: yaml-teams-client
      client_secret: yaml-teams-secret
      tenant_id: yaml-tenant
      port: 3978
      service_url: https://smba.trafficmanager.net/teams/
      allow_from: ["aad-1"]
  google_chat:
    enabled: true
    extra:
      project_id: yaml-project
      subscription_name: projects/yaml-project/subscriptions/hermes
      service_account_json: yaml-sa.json
      allow_from: ["user@example.com"]
  irc:
    enabled: true
    extra:
      server: irc.libera.chat
      channel: "#hermes"
      nickname: hermes-bot
      use_tls: true
      allowed_users: ["alice"]
  line:
    enabled: true
    extra:
      channel_access_token: yaml-line-token
      channel_secret: yaml-line-secret
      host: 0.0.0.0
      port: 8646
      public_url: https://line.example.com
      allowed_users: ["U1"]
      allowed_groups: ["C1"]
      allowed_rooms: ["R1"]
      slow_response_threshold: "45"
  simplex:
    enabled: true
    extra:
      ws_url: ws://127.0.0.1:5225
      allowed_users: ["contact-1"]
"##,
        )
        .unwrap();
        let mut env = HashMap::new();
        env.insert("TEAMS_CLIENT_ID".to_string(), "env-teams-client".to_string());
        env.insert("TEAMS_CLIENT_SECRET".to_string(), "env-teams-secret".to_string());
        env.insert("TEAMS_TENANT_ID".to_string(), "env-tenant".to_string());
        env.insert("TEAMS_HOME_CHANNEL".to_string(), "teams-home".to_string());
        env.insert("GOOGLE_CHAT_PROJECT_ID".to_string(), "env-project".to_string());
        env.insert(
            "GOOGLE_CHAT_SUBSCRIPTION_NAME".to_string(),
            "projects/env-project/subscriptions/hermes".to_string(),
        );
        env.insert("GOOGLE_CHAT_SERVICE_ACCOUNT_JSON".to_string(), "env-sa.json".to_string());
        env.insert("GOOGLE_CHAT_HOME_CHANNEL".to_string(), "spaces/AAA".to_string());
        env.insert("IRC_SERVER".to_string(), "irc.oftc.net".to_string());
        env.insert("IRC_CHANNEL".to_string(), "#ops".to_string());
        env.insert("IRC_NICKNAME".to_string(), "ops-bot".to_string());
        env.insert("IRC_HOME_CHANNEL".to_string(), "#reports".to_string());
        env.insert("LINE_CHANNEL_ACCESS_TOKEN".to_string(), "env-line-token".to_string());
        env.insert("LINE_CHANNEL_SECRET".to_string(), "env-line-secret".to_string());
        env.insert("LINE_HOME_CHANNEL".to_string(), "U-home".to_string());
        env.insert("SIMPLEX_WS_URL".to_string(), "ws://127.0.0.1:5226".to_string());
        env.insert("SIMPLEX_HOME_CHANNEL".to_string(), "contact-home".to_string());

        let values = build_hermes_channel_config_values(&config, &env);

        assert_eq!(values["teams"]["clientId"], "env-teams-client");
        assert_eq!(values["teams"]["clientSecret"], "env-teams-secret");
        assert_eq!(values["teams"]["tenantId"], "env-tenant");
        assert_eq!(values["teams"]["homeChannel"], "teams-home");
        assert_eq!(values["teams"]["allowFrom"], "aad-1");
        assert_eq!(values["google_chat"]["projectId"], "env-project");
        assert_eq!(values["google_chat"]["subscriptionName"], "projects/env-project/subscriptions/hermes");
        assert_eq!(values["google_chat"]["serviceAccountJson"], "env-sa.json");
        assert_eq!(values["google_chat"]["homeChannel"], "spaces/AAA");
        assert_eq!(values["irc"]["server"], "irc.oftc.net");
        assert_eq!(values["irc"]["channel"], "#ops");
        assert_eq!(values["irc"]["nickname"], "ops-bot");
        assert_eq!(values["irc"]["homeChannel"], "#reports");
        assert_eq!(values["irc"]["useTls"], true);
        assert_eq!(values["irc"]["allowFrom"], "alice");
        assert_eq!(values["line"]["channelAccessToken"], "env-line-token");
        assert_eq!(values["line"]["channelSecret"], "env-line-secret");
        assert_eq!(values["line"]["homeChannel"], "U-home");
        assert_eq!(values["line"]["allowedGroups"], "C1");
        assert_eq!(values["line"]["allowedRooms"], "R1");
        assert_eq!(values["simplex"]["wsUrl"], "ws://127.0.0.1:5226");
        assert_eq!(values["simplex"]["homeChannel"], "contact-home");
        assert_eq!(values["simplex"]["allowFrom"], "contact-1");
    }

    #[test]
    fn plugin_platform_save_writes_runtime_fields_and_env() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());

        merge_hermes_channel_config(
            &mut config,
            "teams",
            &crate::jv!({
                "enabled": true,
                "clientId": "teams-client",
                "clientSecret": "teams-secret",
                "tenantId": "tenant-1",
                "port": "3978",
                "serviceUrl": "https://smba.trafficmanager.net/teams/",
                "allowFrom": "aad-1, aad-2",
                "allowAllUsers": false,
                "homeChannel": "19:abc@thread.tacv2",
                "homeChannelName": "Ops",
            }),
        )
        .unwrap();

        assert_eq!(config["platforms"]["teams"]["extra"]["client_id"], serde_yaml::Value::Null);
        assert_eq!(config["platforms"]["teams"]["extra"]["client_secret"], serde_yaml::Value::Null);
        assert_eq!(config["platforms"]["teams"]["extra"]["tenant_id"], serde_yaml::Value::Null);
        assert_eq!(config["platforms"]["teams"]["extra"]["port"].as_i64(), Some(3978));
        assert_eq!(
            config["platforms"]["teams"]["extra"]["service_url"].as_str(),
            Some("https://smba.trafficmanager.net/teams/")
        );
        assert_eq!(
            config["platforms"]["teams"]["extra"]["allow_from"]
                .as_sequence()
                .unwrap()
                .iter()
                .filter_map(|item| item.as_str())
                .collect::<Vec<_>>(),
            vec!["aad-1", "aad-2"]
        );

        merge_hermes_channel_config(
            &mut config,
            "google_chat",
            &crate::jv!({
                "enabled": true,
                "projectId": "project-1",
                "subscriptionName": "projects/project-1/subscriptions/hermes",
                "serviceAccountJson": "C:\\keys\\sa.json",
                "allowFrom": "user@example.com",
                "allowAllUsers": true,
                "homeChannel": "spaces/AAA",
                "homeChannelName": "Ops Space",
            }),
        )
        .unwrap();

        assert_eq!(config["platforms"]["google_chat"]["extra"]["project_id"].as_str(), Some("project-1"));
        assert_eq!(
            config["platforms"]["google_chat"]["extra"]["subscription_name"].as_str(),
            Some("projects/project-1/subscriptions/hermes")
        );
        assert_eq!(
            config["platforms"]["google_chat"]["extra"]["service_account_json"],
            serde_yaml::Value::Null
        );

        merge_hermes_channel_config(
            &mut config,
            "irc",
            &crate::jv!({
                "enabled": true,
                "server": "irc.libera.chat",
                "port": "6697",
                "nickname": "hermes-bot",
                "channel": "#hermes",
                "useTls": true,
                "serverPassword": "server-secret",
                "nickservPassword": "nick-secret",
                "allowFrom": "alice, bob",
                "allowAllUsers": false,
                "homeChannel": "#reports",
                "homeChannelName": "reports",
            }),
        )
        .unwrap();

        assert_eq!(config["platforms"]["irc"]["extra"]["server"].as_str(), Some("irc.libera.chat"));
        assert_eq!(config["platforms"]["irc"]["extra"]["port"].as_i64(), Some(6697));
        assert_eq!(config["platforms"]["irc"]["extra"]["use_tls"].as_bool(), Some(true));
        assert_eq!(config["platforms"]["irc"]["extra"]["server_password"], serde_yaml::Value::Null);
        assert_eq!(config["platforms"]["irc"]["extra"]["nickserv_password"], serde_yaml::Value::Null);

        merge_hermes_channel_config(
            &mut config,
            "line",
            &crate::jv!({
                "enabled": true,
                "channelAccessToken": "line-token",
                "channelSecret": "line-secret",
                "port": "8646",
                "host": "0.0.0.0",
                "publicUrl": "https://line.example.com",
                "allowFrom": "U1",
                "allowedGroups": "C1",
                "allowedRooms": "R1",
                "allowAllUsers": false,
                "homeChannel": "U-home",
                "slowResponseThreshold": "45",
            }),
        )
        .unwrap();

        assert_eq!(config["platforms"]["line"]["extra"]["channel_access_token"], serde_yaml::Value::Null);
        assert_eq!(config["platforms"]["line"]["extra"]["channel_secret"], serde_yaml::Value::Null);
        assert_eq!(config["platforms"]["line"]["extra"]["port"].as_i64(), Some(8646));
        assert_eq!(
            config["platforms"]["line"]["extra"]["allowed_groups"]
                .as_sequence()
                .unwrap()
                .iter()
                .filter_map(|item| item.as_str())
                .collect::<Vec<_>>(),
            vec!["C1"]
        );

        merge_hermes_channel_config(
            &mut config,
            "simplex",
            &crate::jv!({
                "enabled": true,
                "wsUrl": "ws://127.0.0.1:5225",
                "allowFrom": "contact-1",
                "allowAllUsers": true,
                "homeChannel": "group:ops",
                "homeChannelName": "Ops",
            }),
        )
        .unwrap();

        assert_eq!(config["platforms"]["simplex"]["extra"]["ws_url"].as_str(), Some("ws://127.0.0.1:5225"));

        let env = build_hermes_channel_env_updates(
            "line",
            &crate::jv!({
                "channelAccessToken": "line-token",
                "channelSecret": "line-secret",
                "port": "8646",
                "host": "0.0.0.0",
                "publicUrl": "https://line.example.com",
                "allowFrom": "U1",
                "allowedGroups": "C1",
                "allowedRooms": "R1",
                "allowAllUsers": false,
                "homeChannel": "U-home",
                "slowResponseThreshold": "45",
            }),
        );

        assert!(env.contains(&("LINE_CHANNEL_ACCESS_TOKEN".to_string(), "line-token".to_string())));
        assert!(env.contains(&("LINE_ALLOWED_GROUPS".to_string(), "C1".to_string())));
        assert!(env.contains(&("LINE_HOME_CHANNEL".to_string(), "U-home".to_string())));
    }