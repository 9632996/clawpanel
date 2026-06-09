use serde_json::Value;
use std::path::PathBuf;

use super::hermes_runtime::{hermes_home, run_silent};

#[tauri::command]
pub async fn hermes_profiles_list() -> Result<Value, String> {
    let output = match run_silent("hermes", &["profile", "list"]) {
        Ok(s) => s,
        Err(_) => return Ok(crate::jv!({ "active": "default", "profiles": [] })),
    };
    let mut active = "default".to_string();
    let mut profiles: Vec<Value> = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.contains("Profile") || trimmed.starts_with('─') || trimmed.starts_with('-') {
            continue;
        }
        let is_active = trimmed.starts_with('◆');
        let row = trimmed.trim_start_matches('◆').trim();
        let parts: Vec<&str> = row.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let name = parts[0];
        if name != "default"
            && !name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        {
            continue;
        }
        let gateway_idx = parts.iter().position(|p| *p == "running" || *p == "stopped").unwrap_or(2);
        if gateway_idx <= 1 || gateway_idx >= parts.len() {
            continue;
        }
        let model = parts[1..gateway_idx].join(" ");
        let gateway = parts[gateway_idx];
        let alias = parts.get(gateway_idx + 1).copied().unwrap_or("—");
        if is_active {
            active = name.to_string();
        }
        profiles.push(crate::jv!({
            "name": name,
            "active": is_active,
            "model": if model == "—" { "" } else { &model },
            "gatewayRunning": gateway == "running",
            "alias": if alias == "—" { "" } else { alias },
        }));
    }
    if !profiles
        .iter()
        .any(|p| p.get("active").and_then(|v| v.as_bool()).unwrap_or(false))
    {
        if let Some(p) = profiles
            .iter_mut()
            .find(|p| p.get("name").and_then(|v| v.as_str()) == Some("default"))
        {
            if let Some(obj) = p.as_object_mut() {
                obj.insert("active".to_string(), Value::Bool(true));
            }
        }
    }
    Ok(crate::jv!({ "active": active, "profiles": profiles }))
}

#[tauri::command]
pub async fn hermes_profile_use(name: String) -> Result<String, String> {
    run_silent("hermes", &["profile", "use", &name])?;
    Ok("ok".into())
}

#[tauri::command]
pub async fn hermes_logs_list() -> Result<Value, String> {
    let logs_dir = hermes_home().join("logs");
    if !logs_dir.exists() {
        return Ok(crate::jv!([]));
    }
    let mut files: Vec<Value> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&logs_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".log") && !name.ends_with(".txt") && !name.ends_with(".jsonl") {
                continue;
            }
            let (size, modified) = if let Ok(meta) = entry.metadata() {
                let sz = meta.len();
                let mt = meta
                    .modified()
                    .ok()
                    .and_then(|t| {
                        t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| {
                            let secs = d.as_secs() as i64;
                            // Simple ISO-ish format
                            chrono_simple(secs)
                        })
                    })
                    .unwrap_or_default();
                (sz, mt)
            } else {
                (0, String::new())
            };
            files.push(crate::jv!({
                "name": name,
                "size": size,
                "modified": modified,
            }));
        }
    }
    files.sort_by(|a, b| {
        let ma = a["modified"].as_str().unwrap_or("");
        let mb = b["modified"].as_str().unwrap_or("");
        mb.cmp(ma)
    });
    Ok(Value::Array(files))
}

/// Simple timestamp formatter (no chrono crate dependency)
fn chrono_simple(epoch_secs: i64) -> String {
    // Use system time formatting via std
    let d = std::time::UNIX_EPOCH + std::time::Duration::from_secs(epoch_secs as u64);
    // Format as ISO string via debug (rough but functional)
    format!("{d:?}")
}

