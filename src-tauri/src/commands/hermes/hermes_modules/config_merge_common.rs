
/// 合并 Hermes config.yaml：只更新 model 区块（default/base_url），
/// 保留用户自定义的 hooks、skills、cron、session 等其他顶级 section。
fn merge_hermes_config_yaml(
    existing: &str,
    model_str: &str,
    base_url_line: &str,
    provider_line: &str,
    api_key_line: &str,
) -> String {
    let mut result = Vec::new();
    let mut in_model_block = false;
    let mut model_block_written = false;
    let lines: Vec<&str> = existing.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed == "model:" || trimmed.starts_with("model:") {
            // 进入 model 区块，写入新的 model 配置
            in_model_block = true;
            model_block_written = true;
            result.push("model:".to_string());
            result.push(format!("  default: {model_str}"));
            if !base_url_line.is_empty() {
                // base_url_line 已包含 "  base_url: xxx\n" 格式
                result.push(base_url_line.trim_end().to_string());
            }
            // provider_line 仅在非空时写入，确保模型路由稳定。
            if !provider_line.is_empty() {
                result.push(provider_line.trim_end().to_string());
            }
            if !api_key_line.is_empty() {
                result.push(api_key_line.trim_end().to_string());
            }
            i += 1;
            // 跳过旧 model 区块的缩进行
            while i < lines.len() {
                let next = lines[i];
                let next_trimmed = next.trim();
                // 空行或缩进行（属于 model 区块）继续跳过
                if next_trimmed.is_empty() {
                    i += 1;
                    continue;
                }
                if next.starts_with("  ") || next.starts_with('\t') {
                    i += 1;
                    continue;
                }
                // 遇到新的顶级 key，停止跳过
                break;
            }
            continue;
        }

        if in_model_block && !trimmed.is_empty() && !line.starts_with("  ") && !line.starts_with('\t') {
            in_model_block = false;
        }

        if !in_model_block {
            result.push(line.to_string());
        }
        i += 1;
    }

    // 如果原文件没有 model: 区块（异常情况），追加
    if !model_block_written {
        result.push("model:".to_string());
        result.push(format!("  default: {model_str}"));
        if !base_url_line.is_empty() {
            result.push(base_url_line.trim_end().to_string());
        }
        if !provider_line.is_empty() {
            result.push(provider_line.trim_end().to_string());
        }
        if !api_key_line.is_empty() {
            result.push(api_key_line.trim_end().to_string());
        }
    }

    // 确保 platform_toolsets 和 platforms 存在（首次合并保底）
    let joined = result.join("\n");
    let mut final_content = joined.clone();
    if !final_content.contains("platform_toolsets:") {
        final_content.push_str("\nplatform_toolsets:\n  api_server:\n    - hermes-api-server\n");
    }
    if !final_content.contains("terminal:") {
        final_content.push_str("terminal:\n  backend: local\n");
    }
    if !final_content.contains("platforms:") {
        final_content.push_str("platforms:\n  api_server:\n    enabled: true\n");
    }
    if !final_content.ends_with('\n') {
        final_content.push('\n');
    }
    final_content
}

/// 合并 .env 文件：更新 managed_keys 对应的值，保留用户自定义的其他环境变量。
fn merge_env_file(existing: &str, managed_keys: &[&str], new_pairs: &[(String, String)]) -> String {
    let mut result = Vec::new();
    let _new_keys: std::collections::HashSet<&str> = new_pairs.iter().map(|(k, _)| k.as_str()).collect();

    // 保留非 managed 的行，跳过 managed 的行（后面追加新值）
    for line in existing.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            result.push(line.to_string());
            continue;
        }
        if let Some(eq_pos) = trimmed.find('=') {
            let key = trimmed[..eq_pos].trim();
            if managed_keys.contains(&key) {
                // 跳过 managed key（后面追加新值）
                continue;
            }
        }
        result.push(line.to_string());
    }

    // 追加新的 managed key=value
    for (k, v) in new_pairs {
        result.push(format!("{k}={v}"));
    }

    let mut content = result.join("\n");
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content
}

