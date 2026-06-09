use serde_json::Value;

use super::hermes_providers;
use super::hermes_runtime::hermes_home;

// ============================================================================
// .env editor commands
//
// Users may need to set custom environment variables for Hermes (e.g.
// `TAVILY_API_KEY` for the tavily skill, `HTTP_PROXY`, etc.). These keys
// live in ~/.hermes/.env alongside the ClawPanel-managed provider keys.
//
// The three commands below:
//   * `hermes_env_read_unmanaged` — returns every key in .env that is NOT
//      managed by Zhizhua Workbench (i.e. not in `hermes_providers::all_managed_env_keys`)
//   * `hermes_env_set`            — writes or updates an unmanaged key
//   * `hermes_env_delete`         — removes an unmanaged key
//
// All three refuse to touch `all_managed_env_keys` to prevent users from
// accidentally clobbering provider keys from the editor UI (those should
// be configured via the setup page / configure_hermes).
// ============================================================================

/// Lenient .env parser shared by the three commands below.
/// Returns a Vec of (key, value, original_line_index) for every `KEY=VALUE`
/// pair. Comments and blanks are preserved by line index but not returned.
pub(super) fn parse_env_file_lines(raw: &str) -> Vec<(String, String, usize)> {
    let mut out = Vec::new();
    for (i, line) in raw.lines().enumerate() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = t.split_once('=') {
            let k = k.trim().to_string();
            if k.is_empty() {
                continue;
            }
            out.push((k, v.to_string(), i));
        }
    }
    out
}

/// Return every non-managed `KEY=VALUE` pair from ~/.hermes/.env.
///
/// Output is ordered by the order of appearance in the file. Managed keys
/// (provider API keys, base URLs, `GATEWAY_ALLOW_ALL_USERS`, `API_SERVER_KEY`)
/// are filtered out — those are surfaced separately in the config UI.
#[tauri::command]
pub fn hermes_env_read_unmanaged() -> Result<Vec<(String, String)>, String> {
    let env_path = hermes_home().join(".env");
    if !env_path.exists() {
        return Ok(Vec::new());
    }

    let raw = std::fs::read_to_string(&env_path).map_err(|e| format!("Failed to read .env: {e}"))?;

    let managed = hermes_providers::all_managed_env_keys();
    let mut out: Vec<(String, String)> = Vec::new();
    let mut seen = std::collections::HashSet::<String>::new();
    for (k, v, _) in parse_env_file_lines(&raw) {
        if managed.contains(&k.as_str()) {
            continue;
        }
        if seen.insert(k.clone()) {
            out.push((k, v));
        }
    }
    Ok(out)
}

/// Write or update a single unmanaged env var in ~/.hermes/.env.
///
/// Refuses to write keys in `hermes_providers::all_managed_env_keys`.
/// Creates the file (and parent dir) if missing.
#[tauri::command]
pub fn hermes_env_set(key: String, value: String) -> Result<(), String> {
    let key = key.trim().to_string();
    if key.is_empty() {
        return Err("Key cannot be empty".into());
    }
    // Basic sanity: env var keys are typically A-Z0-9_
    if !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(format!("Invalid env var key '{key}': only [A-Z0-9_] are allowed"));
    }
    let managed = hermes_providers::all_managed_env_keys();
    if managed.contains(&key.as_str()) {
        return Err(format!(
            "'{key}' is managed by the workbench; please configure it via the provider setup page"
        ));
    }

    let env_path = hermes_home().join(".env");
    if let Some(parent) = env_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create .hermes dir: {e}"))?;
    }

    let raw = if env_path.exists() {
        std::fs::read_to_string(&env_path).map_err(|e| format!("Failed to read .env: {e}"))?
    } else {
        String::new()
    };

    // Preserve file structure: if the key already exists, update the first
    // occurrence and leave the rest (which would be dead code anyway for
    // dotenv loaders) alone. Otherwise append a new line.
    let lines: Vec<&str> = raw.lines().collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len() + 1);
    let mut replaced = false;
    for line in lines.iter() {
        let t = line.trim();
        if t.starts_with('#') || t.is_empty() {
            out.push(line.to_string());
            continue;
        }
        if let Some((k, _)) = t.split_once('=') {
            if k.trim() == key && !replaced {
                out.push(format!("{key}={value}"));
                replaced = true;
                continue;
            }
        }
        out.push(line.to_string());
    }
    if !replaced {
        out.push(format!("{key}={value}"));
    }
    let mut content = out.join("\n");
    if !content.ends_with('\n') {
        content.push('\n');
    }
    std::fs::write(&env_path, content).map_err(|e| format!("Failed to write .env: {e}"))?;
    Ok(())
}

