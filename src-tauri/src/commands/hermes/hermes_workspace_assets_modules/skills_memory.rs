#[tauri::command]
pub async fn hermes_skill_detail(file_path: String) -> Result<String, String> {
    let skills_dir = hermes_home().join("skills");
    let resolved = PathBuf::from(&file_path);
    let canonical = resolved.canonicalize().map_err(|e| format!("Path error: {e}"))?;
    let canonical_dir = skills_dir.canonicalize().map_err(|e| format!("Path error: {e}"))?;
    if !canonical.starts_with(&canonical_dir) {
        return Err("Access denied".into());
    }
    std::fs::read_to_string(&canonical).map_err(|e| format!("Failed to read skill: {e}"))
}

// ============================================================================
// Skills — enable/disable toggle (Phase 3)
// ============================================================================

/// Toggle a skill's enabled state by mutating `config.yaml`'s
/// `skills.disabled` list.
///
/// * `enabled = true`  → remove `name` from disabled list
/// * `enabled = false` → add `name` to disabled list
///
/// A `config.yaml.bak-<epoch>` backup is written before any mutation so
/// users can always recover a broken config.
#[tauri::command]
pub async fn hermes_skill_toggle(name: String, enabled: bool) -> Result<Value, String> {
    if name.is_empty() {
        return Err("Skill name is required".into());
    }
    let config_path = hermes_home().join("config.yaml");
    let raw = std::fs::read_to_string(&config_path).map_err(|e| format!("Failed to read config.yaml: {e}"))?;

    // Write a timestamped backup before any mutation.
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup_path = hermes_home().join(format!("config.yaml.bak-{ts}"));
    let _ = std::fs::write(&backup_path, &raw);

    let patched = patch_yaml_toggle_skill(&raw, &name, enabled);
    std::fs::write(&config_path, &patched).map_err(|e| format!("Failed to write config.yaml: {e}"))?;

    Ok(crate::jv!({
        "ok": true,
        "skill": name,
        "enabled": enabled,
        "backup": backup_path.to_string_lossy(),
    }))
}

/// YAML patcher: add/remove `name` from `skills.disabled[]`.
///
/// Careful to preserve line ordering + indentation + other sections so that
/// user-edited comments and custom keys survive round-trips.
fn patch_yaml_toggle_skill(raw: &str, name: &str, enabled: bool) -> String {
    let mut lines: Vec<String> = raw.lines().map(str::to_string).collect();

    // Find `skills:` top-level key.
    let skills_idx = lines.iter().position(|l| {
        let trimmed = l.trim_end();
        let indent = trimmed.len() - trimmed.trim_start().len();
        indent == 0 && trimmed.trim_start().starts_with("skills:")
    });

    // If no `skills:` block exists yet, synthesize one.
    if skills_idx.is_none() {
        if enabled {
            // Already enabled (not in any disabled list). Nothing to do.
            return raw.to_string();
        }
        // Append a new skills.disabled block.
        if !raw.is_empty() && !raw.ends_with('\n') {
            lines.push(String::new());
        }
        lines.push("skills:".to_string());
        lines.push("  disabled:".to_string());
        lines.push(format!("    - {name}"));
        lines.push(String::new());
        return lines.join("\n");
    }

    let Some(skills_idx) = skills_idx else {
        return raw.to_string();
    };

    // Find `disabled:` under skills.
    let mut disabled_idx: Option<usize> = None;
    let mut i = skills_idx + 1;
    while i < lines.len() {
        let trimmed = lines[i].trim_end();
        let indent = trimmed.len() - trimmed.trim_start().len();
        if !trimmed.is_empty() && indent == 0 {
            break; // left the skills block
        }
        if indent == 2 && trimmed.trim_start().starts_with("disabled:") {
            disabled_idx = Some(i);
            break;
        }
        i += 1;
    }

    // Create a `disabled:` list if absent.
    if disabled_idx.is_none() {
        if enabled {
            // Already not disabled — nothing to do.
            return raw.to_string();
        }
        let insert_at = skills_idx + 1;
        lines.insert(insert_at, "  disabled:".to_string());
        lines.insert(insert_at + 1, format!("    - {name}"));
        return lines.join("\n");
    }

    let Some(disabled_idx) = disabled_idx else {
        return raw.to_string();
    };

    // Collect existing list item line indices + their values.
    let mut item_rows: Vec<(usize, String)> = Vec::new();
    let mut j = disabled_idx + 1;
    while j < lines.len() {
        let trimmed = lines[j].trim_end();
        let indent = trimmed.len() - trimmed.trim_start().len();
        if !trimmed.is_empty() && indent < 4 {
            break;
        }
        let body = trimmed.trim_start();
        if body.starts_with("- ") {
            let v = body
                .trim_start_matches("- ")
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            item_rows.push((j, v));
        }
        j += 1;
    }

    let has_item = item_rows.iter().any(|(_, v)| v == name);

    if enabled {
        // Remove all rows that match.
        if !has_item {
            return raw.to_string();
        }
        let to_remove: Vec<usize> = item_rows.iter().filter(|(_, v)| v == name).map(|(i, _)| *i).collect();
        for idx in to_remove.iter().rev() {
            lines.remove(*idx);
        }
    } else {
        if has_item {
            return raw.to_string();
        }
        // Insert right after the `disabled:` key line or at the end of
        // existing items — whichever produces stable ordering.
        let insert_at = item_rows.last().map(|(i, _)| *i + 1).unwrap_or(disabled_idx + 1);
        lines.insert(insert_at, format!("    - {name}"));
    }

    lines.join("\n")
}

