
fn build_hermes_curator_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let curator = root.and_then(|map| yaml_get_mapping(map, "curator"));
    let backup = curator.and_then(|map| yaml_get_mapping(map, "backup"));

    crate::jv!({
        "curatorEnabled": curator.and_then(|map| yaml_bool_field(map, "enabled")).unwrap_or(true),
        "curatorIntervalHours": curator
            .map(|map| bounded_hermes_i64(yaml_i64_field(map, "interval_hours"), 168, 1, 87600))
            .unwrap_or(168),
        "curatorMinIdleHours": curator
            .map(|map| bounded_hermes_i64(yaml_i64_field(map, "min_idle_hours"), 2, 0, 87600))
            .unwrap_or(2),
        "curatorStaleAfterDays": curator
            .map(|map| bounded_hermes_i64(yaml_i64_field(map, "stale_after_days"), 30, 1, 36500))
            .unwrap_or(30),
        "curatorArchiveAfterDays": curator
            .map(|map| bounded_hermes_i64(yaml_i64_field(map, "archive_after_days"), 90, 1, 36500))
            .unwrap_or(90),
        "curatorBackupEnabled": backup.and_then(|map| yaml_bool_field(map, "enabled")).unwrap_or(true),
        "curatorBackupKeep": backup
            .map(|map| bounded_hermes_i64(yaml_i64_field(map, "keep"), 5, 0, 1000))
            .unwrap_or(5),
    })
}

fn merge_hermes_curator_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_curator_config_values(config);
    let curator_interval_hours = validate_hermes_i64(
        if form.get("curatorIntervalHours").is_some() {
            form_i64(form, "curatorIntervalHours")
        } else {
            Some(current["curatorIntervalHours"].as_i64().unwrap_or(168))
        },
        "curator.interval_hours",
        168,
        1,
        87600,
    )?;
    let curator_min_idle_hours = validate_hermes_i64(
        if form.get("curatorMinIdleHours").is_some() {
            form_i64(form, "curatorMinIdleHours")
        } else {
            Some(current["curatorMinIdleHours"].as_i64().unwrap_or(2))
        },
        "curator.min_idle_hours",
        2,
        0,
        87600,
    )?;
    let curator_stale_after_days = validate_hermes_i64(
        if form.get("curatorStaleAfterDays").is_some() {
            form_i64(form, "curatorStaleAfterDays")
        } else {
            Some(current["curatorStaleAfterDays"].as_i64().unwrap_or(30))
        },
        "curator.stale_after_days",
        30,
        1,
        36500,
    )?;
    let curator_archive_after_days = validate_hermes_i64(
        if form.get("curatorArchiveAfterDays").is_some() {
            form_i64(form, "curatorArchiveAfterDays")
        } else {
            Some(current["curatorArchiveAfterDays"].as_i64().unwrap_or(90))
        },
        "curator.archive_after_days",
        90,
        1,
        36500,
    )?;
    if curator_archive_after_days < curator_stale_after_days {
        return Err("curator.archive_after_days 必须大于或等于 curator.stale_after_days".to_string());
    }
    let curator_backup_keep = validate_hermes_i64(
        if form.get("curatorBackupKeep").is_some() {
            form_i64(form, "curatorBackupKeep")
        } else {
            Some(current["curatorBackupKeep"].as_i64().unwrap_or(5))
        },
        "curator.backup.keep",
        5,
        0,
        1000,
    )?;

    let root = ensure_yaml_object(config)?;
    let curator = yaml_child_object(root, "curator")?;
    curator.insert(
        yaml_key("enabled"),
        serde_yaml::Value::Bool(
            form_bool(form, "curatorEnabled").unwrap_or_else(|| current["curatorEnabled"].as_bool().unwrap_or(true)),
        ),
    );
    curator.insert(yaml_key("interval_hours"), serde_yaml::Value::Number(curator_interval_hours.into()));
    curator.insert(yaml_key("min_idle_hours"), serde_yaml::Value::Number(curator_min_idle_hours.into()));
    curator.insert(yaml_key("stale_after_days"), serde_yaml::Value::Number(curator_stale_after_days.into()));
    curator.insert(
        yaml_key("archive_after_days"),
        serde_yaml::Value::Number(curator_archive_after_days.into()),
    );
    let backup = yaml_child_object(curator, "backup")?;
    backup.insert(
        yaml_key("enabled"),
        serde_yaml::Value::Bool(
            form_bool(form, "curatorBackupEnabled").unwrap_or_else(|| current["curatorBackupEnabled"].as_bool().unwrap_or(true)),
        ),
    );
    backup.insert(yaml_key("keep"), serde_yaml::Value::Number(curator_backup_keep.into()));
    Ok(())
}