/// Remove an unmanaged env var from ~/.hermes/.env.
///
/// Refuses to delete keys in `hermes_providers::all_managed_env_keys`.
/// No-op if the key doesn't exist.
#[tauri::command]
pub fn hermes_env_delete(key: String) -> Result<(), String> {
    let key = key.trim().to_string();
    if key.is_empty() {
        return Err("Key cannot be empty".into());
    }
    let managed = hermes_providers::all_managed_env_keys();
    if managed.contains(&key.as_str()) {
        return Err(format!(
            "'{key}' is managed by the workbench; please configure it via the provider setup page"
        ));
    }

    let env_path = hermes_home().join(".env");
    if !env_path.exists() {
        return Ok(());
    }
    let raw = std::fs::read_to_string(&env_path).map_err(|e| format!("Failed to read .env: {e}"))?;

    let lines: Vec<&str> = raw.lines().collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    for line in lines.iter() {
        let t = line.trim();
        if t.starts_with('#') || t.is_empty() {
            out.push(line.to_string());
            continue;
        }
        if let Some((k, _)) = t.split_once('=') {
            if k.trim() == key {
                continue; // drop
            }
        }
        out.push(line.to_string());
    }
    let mut content = out.join("\n");
    if !content.ends_with('\n') {
        content.push('\n');
    }
    std::fs::write(&env_path, content).map_err(|e| format!("Failed to write .env: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn hermes_config_raw_read() -> Result<Value, String> {
    let path = hermes_home().join("config.yaml");
    let yaml = std::fs::read_to_string(&path).unwrap_or_default();
    Ok(crate::jv!({ "yaml": yaml }))
}

fn validate_hermes_config_raw_yaml(yaml_text: &str) -> Result<(), String> {
    if yaml_text.trim().is_empty() {
        return Ok(());
    }
    let parsed: serde_yaml::Value = serde_yaml::from_str(yaml_text).map_err(|e| format!("config.yaml YAML 格式错误: {e}"))?;
    if parsed.as_mapping().is_none() {
        return Err("config.yaml 顶层必须是对象".into());
    }
    Ok(())
}

#[tauri::command]
pub fn hermes_config_raw_write(yaml_text: String) -> Result<Value, String> {
    validate_hermes_config_raw_yaml(&yaml_text)?;
    let path = hermes_home().join("config.yaml");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create config dir: {e}"))?;
    }
    let mut backup_path: Option<String> = None;
    if path.exists() {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let backup = path.with_extension(format!("yaml.bak-{ts}"));
        if std::fs::copy(&path, &backup).is_ok() {
            backup_path = Some(backup.to_string_lossy().to_string());
        }
    }
    std::fs::write(&path, yaml_text).map_err(|e| format!("Failed to write config.yaml: {e}"))?;
    Ok(crate::jv!({ "ok": true, "backup": backup_path.unwrap_or_default() }))
}

#[tauri::command]
pub fn hermes_env_reveal(key: String) -> Result<Value, String> {
    let key = key.trim().to_string();
    if key.is_empty() {
        return Err("Key cannot be empty".into());
    }
    let env_path = hermes_home().join(".env");
    let raw = std::fs::read_to_string(&env_path).map_err(|e| format!("Failed to read .env: {e}"))?;
    for (k, v, _) in parse_env_file_lines(&raw) {
        if k == key {
            return Ok(crate::jv!({ "key": key, "value": v }));
        }
    }
    Err(format!("{key} not found in .env"))
}

#[cfg(test)]
mod hermes_config_raw_tests {
    use super::validate_hermes_config_raw_yaml;

    #[test]
    fn rejects_invalid_raw_config_yaml_before_write() {
        let err = validate_hermes_config_raw_yaml("model:\n  default: gpt-4o\n    provider: openai\n").unwrap_err();
        assert!(err.contains("config.yaml YAML 格式错误"));
    }

    #[test]
    fn rejects_non_object_raw_config_yaml_before_write() {
        let err = validate_hermes_config_raw_yaml("- model\n- display\n").unwrap_err();
        assert!(err.contains("config.yaml 顶层必须是对象"));
    }

    #[test]
    fn accepts_empty_and_mapping_raw_config_yaml() {
        validate_hermes_config_raw_yaml("").unwrap();
        validate_hermes_config_raw_yaml("model:\n  default: gpt-4o\n").unwrap();
    }
}
