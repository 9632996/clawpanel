//! Hermes Provider Registry — ClawPanel's built-in provider catalog
//! by Hermes Agent, with their auth schemes, env vars, base URLs, and known
//! model catalogs.
//!
//! This module is intentionally self-contained: it must NOT depend on any
//! runtime state. The static data is queried by commands in `hermes.rs`
//! and surfaced to the frontend via `hermes_list_providers`.

use serde::Serialize;

// =============================================================================
// Data model
// =============================================================================

/// - `api_key`: traditional env-var based key (`<PROVIDER>_API_KEY`, etc.)
/// - `oauth_device_code`: interactive device-code OAuth flow (Nous)
/// - `oauth_external`: OAuth handled by external process (Codex, Qwen)
/// - `external_process`: backing process handles auth (Copilot ACP)
pub const AUTH_API_KEY: &str = "api_key";
pub const AUTH_OAUTH_DEVICE: &str = "oauth_device_code";
pub const AUTH_OAUTH_EXTERNAL: &str = "oauth_external";
pub const AUTH_EXTERNAL_PROCESS: &str = "external_process";
pub const AUTH_AWS_SDK: &str = "aws_sdk";
pub const AUTH_OAUTH_MINIMAX: &str = "oauth_minimax";

/// Transport negotiated with the provider.
pub const TRANSPORT_OPENAI_CHAT: &str = "openai_chat";
pub const TRANSPORT_ANTHROPIC: &str = "anthropic_messages";
pub const TRANSPORT_GOOGLE: &str = "google_gemini";
pub const TRANSPORT_CODEX: &str = "codex_responses";

/// `/models` probe strategy used by `hermes_fetch_models`.
///
/// Note: all OpenAI-compatible providers (including Gemini via its OpenAI
/// adapter) use `PROBE_OPENAI`. A separate `PROBE_GOOGLE` was considered for
/// native Google Gemini API probing, but in practice every provider we
/// support uses one of these three strategies.
pub const PROBE_OPENAI: &str = "openai";
pub const PROBE_ANTHROPIC: &str = "anthropic";
pub const PROBE_NONE: &str = "none";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HermesProvider {
    /// Stable provider identifier.
    pub id: &'static str,
    /// Human-readable display name.
    pub name: &'static str,
    /// See AUTH_* constants above.
    pub auth_type: &'static str,
    /// Default inference base URL.
    pub base_url: &'static str,
    /// Env var name for overriding `base_url` (empty string = none).
    pub base_url_env_var: &'static str,
    /// Env vars checked in priority order for API key (empty for OAuth/external).
    pub api_key_env_vars: &'static [&'static str],
    /// See TRANSPORT_* constants above.
    pub transport: &'static str,
    /// See PROBE_* constants above.
    pub models_probe: &'static str,
    /// Known static model list.
    pub models: &'static [&'static str],
    /// True for aggregators/routers (OpenRouter, AI Gateway, etc.) — users
    /// must explicitly specify a model since there is no sensible default.
    pub is_aggregator: bool,
    /// Hint for the UI when the CLI must be used for login (OAuth providers).
    pub cli_auth_hint: &'static str,
}

// =============================================================================
// Static registry
// =============================================================================

include!("hermes_providers_modules/catalog.rs");

// =============================================================================
// Query helpers
// =============================================================================

/// Look up a provider by stable id.
pub fn get_provider(id: &str) -> Option<&'static HermesProvider> {
    ALL_PROVIDERS.iter().find(|p| p.id == id)
}

/// Primary env var for writing the API key for a given provider.
/// Returns `None` for OAuth / external_process providers.
pub fn primary_api_key_env(provider_id: &str) -> Option<&'static str> {
    get_provider(provider_id).and_then(|p| p.api_key_env_vars.first().copied())
}

/// Env var for overriding the base URL (empty string if provider has no such var).
pub fn primary_base_url_env(provider_id: &str) -> Option<&'static str> {
    get_provider(provider_id).and_then(|p| {
        if p.base_url_env_var.is_empty() {
            None
        } else {
            Some(p.base_url_env_var)
        }
    })
}

/// All env var keys that ClawPanel manages across every provider.
/// Used by `configure_hermes::merge_env_file` to know which keys to clear
/// when the user switches providers. This is the union of:
///   - all `api_key_env_vars` across providers
///   - all non-empty `base_url_env_var` values
///   - the two ClawPanel-specific env vars (`GATEWAY_ALLOW_ALL_USERS`,
///     `API_SERVER_KEY`)
pub fn all_managed_env_keys() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for p in ALL_PROVIDERS {
        for ev in p.api_key_env_vars {
            if !out.contains(ev) {
                out.push(ev);
            }
        }
        if !p.base_url_env_var.is_empty() && !out.contains(&p.base_url_env_var) {
            out.push(p.base_url_env_var);
        }
    }
    // ClawPanel-specific keys
    for extra in &[
        "GATEWAY_ALLOW_ALL_USERS",
        "API_SERVER_KEY",
        "AIZUOPIN_API_KEY",
        "CUSTOM_BASE_URL",
    ] {
        if !out.contains(extra) {
            out.push(extra);
        }
    }
    out
}

