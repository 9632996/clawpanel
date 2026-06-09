
fn normalize_hermes_mcp_sampling(value: &Value, key: &str) -> Result<Value, String> {
    let Some(config) = value.as_object() else {
        return Err(format!("{key} 必须是 JSON 对象"));
    };
    let mut sampling = config.clone();

    if let Some(enabled) = sampling.get("enabled") {
        if !enabled.is_boolean() {
            return Err(format!("{key}.enabled 必须是布尔值"));
        }
    }

    if sampling.contains_key("model") {
        let empty = sampling
            .get("model")
            .is_some_and(|value| value.is_null() || value.as_str().is_some_and(|text| text.trim().is_empty()));
        if empty {
            sampling.remove("model");
        } else {
            let Some(model) = sampling.get("model").and_then(|value| value.as_str()) else {
                return Err(format!("{key}.model 必须是字符串"));
            };
            sampling.insert("model".to_string(), Value::String(model.trim().to_string()));
        }
    }

    for (field, fallback, min, max) in [
        ("max_tokens_cap", 4096, 1, 1_000_000),
        ("timeout", 30, 1, 86400),
        ("max_rpm", 10, 1, 100000),
        ("max_tool_rounds", 5, 0, 1000),
    ] {
        if let Some(raw) = sampling.get(field).cloned() {
            let parsed = if let Some(value) = raw.as_i64() {
                Some(value)
            } else if let Some(value) = raw.as_u64() {
                i64::try_from(value).ok()
            } else if let Some(value) = raw.as_str() {
                value.trim().parse::<i64>().ok()
            } else {
                None
            };
            let parsed = parsed.ok_or_else(|| format!("{key}.{field} 必须是整数"))?;
            let parsed = validate_hermes_i64(Some(parsed), &format!("{key}.{field}"), fallback, min, max)?;
            sampling.insert(field.to_string(), Value::Number(parsed.into()));
        }
    }

    if let Some(allowed_models) = sampling.get("allowed_models") {
        let allowed_models = normalize_hermes_json_string_array(allowed_models, &format!("{key}.allowed_models"))?;
        sampling.insert("allowed_models".to_string(), Value::Array(allowed_models));
    }

    if sampling.contains_key("log_level") {
        let empty = sampling
            .get("log_level")
            .is_some_and(|value| value.is_null() || value.as_str().is_some_and(|text| text.trim().is_empty()));
        if empty {
            sampling.remove("log_level");
        } else {
            let Some(level) = sampling.get("log_level").and_then(|value| value.as_str()) else {
                return Err(format!("{key}.log_level 必须是字符串"));
            };
            let level = level.trim().to_ascii_lowercase();
            if !matches!(level.as_str(), "debug" | "info" | "warning" | "error") {
                return Err(format!("{key}.log_level 必须是 debug、info、warning 或 error"));
            }
            sampling.insert("log_level".to_string(), Value::String(level));
        }
    }

    Ok(Value::Object(sampling))
}

