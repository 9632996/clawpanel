use serde::Deserialize;
use serde_json::{Map, Value};
use std::{fs, path::PathBuf};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyModelInput {
    id: Option<String>,
    name: Option<String>,
    base_url: String,
    api_key: String,
    model: String,
    provider: Option<String>,
    protocol: Option<String>,
    relay_mode: Option<bool>,
}

struct ToolTemplate {
    id: &'static str,
    name: &'static str,
    category: &'static str,
    files: &'static [&'static str],
    protocols: &'static [&'static str],
}

const TOOLS: &[ToolTemplate] = &[
    ToolTemplate {
        id: "openclaw",
        name: "OpenClaw",
        category: "agent",
        files: &["data/config/openclaw.json", "data/config/model-credentials.env"],
        protocols: &["openai"],
    },
    ToolTemplate {
        id: "hermes",
        name: "Hermes Agent",
        category: "agent",
        files: &["data/config/hermes/config.yaml", "data/config/hermes/.env"],
        protocols: &["openai"],
    },
    ToolTemplate {
        id: "codex",
        name: "Codex CLI",
        category: "coding-agent",
        files: &["data/config/codex/config.toml", "data/config/codex/auth.json"],
        protocols: &["openai", "responses"],
    },
    ToolTemplate {
        id: "codewhale",
        name: "CodeWhale",
        category: "coding-agent",
        files: &["data/config/codewhale/config.toml"],
        protocols: &["openai"],
    },
];

#[tauri::command]
pub fn list_model_tools() -> Result<Value, String> {
    Ok(Value::Array(
        TOOLS
            .iter()
            .map(|t| {
                crate::jv!({
                    "id": t.id,
                    "name": t.name,
                    "category": t.category,
                    "configFiles": t.files,
                    "apiProtocol": t.protocols,
                })
            })
            .collect(),
    ))
}

#[tauri::command]
pub async fn apply_model_to_tool(tool_id: String, model_info: ApplyModelInput) -> Result<Value, String> {
    let tool = TOOLS
        .iter()
        .find(|t| t.id == tool_id)
        .ok_or_else(|| format!("不支持的工具: {tool_id}"))?;
    let model = ResolvedModel::from_input(model_info)?;
    write_model_credentials(&model)?;
    let files = match tool.id {
        "openclaw" => apply_openclaw_model(&model)?,
        "hermes" => apply_hermes_model(&model).await?,
        "codex" => apply_codex_model(&model)?,
        "codewhale" => apply_codewhale_model(&model)?,
        _ => return Err(format!("不支持的工具: {}", tool.id)),
    };
    Ok(crate::jv!({
        "success": true,
        "toolId": tool.id,
        "provider": model.provider,
        "activeModel": model.model,
        "files": files,
        "message": format!("已将 {} 应用到 {}", model.model, tool.name),
    }))
}

#[tauri::command]
pub fn restore_tool_model_config(tool_id: String) -> Result<Value, String> {
    let tool = TOOLS
        .iter()
        .find(|t| t.id == tool_id)
        .ok_or_else(|| format!("不支持的工具: {tool_id}"))?;
    if matches!(tool.id, "codex" | "codewhale") {
        let dir = config_dir().join(tool.id);
        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(|e| format!("清理 {} 失败: {e}", dir.display()))?;
        }
    }
    Ok(crate::jv!({ "success": true, "toolId": tool.id }))
}

struct ResolvedModel {
    provider: String,
    name: String,
    base_url: String,
    api_key: String,
    model: String,
    protocol: String,
    env_key: String,
    relay_mode: bool,
}