fn build_hermes_quick_commands_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let quick_commands = root
        .and_then(|map| yaml_get(map, "quick_commands"))
        .and_then(|value| value.as_mapping())
        .and_then(|mapping| serde_json::to_value(mapping).ok())
        .unwrap_or_else(|| crate::jv!({}));
    let quick_commands_json = serde_json::to_string_pretty(&quick_commands).unwrap_or_else(|_| "{}".to_string());

    crate::jv!({
        "quickCommandsJson": quick_commands_json,
    })
}

fn validate_hermes_quick_commands(value: Value) -> Result<serde_json::Map<String, Value>, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "quick_commands 必须是 JSON 对象".to_string())?;
    let mut normalized = serde_json::Map::new();
    for (raw_name, raw_command) in object {
        let name = raw_name.trim().trim_start_matches('/').to_string();
        if name.is_empty() {
            return Err("quick_commands 命令名不能为空".to_string());
        }
        let command_object = raw_command
            .as_object()
            .ok_or_else(|| format!("quick_commands.{name} 必须是对象"))?;
        let mut command = command_object.clone();
        let command_type = command
            .get("type")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if !matches!(command_type.as_str(), "exec" | "alias") {
            return Err(format!("quick_commands.{name}.type 必须是 exec 或 alias"));
        }
        command.insert("type".to_string(), Value::String(command_type.clone()));
        if command_type == "exec" {
            let shell_command = command
                .get("command")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .trim()
                .to_string();
            if shell_command.is_empty() {
                return Err(format!("quick_commands.{name}.command 不能为空"));
            }
            command.insert("command".to_string(), Value::String(shell_command));
        }
        if command_type == "alias" {
            let target = command
                .get("target")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .trim()
                .to_string();
            if !target.starts_with('/') {
                return Err(format!("quick_commands.{name}.target 必须以 / 开头"));
            }
            command.insert("target".to_string(), Value::String(target));
        }
        normalized.insert(name, Value::Object(command));
    }
    Ok(normalized)
}

fn parse_hermes_quick_commands_json(raw: Option<String>) -> Result<serde_json::Map<String, Value>, String> {
    let text = raw.unwrap_or_default();
    let text = text.trim();
    if text.is_empty() {
        return Ok(serde_json::Map::new());
    }
    let value: Value = serde_json::from_str(text).map_err(|err| format!("quick_commands JSON 格式错误: {err}"))?;
    validate_hermes_quick_commands(value)
}

fn merge_hermes_quick_commands_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_quick_commands_config_values(config);
    let quick_commands = parse_hermes_quick_commands_json(
        form_string(form, "quickCommandsJson").or_else(|| current["quickCommandsJson"].as_str().map(ToString::to_string)),
    )?;

    let root = ensure_yaml_object(config)?;
    if quick_commands.is_empty() {
        root.remove(yaml_key("quick_commands"));
    } else {
        let json_value = Value::Object(quick_commands);
        let yaml_value = serde_yaml::to_value(json_value).map_err(|err| format!("quick_commands 转换 YAML 失败: {err}"))?;
        root.insert(yaml_key("quick_commands"), yaml_value);
    }
    Ok(())
}

fn normalize_hermes_model_config_string(value: Option<String>, key: &str, required: bool) -> Result<String, String> {
    let text = value.unwrap_or_default().trim().to_string();
    if text.is_empty() && required {
        return Err(format!("{key} 不能为空"));
    }
    Ok(text)
}