fn validate_hermes_mcp_servers(value: &Value) -> Result<serde_json::Map<String, Value>, String> {
    let Some(map) = value.as_object() else {
        return Err("mcp_servers 必须是 JSON 对象".to_string());
    };
    let mut normalized = serde_json::Map::new();
    for (raw_name, raw_config) in map {
        let name = raw_name.trim();
        if !is_hermes_mcp_server_name(name) {
            return Err(format!(
                "mcp_servers.{} 服务名只能包含字母、数字、下划线、点和短横线",
                if name.is_empty() { "<empty>" } else { raw_name }
            ));
        }
        let Some(config) = raw_config.as_object() else {
            return Err(format!("mcp_servers.{name} 必须是 JSON 对象"));
        };
        let mut entry = config.clone();
        let command = entry
            .get("command")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .trim()
            .to_string();
        let url = entry
            .get("url")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .trim()
            .to_string();
        let command_is_empty = command.is_empty();
        let url_is_empty = url.is_empty();
        if entry.contains_key("command") {
            if command_is_empty {
                return Err(format!("mcp_servers.{name}.command 不能为空"));
            }
            entry.insert("command".to_string(), Value::String(command));
        }
        if entry.contains_key("url") {
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                return Err(format!("mcp_servers.{name}.url 必须以 http:// 或 https:// 开头"));
            }
            entry.insert("url".to_string(), Value::String(url));
        }
        if command_is_empty && url_is_empty {
            return Err(format!("mcp_servers.{name} 需要 command 或 url"));
        }
        if let Some(args) = entry.get("args") {
            let args = normalize_hermes_json_string_array(args, &format!("mcp_servers.{name}.args"))?;
            entry.insert("args".to_string(), Value::Array(args));
        }
        if let Some(env) = entry.get("env") {
            let env = normalize_hermes_json_string_map(env, &format!("mcp_servers.{name}.env"))?;
            entry.insert("env".to_string(), Value::Object(env));
        }
        if let Some(headers) = entry.get("headers") {
            let headers = normalize_hermes_json_string_map(headers, &format!("mcp_servers.{name}.headers"))?;
            entry.insert("headers".to_string(), Value::Object(headers));
        }
        normalize_hermes_mcp_timeout(&mut entry, "timeout", &format!("mcp_servers.{name}.timeout"))?;
        normalize_hermes_mcp_timeout(&mut entry, "connect_timeout", &format!("mcp_servers.{name}.connect_timeout"))?;
        if let Some(sampling) = entry.get("sampling").cloned() {
            let sampling = normalize_hermes_mcp_sampling(&sampling, &format!("mcp_servers.{name}.sampling"))?;
            entry.insert("sampling".to_string(), sampling);
        }
        normalized.insert(name.to_string(), Value::Object(entry));
    }
    Ok(normalized)
}

fn parse_hermes_mcp_servers_json(raw: Option<String>) -> Result<serde_json::Map<String, Value>, String> {
    let text = raw.unwrap_or_default().trim().to_string();
    if text.is_empty() {
        return Ok(serde_json::Map::new());
    }
    let value: Value = serde_json::from_str(&text).map_err(|err| format!("mcp_servers JSON 格式错误: {err}"))?;
    validate_hermes_mcp_servers(&value)
}

fn build_hermes_mcp_servers_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let mcp_servers = root
        .and_then(|map| map.get(yaml_key("mcp_servers")))
        .and_then(|value| serde_json::to_value(value).ok())
        .and_then(|value| validate_hermes_mcp_servers(&value).ok())
        .unwrap_or_default();

    crate::jv!({
        "mcpServersJson": serde_json::to_string_pretty(&Value::Object(mcp_servers)).unwrap_or_else(|_| "{}".to_string()),
    })
}

fn merge_hermes_mcp_servers_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_mcp_servers_config_values(config);
    let mcp_servers = parse_hermes_mcp_servers_json(
        form_string(form, "mcpServersJson").or_else(|| current["mcpServersJson"].as_str().map(ToString::to_string)),
    )?;

    let root = ensure_yaml_object(config)?;
    if mcp_servers.is_empty() {
        root.remove(yaml_key("mcp_servers"));
    } else {
        let yaml_value =
            serde_yaml::to_value(Value::Object(mcp_servers)).map_err(|err| format!("mcp_servers 转换 YAML 失败: {err}"))?;
        root.insert(yaml_key("mcp_servers"), yaml_value);
    }
    Ok(())
}

fn is_hermes_provider_override_name(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
}

fn is_hermes_provider_model_name(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && !value.split('/').any(|part| part == "..")
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '/' | ':' | '@' | '+' | '-'))
}