// ---------------------------------------------------------------------------
// Hermes 渠道配置 — 读写 ~/.hermes/config.yaml 的 platforms.<platform>，
// 并同步 Hermes 运行时仍会读取的 .env 变量。
// ---------------------------------------------------------------------------

const HERMES_CHANNEL_PLATFORMS: [&str; 10] = [
    "telegram",
    "discord",
    "slack",
    "feishu",
    "dingtalk",
    "teams",
    "google_chat",
    "irc",
    "line",
    "simplex",
];

const HERMES_DISPLAY_TOOL_PROGRESS_VALUES: [&str; 4] = ["off", "new", "all", "verbose"];
const HERMES_DISPLAY_STREAMING_VALUES: [&str; 3] = ["inherit", "true", "false"];
const HERMES_TELEGRAM_REPLY_TO_MODE_VALUES: [&str; 3] = ["off", "first", "all"];
const HERMES_PROMPT_CACHE_TTLS: [&str; 2] = ["5m", "1h"];
const HERMES_PROVIDER_ROUTING_SORTS: [&str; 3] = ["price", "throughput", "latency"];
const HERMES_PROVIDER_ROUTING_DATA_COLLECTION: [&str; 2] = ["allow", "deny"];
const HERMES_AUXILIARY_PROVIDERS: [&str; 7] = ["auto", "openrouter", "nous", "gemini", "ollama-cloud", "codex", "main"];

fn normalize_hermes_channel_platform(platform: &str) -> Option<&'static str> {
    let platform = platform.trim().to_ascii_lowercase();
    HERMES_CHANNEL_PLATFORMS.iter().copied().find(|item| *item == platform)
}

fn normalize_hermes_display_tool_progress(value: Option<String>, strict: bool, key: &str) -> Result<String, String> {
    let progress = value.unwrap_or_default().trim().to_ascii_lowercase();
    let progress = if progress.is_empty() { "all".to_string() } else { progress };
    if HERMES_DISPLAY_TOOL_PROGRESS_VALUES.contains(&progress.as_str()) {
        Ok(progress)
    } else if strict {
        Err(format!("{key} 必须是 off、new、all 或 verbose"))
    } else {
        Ok("all".to_string())
    }
}

fn normalize_hermes_display_tool_prefix(value: Option<String>, strict: bool) -> Result<String, String> {
    let prefix = value.unwrap_or_default().trim().to_string();
    let prefix = if prefix.is_empty() { "┊".to_string() } else { prefix };
    if prefix.chars().count() <= 8 && !prefix.contains(['\r', '\n', '\t']) {
        Ok(prefix)
    } else if strict {
        Err("display.tool_prefix 必须是 1 到 8 个字符，且不能包含换行或制表符".to_string())
    } else {
        Ok("┊".to_string())
    }
}

fn normalize_hermes_display_streaming_text(value: Option<String>, strict: bool, key: &str) -> Result<String, String> {
    let streaming = value.unwrap_or_default().trim().to_ascii_lowercase();
    let streaming = if streaming.is_empty() {
        "inherit".to_string()
    } else {
        streaming
    };
    if HERMES_DISPLAY_STREAMING_VALUES.contains(&streaming.as_str()) {
        Ok(streaming)
    } else if strict {
        Err(format!("{key} 必须是 inherit、true 或 false"))
    } else {
        Ok("inherit".to_string())
    }
}

fn normalize_hermes_telegram_reply_to_mode(value: Option<String>, strict: bool) -> Result<String, String> {
    let mode = value.unwrap_or_default().trim().to_ascii_lowercase();
    let mode = if mode.is_empty() { "first".to_string() } else { mode };
    if HERMES_TELEGRAM_REPLY_TO_MODE_VALUES.contains(&mode.as_str()) {
        Ok(mode)
    } else if strict {
        Err("platforms.telegram.extra.reply_to_mode 必须是 off、first 或 all".to_string())
    } else {
        Ok("first".to_string())
    }
}