fn hermes_model_form_string(form: &Value, form_key: &str, yaml_key: &str, current: &Value) -> Result<Option<String>, String> {
    if let Some(value) = form.get(form_key) {
        if let Some(text) = value.as_str() {
            return Ok(Some(text.to_string()));
        }
        return Err(format!("{yaml_key} 必须是字符串"));
    }
    Ok(current.as_str().map(ToString::to_string))
}

fn optional_hermes_model_i64_field(
    form: &Value,
    form_key: &str,
    yaml_key_name: &str,
    current: &Value,
) -> Result<Option<i64>, String> {
    let raw = if let Some(value) = form.get(form_key) {
        if value.is_null() {
            None
        } else if let Some(text) = value.as_str() {
            let text = text.trim();
            if text.is_empty() {
                None
            } else {
                Some(text.parse::<i64>().map_err(|_| format!("{yaml_key_name} 必须是整数"))?)
            }
        } else if let Some(value) = value.as_i64() {
            Some(value)
        } else if let Some(value) = value.as_u64() {
            Some(i64::try_from(value).map_err(|_| format!("{yaml_key_name} 必须是整数"))?)
        } else {
            return Err(format!("{yaml_key_name} 必须是整数"));
        }
    } else if let Some(text) = current.as_str() {
        let text = text.trim();
        if text.is_empty() {
            None
        } else {
            Some(text.parse::<i64>().map_err(|_| format!("{yaml_key_name} 必须是整数"))?)
        }
    } else {
        None
    };

    match raw {
        Some(value) if (1..=10_000_000).contains(&value) => Ok(Some(value)),
        Some(_) => Err(format!("{yaml_key_name} 必须在 1-10000000 范围内")),
        None => Ok(None),
    }
}

fn build_hermes_model_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let model = root
        .and_then(|map| map.get(yaml_key("model")))
        .and_then(|value| value.as_mapping());
    let model_default = model
        .and_then(|map| map.get(yaml_key("default")).or_else(|| map.get(yaml_key("model"))))
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .trim()
        .to_string();
    let provider = model
        .and_then(|map| map.get(yaml_key("provider")))
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "auto".to_string());
    let base_url = model
        .and_then(|map| map.get(yaml_key("base_url")))
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .trim()
        .to_string();
    let context_length = model
        .and_then(|map| yaml_i64_field(map, "context_length"))
        .filter(|value| *value > 0)
        .map(|value| value.to_string())
        .unwrap_or_default();
    let max_tokens = model
        .and_then(|map| yaml_i64_field(map, "max_tokens"))
        .filter(|value| *value > 0)
        .map(|value| value.to_string())
        .unwrap_or_default();

    crate::jv!({
        "modelDefault": model_default,
        "modelProvider": provider,
        "modelBaseUrl": base_url,
        "modelContextLength": context_length,
        "modelMaxTokens": max_tokens,
    })
}

fn merge_hermes_model_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_model_config_values(config);
    let model_default = normalize_hermes_model_config_string(
        hermes_model_form_string(form, "modelDefault", "model.default", &current["modelDefault"])?,
        "model.default",
        true,
    )?;
    let provider = normalize_hermes_model_config_string(
        hermes_model_form_string(form, "modelProvider", "model.provider", &current["modelProvider"])?,
        "model.provider",
        true,
    )?;
    let base_url = normalize_hermes_model_config_string(
        hermes_model_form_string(form, "modelBaseUrl", "model.base_url", &current["modelBaseUrl"])?,
        "model.base_url",
        false,
    )?;
    let context_length =
        optional_hermes_model_i64_field(form, "modelContextLength", "model.context_length", &current["modelContextLength"])?;
    let max_tokens = optional_hermes_model_i64_field(form, "modelMaxTokens", "model.max_tokens", &current["modelMaxTokens"])?;

    let root = ensure_yaml_object(config)?;
    let mut model = root
        .get(yaml_key("model"))
        .and_then(|value| value.as_mapping())
        .cloned()
        .unwrap_or_default();
    model.insert(yaml_key("default"), serde_yaml::Value::String(model_default));
    model.insert(yaml_key("provider"), serde_yaml::Value::String(provider));
    if base_url.is_empty() {
        model.remove(yaml_key("base_url"));
    } else {
        model.insert(yaml_key("base_url"), serde_yaml::Value::String(base_url));
    }
    if let Some(context_length) = context_length {
        model.insert(yaml_key("context_length"), serde_yaml::Value::Number(context_length.into()));
    } else {
        model.remove(yaml_key("context_length"));
    }
    if let Some(max_tokens) = max_tokens {
        model.insert(yaml_key("max_tokens"), serde_yaml::Value::Number(max_tokens.into()));
    } else {
        model.remove(yaml_key("max_tokens"));
    }
    model.remove(yaml_key("model"));
    root.insert(yaml_key("model"), serde_yaml::Value::Mapping(model));
    Ok(())
}

