
#[tauri::command]
pub fn hermes_tts_voice_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_tts_voice_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_tts_voice_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_terminal_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_terminal_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_terminal_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_terminal_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_terminal_config_values(&config),
    }))
}

// ---------------------------------------------------------------------------
// hermes_read_config — 读取 Hermes config.yaml + .env
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn hermes_read_config() -> Result<Value, String> {
    use super::hermes_providers;

    let home = hermes_home();
    let config_path = home.join("config.yaml");
    let env_path = home.join(".env");
    let _ = sanitize_hermes_openrouter_custom_mismatch();

    // 读取 config.yaml
    let config_raw = std::fs::read_to_string(&config_path).unwrap_or_default();
    let mut model_name = String::new();
    let mut base_url_from_yaml = String::new();
    let mut provider_from_yaml = String::new();
    let mut in_model = false;
    for line in config_raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("model:") {
            in_model = true;
            // `model: "xxx"` 单行格式
            if let Some(v) = trimmed.strip_prefix("model:").map(|s| s.trim().trim_matches('"')) {
                if !v.is_empty() && !v.contains(':') {
                    model_name = v.to_string();
                }
            }
            continue;
        }
        if in_model {
            if trimmed.starts_with("default:") {
                model_name = trimmed
                    .strip_prefix("default:")
                    .unwrap_or_default()
                    .trim()
                    .trim_matches('"')
                    .to_string();
            } else if trimmed.starts_with("base_url:") {
                base_url_from_yaml = trimmed
                    .strip_prefix("base_url:")
                    .unwrap_or_default()
                    .trim()
                    .trim_matches('"')
                    .to_string();
            } else if trimmed.starts_with("provider:") {
                provider_from_yaml = trimmed
                    .strip_prefix("provider:")
                    .unwrap_or_default()
                    .trim()
                    .trim_matches('"')
                    .to_string();
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with('-') {
                in_model = false;
            }
        }
    }

    // 读取 .env 到 key→value map
    let env_raw = std::fs::read_to_string(&env_path).unwrap_or_default();
    let env_map: std::collections::HashMap<String, String> = env_raw
        .lines()
        .filter_map(|line| {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                return None;
            }
            t.split_once('=').map(|(k, v)| (k.trim().to_string(), v.to_string()))
        })
        .collect();

    // 推断 provider：优先 config.yaml.model.provider，其次从 .env 反查
    let mut provider_id: String = if !provider_from_yaml.is_empty() {
        provider_from_yaml.clone()
    } else {
        let keys_refs: Vec<&str> = env_map.keys().map(|s| s.as_str()).collect();
        hermes_providers::infer_provider_from_env_keys(&keys_refs)
            .map(String::from)
            .unwrap_or_default()
    };
    if provider_id == "custom" && env_map.contains_key("AIZUOPIN_API_KEY") {
        let base_hint = if !base_url_from_yaml.is_empty() {
            base_url_from_yaml.clone()
        } else {
            env_map.get("OPENAI_BASE_URL").cloned().unwrap_or_default()
        };
        let base = normalize_provider_url(&base_hint);
        if base == "https://ai.iazp.cn/v1" {
            provider_id = "aizuopin".to_string();
        }
    }

    // 按 provider 的 api_key_env_vars 顺序拿 api_key
    let api_key: String = hermes_providers::get_provider(&provider_id)
        .and_then(|p| p.api_key_env_vars.iter().find_map(|ev| env_map.get(*ev).cloned()))
        .unwrap_or_default();

    // 有效 base_url：优先 config.yaml.model.base_url，其次 provider 的 base_url_env_var
    let effective_base_url: String = if !base_url_from_yaml.is_empty() {
        base_url_from_yaml.clone()
    } else {
        hermes_providers::get_provider(&provider_id)
            .and_then(|p| {
                if p.base_url_env_var.is_empty() {
                    None
                } else {
                    env_map.get(p.base_url_env_var).cloned()
                }
            })
            .unwrap_or_default()
    };

    // UI 显示用短名（去掉 provider/ 前缀），如 openai/QC-S05 → QC-S05
    let display_model = if let Some(pos) = model_name.find('/') {
        model_name[pos + 1..].to_string()
    } else {
        model_name.clone()
    };

    Ok(crate::jv!({
        "model": display_model,
        "model_raw": model_name,
        "base_url": effective_base_url,
        "provider": provider_id,
        "api_key": api_key,
        "config_exists": config_path.exists(),
    }))
}