fn normalize_hermes_display_streaming_yaml(value: Option<&serde_yaml::Value>, strict: bool, key: &str) -> Result<String, String> {
    if let Some(value) = value {
        if let Some(value) = value.as_bool() {
            return Ok(if value { "true" } else { "false" }.to_string());
        }
        if let Some(value) = value.as_str() {
            return normalize_hermes_display_streaming_text(Some(value.to_string()), strict, key);
        }
    }
    normalize_hermes_display_streaming_text(None, strict, key)
}

fn normalize_hermes_display_streaming_json(value: Option<&Value>, strict: bool, key: &str) -> Result<String, String> {
    if let Some(value) = value {
        if let Some(value) = value.as_bool() {
            return Ok(if value { "true" } else { "false" }.to_string());
        }
        if let Some(value) = value.as_str() {
            return normalize_hermes_display_streaming_text(Some(value.to_string()), strict, key);
        }
    }
    normalize_hermes_display_streaming_text(None, strict, key)
}

fn normalize_hermes_prompt_cache_ttl(value: Option<String>, strict: bool) -> Result<String, String> {
    let ttl = value.unwrap_or_default().trim().to_ascii_lowercase();
    let ttl = if ttl.is_empty() { "5m".to_string() } else { ttl };
    if HERMES_PROMPT_CACHE_TTLS.contains(&ttl.as_str()) {
        Ok(ttl)
    } else if strict {
        Err("prompt_caching.cache_ttl 必须是 5m 或 1h".to_string())
    } else {
        Ok("5m".to_string())
    }
}

fn normalize_hermes_provider_routing_sort(value: Option<String>, strict: bool) -> Result<String, String> {
    let sort = value.unwrap_or_default().trim().to_ascii_lowercase();
    let sort = if sort.is_empty() { "price".to_string() } else { sort };
    if HERMES_PROVIDER_ROUTING_SORTS.contains(&sort.as_str()) {
        Ok(sort)
    } else if strict {
        Err("provider_routing.sort 必须是 price、throughput 或 latency".to_string())
    } else {
        Ok("price".to_string())
    }
}

fn normalize_hermes_provider_routing_data_collection(value: Option<String>, strict: bool) -> Result<String, String> {
    let data_collection = value.unwrap_or_default().trim().to_ascii_lowercase();
    let data_collection = if data_collection.is_empty() {
        "allow".to_string()
    } else {
        data_collection
    };
    if HERMES_PROVIDER_ROUTING_DATA_COLLECTION.contains(&data_collection.as_str()) {
        Ok(data_collection)
    } else if strict {
        Err("provider_routing.data_collection 必须是 allow 或 deny".to_string())
    } else {
        Ok("allow".to_string())
    }
}

fn normalize_hermes_provider_routing_list(raw: Option<String>, key: &str) -> Result<Vec<String>, String> {
    let mut values = Vec::new();
    for item in normalize_hermes_multiline_list(raw) {
        let provider = item.trim().to_ascii_lowercase();
        if provider.is_empty() {
            continue;
        }
        if !provider
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
        {
            return Err(format!("{key} 只能包含 provider slug，每行一个"));
        }
        if !values.contains(&provider) {
            values.push(provider);
        }
    }
    Ok(values)
}

fn normalize_hermes_env_name_list(raw: Option<String>, key: &str) -> Result<Vec<String>, String> {
    let mut values = Vec::new();
    for item in normalize_hermes_multiline_list(raw) {
        let name = item.trim().to_string();
        if name.is_empty() {
            continue;
        }
        let mut chars = name.chars();
        let valid_first = chars.next().map(|ch| ch.is_ascii_alphabetic() || ch == '_').unwrap_or(false);
        let valid_rest = chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
        if !valid_first || !valid_rest {
            return Err(format!("{key} 只能填写环境变量名，每行一个，例如 GITHUB_TOKEN"));
        }
        if !values.contains(&name) {
            values.push(name);
        }
    }
    Ok(values)
}