#[tauri::command]
pub async fn hermes_logs_read(name: String, lines: Option<usize>, level: Option<String>) -> Result<Value, String> {
    let max_lines = lines.unwrap_or(200);
    let log_path = hermes_home().join("logs").join(&name);
    if !log_path.exists() {
        return Err(format!("Log file not found: {name}"));
    }
    // Security: ensure path is within logs dir
    let logs_dir = hermes_home().join("logs");
    let canonical = log_path.canonicalize().map_err(|e| format!("Path error: {e}"))?;
    let canonical_dir = logs_dir.canonicalize().map_err(|e| format!("Path error: {e}"))?;
    if !canonical.starts_with(&canonical_dir) {
        return Err("Access denied".into());
    }

    let content = std::fs::read_to_string(&canonical).map_err(|e| format!("Failed to read log: {e}"))?;
    let all_lines: Vec<&str> = content.lines().collect();
    let start = if all_lines.len() > max_lines {
        all_lines.len() - max_lines
    } else {
        0
    };
    let tail = &all_lines[start..];

    let level_upper = level.as_deref().unwrap_or("").to_uppercase();
    let mut entries: Vec<Value> = Vec::new();
    // Regex-like manual parsing: "TIMESTAMP LEVEL MESSAGE"
    for line in tail {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        // Try to parse structured log: "2024-01-01 12:00:00 INFO message..."
        let parsed = parse_log_line(t);
        if !level_upper.is_empty() && level_upper != "ALL" {
            if let Some(ref lvl) = parsed.level {
                if lvl.to_uppercase() != level_upper {
                    continue;
                }
            } else {
                continue; // skip raw lines when filtering by level
            }
        }
        entries.push(match (parsed.timestamp, parsed.level, parsed.message) {
            (Some(ts), Some(lvl), Some(msg)) => crate::jv!({
                "timestamp": ts,
                "level": lvl,
                "message": msg,
                "raw": t,
            }),
            _ => crate::jv!({ "raw": t }),
        });
    }
    Ok(Value::Array(entries))
}

struct ParsedLogLine {
    timestamp: Option<String>,
    level: Option<String>,
    message: Option<String>,
}

fn parse_log_line(line: &str) -> ParsedLogLine {
    // Pattern: "YYYY-MM-DD HH:MM:SS LEVEL rest..." or "HH:MM:SS LEVEL rest..."
    let parts: Vec<&str> = line.splitn(4, char::is_whitespace).collect();
    if parts.len() >= 3 {
        // Check if first two parts look like a timestamp
        let maybe_date = parts[0];
        let maybe_time = parts[1];
        if (maybe_date.len() == 10 && maybe_date.contains('-')) && (maybe_time.len() >= 8 && maybe_time.contains(':')) {
            let ts = format!("{maybe_date} {maybe_time}");
            let lvl = parts[2].to_string();
            let msg = if parts.len() > 3 {
                parts[3].to_string()
            } else {
                String::new()
            };
            return ParsedLogLine {
                timestamp: Some(ts),
                level: Some(lvl),
                message: Some(msg),
            };
        }
    }
    // Fallback: check if first part is time-like
    if parts.len() >= 2 && parts[0].contains(':') && parts[0].len() >= 8 {
        let ts = parts[0].to_string();
        let lvl = parts[1].to_string();
        let msg = parts[2..].join(" ");
        return ParsedLogLine {
            timestamp: Some(ts),
            level: Some(lvl),
            message: Some(msg),
        };
    }
    ParsedLogLine {
        timestamp: None,
        level: None,
        message: None,
    }
}

/// Extract the first `# Heading` or the first long prose line from Markdown,
/// used as a skill's canonical name/description.
fn md_first_heading(content: &str) -> Option<String> {
    content
        .lines()
        .find(|l| l.starts_with("# "))
        .map(|l| l[2..].trim().to_string())
}

fn md_first_description(content: &str) -> String {
    content
        .lines()
        .find(|l| !l.starts_with('#') && !l.trim().is_empty() && l.trim().len() > 10)
        .map(|l| {
            let s = l.trim();
            if s.len() > 200 {
                format!("{}...", &s[..200])
            } else {
                s.to_string()
            }
        })
        .unwrap_or_default()
}