impl ResolvedModel {
    fn from_input(input: ApplyModelInput) -> Result<Self, String> {
        let provider = input
            .provider
            .or_else(|| input.id.as_deref().and_then(|id| id.split('/').next()).map(str::to_string))
            .unwrap_or_else(|| "custom".to_string());
        validate_id(&provider, "provider")?;
        let model = input.model.trim().to_string();
        if model.is_empty() {
            return Err("模型不能为空".into());
        }
        let base_url = normalize_base_url(&input.base_url);
        if base_url.is_empty() {
            return Err("Base URL 不能为空".into());
        }
        let env_key = provider_env_key(&provider);
        let api_key = resolve_api_key(input.api_key.trim(), &env_key)?;
        if api_key.is_empty() {
            return Err("API Key 不能为空".into());
        }
        Ok(Self {
            provider,
            name: input.name.unwrap_or_default(),
            base_url,
            api_key,
            model,
            protocol: input.protocol.unwrap_or_else(|| "openai".into()),
            env_key,
            relay_mode: input.relay_mode.unwrap_or(true),
        })
    }
}

fn root_dir() -> PathBuf {
    super::portable_product_root().unwrap_or_else(super::openclaw_dir)
}

fn config_dir() -> PathBuf {
    let root = root_dir();
    let portable = root.join("data").join("config");
    if portable.is_dir() || root.join("app").is_dir() {
        portable
    } else {
        root
    }
}

fn validate_id(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} 不能为空"));
    }
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        Ok(())
    } else {
        Err(format!("{label} 只能包含字母、数字、-、_、."))
    }
}

fn normalize_base_url(value: &str) -> String {
    let value = value.trim().trim_end_matches('/');
    if value.eq_ignore_ascii_case("https://ai.iazp.cn") {
        "https://ai.iazp.cn/v1".into()
    } else {
        value.into()
    }
}

fn provider_env_key(provider: &str) -> String {
    match provider.to_ascii_lowercase().as_str() {
        "aizuopin" => "AIZUOPIN_API_KEY".into(),
        "openai" => "OPENAI_API_KEY".into(),
        "deepseek" | "deepseek-cn" | "deepseek-china" | "deepseek_china" => "DEEPSEEK_API_KEY".into(),
        "moonshot" => "MOONSHOT_API_KEY".into(),
        "xiaomi" | "mimo" | "xiaomi-mimo" | "xiaomi_mimo" => "MIMO_API_KEY".into(),
        other => format!("{}_API_KEY", other.replace(['-', '.'], "_").to_ascii_uppercase()),
    }
}

fn resolve_api_key(input: &str, fallback_env: &str) -> Result<String, String> {
    if input.is_empty() {
        return Ok(read_secret(fallback_env).unwrap_or_default());
    }
    if let Some(env) = input.strip_prefix("$env:") {
        let env = env.trim();
        return read_secret(env).ok_or_else(|| format!("未找到环境变量或凭据: {env}"));
    }
    Ok(input.to_string())
}