fn is_hermes_model_alias_name(value: &str) -> bool {
    let text = value.trim();
    !text.is_empty()
        && text
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
}

fn normalize_hermes_model_alias_string(
    entry: &mut serde_json::Map<String, Value>,
    field: &str,
    key: &str,
    required: bool,
) -> Result<(), String> {
    let empty = entry
        .get(field)
        .is_none_or(|value| value.is_null() || value.as_str().is_some_and(|text| text.trim().is_empty()));
    if empty {
        if required {
            return Err(format!("{key}.{field} 不能为空"));
        }
        entry.remove(field);
        return Ok(());
    }
    let Some(value) = entry.get(field).and_then(|value| value.as_str()) else {
        return Err(format!("{key}.{field} 必须是字符串"));
    };
    let value = value.trim().to_string();
    if value.is_empty() {
        if required {
            return Err(format!("{key}.{field} 不能为空"));
        }
        entry.remove(field);
    } else {
        entry.insert(field.to_string(), Value::String(value));
    }
    Ok(())
}

fn validate_hermes_model_aliases(value: &Value) -> Result<serde_json::Map<String, Value>, String> {
    let Some(object) = value.as_object() else {
        return Err("model_aliases 必须是 JSON 对象".to_string());
    };
    let mut normalized = serde_json::Map::new();
    for (raw_alias, raw_config) in object {
        let alias = raw_alias.trim();
        if !is_hermes_model_alias_name(alias) {
            return Err(format!(
                "model_aliases.{} 别名只能包含字母、数字、下划线、点和短横线",
                if raw_alias.is_empty() { "<empty>" } else { raw_alias }
            ));
        }
        let Some(config) = raw_config.as_object() else {
            return Err(format!("model_aliases.{alias} 必须是 JSON 对象"));
        };
        let mut entry = config.clone();
        let key = format!("model_aliases.{alias}");
        normalize_hermes_model_alias_string(&mut entry, "model", &key, true)?;
        normalize_hermes_model_alias_string(&mut entry, "provider", &key, false)?;
        normalize_hermes_model_alias_string(&mut entry, "base_url", &key, false)?;
        normalized.insert(alias.to_string(), Value::Object(entry));
    }
    Ok(normalized)
}

fn parse_hermes_model_aliases_json(raw: Option<String>) -> Result<serde_json::Map<String, Value>, String> {
    let text = raw.unwrap_or_default().trim().to_string();
    if text.is_empty() {
        return Ok(serde_json::Map::new());
    }
    let value: Value = serde_json::from_str(&text).map_err(|err| format!("model_aliases JSON 格式错误: {err}"))?;
    validate_hermes_model_aliases(&value)
}

fn build_hermes_model_aliases_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let model_aliases = root
        .and_then(|map| map.get(yaml_key("model_aliases")))
        .and_then(|value| serde_json::to_value(value).ok())
        .and_then(|value| validate_hermes_model_aliases(&value).ok())
        .unwrap_or_default();

    crate::jv!({
        "modelAliasesJson": serde_json::to_string_pretty(&Value::Object(model_aliases)).unwrap_or_else(|_| "{}".to_string()),
    })
}