fn normalize_hermes_provider_timeout(entry: &mut serde_json::Map<String, Value>, field: &str, key: &str) -> Result<(), String> {
    if !entry.contains_key(field) || entry.get(field).is_some_and(|value| value.is_null()) {
        entry.remove(field);
        return Ok(());
    }
    let value = entry.get(field).cloned().unwrap_or(Value::Null);
    let parsed = if let Some(value) = value.as_i64() {
        Some(value)
    } else if let Some(value) = value.as_u64() {
        i64::try_from(value).ok()
    } else if let Some(value) = value.as_str() {
        let text = value.trim();
        if text.is_empty() {
            None
        } else {
            text.parse::<i64>().ok()
        }
    } else {
        None
    };
    let parsed = parsed.ok_or_else(|| format!("{key} 必须是整数"))?;
    let parsed = validate_hermes_i64(Some(parsed), key, 300, 1, 86400)?;
    entry.insert(field.to_string(), Value::Number(parsed.into()));
    Ok(())
}

fn validate_hermes_provider_model_overrides(value: &Value, key: &str) -> Result<serde_json::Map<String, Value>, String> {
    let Some(map) = value.as_object() else {
        return Err(format!("{key} 必须是 JSON 对象"));
    };
    let mut normalized = serde_json::Map::new();
    for (raw_model, raw_config) in map {
        let model = raw_model.trim();
        if !is_hermes_provider_model_name(model) {
            return Err(format!("{key}.{model} 模型名只能包含字母、数字、下划线、点、斜杠、冒号、@、加号和短横线"));
        }
        let Some(config) = raw_config.as_object() else {
            return Err(format!("{key}.{model} 必须是 JSON 对象"));
        };
        let mut entry = config.clone();
        normalize_hermes_provider_timeout(&mut entry, "timeout_seconds", &format!("{key}.{model}.timeout_seconds"))?;
        normalize_hermes_provider_timeout(&mut entry, "stale_timeout_seconds", &format!("{key}.{model}.stale_timeout_seconds"))?;
        normalized.insert(model.to_string(), Value::Object(entry));
    }
    Ok(normalized)
}

fn validate_hermes_provider_overrides(value: &Value) -> Result<serde_json::Map<String, Value>, String> {
    let Some(map) = value.as_object() else {
        return Err("providers 必须是 JSON 对象".to_string());
    };
    let mut normalized = serde_json::Map::new();
    for (raw_provider, raw_config) in map {
        let provider = raw_provider.trim().to_ascii_lowercase();
        if !is_hermes_provider_override_name(&provider) {
            return Err(format!("providers.{raw_provider} provider 名只能包含字母、数字、下划线、点和短横线"));
        }
        let Some(config) = raw_config.as_object() else {
            return Err(format!("providers.{provider} 必须是 JSON 对象"));
        };
        let mut entry = config.clone();
        normalize_hermes_provider_timeout(
            &mut entry,
            "request_timeout_seconds",
            &format!("providers.{provider}.request_timeout_seconds"),
        )?;
        normalize_hermes_provider_timeout(
            &mut entry,
            "stale_timeout_seconds",
            &format!("providers.{provider}.stale_timeout_seconds"),
        )?;
        if let Some(models) = entry.get("models") {
            let models = validate_hermes_provider_model_overrides(models, &format!("providers.{provider}.models"))?;
            entry.insert("models".to_string(), Value::Object(models));
        }
        normalized.insert(provider, Value::Object(entry));
    }
    Ok(normalized)
}

fn parse_hermes_provider_overrides_json(raw: Option<String>) -> Result<serde_json::Map<String, Value>, String> {
    let text = raw.unwrap_or_default().trim().to_string();
    if text.is_empty() {
        return Ok(serde_json::Map::new());
    }
    let value: Value = serde_json::from_str(&text).map_err(|err| format!("providers JSON 格式错误: {err}"))?;
    validate_hermes_provider_overrides(&value)
}

fn build_hermes_provider_overrides_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let providers = root
        .and_then(|map| map.get(yaml_key("providers")))
        .and_then(|value| serde_json::to_value(value).ok())
        .and_then(|value| validate_hermes_provider_overrides(&value).ok())
        .unwrap_or_default();

    crate::jv!({
        "providerOverridesJson": serde_json::to_string_pretty(&Value::Object(providers)).unwrap_or_else(|_| "{}".to_string()),
    })
}