/// Recursively list all files inside a skill directory. Returns an array
/// of `{ path, name, isDir }` where `path` is relative to `~/.hermes/`.
/// Skips the top-level `SKILL.md` because the UI already renders it
/// separately in the detail pane.
#[tauri::command]
pub async fn hermes_skill_files(category: String, skill: String) -> Result<Value, String> {
    let skills_root = hermes_home().join("skills");
    let skill_dir = skills_root.join(&category).join(&skill);
    if !skill_dir.exists() || !skill_dir.is_dir() {
        return Ok(crate::jv!([]));
    }

    let mut out: Vec<Value> = Vec::new();
    fn walk(root: &PathBuf, rel_base: &str, out: &mut Vec<Value>) {
        let entries = match std::fs::read_dir(root) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let rel = if rel_base.is_empty() {
                name.clone()
            } else {
                format!("{rel_base}/{name}")
            };
            let full = root.join(&name);
            let is_dir = full.is_dir();
            // Skip the flagship SKILL.md at the root level.
            if rel_base.is_empty() && name == "SKILL.md" {
                continue;
            }
            out.push(crate::jv!({
                "path": rel,
                "name": name,
                "isDir": is_dir,
            }));
            if is_dir {
                walk(&full, &rel, out);
            }
        }
    }
    walk(&skill_dir, "", &mut out);
    out.sort_by(|a, b| a["path"].as_str().unwrap_or("").cmp(b["path"].as_str().unwrap_or("")));
    Ok(Value::Array(out))
}

/// Write (create/update) a skill file. Path must be inside
/// `~/.hermes/skills/`. Intermediate directories are auto-created.
#[tauri::command]
pub async fn hermes_skill_write(file_path: String, content: String) -> Result<String, String> {
    let skills_dir = hermes_home().join("skills");
    let target = PathBuf::from(&file_path);

    // Ensure the target lives under the skills directory. We compare
    // absolute-normalized paths to allow writing *new* files (which cannot
    // be canonicalized yet) while still rejecting traversal.
    let skills_canon = skills_dir
        .canonicalize()
        .map_err(|e| format!("Skills dir not accessible: {e}"))?;
    let target_abs = if target.is_absolute() {
        target.clone()
    } else {
        skills_dir.join(&target)
    };
    let parent = target_abs.parent().ok_or_else(|| "Invalid target path".to_string())?;
    std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {e}"))?;
    let parent_canon = parent.canonicalize().map_err(|e| format!("Path error: {e}"))?;
    if !parent_canon.starts_with(&skills_canon) {
        return Err("Access denied".into());
    }
    std::fs::write(&target_abs, &content).map_err(|e| format!("Failed to write skill: {e}"))?;
    Ok("ok".into())
}

