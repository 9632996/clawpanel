
/// 刷新 enhanced_path 缓存，使新设置的 Node.js 路径立即生效
#[tauri::command]
pub fn invalidate_path_cache() -> Result<(), String> {
    super::refresh_enhanced_path();
    crate::commands::service::invalidate_cli_detection_cache();
    Ok(())
}

#[cfg(test)]
mod write_openclaw_config_merge_tests {
    use super::merge_configs_preserving_fields;

    /// Regression guard: Issue #127 merge keeps full provider map when the UI payload
    /// only touches one provider — `sync_providers_to_agent_models` must use the same
    /// merged view (see `write_openclaw_config`), not the raw `config` argument.
    #[test]
    fn partial_models_merge_retains_other_providers() {
        let existing = crate::jv!({
            "models": {
                "providers": {
                    "a": { "models": [{ "id": "m1" }] },
                    "b": { "models": [{ "id": "m2" }] }
                }
            }
        });
        let new = crate::jv!({
            "models": {
                "providers": {
                    "a": {
                        "baseUrl": "http://example",
                        "models": [{ "id": "m1" }]
                    }
                }
            }
        });
        let merged = merge_configs_preserving_fields(&existing, &new);
        let prov = merged
            .pointer("/models/providers")
            .and_then(|p| p.as_object())
            .expect("merged.models.providers");
        assert!(prov.contains_key("a"));
        assert!(
            prov.contains_key("b"),
            "merged config must retain provider b when the write payload omits it"
        );
        assert_eq!(prov["a"]["baseUrl"], crate::jv!("http://example"));
    }
}
