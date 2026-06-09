use serde_json::Value;
use std::fs;

use super::config_model_common::{find_json_string, first_env_ref, home_path, is_valid_env_key};

#[allow(clippy::too_many_arguments)]
fn push_client_candidate(
    out: &mut Vec<Value>,
    id: &str,
    source: &str,
    source_path: &str,
    provider_key: &str,
    display_name: &str,
    base_url: &str,
    api: &str,
    api_key: &str,
    api_key_status: &str,
    models: Vec<String>,
    importable: bool,
    auth_hint: &str,
    warning: &str,
) {
    out.push(crate::jv!({
        "id": id,
        "source": source,
        "sourcePath": source_path,
        "providerKey": provider_key,
        "displayName": display_name,
        "baseUrl": base_url,
        "api": api,
        "apiKey": api_key,
        "apiKeyStatus": api_key_status,
        "models": models,
        "importable": importable,
        "authHint": auth_hint,
        "warning": warning,
    }));
}

#[allow(clippy::too_many_arguments)]
fn scan_json_client_file(
    out: &mut Vec<Value>,
    id: &str,
    source: &str,
    parts: &[&str],
    provider_key: &str,
    display_name: &str,
    base_url: &str,
    api: &str,
    env_keys: &[&str],
    default_model: &str,
) {
    let Some(path) = home_path(parts) else {
        return;
    };
    if !path.exists() {
        return;
    }
    let model = fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|value| find_json_string(&value, &["model", "defaultModel", "modelName"], 0))
        .unwrap_or_else(|| default_model.to_string());
    let (api_key, status) = first_env_ref(env_keys);
    let warning = if status == "missing" {
        "未在当前进程环境中检测到对应 API Key 环境变量，导入后需要在 OpenClaw env 或 .env 中补齐。"
    } else {
        ""
    };
    push_client_candidate(
        out,
        id,
        source,
        &path.to_string_lossy(),
        provider_key,
        display_name,
        base_url,
        api,
        &api_key,
        &status,
        vec![model],
        true,
        "",
        warning,
    );
}

#[tauri::command]
pub fn scan_model_client_configs() -> Result<Value, String> {
    let mut candidates = Vec::new();
    if let Some(path) = home_path(&[".codex", "config.toml"]) {
        if let Ok(raw) = fs::read_to_string(&path) {
            let blocks = super::config_model_runtime::parse_simple_config_blocks(&raw);
            let root = blocks.get("").cloned().unwrap_or_default();
            let provider_id = root.get("model_provider").cloned().unwrap_or_else(|| "openai".into());
            let section = blocks
                .get(&format!("model_providers.{provider_id}"))
                .cloned()
                .unwrap_or_default();
            let model = root
                .get("model")
                .cloned()
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "gpt-5.1-codex-mini".into());
            let base_url = section.get("base_url").cloned().unwrap_or_else(|| {
                if provider_id.contains("codex") {
                    "https://chatgpt.com/backend-api/codex".into()
                } else {
                    "https://api.openai.com/v1".into()
                }
            });
            let wire_api = section.get("wire_api").cloned().unwrap_or_default();
            let explicit_env_key = section.get("env_key").cloned().filter(|v| is_valid_env_key(v));
            let env_key = explicit_env_key.or_else(|| {
                if provider_id == "openai" {
                    Some("OPENAI_API_KEY".into())
                } else {
                    None
                }
            });
            let is_external_codex = provider_id.contains("codex") || base_url.contains("chatgpt.com/backend-api/codex");
            let api = if is_external_codex {
                "openai-codex-responses"
            } else if wire_api.contains("responses") {
                "openai-responses"
            } else {
                "openai-completions"
            };
            let (api_key, status) = if let Some(key) = env_key.as_deref() {
                if std::env::var(key).map(|v| !v.trim().is_empty()).unwrap_or(false) {
                    (format!("${{{key}}}"), "found")
                } else {
                    (format!("${{{key}}}"), "missing")
                }
            } else {
                (String::new(), "none")
            };
            let provider_key = if provider_id == "openai" {
                "codex-openai".to_string()
            } else {
                format!("codex-{provider_id}")
            };
            let warning = if is_external_codex {
                "ChatGPT/Codex OAuth 令牌不会导入到 OpenClaw。请优先使用 Hermes 的 openai-codex 登录。"
            } else if status == "none" {
                "Codex 配置没有声明可安全引用的 env_key，无法自动导入 API Key。请在 Codex 配置中添加 env_key，或在 OpenClaw 中手动配置服务商密钥。"
            } else if status == "missing" {
                "未在当前进程环境中检测到 Codex 配置引用的 API Key 环境变量，导入后需要在 OpenClaw env 或 .env 中补齐。"
            } else {
                ""
            };
            push_client_candidate(
                &mut candidates,
                "codex-cli",
                "Codex CLI",
                &path.to_string_lossy(),
                &provider_key,
                &format!("Codex CLI / {provider_id}"),
                &base_url,
                api,
                &api_key,
                status,
                vec![model],
                !is_external_codex && status != "none",
                if is_external_codex {
                    "hermes auth login openai-codex"
                } else {
                    ""
                },
                warning,
            );
        }
    }
    scan_json_client_file(
        &mut candidates,
        "claude-code",
        "Claude Code",
        &[".claude", "settings.json"],
        "anthropic",
        "Anthropic / Claude Code",
        "https://api.anthropic.com/v1",
        "anthropic-messages",
        &["ANTHROPIC_API_KEY", "ANTHROPIC_TOKEN"],
        "claude-sonnet-4-5-20250514",
    );
    scan_json_client_file(
        &mut candidates,
        "gemini-cli",
        "Gemini CLI",
        &[".gemini", "settings.json"],
        "google",
        "Google Gemini CLI",
        "https://generativelanguage.googleapis.com/v1beta",
        "google-generative-ai",
        &["GEMINI_API_KEY", "GOOGLE_API_KEY"],
        "gemini-2.5-pro",
    );
    for (env_key, provider_key, display_name, base_url, api, model) in [
        (
            "OPENAI_API_KEY",
            "openai-env",
            "OpenAI 环境变量",
            std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".into()),
            "openai-completions",
            std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".into()),
        ),
        (
            "ANTHROPIC_API_KEY",
            "anthropic-env",
            "Anthropic 环境变量",
            "https://api.anthropic.com/v1".into(),
            "anthropic-messages",
            std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| "claude-sonnet-4-5-20250514".into()),
        ),
        (
            "GEMINI_API_KEY",
            "gemini-env",
            "Gemini 环境变量",
            "https://generativelanguage.googleapis.com/v1beta".into(),
            "google-generative-ai",
            std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-2.5-pro".into()),
        ),
    ] {
        if std::env::var(env_key).map(|v| !v.trim().is_empty()).unwrap_or(false) {
            push_client_candidate(
                &mut candidates,
                provider_key,
                "Environment",
                env_key,
                provider_key,
                display_name,
                &base_url,
                api,
                &format!("${{{env_key}}}"),
                "found",
                vec![model],
                true,
                "",
                "",
            );
        }
    }
    Ok(crate::jv!({ "candidates": candidates }))
}