fn read_secret(env_key: &str) -> Option<String> {
    std::env::var(env_key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .or_else(|| read_credentials_file(env_key))
}

fn read_credentials_file(env_key: &str) -> Option<String> {
    let content = fs::read_to_string(config_dir().join("model-credentials.env")).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        if key.trim() == env_key {
            let value = value.trim().to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn write_model_credentials(model: &ResolvedModel) -> Result<PathBuf, String> {
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("创建配置目录失败: {e}"))?;
    let path = dir.join("model-credentials.env");
    let managed = [model.env_key.as_str(), "AIZUOPIN_API_KEY", "OPENAI_API_KEY", "CUSTOM_API_KEY"];
    let mut lines = Vec::new();
    if let Ok(existing) = fs::read_to_string(&path) {
        for line in existing.lines() {
            let keep = line
                .trim()
                .split_once('=')
                .map(|(k, _)| !managed.contains(&k.trim()))
                .unwrap_or(true);
            if keep {
                lines.push(line.to_string());
            }
        }
    }
    lines.push(format!("{}={}", model.env_key, model.api_key));
    for alias in ["AIZUOPIN_API_KEY", "OPENAI_API_KEY", "CUSTOM_API_KEY"] {
        if alias != model.env_key {
            lines.push(format!("{alias}={}", model.api_key));
        }
    }
    fs::write(&path, format!("{}\n", lines.join("\n"))).map_err(|e| format!("写入 {} 失败: {e}", path.display()))?;
    Ok(path)
}

fn escape_toml(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn escape_toml_key(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .collect()
}

fn display_name(model: &ResolvedModel) -> String {
    if model.name.trim().is_empty() {
        model.provider.clone()
    } else {
        model.name.clone()
    }
}

fn codewhale_cli_provider(provider: &str) -> &str {
    if provider.eq_ignore_ascii_case("aizuopin") {
        "openai"
    } else {
        provider
    }
}

fn apply_openclaw_model(model: &ResolvedModel) -> Result<Vec<String>, String> {
    let mut config = crate::commands::config::load_openclaw_json().unwrap_or_else(|_| crate::jv!({}));
    ensure_object_path(&mut config, &["models", "providers"])?;
    let providers = config
        .pointer_mut("/models/providers")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "models.providers 不是对象".to_string())?;
    let provider = providers
        .entry(model.provider.clone())
        .or_insert_with(|| crate::jv!({}))
        .as_object_mut()
        .ok_or_else(|| "provider 配置不是对象".to_string())?;
    provider.insert("baseUrl".into(), Value::String(model.base_url.clone()));
    provider.insert("apiKey".into(), crate::jv!({ "$env": model.env_key }));
    provider.insert("api".into(), Value::String(model.protocol.clone()));
    ensure_model_in_provider(provider, &model.model);
    ensure_object_path(&mut config, &["agents", "defaults", "model"])?;
    config["agents"]["defaults"]["model"]["primary"] = Value::String(format!("{}/{}", model.provider, model.model));
    crate::commands::config::save_openclaw_json(&config)?;
    Ok(vec![super::openclaw_dir().join("openclaw.json").display().to_string()])
}

async fn apply_hermes_model(model: &ResolvedModel) -> Result<Vec<String>, String> {
    let result = crate::commands::hermes::configure_hermes(
        model.provider.clone(),
        model.api_key.clone(),
        Some(model.model.clone()),
        Some(model.base_url.clone()),
    )
    .await?;
    update_panel_engine_model("hermes", model)?;
    Ok(vec![result])
}

fn apply_codex_model(model: &ResolvedModel) -> Result<Vec<String>, String> {
    let home = config_dir().join("codex");
    fs::create_dir_all(&home).map_err(|e| format!("创建 Codex 配置目录失败: {e}"))?;
    let wire_api = if model.relay_mode || model.provider.eq_ignore_ascii_case("aizuopin") {
        "responses"
    } else {
        "chat"
    };
    let config_path = home.join("config.toml");
    let content = format!(
        r#"# Codex configuration (managed by Zhizhua Workbench)
model = "{model}"
model_provider = "{provider}"
approval_policy = "on-request"
sandbox_mode = "workspace-write"
preferred_auth_method = "apikey"
check_for_update_on_startup = false

[model_providers.{provider}]
name = "{name}"
base_url = "{base_url}"
env_key = "{env_key}"
wire_api = "{wire_api}"
requires_openai_auth = false
supports_websockets = false
"#,
        model = escape_toml(&model.model),
        provider = escape_toml_key(&model.provider),
        name = escape_toml(&display_name(model)),
        base_url = escape_toml(&model.base_url),
        env_key = escape_toml(&model.env_key),
        wire_api = wire_api,
    );
    fs::write(&config_path, content).map_err(|e| format!("写入 {} 失败: {e}", config_path.display()))?;

    let auth_path = home.join("auth.json");
    let auth = serde_json::to_string_pretty(&crate::jv!({
        "OPENAI_API_KEY": model.api_key,
        "tokens": crate::jv!(null),
        "last_refresh": crate::jv!(null),
    }))
    .map_err(|e| format!("序列化 auth.json 失败: {e}"))?;
    fs::write(&auth_path, auth).map_err(|e| format!("写入 {} 失败: {e}", auth_path.display()))?;
    update_panel_engine_model("codex", model)?;
    Ok(vec![config_path.display().to_string(), auth_path.display().to_string()])
}

fn apply_codewhale_model(model: &ResolvedModel) -> Result<Vec<String>, String> {
    let home = config_dir().join("codewhale");
    fs::create_dir_all(&home).map_err(|e| format!("创建 CodeWhale 配置目录失败: {e}"))?;
    let provider = codewhale_cli_provider(&model.provider);
    let config_path = home.join("config.toml");
    let content = format!(
        r#"# CodeWhale configuration (managed by Zhizhua Workbench)
provider = "{provider}"
model = "{model}"
auth_mode = "api_key"

[providers.{provider}]
base_url = "{base_url}"
api_key = "{api_key}"
"#,
        provider = escape_toml(provider),
        model = escape_toml(&model.model),
        base_url = escape_toml(&model.base_url),
        api_key = escape_toml(&model.api_key),
    );
    fs::write(&config_path, content).map_err(|e| format!("写入 {} 失败: {e}", config_path.display()))?;
    update_panel_engine_model("codewhale", model)?;
    Ok(vec![config_path.display().to_string()])
}

fn ensure_object_path(value: &mut Value, path: &[&str]) -> Result<(), String> {
    let mut current = value;
    for key in path {
        if !current.is_object() {
            *current = Value::Object(Map::new());
        }
        let object = current.as_object_mut().ok_or_else(|| format!("{key} 不是对象"))?;
        current = object.entry((*key).to_string()).or_insert_with(|| Value::Object(Map::new()));
    }
    if !current.is_object() {
        *current = Value::Object(Map::new());
    }
    Ok(())
}

fn ensure_model_in_provider(provider: &mut Map<String, Value>, model_id: &str) {
    let entry = provider.entry("models").or_insert_with(|| Value::Array(Vec::new()));
    if !entry.is_array() {
        *entry = Value::Array(Vec::new());
    }
    let Some(models) = entry.as_array_mut() else {
        return;
    };
    let exists = models
        .iter()
        .any(|item| item.as_str() == Some(model_id) || item.get("id").and_then(Value::as_str) == Some(model_id));
    if !exists {
        models.push(crate::jv!({ "id": model_id, "input": ["text", "image"] }));
    }
}

fn update_panel_engine_model(engine: &str, model: &ResolvedModel) -> Result<(), String> {
    let mut panel = super::read_panel_config_value().unwrap_or_else(|| crate::jv!({}));
    if !panel.is_object() {
        panel = crate::jv!({});
    }
    let obj = panel.as_object_mut().ok_or_else(|| "面板配置不是对象".to_string())?;
    let section = obj
        .entry(engine.to_string())
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| format!("{engine} 配置不是对象"))?;
    section.insert("provider".into(), Value::String(model.provider.clone()));
    section.insert("baseUrl".into(), Value::String(model.base_url.clone()));
    section.insert("model".into(), Value::String(model.model.clone()));
    let providers = section
        .entry("providers")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| format!("{engine}.providers 不是对象"))?;
    providers.insert(
        model.provider.clone(),
        crate::jv!({
            "name": display_name(model),
            "baseUrl": model.base_url,
            "model": model.model,
            "envKey": model.env_key,
        }),
    );

    let root = root_dir();
    let path = if root.join("data").join("config").is_dir() {
        root.join("data").join("config").join("zhizhua-workbench.json")
    } else {
        super::panel_config_path()
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建 {} 失败: {e}", parent.display()))?;
    }
    let data = serde_json::to_string_pretty(&panel).map_err(|e| format!("序列化面板配置失败: {e}"))?;
    fs::write(&path, data).map_err(|e| format!("写入 {} 失败: {e}", path.display()))
}