fn normalize_hermes_shell_init_file_list(raw: Option<String>, key: &str) -> Result<Vec<String>, String> {
    let mut values = Vec::new();
    for item in normalize_hermes_multiline_list(raw) {
        let path = item.trim().to_string();
        if path.is_empty() {
            continue;
        }
        if path.chars().any(|ch| ch.is_control() || ch.is_whitespace()) {
            return Err(format!("{key} 每行只能填写一个 shell 初始化文件路径，路径不能包含空白字符"));
        }
        if !path.chars().all(|ch| {
            ch.is_ascii_alphanumeric() || matches!(ch, '~' | '$' | '%' | '{' | '}' | '_' | '.' | '/' | '\\' | ':' | '-')
        }) {
            return Err(format!("{key} 只能包含路径字符、~、环境变量占位、点、斜杠、冒号和短横线"));
        }
        if !values.contains(&path) {
            values.push(path);
        }
    }
    Ok(values)
}

fn validate_hermes_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    let valid_first = chars.next().map(|ch| ch.is_ascii_alphabetic() || ch == '_').unwrap_or(false);
    valid_first && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn normalize_hermes_docker_env_json(raw: Option<String>, key: &str) -> Result<serde_json::Map<String, Value>, String> {
    let text = raw.unwrap_or_default().trim().to_string();
    if text.is_empty() {
        return Ok(serde_json::Map::new());
    }
    let value: Value = serde_json::from_str(&text).map_err(|err| format!("{key} JSON 格式错误: {err}"))?;
    let object = value.as_object().ok_or_else(|| format!("{key} 必须是 JSON object"))?;
    let mut normalized = serde_json::Map::new();
    for (name, raw_value) in object {
        if !validate_hermes_env_name(name) {
            return Err(format!("{key} 只能使用合法环境变量名作为 key"));
        }
        let value = if let Some(value) = raw_value.as_str() {
            value.to_string()
        } else if let Some(value) = raw_value.as_i64() {
            value.to_string()
        } else if let Some(value) = raw_value.as_u64() {
            value.to_string()
        } else if let Some(value) = raw_value.as_f64() {
            if value.is_finite() {
                value.to_string()
            } else {
                return Err(format!("{key}.{name} 只能是字符串、数字或布尔值"));
            }
        } else if let Some(value) = raw_value.as_bool() {
            value.to_string()
        } else {
            return Err(format!("{key}.{name} 只能是字符串、数字或布尔值"));
        };
        normalized.insert(name.to_string(), Value::String(value));
    }
    Ok(normalized)
}

fn normalize_hermes_docker_volume_list(raw: Option<String>, key: &str) -> Result<Vec<String>, String> {
    let mut values = Vec::new();
    for item in normalize_hermes_multiline_list(raw) {
        let volume = item.trim().to_string();
        if !volume.contains(':') || volume.chars().any(|ch| ch.is_control() || ch.is_whitespace()) {
            return Err(format!("{key} 每行一个 Docker volume 映射，例如 /host/path:/container/path"));
        }
        if !values.contains(&volume) {
            values.push(volume);
        }
    }
    Ok(values)
}

fn normalize_hermes_docker_extra_args_list(raw: Option<String>, key: &str) -> Result<Vec<String>, String> {
    let mut values = Vec::new();
    for item in normalize_hermes_multiline_list(raw) {
        let arg = item.trim().to_string();
        if !arg.starts_with('-') || arg.chars().any(|ch| ch.is_control() || ch.is_whitespace()) {
            return Err(format!("{key} 每行一个 Docker 参数，必须以 - 开头，例如 --network=host"));
        }
        if !values.contains(&arg) {
            values.push(arg);
        }
    }
    Ok(values)
}