fn merge_hermes_model_aliases_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_model_aliases_config_values(config);
    let model_aliases = parse_hermes_model_aliases_json(
        form_string(form, "modelAliasesJson").or_else(|| current["modelAliasesJson"].as_str().map(ToString::to_string)),
    )?;

    let root = ensure_yaml_object(config)?;
    if model_aliases.is_empty() {
        root.remove(yaml_key("model_aliases"));
    } else {
        let yaml_value =
            serde_yaml::to_value(Value::Object(model_aliases)).map_err(|err| format!("model_aliases 转换 YAML 失败: {err}"))?;
        root.insert(yaml_key("model_aliases"), yaml_value);
    }
    Ok(())
}

fn is_hermes_hook_event(value: &str) -> bool {
    matches!(
        value,
        "pre_tool_call"
            | "post_tool_call"
            | "pre_llm_call"
            | "post_llm_call"
            | "pre_api_request"
            | "post_api_request"
            | "on_session_start"
            | "on_session_end"
            | "on_session_finalize"
            | "on_session_reset"
            | "subagent_stop"
    )
}

fn normalize_hermes_hook_timeout(entry: &mut serde_json::Map<String, Value>, key: &str) -> Result<(), String> {
    if !entry.contains_key("timeout")
        || entry
            .get("timeout")
            .is_some_and(|value| value.is_null() || value.as_str().is_some_and(|text| text.trim().is_empty()))
    {
        entry.remove("timeout");
        return Ok(());
    }
    let value = entry.get("timeout").cloned().unwrap_or(Value::Null);
    let parsed = if let Some(value) = value.as_i64() {
        Some(value)
    } else if let Some(value) = value.as_u64() {
        i64::try_from(value).ok()
    } else if let Some(value) = value.as_str() {
        value.trim().parse::<i64>().ok()
    } else {
        None
    };
    let parsed = parsed.ok_or_else(|| format!("{key}.timeout 必须是整数"))?;
    let parsed = validate_hermes_i64(Some(parsed), &format!("{key}.timeout"), 30, 1, 86400)?;
    entry.insert("timeout".to_string(), Value::Number(parsed.into()));
    Ok(())
}

fn validate_hermes_hooks(value: &Value) -> Result<serde_json::Map<String, Value>, String> {
    let Some(map) = value.as_object() else {
        return Err("hooks 必须是 JSON 对象".to_string());
    };
    let mut normalized = serde_json::Map::new();
    for (raw_event, raw_entries) in map {
        let event = raw_event.trim();
        if !is_hermes_hook_event(event) {
            return Err(format!("hooks.{} 事件名不受支持", if event.is_empty() { "<empty>" } else { raw_event }));
        }
        let Some(entries) = raw_entries.as_array() else {
            return Err(format!("hooks.{event} 必须是数组"));
        };
        let mut normalized_entries = Vec::new();
        for (index, raw_entry) in entries.iter().enumerate() {
            let key = format!("hooks.{event}.{index}");
            let Some(config) = raw_entry.as_object() else {
                return Err(format!("{key} 必须是 JSON 对象"));
            };
            let mut entry = config.clone();
            let command = entry
                .get("command")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .trim()
                .to_string();
            if command.is_empty() {
                return Err(format!("{key}.command 不能为空"));
            }
            entry.insert("command".to_string(), Value::String(command));
            if let Some(matcher) = entry.get("matcher") {
                let Some(matcher) = matcher.as_str() else {
                    return Err(format!("{key}.matcher 必须是字符串"));
                };
                entry.insert("matcher".to_string(), Value::String(matcher.trim().to_string()));
            }
            normalize_hermes_hook_timeout(&mut entry, &key)?;
            normalized_entries.push(Value::Object(entry));
        }
        if !normalized_entries.is_empty() {
            normalized.insert(event.to_string(), Value::Array(normalized_entries));
        }
    }
    Ok(normalized)
}

