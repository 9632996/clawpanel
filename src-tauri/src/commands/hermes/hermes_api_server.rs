use tauri::Emitter;

use super::hermes_runtime::hermes_home;

// ============================================================================
// api_server guardian
//
// ClawPanel's Hermes integration requires `platforms.api_server.enabled: true`
// in ~/.hermes/config.yaml so that `hermes gateway run` exposes the
// /v1/runs endpoint we depend on. The setting is written once by
// `configure_hermes`, but config changes can remove it.
//   * Migration scripts accidentally drop the section.
//
// Rather than silently failing at Gateway start time with an opaque
// "endpoint not found" error, this guardian checks before every start and
// auto-heals the config. A timestamped backup (config.yaml.bak-<epoch>)
// is written before any mutation so users can always roll back.
// ============================================================================

/// Scan a YAML string for `platforms.api_server.enabled: true` and return
/// true only when that exact path exists with a truthy value.
fn config_has_api_server_enabled(raw: &str) -> bool {
    let mut in_platforms = false;
    let mut in_api_server = false;
    for line in raw.lines() {
        // Strip comments (crude, but matches the simple YAML we write).
        let line = match line.find('#') {
            Some(i) => &line[..i],
            None => line,
        };
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            continue;
        }
        let indent = trimmed.len() - trimmed.trim_start().len();

        if indent == 0 {
            in_platforms = trimmed.trim_start().starts_with("platforms:");
            in_api_server = false;
            continue;
        }
        if !in_platforms {
            continue;
        }
        // Inside platforms:
        if indent <= 2 {
            in_api_server = trimmed.trim_start().starts_with("api_server:");
            continue;
        }
        if !in_api_server {
            continue;
        }
        // Inside platforms.api_server:
        let t = trimmed.trim_start();
        if let Some(rest) = t.strip_prefix("enabled:") {
            let v = rest.trim().trim_matches(|c: char| c == '"' || c == '\'');
            return matches!(v.to_ascii_lowercase().as_str(), "true" | "yes" | "on" | "1");
        }
    }
    false
}

/// Produce a patched YAML that guarantees
/// `platforms.api_server.enabled: true` is present, preserving everything
/// else verbatim. If the config already has the setting (as `true`) this
/// returns the original text unchanged.
fn patch_yaml_ensure_api_server(raw: &str) -> String {
    if config_has_api_server_enabled(raw) {
        return raw.to_string();
    }

    // Strategy:
    //   * If `platforms:` exists, inject / replace api_server subtree under it.
    //   * Otherwise append a new top-level `platforms:` block at EOF.
    let lines: Vec<&str> = raw.lines().collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len() + 4);
    let mut platforms_found = false;
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_end();
        let indent = trimmed.len() - trimmed.trim_start().len();

        if indent == 0 && trimmed.trim_start().starts_with("platforms:") {
            // Copy the platforms: header
            out.push(line.to_string());
            platforms_found = true;
            i += 1;
            // Accumulate children and drop the existing api_server subtree
            // (we'll rewrite it at the top of the block). Keep siblings.
            let mut accumulated_children: Vec<String> = Vec::new();
            let mut skipping_api_server = false;
            while i < lines.len() {
                let l = lines[i];
                let t = l.trim_end();
                let ind = t.len() - t.trim_start().len();
                if ind == 0 && !t.is_empty() {
                    break; // leaving platforms block
                }
                if ind <= 2 {
                    skipping_api_server = t.trim_start().starts_with("api_server:");
                }
                if !skipping_api_server {
                    accumulated_children.push(l.to_string());
                }
                i += 1;
            }
            // Inject a fresh api_server entry at the top of platforms:
            out.push("  api_server:".into());
            out.push("    enabled: true".into());
            out.extend(accumulated_children);
            continue;
        }
        out.push(line.to_string());
        i += 1;
    }

    if !platforms_found {
        if let Some(last) = out.last() {
            if !last.is_empty() {
                out.push(String::new());
            }
        }
        out.push("platforms:".into());
        out.push("  api_server:".into());
        out.push("    enabled: true".into());
    }

    let mut content = out.join("\n");
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content
}