fn merge_hermes_provider_overrides_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_provider_overrides_config_values(config);
    let providers = parse_hermes_provider_overrides_json(
        form_string(form, "providerOverridesJson").or_else(|| current["providerOverridesJson"].as_str().map(ToString::to_string)),
    )?;

    let root = ensure_yaml_object(config)?;
    if providers.is_empty() {
        root.remove(yaml_key("providers"));
    } else {
        let yaml_value =
            serde_yaml::to_value(Value::Object(providers)).map_err(|err| format!("providers 转换 YAML 失败: {err}"))?;
        root.insert(yaml_key("providers"), yaml_value);
    }
    Ok(())
}

fn normalize_hermes_toolset_list(raw: Option<String>) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    for item in normalize_hermes_multiline_list(raw) {
        if !item
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
        {
            return Err("agent.disabled_toolsets 只能包含字母、数字、下划线、点和短横线".to_string());
        }
        if !normalized.iter().any(|existing| existing == &item) {
            normalized.push(item);
        }
    }
    Ok(normalized)
}

fn default_hermes_platform_toolsets() -> serde_json::Map<String, Value> {
    let defaults = [
        ("cli", "hermes-cli"),
        ("telegram", "hermes-telegram"),
        ("discord", "hermes-discord"),
        ("whatsapp", "hermes-whatsapp"),
        ("slack", "hermes-slack"),
        ("signal", "hermes-signal"),
        ("homeassistant", "hermes-homeassistant"),
        ("qqbot", "hermes-qqbot"),
        ("yuanbao", "hermes-yuanbao"),
        ("teams", "hermes-teams"),
        ("google_chat", "hermes-google_chat"),
    ];
    defaults
        .into_iter()
        .map(|(platform, toolset)| (platform.to_string(), Value::Array(vec![Value::String(toolset.to_string())])))
        .collect()
}

fn normalize_hermes_toolset_values(value: &Value, field_name: &str) -> Result<Vec<String>, String> {
    let Some(items) = value.as_array() else {
        return Err(format!("{field_name} 必须是工具集数组"));
    };
    let mut normalized = Vec::new();
    for item in items {
        let Some(text) = item.as_str() else {
            return Err(format!("{field_name} 只能包含字符串工具集"));
        };
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        if !text
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
        {
            return Err(format!("{field_name} 只能包含字母、数字、下划线、点和短横线"));
        }
        if !normalized.iter().any(|existing| existing == text) {
            normalized.push(text.to_string());
        }
    }
    if normalized.is_empty() {
        return Err(format!("{field_name} 至少需要一个工具集"));
    }
    Ok(normalized)
}

fn validate_hermes_platform_toolsets(value: &Value) -> Result<serde_json::Map<String, Value>, String> {
    let Some(map) = value.as_object() else {
        return Err("platform_toolsets 必须是 JSON 对象".to_string());
    };
    let mut normalized = serde_json::Map::new();
    for (platform, toolsets) in map {
        if platform.is_empty()
            || !platform
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
        {
            return Err(format!("platform_toolsets.{platform} 平台名只能包含字母、数字、下划线、点和短横线"));
        }
        let values = normalize_hermes_toolset_values(toolsets, &format!("platform_toolsets.{platform}"))?;
        normalized.insert(platform.clone(), Value::Array(values.into_iter().map(Value::String).collect()));
    }
    Ok(normalized)
}

fn parse_hermes_platform_toolsets_json(raw: Option<String>) -> Result<serde_json::Map<String, Value>, String> {
    let text = raw.unwrap_or_default().trim().to_string();
    if text.is_empty() {
        return Ok(serde_json::Map::new());
    }
    let value: Value = serde_json::from_str(&text).map_err(|err| format!("platform_toolsets JSON 格式错误: {err}"))?;
    validate_hermes_platform_toolsets(&value)
}

fn build_hermes_agent_toolsets_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let disabled_toolsets = root
        .and_then(|map| yaml_get_mapping(map, "agent"))
        .map(|map| yaml_string_sequence_field(map, "disabled_toolsets").join("\n"))
        .unwrap_or_default();

    crate::jv!({
        "disabledToolsets": disabled_toolsets,
    })
}