fn normalize_hermes_auxiliary_provider(value: Option<String>, key: &str, strict: bool) -> Result<String, String> {
    let provider = value.unwrap_or_default().trim().to_ascii_lowercase();
    let provider = if provider.is_empty() { "auto".to_string() } else { provider };
    if HERMES_AUXILIARY_PROVIDERS.contains(&provider.as_str()) {
        Ok(provider)
    } else if strict {
        Err(format!("{key} 必须是 auto、openrouter、nous、gemini、ollama-cloud、codex 或 main"))
    } else {
        Ok("auto".to_string())
    }
}

fn normalize_hermes_auxiliary_model(value: Option<String>, key: &str, strict: bool) -> Result<String, String> {
    let model = value.unwrap_or_default().trim().to_string();
    if model.is_empty() {
        return Ok(String::new());
    }
    if !model.split('/').any(|part| part == "..")
        && model
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '/' | ':' | '@' | '+' | '-'))
    {
        Ok(model)
    } else if strict {
        Err(format!("{key} 只能包含字母、数字、下划线、点、斜杠、冒号、@、加号和短横线"))
    } else {
        Ok(String::new())
    }
}

fn yaml_key(key: &str) -> serde_yaml::Value {
    serde_yaml::Value::String(key.to_string())
}

fn yaml_get<'a>(map: &'a serde_yaml::Mapping, key: &str) -> Option<&'a serde_yaml::Value> {
    map.get(yaml_key(key))
}

fn yaml_get_mapping<'a>(map: &'a serde_yaml::Mapping, key: &str) -> Option<&'a serde_yaml::Mapping> {
    yaml_get(map, key).and_then(|v| v.as_mapping())
}

fn yaml_string_field(map: &serde_yaml::Mapping, key: &str) -> Option<String> {
    yaml_get(map, key).and_then(|v| v.as_str()).map(|v| v.to_string())
}

fn set_optional_yaml_string(map: &mut serde_yaml::Mapping, key: &str, value: String) {
    if value.is_empty() {
        map.remove(yaml_key(key));
    } else {
        map.insert(yaml_key(key), serde_yaml::Value::String(value));
    }
}

fn normalize_hermes_camofox_identity(value: Option<String>, key: &str) -> Result<String, String> {
    let text = value.unwrap_or_default().trim().to_string();
    if text.is_empty() {
        return Ok(String::new());
    }
    if text
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | ':' | '@' | '+' | '-'))
    {
        Ok(text)
    } else {
        Err(format!("{key} 只能包含字母、数字、下划线、点、冒号、@、加号和短横线"))
    }
}

