pub fn extract_json_pub(text: &str) -> Option<Value> {
    extract_json(text)
}

/// Extract the first valid JSON object or array from a string that may contain
/// non-JSON lines (Node.js warnings, npm update prompts, ANSI codes, etc.)
fn extract_json(text: &str) -> Option<Value> {
    // Pre-processing: clean up common CLI output artifacts
    let cleaned = clean_cli_output(text);

    // Try parsing the whole string first (fast path)
    if let Ok(v) = serde_json::from_str::<Value>(&cleaned) {
        return Some(v);
    }

    // Find the first '{' or '[' and try parsing from there
    for (i, ch) in cleaned.char_indices() {
        if ch == '{' || ch == '[' {
            // Try direct parsing first
            if let Ok(v) = serde_json::from_str::<Value>(&cleaned[i..]) {
                return Some(v);
            }
            // Try with a streaming deserializer to handle trailing content
            let mut de = serde_json::Deserializer::from_str(&cleaned[i..]).into_iter::<Value>();
            if let Some(Ok(v)) = de.next() {
                return Some(v);
            }
        }
    }
    None
}

/// Clean up CLI output by removing common non-JSON artifacts:
/// - ANSI escape sequences (color codes)
/// - npm/node progress bars
/// - Multiple leading/trailing whitespace
/// - Debug log prefixes
fn clean_cli_output(text: &str) -> String {
    let mut result = text.to_string();

    // 1. Remove ANSI escape sequences
    // Common patterns: \x1b[...m, \x1b[...;...m, ESC[...m
    if let Ok(ansi_regex) = regex::Regex::new(r"\x1b\[[0-9;]*m") {
        result = ansi_regex.replace_all(&result, "").to_string();
    }

    // 2. Remove npm/node progress bar characters
    // Pattern: ████░░░░░░ 50% | some info
    if let Ok(progress_regex) = regex::Regex::new(r"[█▓▒░│┼┤├┬┴]+[│].*?\r?\n") {
        result = progress_regex.replace_all(&result, "").to_string();
    }

    // 3. Remove lines that are purely ANSI cursor control sequences
    // Like \r (carriage return for overwriting), \x1b[?25l (hide cursor), etc.
    if let Ok(cursor_regex) = regex::Regex::new(r"\x1b\[[?][0-9]+[a-zA-Z]") {
        result = cursor_regex.replace_all(&result, "").to_string();
    }

    // 4. Remove "Download" / "Installing" progress prefixes common in npm
    if let Ok(npm_progress_regex) =
        regex::Regex::new(r"^\s*(added|removed|changed|up to date)?\s*\d+\s*(package)?s?\s*(in\s+\d+s)?\s*(✓|✔|:)?\s*\r?$")
    {
        result = npm_progress_regex.replace_all(&result, "").to_string();
    }

    // 5. Normalize line endings and remove empty lines at the start
    let lines: Vec<&str> = result.lines().map(|l| l.trim_end_matches(['\r', '\n'])).collect();

    // Skip leading empty/whitespace-only lines
    let start_idx = lines.iter().position(|l| !l.trim().is_empty()).unwrap_or(0);
    let relevant_lines = &lines[start_idx..];

    // 6. Find the first line that starts JSON and return from there to end
    for (i, line) in relevant_lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            return relevant_lines[i..].join("\n");
        }
    }

    // 7. Otherwise, rejoin and let extract_json handle it
    result.lines().map(|l| l.trim()).collect::<Vec<_>>().join("\n")
}