/// Read `config.yaml` and return the list of `skills.disabled` entries.
/// Gracefully handles missing file / missing section → empty list.
///
/// The disable mechanism uses the `skills.disabled` list:
///
/// ```yaml
/// skills:
///   disabled:
///     - web_search
///     - file_tools
/// ```
fn read_disabled_skills() -> Vec<String> {
    let config_path = hermes_home().join("config.yaml");
    let raw = match std::fs::read_to_string(&config_path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let mut disabled: Vec<String> = Vec::new();
    let mut in_skills = false;
    let mut in_disabled = false;
    for line in raw.lines() {
        // Strip trailing comments.
        let line = match line.find('#') {
            Some(i) => &line[..i],
            None => line,
        };
        let trimmed_full = line.trim_end();
        if trimmed_full.is_empty() {
            continue;
        }
        let indent = trimmed_full.len() - trimmed_full.trim_start().len();
        let body = trimmed_full.trim_start();

        if indent == 0 {
            in_skills = body.starts_with("skills:");
            in_disabled = false;
        } else if in_skills && indent == 2 && body.starts_with("disabled:") {
            in_disabled = true;
        } else if in_skills && in_disabled && indent >= 4 && body.starts_with("- ") {
            // Strip the `- ` prefix and any surrounding quotes.
            let name = body.trim_start_matches("- ").trim().trim_matches('"').trim_matches('\'');
            if !name.is_empty() {
                disabled.push(name.to_string());
            }
        } else if indent <= 2 {
            // Left the disabled list.
            in_disabled = false;
        }
    }
    disabled
}

/// Shape returned to the frontend.
#[tauri::command]
pub async fn hermes_skills_list() -> Result<Value, String> {
    let skills_dir = hermes_home().join("skills");
    if !skills_dir.exists() {
        return Ok(crate::jv!([]));
    }
    let disabled_names = read_disabled_skills();
    let is_enabled = |name: &str| -> bool { !disabled_names.iter().any(|d| d == name) };

    let mut categories: Vec<Value> = Vec::new();
    let entries = std::fs::read_dir(&skills_dir).map_err(|e| format!("Failed to read skills dir: {e}"))?;

    for entry in entries.flatten() {
        let ft = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let cat_name = entry.file_name().to_string_lossy().to_string();
        if cat_name.starts_with('.') {
            continue;
        }

        if ft.is_dir() {
            let cat_dir = skills_dir.join(&cat_name);

            // Category description from optional DESCRIPTION.md
            let cat_desc = std::fs::read_to_string(cat_dir.join("DESCRIPTION.md"))
                .ok()
                .map(|c| md_first_heading(&c).unwrap_or_else(|| c.trim().lines().next().unwrap_or("").to_string()))
                .unwrap_or_default();

            let mut skills: Vec<Value> = Vec::new();
            if let Ok(files) = std::fs::read_dir(&cat_dir) {
                for f in files.flatten() {
                    let fname = f.file_name().to_string_lossy().to_string();
                    let fpath = cat_dir.join(&fname);
                    let ftype = match f.file_type() {
                        Ok(t) => t,
                        Err(_) => continue,
                    };

                    // Structured skill: <category>/<skill>/SKILL.md
                    if ftype.is_dir() {
                        let skill_md = fpath.join("SKILL.md");
                        if !skill_md.exists() {
                            continue;
                        }
                        let content = std::fs::read_to_string(&skill_md).unwrap_or_default();
                        let display = md_first_heading(&content).unwrap_or_else(|| fname.clone());
                        let desc = md_first_description(&content);
                        skills.push(crate::jv!({
                            "file": fname.clone(),
                            "name": display,
                            "slug": fname.clone(),
                            "description": desc,
                            "path": skill_md.to_string_lossy(),
                            "skill_dir": fpath.to_string_lossy(),
                            "isDir": true,
                            "enabled": is_enabled(&fname),
                        }));
                        continue;
                    }

                    // Legacy flat skill: <category>/<name>.md
                    if !fname.ends_with(".md") || fname == "DESCRIPTION.md" {
                        continue;
                    }
                    let content = std::fs::read_to_string(&fpath).unwrap_or_default();
                    let slug = fname.trim_end_matches(".md").to_string();
                    let display = md_first_heading(&content).unwrap_or_else(|| slug.clone());
                    let desc = md_first_description(&content);
                    skills.push(crate::jv!({
                        "file": fname,
                        "name": display,
                        "slug": slug.clone(),
                        "description": desc,
                        "path": fpath.to_string_lossy(),
                        "isDir": false,
                        "enabled": is_enabled(&slug),
                    }));
                }
            }
            if !skills.is_empty() {
                skills.sort_by(|a, b| a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")));
                categories.push(crate::jv!({
                    "category": cat_name,
                    "description": cat_desc,
                    "skills": skills,
                }));
            }
        } else if cat_name.ends_with(".md") && cat_name != "DESCRIPTION.md" {
            // Uncategorized top-level skill file.
            let fpath = skills_dir.join(&cat_name);
            let content = std::fs::read_to_string(&fpath).unwrap_or_default();
            let slug = cat_name.trim_end_matches(".md").to_string();
            let display = md_first_heading(&content).unwrap_or_else(|| slug.clone());
            categories.push(crate::jv!({
                "category": "_root",
                "description": "",
                "skills": [{
                    "file": cat_name,
                    "name": display,
                    "slug": slug.clone(),
                    "description": md_first_description(&content),
                    "path": fpath.to_string_lossy(),
                    "isDir": false,
                    "enabled": is_enabled(&slug),
                }],
            }));
        }
    }

    categories.sort_by(|a, b| a["category"].as_str().unwrap_or("").cmp(b["category"].as_str().unwrap_or("")));

    Ok(Value::Array(categories))
}

include!("hermes_workspace_assets_modules/skills_memory.rs");
