use serde_json::Value;

use super::hermes_runtime::{hermes_home, run_silent};

fn hermes_dashboard_theme_name(raw: &str) -> String {
    let mut in_dashboard = false;
    for line in raw.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 0 {
            in_dashboard = t == "dashboard:" || t.starts_with("dashboard:");
            if t.starts_with("dashboard:") && t != "dashboard:" {
                return t
                    .trim_start_matches("dashboard:")
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
            }
            continue;
        }
        if in_dashboard && t.starts_with("theme:") {
            return t
                .trim_start_matches("theme:")
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
        }
    }
    "default".into()
}

fn patch_dashboard_theme(raw: &str, name: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut in_dashboard = false;
    let mut dashboard_seen = false;
    let mut theme_written = false;
    for line in raw.lines() {
        let t = line.trim();
        let indent = line.len() - line.trim_start().len();
        if indent == 0 && !t.is_empty() && !t.starts_with('#') {
            if in_dashboard && !theme_written {
                out.push(format!("  theme: {name}"));
                theme_written = true;
            }
            in_dashboard = t == "dashboard:" || t.starts_with("dashboard:");
            if in_dashboard {
                dashboard_seen = true;
            }
        }
        if in_dashboard && indent > 0 && t.starts_with("theme:") {
            out.push(format!("{}theme: {name}", " ".repeat(indent)));
            theme_written = true;
            continue;
        }
        out.push(line.to_string());
    }
    if in_dashboard && !theme_written {
        out.push(format!("  theme: {name}"));
    }
    if !dashboard_seen {
        if out.last().map(|s| !s.is_empty()).unwrap_or(false) {
            out.push(String::new());
        }
        out.push("dashboard:".into());
        out.push(format!("  theme: {name}"));
    }
    let mut content = out.join("\n");
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content
}

#[tauri::command]
pub fn hermes_dashboard_themes() -> Result<Value, String> {
    let config_raw = std::fs::read_to_string(hermes_home().join("config.yaml")).unwrap_or_default();
    let active = hermes_dashboard_theme_name(&config_raw);
    let mut themes = vec![
        crate::jv!({ "name": "default", "label": "Default", "description": "Hermes default dashboard theme" }),
        crate::jv!({ "name": "midnight", "label": "Midnight", "description": "Dark blue dashboard theme" }),
        crate::jv!({ "name": "ember", "label": "Ember", "description": "Warm dashboard theme" }),
        crate::jv!({ "name": "mono", "label": "Mono", "description": "Monochrome dashboard theme" }),
        crate::jv!({ "name": "cyberpunk", "label": "Cyberpunk", "description": "Neon dashboard theme" }),
        crate::jv!({ "name": "rose", "label": "Rose", "description": "Soft rose dashboard theme" }),
    ];
    let dir = hermes_home().join("dashboard-themes");
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let ext_ok = path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("yaml") || s.eq_ignore_ascii_case("yml"))
                .unwrap_or(false);
            if !ext_ok {
                continue;
            }
            if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                themes.push(crate::jv!({
                    "name": name,
                    "label": name,
                    "description": "User dashboard theme",
                }));
            }
        }
    }
    Ok(crate::jv!({ "themes": themes, "active": active }))
}

#[tauri::command]
pub fn hermes_dashboard_theme_set(name: String) -> Result<Value, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Theme name cannot be empty".into());
    }
    let path = hermes_home().join("config.yaml");
    let raw = std::fs::read_to_string(&path).unwrap_or_default();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create config dir: {e}"))?;
    }
    std::fs::write(&path, patch_dashboard_theme(&raw, &name)).map_err(|e| format!("Failed to write config.yaml: {e}"))?;
    Ok(crate::jv!({ "ok": true, "theme": name }))
}

fn scan_dashboard_plugins() -> Vec<Value> {
    let mut plugins = Vec::new();
    let mut seen = std::collections::HashSet::<String>::new();
    let roots = [hermes_home().join("plugins")];
    for root in roots {
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let dir = entry.path();
                if !dir.is_dir() {
                    continue;
                }
                let manifest = dir.join("dashboard").join("manifest.json");
                if !manifest.exists() {
                    continue;
                }
                let raw = match std::fs::read_to_string(&manifest) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let data: Value = match serde_json::from_str(&raw) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let name = data
                    .get("name")
                    .and_then(|v| v.as_str())
                    .or_else(|| dir.file_name().and_then(|s| s.to_str()))
                    .unwrap_or("");
                if name.is_empty() || !seen.insert(name.to_string()) {
                    continue;
                }
                let tab = data
                    .get("tab")
                    .cloned()
                    .unwrap_or_else(|| crate::jv!({ "path": format!("/{name}"), "position": "end" }));
                plugins.push(crate::jv!({
                    "name": name,
                    "label": data.get("label").and_then(|v| v.as_str()).unwrap_or(name),
                    "description": data.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                    "icon": data.get("icon").and_then(|v| v.as_str()).unwrap_or("Puzzle"),
                    "version": data.get("version").and_then(|v| v.as_str()).unwrap_or("0.0.0"),
                    "tab": tab,
                    "slots": data.get("slots").cloned().unwrap_or_else(|| crate::jv!([])),
                    "entry": data.get("entry").and_then(|v| v.as_str()).unwrap_or("dist/index.js"),
                    "css": data.get("css").cloned().unwrap_or(Value::Null),
                    "has_api": data.get("api").is_some(),
                    "source": "user",
                }));
            }
        }
    }
    plugins
}

#[tauri::command]
pub fn hermes_dashboard_plugins() -> Result<Value, String> {
    Ok(Value::Array(scan_dashboard_plugins()))
}

#[tauri::command]
pub fn hermes_dashboard_plugins_rescan() -> Result<Value, String> {
    let plugins = scan_dashboard_plugins();
    Ok(crate::jv!({ "ok": true, "count": plugins.len() }))
}

#[tauri::command]
pub fn hermes_toolsets_list() -> Result<Value, String> {
    let output = run_silent("hermes", &["tools", "list", "--platform", "cli"]).unwrap_or_default();
    Ok(crate::jv!({ "raw": output }))
}

#[tauri::command]
pub fn hermes_cron_jobs_list() -> Result<Value, String> {
    let path = hermes_home().join("cron").join("jobs.json");
    if !path.exists() {
        return Ok(Value::Array(Vec::new()));
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("Failed to read cron jobs: {e}"))?;
    serde_json::from_str::<Value>(&raw).map_err(|e| format!("Failed to parse cron jobs: {e}"))
}