/// Resolve `memory|user|soul` to its filename inside `~/.hermes/memories/`.
fn memory_file_name(kind: &str) -> Option<&'static str> {
    match kind {
        "memory" => Some("MEMORY.md"),
        "user" => Some("USER.md"),
        "soul" => Some("SOUL.md"),
        _ => None,
    }
}

#[tauri::command]
pub async fn hermes_memory_read(r#type: Option<String>) -> Result<String, String> {
    let kind = r#type.as_deref().unwrap_or("memory");
    let file_name = memory_file_name(kind).ok_or_else(|| format!("Invalid memory kind '{kind}' (expected memory|user|soul)"))?;
    let file_path = hermes_home().join("memories").join(file_name);
    if !file_path.exists() {
        return Ok(String::new());
    }
    std::fs::read_to_string(&file_path).map_err(|e| format!("Failed to read memory: {e}"))
}

#[tauri::command]
pub async fn hermes_memory_write(r#type: Option<String>, content: String) -> Result<String, String> {
    let kind = r#type.as_deref().unwrap_or("memory");
    let file_name = memory_file_name(kind).ok_or_else(|| format!("Invalid memory kind '{kind}' (expected memory|user|soul)"))?;
    let mem_dir = hermes_home().join("memories");
    std::fs::create_dir_all(&mem_dir).map_err(|e| format!("Failed to create dir: {e}"))?;
    let file_path = mem_dir.join(file_name);
    std::fs::write(&file_path, &content).map_err(|e| format!("Failed to write memory: {e}"))?;
    Ok("ok".into())
}

/// Read all memory sections (memory/user/soul) in one call, returning content
/// + last-modified UNIX timestamp (seconds) for each. A missing file yields an
/// empty string and `None` mtime — the caller shows "not yet written" state.
///
/// Shape is optimized for the frontend memory layout.
#[tauri::command]
pub async fn hermes_memory_read_all() -> Result<Value, String> {
    let mem_dir = hermes_home().join("memories");
    let section = |kind: &str| -> (String, Option<u64>) {
        let name = match memory_file_name(kind) {
            Some(n) => n,
            None => return (String::new(), None),
        };
        let path = mem_dir.join(name);
        if !path.exists() {
            return (String::new(), None);
        }
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let mtime = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        (content, mtime)
    };
    let (memory, memory_mtime) = section("memory");
    let (user, user_mtime) = section("user");
    let (soul, soul_mtime) = section("soul");
    Ok(crate::jv!({
        "memory": memory,
        "user": user,
        "soul": soul,
        "memory_mtime": memory_mtime,
        "user_mtime": user_mtime,
        "soul_mtime": soul_mtime,
    }))
}

fn downloads_dir_fallback() -> PathBuf {
    dirs::download_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn safe_download_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

/// Read an entire log file and save it to the user's Downloads/ZhizhuaWorkbench
/// directory. We refuse path traversal and only allow files whose canonical
/// path lives inside `~/.hermes/logs/`.
#[tauri::command]
pub async fn hermes_logs_download(name: String) -> Result<Value, String> {
    // Reject traversal before any disk access.
    if name.is_empty() || name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err("Invalid log file name".into());
    }
    let logs_dir = hermes_home().join("logs");
    let file_path = logs_dir.join(&name);
    // Canonicalize both sides to ensure symlinks/relative segments can't
    // escape the logs directory.
    let canon_dir = logs_dir.canonicalize().map_err(|e| format!("Logs dir not found: {e}"))?;
    let canon_file = file_path.canonicalize().map_err(|e| format!("Log file not found: {e}"))?;
    if !canon_file.starts_with(&canon_dir) {
        return Err("Access denied".into());
    }
    let content = std::fs::read_to_string(&canon_file).map_err(|e| format!("Failed to read log: {e}"))?;
    let out_dir = downloads_dir_fallback().join("ZhizhuaWorkbench");
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("Failed to create download dir: {e}"))?;
    let out_path = out_dir.join(safe_download_filename(&name));
    std::fs::write(&out_path, content).map_err(|e| format!("Failed to save log: {e}"))?;
    Ok(crate::jv!({
        "path": out_path.to_string_lossy().to_string(),
    }))
}