fn yaml_string_sequence_field(map: &serde_yaml::Mapping, key: &str) -> Vec<String> {
    yaml_get(map, key)
        .and_then(|value| value.as_sequence())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn yaml_docker_env_json_field(map: Option<&serde_yaml::Mapping>, key: &str) -> String {
    let Some(env_map) = map.and_then(|map| yaml_get(map, key)).and_then(|value| value.as_mapping()) else {
        return "{}".to_string();
    };
    let mut lines = Vec::new();
    for (raw_key, raw_value) in env_map {
        let Some(name) = raw_key.as_str() else {
            continue;
        };
        if !validate_hermes_env_name(name) {
            continue;
        }
        let value = if let Some(value) = raw_value.as_str() {
            value.to_string()
        } else if let Some(value) = raw_value.as_i64() {
            value.to_string()
        } else if let Some(value) = raw_value.as_u64() {
            value.to_string()
        } else if let Some(value) = raw_value.as_f64() {
            if value.is_finite() {
                value.to_string()
            } else {
                continue;
            }
        } else if let Some(value) = raw_value.as_bool() {
            value.to_string()
        } else {
            continue;
        };
        let encoded_name = serde_json::to_string(name).unwrap_or_else(|_| "\"\"".to_string());
        let encoded_value = serde_json::to_string(&value).unwrap_or_else(|_| "\"\"".to_string());
        lines.push(format!("  {encoded_name}: {encoded_value}"));
    }
    if lines.is_empty() {
        "{}".to_string()
    } else {
        format!("{{\n{}\n}}", lines.join(",\n"))
    }
}

fn yaml_scalar_string_field(map: &serde_yaml::Mapping, key: &str) -> Option<String> {
    let value = yaml_get(map, key)?;
    if let Some(value) = value.as_str() {
        Some(value.to_string())
    } else if let Some(value) = value.as_i64() {
        Some(value.to_string())
    } else if let Some(value) = value.as_u64() {
        Some(value.to_string())
    } else {
        value.as_f64().map(|value| {
            if value.fract() == 0.0 {
                format!("{value:.0}")
            } else {
                value.to_string()
            }
        })
    }
}

fn yaml_bool_field(map: &serde_yaml::Mapping, key: &str) -> Option<bool> {
    yaml_get(map, key).and_then(|v| v.as_bool())
}

fn yaml_csv_field(map: &serde_yaml::Mapping, key: &str) -> Option<String> {
    let value = yaml_get(map, key)?;
    if let Some(items) = value.as_sequence() {
        let joined = items
            .iter()
            .filter_map(|item| item.as_str().map(str::trim))
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
        if joined.is_empty() {
            None
        } else {
            Some(joined)
        }
    } else {
        value.as_str().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
    }
}

fn insert_json_string_if_present(
    form: &mut serde_json::Map<String, Value>,
    source: &serde_yaml::Mapping,
    yaml_key: &str,
    json_key: &str,
) {
    if let Some(value) = yaml_string_field(source, yaml_key) {
        form.insert(json_key.to_string(), Value::String(value));
    }
}

fn insert_json_scalar_string_if_present(
    form: &mut serde_json::Map<String, Value>,
    source: &serde_yaml::Mapping,
    yaml_key: &str,
    json_key: &str,
) {
    if let Some(value) = yaml_scalar_string_field(source, yaml_key) {
        form.insert(json_key.to_string(), Value::String(value));
    }
}

fn insert_json_bool_if_present(
    form: &mut serde_json::Map<String, Value>,
    source: &serde_yaml::Mapping,
    yaml_key: &str,
    json_key: &str,
) {
    if let Some(value) = yaml_bool_field(source, yaml_key) {
        form.insert(json_key.to_string(), Value::Bool(value));
    }
}

fn insert_json_csv_if_present(
    form: &mut serde_json::Map<String, Value>,
    source: &serde_yaml::Mapping,
    yaml_key: &str,
    json_key: &str,
) {
    if let Some(value) = yaml_csv_field(source, yaml_key) {
        form.insert(json_key.to_string(), Value::String(value));
    }
}

fn hermes_env_value(env_values: &std::collections::HashMap<String, String>, key: &str) -> Option<String> {
    env_values
        .get(key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn read_hermes_channel_env_values() -> std::collections::HashMap<String, String> {
    let env_path = hermes_home().join(".env");
    let raw = std::fs::read_to_string(&env_path).unwrap_or_default();
    let mut values = std::collections::HashMap::new();
    for (key, value, _) in parse_env_file_lines(&raw) {
        values.entry(key).or_insert(value);
    }
    values
}

fn json_form_string(form: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    form.get(key).and_then(|value| value.as_str()).map(|value| value.to_string())
}

fn put_json_string_from_env(
    form: &mut serde_json::Map<String, Value>,
    env_values: &std::collections::HashMap<String, String>,
    env_key: &str,
    json_key: &str,
) {
    if let Some(value) = hermes_env_value(env_values, env_key) {
        form.insert(json_key.to_string(), Value::String(value));
    }
}