/// Guardian called from `hermes_gateway_action` on every `start` request.
/// Returns Ok(()) when the config is healthy (either it was already correct
/// or the patch succeeded). Emits `hermes-config-patched` on auto-heal so
/// the frontend can display a transparent toast.
pub(super) fn ensure_api_server_enabled(app: &tauri::AppHandle) -> Result<(), String> {
    let config_path = hermes_home().join("config.yaml");
    if !config_path.exists() {
        // Nothing to guard — configure_hermes will create a compliant file
        // on first run. Don't auto-create here; that's outside the guard's
        // responsibility.
        return Ok(());
    }
    let raw = std::fs::read_to_string(&config_path).map_err(|e| format!("Failed to read config.yaml: {e}"))?;
    if config_has_api_server_enabled(&raw) {
        return Ok(());
    }

    // Back up with a timestamped filename so we never overwrite an earlier
    // .bak (rapid re-starts would lose history otherwise).
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup_path = config_path.with_extension(format!("yaml.bak-{ts}"));
    let _ = std::fs::write(&backup_path, &raw);

    let patched = patch_yaml_ensure_api_server(&raw);
    std::fs::write(&config_path, &patched).map_err(|e| format!("Failed to write config.yaml: {e}"))?;

    // Inform the frontend so it can surface a toast. Failure to emit is
    // non-fatal — the patch itself already succeeded.
    let _ = app.emit(
        "hermes-config-patched",
        crate::jv!({
            "kind": "api_server_enabled",
            "backup": backup_path.to_string_lossy(),
            "message": "platforms.api_server.enabled 缺失，已自动修复并备份原文件",
        }),
    );
    Ok(())
}

// ============================================================================
// Unit tests for the pure YAML helpers (no filesystem I/O).
// ============================================================================

#[cfg(test)]
mod guardian_tests {
    use super::{config_has_api_server_enabled, patch_yaml_ensure_api_server};

    #[test]
    fn detects_enabled_variants() {
        let yaml = "\
model:
  default: deepseek-chat
platforms:
  api_server:
    enabled: true
";
        assert!(config_has_api_server_enabled(yaml));

        for v in ["true", "True", "TRUE", "yes", "on", "1"] {
            let y = format!("platforms:\n  api_server:\n    enabled: {v}\n");
            assert!(config_has_api_server_enabled(&y), "expected {v} to count as enabled");
        }
    }

    #[test]
    fn detects_missing_or_disabled() {
        assert!(!config_has_api_server_enabled("model:\n  default: foo\n"));
        assert!(!config_has_api_server_enabled("platforms:\n  other:\n    enabled: true\n"));
        assert!(!config_has_api_server_enabled("platforms:\n  api_server:\n    enabled: false\n"));
        assert!(!config_has_api_server_enabled("platforms:\n  api_server:\n    something: else\n"));
    }

    #[test]
    fn ignores_commented_enabled() {
        let yaml = "platforms:\n  api_server:\n    # enabled: true\n";
        assert!(!config_has_api_server_enabled(yaml));
    }

    #[test]
    fn patch_is_noop_when_already_enabled() {
        let yaml = "\
model:
  default: x
platforms:
  api_server:
    enabled: true
";
        assert_eq!(patch_yaml_ensure_api_server(yaml), yaml);
    }

    #[test]
    fn patch_appends_when_no_platforms() {
        let yaml = "model:\n  default: x\n";
        let patched = patch_yaml_ensure_api_server(yaml);
        assert!(config_has_api_server_enabled(&patched));
        assert!(patched.contains("model:"));
        assert!(patched.contains("default: x"));
    }

    #[test]
    fn patch_injects_under_existing_platforms() {
        let yaml = "\
platforms:
  other:
    enabled: true
terminal:
  backend: local
";
        let patched = patch_yaml_ensure_api_server(yaml);
        assert!(config_has_api_server_enabled(&patched));
        assert!(patched.contains("other:"));
        assert!(patched.contains("terminal:"));
        assert!(patched.contains("backend: local"));
    }

    #[test]
    fn patch_replaces_disabled_api_server() {
        let yaml = "\
platforms:
  api_server:
    enabled: false
    extra: keepme
  other:
    enabled: true
";
        let patched = patch_yaml_ensure_api_server(yaml);
        assert!(config_has_api_server_enabled(&patched));
        assert!(patched.contains("other:"));
        assert!(!patched.contains("enabled: false"), "disabled marker should have been removed");
    }
}