// ---------------------------------------------------------------------------
// hermes_read_config_full — 解析整个 config.yaml 为 JSON 返回给前端
//
// 与轻量版 hermes_read_config（仅返回 5 个 model 相关字段）互补：
// 前者用于 model 配置页快速展示，本命令用于「高级配置编辑器」让用户能看到/改
// Gateway 端 14+ 个顶层配置项，比如 quick_commands / streaming / reset_triggers /
// stt_enabled / unauthorized_dm_behavior 等。
//
// 返回值结构：
//   {
//     "exists": true,                       // config.yaml 是否存在
//     "raw": "...yaml string...",            // 原文（给 yaml editor）
//     "config": { ...full json... },         // 整份 yaml 转成 JSON
//     "highlights": {                        // 14 个高价值字段单独抽出，前端直接 .x 访问
//       "streaming": {...}, "stt_enabled": true, "quick_commands": {...},
//       "reset_triggers": [...], "default_reset_policy": {...},
//       "unauthorized_dm_behavior": "pair", "session_store_max_age_days": 90,
//       "always_log_local": true,
//       "group_sessions_per_user": false, "thread_sessions_per_user": false,
//       ... 等
//     }
//   }
#[tauri::command]
pub async fn hermes_read_config_full() -> Result<Value, String> {
    let config_path = hermes_home().join("config.yaml");

    if !config_path.exists() {
        return Ok(crate::jv!({
            "exists": false,
            "raw": "",
            "config": {},
            "highlights": {},
        }));
    }

    let raw = std::fs::read_to_string(&config_path).map_err(|e| format!("Failed to read config.yaml: {e}"))?;

    // 解析 YAML → JSON
    let yaml_value: serde_yaml::Value = serde_yaml::from_str(&raw).map_err(|e| format!("Invalid YAML in config.yaml: {e}"))?;
    let config_json: Value = serde_json::to_value(&yaml_value).map_err(|e| format!("YAML→JSON conversion failed: {e}"))?;

    // 抽取 14 个高价值顶层字段（如不存在保持 null，前端按需渲染）
    let highlight_keys = [
        "streaming",
        "stt_enabled",
        "quick_commands",
        "reset_triggers",
        "default_reset_policy",
        "unauthorized_dm_behavior",
        "session_store_max_age_days",
        "always_log_local",
        "group_sessions_per_user",
        "thread_sessions_per_user",
        "platforms",
        "dashboard",
        "memory",
        "skills",
    ];
    let highlights: serde_json::Map<String, Value> = highlight_keys
        .iter()
        .map(|k| {
            let v = config_json.get(*k).cloned().unwrap_or(Value::Null);
            ((*k).to_string(), v)
        })
        .collect();

    Ok(crate::jv!({
        "exists": true,
        "raw": raw,
        "config": config_json,
        "highlights": Value::Object(highlights),
    }))
}

// ---------------------------------------------------------------------------
// hermes_fetch_models — 从 API 获取模型列表（后端代理，避免 CORS）
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn hermes_fetch_models(
    base_url: String,
    api_key: String,
    api_type: Option<String>,
    provider: Option<String>,
) -> Result<Vec<String>, String> {
    use super::hermes_providers;

    // 如果显式指定了 provider，优先走注册表决定 probe 方式 + fallback
    if let Some(pid) = provider.as_ref() {
        if let Some(pcfg) = hermes_providers::get_provider(pid) {
            // OAuth / external_process / copilot → 不能用 api_key 探测，
            // 直接返回静态 catalog
            if pcfg.models_probe == hermes_providers::PROBE_NONE {
                let mut models: Vec<String> = pcfg.models.iter().map(|s| s.to_string()).collect();
                models.sort();
                return Ok(models);
            }
        }
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    // api_type 优先级：调用方 api_type > provider.transport 推断 > 默认 openai
    let api = api_type.unwrap_or_else(|| {
        provider
            .as_ref()
            .and_then(|pid| hermes_providers::get_provider(pid))
            .map(|p| match p.transport {
                hermes_providers::TRANSPORT_ANTHROPIC => "anthropic-messages".to_string(),
                hermes_providers::TRANSPORT_GOOGLE => "google-generative-ai".to_string(),
                _ => "openai".to_string(),
            })
            .unwrap_or_else(|| "openai".into())
    });

    let mut base = base_url.trim_end_matches('/').to_string();
    // 移除尾部的 chat/completions 等路径
    for suffix in &["/chat/completions", "/completions", "/responses", "/messages", "/models"] {
        if base.ends_with(suffix) {
            base = base[..base.len() - suffix.len()].to_string();
        }
    }

    let resp = match api.as_str() {
        "anthropic-messages" => {
            if !base.ends_with("/v1") {
                base.push_str("/v1");
            }
            client
                .get(format!("{base}/models"))
                .header("anthropic-version", "2023-06-01")
                .header("x-api-key", &api_key)
                .send()
                .await
        }
        "google-generative-ai" | "google-gemini" => client.get(format!("{base}/models?key={api_key}")).send().await,
        _ => {
            client
                .get(format!("{base}/models"))
                .header("Authorization", format!("Bearer {api_key}"))
                .send()
                .await
        }
    }
    .map_err(|e| format!("请求失败: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        let short = if body.len() > 200 { &body[..200] } else { &body };
        return Err(format!("HTTP {status}: {short}"));
    }

    let data: Value = resp.json().await.map_err(|e| format!("JSON 解析失败: {e}"))?;

    let models: Vec<String> = if api.contains("google") {
        data.get("models")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(|s| s.replace("models/", "")))
                    .collect()
            })
            .unwrap_or_default()
    } else {
        data.get("data")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("id").and_then(|n| n.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };

    let mut sorted = models;
    sorted.sort();
    Ok(sorted)
}

// ---------------------------------------------------------------------------
// hermes_update_model — 快速切换模型（只改 config.yaml 的 model.default）
// ---------------------------------------------------------------------------

include!("model_gateway_commands/model_gateway_actions.rs");