fn build_hermes_platform_toolsets_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let platform_toolsets = root
        .and_then(|map| map.get(yaml_key("platform_toolsets")))
        .and_then(|value| serde_json::to_value(value).ok())
        .and_then(|value| validate_hermes_platform_toolsets(&value).ok())
        .unwrap_or_else(default_hermes_platform_toolsets);

    crate::jv!({
        "platformToolsetsJson": serde_json::to_string_pretty(&Value::Object(platform_toolsets)).unwrap_or_else(|_| "{}".to_string()),
    })
}

fn merge_hermes_platform_toolsets_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_platform_toolsets_config_values(config);
    let platform_toolsets = parse_hermes_platform_toolsets_json(
        form_string(form, "platformToolsetsJson").or_else(|| current["platformToolsetsJson"].as_str().map(ToString::to_string)),
    )?;
    let yaml_value = serde_yaml::to_value(Value::Object(platform_toolsets))
        .map_err(|err| format!("platform_toolsets 转换 YAML 失败: {err}"))?;

    let root = ensure_yaml_object(config)?;
    root.insert(yaml_key("platform_toolsets"), yaml_value);
    Ok(())
}

fn merge_hermes_agent_toolsets_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_agent_toolsets_config_values(config);
    let disabled_toolsets = normalize_hermes_toolset_list(
        form_string(form, "disabledToolsets").or_else(|| current["disabledToolsets"].as_str().map(ToString::to_string)),
    )?;

    let root = ensure_yaml_object(config)?;
    let agent = yaml_child_object(root, "agent")?;
    agent.insert(
        yaml_key("disabled_toolsets"),
        serde_yaml::Value::Sequence(disabled_toolsets.into_iter().map(serde_yaml::Value::String).collect()),
    );
    Ok(())
}

fn normalize_hermes_image_input_mode(value: Option<String>, strict: bool) -> Result<String, String> {
    let mode = value.unwrap_or_default().trim().to_ascii_lowercase();
    let mode = if mode.is_empty() { "auto".to_string() } else { mode };
    if matches!(mode.as_str(), "auto" | "native" | "text") {
        return Ok(mode);
    }
    if strict {
        Err("agent.image_input_mode 必须是 auto、native 或 text".to_string())
    } else {
        Ok("auto".to_string())
    }
}

fn normalize_hermes_reasoning_effort(value: Option<String>, strict: bool) -> Result<String, String> {
    let effort = value.unwrap_or_default().trim().to_ascii_lowercase();
    let effort = if effort.is_empty() { "medium".to_string() } else { effort };
    if matches!(effort.as_str(), "xhigh" | "high" | "medium" | "low" | "minimal" | "none") {
        return Ok(effort);
    }
    if strict {
        Err("agent.reasoning_effort 必须是 xhigh、high、medium、low、minimal 或 none".to_string())
    } else {
        Ok("medium".to_string())
    }
}

fn validate_hermes_personalities(value: &Value) -> Result<serde_json::Map<String, Value>, String> {
    let Some(map) = value.as_object() else {
        return Err("agent.personalities 必须是 JSON 对象".to_string());
    };
    let mut normalized = serde_json::Map::new();
    for (raw_name, raw_prompt) in map {
        let name = raw_name.trim();
        if name.is_empty() {
            return Err("agent.personalities 名称不能为空".to_string());
        }
        if !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
        {
            return Err(format!("agent.personalities.{name} 名称只能包含字母、数字、下划线、点和短横线"));
        }
        let Some(prompt) = raw_prompt.as_str() else {
            return Err(format!("agent.personalities.{name} 必须是字符串"));
        };
        let prompt = prompt.trim();
        if prompt.is_empty() {
            return Err(format!("agent.personalities.{name} 不能为空"));
        }
        normalized.insert(name.to_string(), Value::String(prompt.to_string()));
    }
    Ok(normalized)
}