/// 根据 agentId 解析该 Agent 的 workspace/skills 目录
/// 如果 agentId 为 None 或 "main"，返回 None（使用默认的 ~/.openclaw/skills）
fn resolve_agent_skills_dir(agent_id: Option<&str>) -> Option<std::path::PathBuf> {
    let id = agent_id.map(|s| s.trim()).filter(|s| !s.is_empty() && *s != "main")?;
    // 读取 openclaw.json 获取 agent workspace
    let config = super::config::load_openclaw_json().ok()?;
    let workspace = config
        .get("agents")
        .and_then(|a| a.get("list"))
        .and_then(|l| l.as_array())
        .and_then(|list| {
            list.iter()
                .find(|a| a.get("id").and_then(|v| v.as_str()) == Some(id))
                .and_then(|a| a.get("workspace"))
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| {
            // 默认：~/.openclaw/agents/{id}/workspace
            super::openclaw_dir()
                .join("agents")
                .join(id)
                .join("workspace")
                .to_string_lossy()
                .to_string()
        });
    let expanded = super::agent_workspace::expand_user_path_pub(&workspace);
    Some(expanded.join("skills"))
}

fn custom_skill_roots_for_agent(agent_skills_dir: Option<&std::path::Path>) -> Vec<(std::path::PathBuf, &'static str)> {
    let mut roots = Vec::new();

    // 如果指定了 agent 的 skills 目录，优先放在第一位
    if let Some(agent_dir) = agent_skills_dir {
        roots.push((agent_dir.to_path_buf(), "智能体自定义"));
    } else {
        // 默认 agent 使用全局 skills 目录
        roots.push((super::openclaw_dir().join("skills"), "智爪自定义"));
    }

    if let Some(home) = dirs::home_dir() {
        let claude_skills = home.join(".claude").join("skills");
        if !roots.iter().any(|(dir, _)| dir == &claude_skills) {
            roots.push((claude_skills, "Claude 用户技能"));
        }
    }
    // 从已解析的 CLI 路径推导 npm 包内的 bundled skills 目录
    if let Some(cli_path) = crate::utils::resolve_openclaw_cli_path() {
        let cli = std::path::PathBuf::from(&cli_path);
        let cli = std::fs::canonicalize(&cli).unwrap_or(cli);
        for pkg_root in [cli.parent(), cli.parent().and_then(|p| p.parent())].into_iter().flatten() {
            let bundled = pkg_root.join("skills");
            if bundled.is_dir() && !roots.iter().any(|(dir, _)| dir == &bundled) {
                roots.push((bundled, "智爪运行时内置"));
                break;
            }
        }
    }
    #[cfg(target_os = "windows")]
    if let Some(prefix) = super::windows_npm_global_prefix() {
        let bundled = std::path::PathBuf::from(&prefix)
            .join("node_modules")
            .join("openclaw")
            .join("skills");
        if bundled.is_dir() && !roots.iter().any(|(dir, _)| dir == &bundled) {
            roots.push((bundled, "智爪运行时内置"));
        }
    }
    roots
}

fn resolve_custom_skill_dir_with_agent(name: &str, agent_skills_dir: Option<&std::path::Path>) -> Option<std::path::PathBuf> {
    custom_skill_roots_for_agent(agent_skills_dir)
        .into_iter()
        .map(|(root, _)| root.join(name))
        .find(|path| path.exists())
}

fn scan_custom_skill_detail(name: &str, agent_skills_dir: Option<&std::path::Path>) -> Option<Value> {
    for (root, source_label) in custom_skill_roots_for_agent(agent_skills_dir) {
        let skill_path = root.join(name);
        if !skill_path.exists() {
            continue;
        }

        let base = scan_single_skill(&skill_path, name);
        let missing_deps = base
            .get("missingDeps")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let eligible = base.get("ready").and_then(|v| v.as_bool()).unwrap_or(false);

        let mut detail = crate::jv!({
            "name": name,
            "description": base.get("description").cloned().unwrap_or(Value::String(String::new())),
            "emoji": base.get("emoji").cloned().unwrap_or(Value::String("🧩".to_string())),
            "eligible": eligible,
            "disabled": false,
            "blockedByAllowlist": false,
            "source": source_label,
            "bundled": false,
            "filePath": skill_path.to_string_lossy().to_string(),
            "homepage": base.get("homepage").cloned().unwrap_or(Value::Null),
            "version": base.get("version").cloned().unwrap_or(Value::Null),
            "author": base.get("author").cloned().unwrap_or(Value::Null),
            "dependencies": base.get("dependencies").cloned().unwrap_or(Value::Array(vec![])),
            "missingDeps": Value::Array(missing_deps.clone()),
            "missing": crate::jv!({
                "bins": [],
                "anyBins": [],
                "env": [],
                "config": [],
                "os": []
            }),
            "requirements": crate::jv!({
                "bins": [],
                "env": [],
                "config": []
            }),
            "install": Value::Array(Vec::new())
        });

        if let Some(full_path) = base.get("fullPath").cloned() {
            detail["fullPath"] = full_path;
        }

        return Some(detail);
    }
    None
}

fn scan_local_skill_entries_for_agent(agent_skills_dir: Option<&std::path::Path>) -> Result<Vec<Value>, String> {
    let mut skills = Vec::new();

    for (skills_dir, source_label) in custom_skill_roots_for_agent(agent_skills_dir) {
        if !skills_dir.exists() {
            continue;
        }

        let entries = std::fs::read_dir(&skills_dir)
            .map_err(|e| format!("读取 Skills 目录失败 ({}): {e}", skills_dir.to_string_lossy()))?;

        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() && !file_type.is_symlink() {
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_string();
            let base = scan_single_skill(&entry.path(), &name);
            let eligible = base.get("ready").and_then(|v| v.as_bool()).unwrap_or(false);
            let mut item = crate::jv!({
                "name": name,
                "description": base.get("description").cloned().unwrap_or(Value::String(String::new())),
                "emoji": base.get("emoji").cloned().unwrap_or(Value::String("🧩".to_string())),
                "eligible": eligible,
                "disabled": false,
                "blockedByAllowlist": false,
                "source": source_label,
                "bundled": false,
                "filePath": entry.path().to_string_lossy().to_string(),
                "homepage": base.get("homepage").cloned().unwrap_or(Value::Null),
                "missing": crate::jv!({
                    "bins": [],
                    "anyBins": [],
                    "env": [],
                    "config": [],
                    "os": []
                }),
                "missingDeps": base.get("missingDeps").cloned().unwrap_or(Value::Array(vec![])),
                "install": Value::Array(Vec::new())
            });

            if let Some(full_path) = base.get("fullPath").cloned() {
                item["fullPath"] = full_path;
            }

            skills.push(item);
        }
    }

    skills.sort_by(|a, b| {
        let an = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let bn = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
        an.cmp(bn)
    });

    Ok(skills)
}

fn scan_local_skill_entries() -> Result<Vec<Value>, String> {
    scan_local_skill_entries_for_agent(None)
}

/// CLI 不可用或当前结果不可用时的兜底：扫描本地自定义 Skills 目录
fn scan_local_skills(cli_diagnostic: Option<Value>, agent_skills_dir: Option<&std::path::Path>) -> Result<Value, String> {
    let roots = custom_skill_roots_for_agent(agent_skills_dir);
    let scanned_roots: Vec<String> = roots
        .iter()
        .map(|(dir, label)| format!("{}: {}", label, dir.to_string_lossy()))
        .collect();
    let skills = scan_local_skill_entries_for_agent(agent_skills_dir)?;
    let cli_available = cli_diagnostic
        .as_ref()
        .and_then(|v| v.get("cliAvailable"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if skills.is_empty() {
        return Ok(crate::jv!({
            "skills": [],
            "source": "local-scan",
            "cliAvailable": cli_available,
            "diagnostic": {
                "status": cli_diagnostic.as_ref().and_then(|v| v.get("status")).and_then(|v| v.as_str()).unwrap_or("no-skills-dir"),
                "message": "未在本地自定义目录中发现 Skills",
                "scannedRoots": scanned_roots,
                "cli": cli_diagnostic
            }
        }));
    }

    // 统计信息
    let total = skills.len();
    let ready_count = skills
        .iter()
        .filter(|s| s.get("eligible").and_then(|v| v.as_bool()).unwrap_or(false))
        .count();
    let missing_deps_count = skills
        .iter()
        .filter(|s| !s.get("eligible").and_then(|v| v.as_bool()).unwrap_or(false))
        .count();

    Ok(crate::jv!({
        "skills": skills,
        "source": "local-scan",
        "cliAvailable": cli_available,
        "summary": {
            "total": total,
            "ready": ready_count,
            "missingDeps": missing_deps_count,
        },
        "diagnostic": {
            "status": cli_diagnostic.as_ref().and_then(|v| v.get("status")).and_then(|v| v.as_str()).unwrap_or("scanned"),
            "scannedAt": chrono::Utc::now().to_rfc3339(),
            "scannedRoots": scanned_roots,
            "cli": cli_diagnostic
        }
    }))
}

/// 扫描单个 Skill 的详细信息
fn scan_single_skill(skill_path: &std::path::Path, name: &str) -> Value {
    let mut result = crate::jv!({
        "name": name,
        "source": "managed",
        "bundled": false,
        "filePath": skill_path.to_string_lossy(),
        "ready": false,
        "missingDeps": Value::Array(Vec::new()),
        "installedDeps": Value::Array(Vec::new()),
    });

    // 1. 检查必要文件
    let skill_md = skill_path.join("SKILL.md");
    let package_json = skill_path.join("package.json");

    let has_skill_md = skill_md.exists();
    let has_package_json = package_json.exists();

    result["hasSkillMd"] = Value::Bool(has_skill_md);
    result["hasPackageJson"] = Value::Bool(has_package_json);

    // 2. 解析 package.json 获取更多信息
    if has_package_json {
        if let Ok(pkg_content) = std::fs::read_to_string(&package_json) {
            if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&pkg_content) {
                // 提取基本信息
                if let Some(version) = pkg.get("version").and_then(|v| v.as_str()) {
                    result["version"] = Value::String(version.to_string());
                }
                if let Some(author) = pkg.get("author").and_then(|v| {
                    v.as_str()
                        .or_else(|| v.as_object().and_then(|o| o.get("name").and_then(|n| n.as_str())))
                }) {
                    result["author"] = Value::String(author.to_string());
                }
                if let Some(desc) = pkg.get("description").and_then(|v| v.as_str()) {
                    result["description"] = Value::String(desc.to_string());
                }
                if let Some(homepage) = pkg.get("homepage").and_then(|v| v.as_str()) {
                    result["homepage"] = Value::String(homepage.to_string());
                }

                // 提取 dependencies
                if let Some(deps) = pkg.get("dependencies").and_then(|v| v.as_object()) {
                    let deps_list: Vec<String> = deps.keys().cloned().collect();
                    result["dependencies"] = Value::Array(deps_list.iter().map(|s| Value::String(s.clone())).collect());

                    // 检测缺少的依赖（简化版：通过检查 node_modules）
                    let missing_deps = detect_missing_dependencies(&deps_list, skill_path);
                    result["missingDeps"] = Value::Array(missing_deps.iter().map(|s| Value::String(s.clone())).collect());
                    result["installedDeps"] = Value::Array(
                        deps_list
                            .iter()
                            .filter(|d| !missing_deps.contains(d))
                            .map(|s| Value::String(s.clone()))
                            .collect(),
                    );
                }

                // 提取 scripts（可能包含 install 后处理等）
                if let Some(scripts) = pkg.get("scripts").and_then(|v| v.as_object()) {
                    let script_names: Vec<String> = scripts.keys().cloned().collect();
                    result["scripts"] = Value::Array(script_names.iter().map(|s| Value::String(s.clone())).collect());
                }
            }
        }
    }

    // 3. 从 SKILL.md frontmatter 提取额外信息
    if has_skill_md {
        if let Some(frontmatter) = parse_skill_frontmatter(&skill_md) {
            // 覆盖或补充 description（SKILL.md 的 description 更权威）
            if let Some(desc) = frontmatter.get("description").and_then(|v| v.as_str()) {
                result["description"] = Value::String(desc.to_string());
            }
            if let Some(full_path) = frontmatter.get("fullPath").and_then(|v| v.as_str()) {
                result["fullPath"] = Value::String(full_path.to_string());
            }
        }
    }

    // 4. 判断 ready 状态
    // Skill ready 需要：1) 有 SKILL.md  2) 没有缺少依赖  3) 依赖已安装
    let has_all_deps = result["missingDeps"].as_array().map(|a| a.is_empty()).unwrap_or(true);
    let has_essential_files = has_skill_md;
    result["ready"] = Value::Bool(has_essential_files && has_all_deps);

    // 5. 检测是否有 node_modules（npm 包已安装）
    let node_modules = skill_path.join("node_modules");
    result["nodeModulesInstalled"] = Value::Bool(node_modules.exists());

    result
}

/// 检测缺少的依赖
fn detect_missing_dependencies(deps: &[String], skill_path: &std::path::Path) -> Vec<String> {
    let node_modules = skill_path.join("node_modules");
    if !node_modules.exists() {
        // node_modules 不存在，所有依赖都算缺失
        return deps.to_vec();
    }

    let mut missing = Vec::new();
    for dep in deps {
        let dep_path = node_modules.join(dep);
        // 检查依赖目录或 @scope/package 格式
        if !dep_path.exists() {
            // 可能是 @scope/package 格式，直接检查目录
            missing.push(dep.clone());
        }
    }
    missing
}

/// 解析 SKILL.md frontmatter，返回键值对
fn parse_skill_frontmatter(path: &std::path::Path) -> Option<Value> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return None,
    };

    // frontmatter 格式: ---\n...\n---
    if !content.starts_with("---") {
        return None;
    }

    let after_first = content[3..].find("---")?;

    let fm_content = &content[3..3 + after_first];
    let mut fm_map = serde_json::Map::new();

    for line in fm_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.contains(':') {
            continue;
        }

        if let Some(colon_pos) = trimmed.find(':') {
            let key = trimmed[..colon_pos].trim().to_string();
            let value = trimmed[colon_pos + 1..].trim();

            // 处理引号包裹的值
            let clean_value = value.trim_matches('"').trim_matches('\'').trim();

            if !key.is_empty() && !clean_value.is_empty() {
                fm_map.insert(key, Value::String(clean_value.to_string()));
            }
        }
    }

    if fm_map.is_empty() {
        None
    } else {
        Some(Value::Object(fm_map))
    }
}