fn parse_hermes_hooks_json(raw: Option<String>) -> Result<serde_json::Map<String, Value>, String> {
    let text = raw.unwrap_or_default().trim().to_string();
    if text.is_empty() {
        return Ok(serde_json::Map::new());
    }
    let value: Value = serde_json::from_str(&text).map_err(|err| format!("hooks JSON 格式错误: {err}"))?;
    validate_hermes_hooks(&value)
}

fn build_hermes_hooks_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let hooks = root
        .and_then(|map| map.get(yaml_key("hooks")))
        .and_then(|value| serde_json::to_value(value).ok())
        .and_then(|value| validate_hermes_hooks(&value).ok())
        .unwrap_or_default();

    crate::jv!({
        "hooksAutoAccept": root.and_then(|map| yaml_bool_field(map, "hooks_auto_accept")).unwrap_or(false),
        "hooksJson": serde_json::to_string_pretty(&Value::Object(hooks)).unwrap_or_else(|_| "{}".to_string()),
    })
}

fn merge_hermes_hooks_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_hooks_config_values(config);
    let hooks = parse_hermes_hooks_json(
        form_string(form, "hooksJson").or_else(|| current["hooksJson"].as_str().map(ToString::to_string)),
    )?;
    let hooks_auto_accept =
        form_bool(form, "hooksAutoAccept").unwrap_or_else(|| current["hooksAutoAccept"].as_bool().unwrap_or(false));

    let root = ensure_yaml_object(config)?;
    root.insert(yaml_key("hooks_auto_accept"), serde_yaml::Value::Bool(hooks_auto_accept));
    if hooks.is_empty() {
        root.remove(yaml_key("hooks"));
    } else {
        let yaml_value = serde_yaml::to_value(Value::Object(hooks)).map_err(|err| format!("hooks 转换 YAML 失败: {err}"))?;
        root.insert(yaml_key("hooks"), yaml_value);
    }
    Ok(())
}

fn is_hermes_mcp_server_name(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
}

fn normalize_hermes_json_string_array(value: &Value, key: &str) -> Result<Vec<Value>, String> {
    let Some(items) = value.as_array() else {
        return Err(format!("{key} 必须是字符串数组"));
    };
    let mut normalized = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let Some(text) = item.as_str() else {
            return Err(format!("{key}.{index} 必须是字符串"));
        };
        normalized.push(Value::String(text.to_string()));
    }
    Ok(normalized)
}

fn normalize_hermes_json_string_map(value: &Value, key: &str) -> Result<serde_json::Map<String, Value>, String> {
    let Some(items) = value.as_object() else {
        return Err(format!("{key} 必须是 JSON 对象"));
    };
    let mut normalized = serde_json::Map::new();
    for (raw_key, raw_value) in items {
        let item_key = raw_key.trim();
        if item_key.is_empty() {
            return Err(format!("{key} 键名不能为空"));
        }
        let Some(text) = raw_value.as_str() else {
            return Err(format!("{key}.{item_key} 必须是字符串"));
        };
        normalized.insert(item_key.to_string(), Value::String(text.to_string()));
    }
    Ok(normalized)
}

fn normalize_hermes_mcp_timeout(entry: &mut serde_json::Map<String, Value>, field: &str, key: &str) -> Result<(), String> {
    if !entry.contains_key(field)
        || entry
            .get(field)
            .is_some_and(|value| value.is_null() || value.as_str().is_some_and(|text| text.trim().is_empty()))
    {
        entry.remove(field);
        return Ok(());
    }
    let value = entry.get(field).cloned().unwrap_or(Value::Null);
    let parsed = if let Some(value) = value.as_i64() {
        Some(value)
    } else if let Some(value) = value.as_u64() {
        i64::try_from(value).ok()
    } else if let Some(value) = value.as_str() {
        value.trim().parse::<i64>().ok()
    } else {
        None
    };
    let parsed = parsed.ok_or_else(|| format!("{key} 必须是整数"))?;
    let parsed = validate_hermes_i64(Some(parsed), key, 120, 1, 86400)?;
    entry.insert(field.to_string(), Value::Number(parsed.into()));
    Ok(())
}