fn parse_hermes_personalities_json(raw: Option<String>) -> Result<serde_json::Map<String, Value>, String> {
    let text = raw.unwrap_or_default().trim().to_string();
    if text.is_empty() {
        return Ok(serde_json::Map::new());
    }
    let value: Value = serde_json::from_str(&text).map_err(|err| format!("agent.personalities JSON 格式错误: {err}"))?;
    validate_hermes_personalities(&value)
}

fn build_hermes_agent_runtime_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let agent = root.and_then(|map| yaml_get_mapping(map, "agent"));

    let image_input_mode =
        normalize_hermes_image_input_mode(agent.and_then(|map| yaml_string_field(map, "image_input_mode")), false)
            .unwrap_or_else(|_| "auto".to_string());
    let reasoning_effort =
        normalize_hermes_reasoning_effort(agent.and_then(|map| yaml_string_field(map, "reasoning_effort")), false)
            .unwrap_or_else(|_| "medium".to_string());
    let personalities = agent
        .and_then(|map| yaml_get(map, "personalities"))
        .and_then(|value| serde_json::to_value(value).ok())
        .and_then(|value| validate_hermes_personalities(&value).ok())
        .unwrap_or_default();

    crate::jv!({
        "agentMaxTurns": agent.map(|map| bounded_hermes_i64(yaml_i64_field(map, "max_turns"), 90, 1, 10000)).unwrap_or(90),
        "gatewayTimeout": agent.map(|map| bounded_hermes_i64(yaml_i64_field(map, "gateway_timeout"), 1800, 0, 604800)).unwrap_or(1800),
        "restartDrainTimeout": agent.map(|map| bounded_hermes_i64(yaml_i64_field(map, "restart_drain_timeout"), 180, 0, 86400)).unwrap_or(180),
        "apiMaxRetries": agent.map(|map| bounded_hermes_i64(yaml_i64_field(map, "api_max_retries"), 3, 1, 20)).unwrap_or(3),
        "gatewayTimeoutWarning": agent.map(|map| bounded_hermes_i64(yaml_i64_field(map, "gateway_timeout_warning"), 900, 0, 604800)).unwrap_or(900),
        "clarifyTimeout": agent.map(|map| bounded_hermes_i64(yaml_i64_field(map, "clarify_timeout"), 600, 0, 86400)).unwrap_or(600),
        "gatewayNotifyInterval": agent.map(|map| bounded_hermes_i64(yaml_i64_field(map, "gateway_notify_interval"), 180, 0, 86400)).unwrap_or(180),
        "gatewayAutoContinueFreshness": agent.map(|map| bounded_hermes_i64(yaml_i64_field(map, "gateway_auto_continue_freshness"), 3600, 0, 604800)).unwrap_or(3600),
        "imageInputMode": image_input_mode,
        "agentVerbose": agent.and_then(|map| yaml_bool_field(map, "verbose")).unwrap_or(false),
        "reasoningEffort": reasoning_effort,
        "personalitiesJson": serde_json::to_string_pretty(&Value::Object(personalities)).unwrap_or_else(|_| "{}".to_string()),
    })
}

fn agent_runtime_i64_value(form: &Value, current: &Value, form_key: &str, default_value: i64) -> Option<i64> {
    if form.get(form_key).is_some() {
        form_i64(form, form_key)
    } else {
        Some(current[form_key].as_i64().unwrap_or(default_value))
    }
}