/// Given the set of env var keys present in a `.env` file, infer the most
/// likely provider. Priority follows `ALL_PROVIDERS` order, so users who have
/// multiple provider keys set will be identified with the first matching
/// canonical provider.
pub fn infer_provider_from_env_keys(keys: &[&str]) -> Option<&'static str> {
    for p in ALL_PROVIDERS {
        if p.api_key_env_vars.is_empty() {
            continue; // Skip OAuth/external
        }
        for ev in p.api_key_env_vars {
            if keys.contains(ev) {
                return Some(p.id);
            }
        }
    }
    None
}

/// Find the first provider whose static model catalog contains the given model
/// name (exact match). Returns `None` on ambiguity (multiple matches) or miss.
pub fn find_provider_by_model(model: &str) -> Option<&'static str> {
    let hits: Vec<&'static str> = ALL_PROVIDERS
        .iter()
        .filter(|p| p.models.contains(&model))
        .map(|p| p.id)
        .collect();
    if hits.len() == 1 {
        Some(hits[0])
    } else {
        None
    }
}

// =============================================================================
// Tauri command
// =============================================================================

/// Return the full provider registry for the frontend. The list is static —
/// clients can cache it for the lifetime of the session.
#[tauri::command]
pub fn hermes_list_providers() -> Vec<HermesProvider> {
    ALL_PROVIDERS.to_vec()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_expected_providers() {
        assert_eq!(ALL_PROVIDERS.len(), 34);
        assert!(get_provider("anthropic").is_some());
        assert!(get_provider("gemini").is_some());
        assert!(get_provider("alibaba-coding-plan").is_some());
        assert!(get_provider("bedrock").is_some());
        assert!(get_provider("lmstudio").is_some());
        assert!(get_provider("nous").is_some());
        assert!(get_provider("custom").is_some());
        assert!(get_provider("nonexistent").is_none());
    }

    #[test]
    fn primary_api_key_env_picks_first() {
        assert_eq!(primary_api_key_env("anthropic"), Some("ANTHROPIC_API_KEY"));
        assert_eq!(primary_api_key_env("gemini"), Some("GOOGLE_API_KEY"));
        assert_eq!(primary_api_key_env("zai"), Some("GLM_API_KEY"));
        assert_eq!(primary_api_key_env("bedrock"), None);
        assert_eq!(primary_api_key_env("nous"), None);
    }

    #[test]
    fn all_managed_env_keys_covers_everything() {
        let keys = all_managed_env_keys();
        assert!(keys.contains(&"ANTHROPIC_API_KEY"));
        assert!(keys.contains(&"DEEPSEEK_API_KEY"));
        assert!(keys.contains(&"GOOGLE_API_KEY"));
        assert!(keys.contains(&"GEMINI_API_KEY"));
        assert!(keys.contains(&"GEMINI_BASE_URL"));
        assert!(keys.contains(&"ALIBABA_CODING_PLAN_API_KEY"));
        assert!(keys.contains(&"LM_API_KEY"));
        assert!(keys.contains(&"GATEWAY_ALLOW_ALL_USERS"));
        assert!(keys.contains(&"API_SERVER_KEY"));
        // No duplicates
        for i in 0..keys.len() {
            for j in (i + 1)..keys.len() {
                assert_ne!(keys[i], keys[j], "duplicate: {}", keys[i]);
            }
        }
    }

    #[test]
    fn infer_provider_from_env_keys_follows_registry_order() {
        // ANTHROPIC appears before DEEPSEEK in ALL_PROVIDERS, so if both are present
        // the anthropic entry wins.
        let keys = vec!["DEEPSEEK_API_KEY", "ANTHROPIC_API_KEY"];
        assert_eq!(infer_provider_from_env_keys(&keys), Some("anthropic"));

        // Only DeepSeek set → matches deepseek.
        let keys = vec!["DEEPSEEK_API_KEY"];
        assert_eq!(infer_provider_from_env_keys(&keys), Some("deepseek"));

        // Secondary anthropic env var still matches.
        let keys = vec!["ANTHROPIC_TOKEN"];
        assert_eq!(infer_provider_from_env_keys(&keys), Some("anthropic"));

        // Unknown key → no match.
        let keys = vec!["UNRELATED_KEY"];
        assert_eq!(infer_provider_from_env_keys(&keys), None);
    }

    #[test]
    fn find_provider_by_model_is_unambiguous() {
        assert_eq!(find_provider_by_model("deepseek-chat"), Some("deepseek"));
        assert_eq!(find_provider_by_model("kimi-for-coding"), None);
        assert_eq!(find_provider_by_model("nonexistent"), None);
    }
}