fn merge_hermes_agent_runtime_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_agent_runtime_config_values(config);
    let agent_max_turns = validate_hermes_i64(
        agent_runtime_i64_value(form, &current, "agentMaxTurns", 90),
        "agent.max_turns",
        90,
        1,
        10000,
    )?;
    let gateway_timeout = validate_hermes_i64(
        agent_runtime_i64_value(form, &current, "gatewayTimeout", 1800),
        "agent.gateway_timeout",
        1800,
        0,
        604800,
    )?;
    let restart_drain_timeout = validate_hermes_i64(
        agent_runtime_i64_value(form, &current, "restartDrainTimeout", 180),
        "agent.restart_drain_timeout",
        180,
        0,
        86400,
    )?;
    let api_max_retries = validate_hermes_i64(
        agent_runtime_i64_value(form, &current, "apiMaxRetries", 3),
        "agent.api_max_retries",
        3,
        1,
        20,
    )?;
    let gateway_timeout_warning = validate_hermes_i64(
        agent_runtime_i64_value(form, &current, "gatewayTimeoutWarning", 900),
        "agent.gateway_timeout_warning",
        900,
        0,
        604800,
    )?;
    let clarify_timeout = validate_hermes_i64(
        agent_runtime_i64_value(form, &current, "clarifyTimeout", 600),
        "agent.clarify_timeout",
        600,
        0,
        86400,
    )?;
    let gateway_notify_interval = validate_hermes_i64(
        agent_runtime_i64_value(form, &current, "gatewayNotifyInterval", 180),
        "agent.gateway_notify_interval",
        180,
        0,
        86400,
    )?;
    let gateway_auto_continue_freshness = validate_hermes_i64(
        agent_runtime_i64_value(form, &current, "gatewayAutoContinueFreshness", 3600),
        "agent.gateway_auto_continue_freshness",
        3600,
        0,
        604800,
    )?;
    let image_input_mode = normalize_hermes_image_input_mode(
        if form.get("imageInputMode").is_some() {
            form_string(form, "imageInputMode")
        } else {
            current["imageInputMode"].as_str().map(ToString::to_string)
        },
        true,
    )?;
    let agent_verbose = form_bool(form, "agentVerbose").unwrap_or_else(|| current["agentVerbose"].as_bool().unwrap_or(false));
    let reasoning_effort = normalize_hermes_reasoning_effort(
        if form.get("reasoningEffort").is_some() {
            form_string(form, "reasoningEffort")
        } else {
            current["reasoningEffort"].as_str().map(ToString::to_string)
        },
        true,
    )?;
    let personalities = parse_hermes_personalities_json(
        form_string(form, "personalitiesJson").or_else(|| current["personalitiesJson"].as_str().map(ToString::to_string)),
    )?;

    let root = ensure_yaml_object(config)?;
    let agent = yaml_child_object(root, "agent")?;
    agent.insert(yaml_key("max_turns"), serde_yaml::Value::Number(agent_max_turns.into()));
    agent.insert(yaml_key("gateway_timeout"), serde_yaml::Value::Number(gateway_timeout.into()));
    agent.insert(yaml_key("restart_drain_timeout"), serde_yaml::Value::Number(restart_drain_timeout.into()));
    agent.insert(yaml_key("api_max_retries"), serde_yaml::Value::Number(api_max_retries.into()));
    agent.insert(
        yaml_key("gateway_timeout_warning"),
        serde_yaml::Value::Number(gateway_timeout_warning.into()),
    );
    agent.insert(yaml_key("clarify_timeout"), serde_yaml::Value::Number(clarify_timeout.into()));
    agent.insert(
        yaml_key("gateway_notify_interval"),
        serde_yaml::Value::Number(gateway_notify_interval.into()),
    );
    agent.insert(
        yaml_key("gateway_auto_continue_freshness"),
        serde_yaml::Value::Number(gateway_auto_continue_freshness.into()),
    );
    agent.insert(yaml_key("image_input_mode"), serde_yaml::Value::String(image_input_mode));
    agent.insert(yaml_key("verbose"), serde_yaml::Value::Bool(agent_verbose));
    agent.insert(yaml_key("reasoning_effort"), serde_yaml::Value::String(reasoning_effort));
    if personalities.is_empty() {
        agent.remove(yaml_key("personalities"));
    } else {
        let yaml_value = serde_yaml::to_value(Value::Object(personalities))
            .map_err(|err| format!("agent.personalities 转换 YAML 失败: {err}"))?;
        agent.insert(yaml_key("personalities"), yaml_value);
    }
    Ok(())
}
