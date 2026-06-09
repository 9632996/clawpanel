use serde_json::Value;

#[cfg(target_os = "windows")]
#[allow(unused_imports)]
use std::os::windows::process::CommandExt;

// 列出所有 Skills 及其状态（纯本地扫描，不依赖 CLI）
/// agent_id: 可选，指定 Agent ID，不同 Agent 有不同的 workspace/skills 目录
#[tauri::command]
pub async fn skills_list(agent_id: Option<String>) -> Result<Value, String> {
    let agent_ws = resolve_agent_skills_dir(agent_id.as_deref());
    scan_local_skills(None, agent_ws.as_deref())
}

/// 查看单个 Skill 详情（纯本地文件解析，不依赖 CLI）
#[tauri::command]
pub async fn skills_info(name: String, agent_id: Option<String>) -> Result<Value, String> {
    let agent_ws = resolve_agent_skills_dir(agent_id.as_deref());
    scan_custom_skill_detail(&name, agent_ws.as_deref()).ok_or_else(|| format!("Skill「{name}」不存在"))
}

/// 检查 Skills 依赖状态（纯本地扫描）
#[tauri::command]
pub async fn skills_check() -> Result<Value, String> {
    let skills = scan_local_skill_entries()?;
    let total = skills.len();
    let ready = skills
        .iter()
        .filter(|s| s.get("eligible").and_then(|v| v.as_bool()).unwrap_or(false))
        .count();
    let missing = total - ready;
    Ok(crate::jv!({
        "total": total,
        "ready": ready,
        "missingDeps": missing,
        "skills": skills,
    }))
}

/// 安装 Skill 依赖（根据 install spec 执行 brew/npm/go/uv/download）
#[tauri::command]
pub async fn skills_install_dep(kind: String, spec: Value) -> Result<Value, String> {
    let path_env = super::enhanced_path();

    let (program, args) = match kind.as_str() {
        "brew" => {
            let formula = spec
                .get("formula")
                .and_then(|v| v.as_str())
                .ok_or("缺少 formula 参数")?
                .to_string();
            ("brew".to_string(), vec!["install".to_string(), formula])
        }
        "node" => {
            let package = spec
                .get("package")
                .and_then(|v| v.as_str())
                .ok_or("缺少 package 参数")?
                .to_string();
            ("npm".to_string(), vec!["install".to_string(), "-g".to_string(), package])
        }
        "go" => {
            let module = spec
                .get("module")
                .and_then(|v| v.as_str())
                .ok_or("缺少 module 参数")?
                .to_string();
            ("go".to_string(), vec!["install".to_string(), module])
        }
        "uv" => {
            let package = spec
                .get("package")
                .and_then(|v| v.as_str())
                .ok_or("缺少 package 参数")?
                .to_string();
            ("uv".to_string(), vec!["tool".to_string(), "install".to_string(), package])
        }
        other => return Err(format!("不支持的安装类型: {other}")),
    };

    let mut cmd = tokio::process::Command::new(&program);
    cmd.args(&args).env("PATH", &path_env);
    super::apply_proxy_env_tokio(&mut cmd);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);
    let output = cmd.output().await.map_err(|e| format!("执行 {program} 失败: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(format!("安装失败 ({program} {}): {}", output.status, stderr.trim()));
    }

    Ok(crate::jv!({
        "success": true,
        "output": stdout.trim(),
    }))
}

/// 搜索 SkillHub（内置 HTTP，不依赖 CLI）
#[tauri::command]
pub async fn skillhub_search(query: String, limit: Option<u32>) -> Result<Value, String> {
    let items = super::skillhub::search(&query, limit.unwrap_or(20)).await?;
    Ok(serde_json::to_value(items).unwrap_or_default())
}

/// 获取全量技能索引（COS CDN，带内存缓存）
#[tauri::command]
pub async fn skillhub_index() -> Result<Value, String> {
    let items = super::skillhub::fetch_index().await?;
    Ok(serde_json::to_value(items).unwrap_or_default())
}

/// 从 SkillHub 安装 Skill（内置 HTTP 下载 + zip 解压）
#[tauri::command]
pub async fn skillhub_install(slug: String, agent_id: Option<String>) -> Result<Value, String> {
    let skills_dir = match resolve_agent_skills_dir(agent_id.as_deref()) {
        Some(dir) => dir,
        None => super::openclaw_dir().join("skills"),
    };
    if !skills_dir.exists() {
        std::fs::create_dir_all(&skills_dir).map_err(|e| format!("创建 skills 目录失败: {e}"))?;
    }
    let installed_path = super::skillhub::install(&slug, &skills_dir).await?;
    Ok(crate::jv!({
        "success": true,
        "slug": slug,
        "path": installed_path.to_string_lossy(),
    }))
}

/// 卸载 Skill（删除 skills/<name>/ 目录）
#[tauri::command]
pub async fn skills_uninstall(name: String, agent_id: Option<String>) -> Result<Value, String> {
    if name.is_empty() || name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err("无效的 Skill 名称".to_string());
    }
    let agent_ws = resolve_agent_skills_dir(agent_id.as_deref());
    let skills_dir =
        resolve_custom_skill_dir_with_agent(&name, agent_ws.as_deref()).ok_or_else(|| format!("Skill「{name}」不存在"))?;
    if !skills_dir.exists() {
        return Err(format!("Skill「{name}」不存在"));
    }
    std::fs::remove_dir_all(&skills_dir).map_err(|e| format!("删除失败: {e}"))?;
    Ok(crate::jv!({ "success": true, "name": name }))
}

/// 验证 Skill 配置是否正确
#[tauri::command]
pub async fn skills_validate(name: String) -> Result<Value, String> {
    if name.is_empty() || name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err("无效的 Skill 名称".to_string());
    }

    let skill_dir = resolve_custom_skill_dir_with_agent(&name, None).ok_or_else(|| format!("Skill「{name}」不存在"))?;
    if !skill_dir.exists() {
        return Err(format!("Skill「{name}」不存在"));
    }

    let skill_md = skill_dir.join("SKILL.md");
    let package_json = skill_dir.join("package.json");

    let mut issues: Vec<Value> = Vec::new();
    let mut warnings: Vec<Value> = Vec::new();
    let mut passed: Vec<String> = Vec::new();

    // 1. 检查 SKILL.md 是否存在
    if !skill_md.exists() {
        issues.push(crate::jv!({
            "level": "error",
            "code": "MISSING_SKILL_MD",
            "message": "缺少 SKILL.md 文件",
            "suggestion": "创建 SKILL.md 文件，包含 skill 的描述和使用说明"
        }));
    } else {
        passed.push("SKILL.md 存在".to_string());

        // 2. 检查 SKILL.md frontmatter 格式
        if let Some(frontmatter) = parse_skill_frontmatter(&skill_md) {
            // 检查必要字段
            let required_fields = ["description", "fullPath"];
            for field in &required_fields {
                if !frontmatter
                    .get(*field)
                    .and_then(|v| v.as_str())
                    .map(|s| !s.is_empty())
                    .unwrap_or(false)
                {
                    issues.push(crate::jv!({
                        "level": "error",
                        "code": "MISSING_REQUIRED_FIELD",
                        "message": format!("SKILL.md frontmatter 缺少必要字段: {}", field),
                        "field": field,
                        "suggestion": format!("在 frontmatter 中添加 {}: <值>", field)
                    }));
                } else {
                    passed.push(format!("frontmatter.{} 字段存在且非空", field));
                }
            }

            // 检查 fullPath 格式（应该是绝对路径或 ~ 开头）
            if let Some(fp) = frontmatter.get("fullPath").and_then(|v| v.as_str()) {
                // Windows 路径以盘符开头（如 C:\），Unix 以 / 或 ~ 或 . 开头
                let is_valid_path = fp.starts_with('/')
                    || fp.starts_with('~')
                    || fp.starts_with('.')
                    || (fp.len() >= 3 && fp.as_bytes()[1] == b':' && (fp.as_bytes()[2] == b'\\' || fp.as_bytes()[2] == b'/'));
                if !is_valid_path {
                    warnings.push(crate::jv!({
                        "level": "warning",
                        "code": "INVALID_FULLPATH_FORMAT",
                        "message": format!("fullPath 格式可能不正确: {}", fp),
                        "suggestion": "建议使用绝对路径或 ~ 开头"
                    }));
                }
            }
        } else {
            issues.push(crate::jv!({
                "level": "error",
                "code": "INVALID_FRONTMATTER",
                "message": "SKILL.md frontmatter 格式不正确",
                "suggestion": "确保 frontmatter 以 --- 开头和结尾，包含正确的 YAML 格式"
            }));
        }

        // 3. 检查 SKILL.md 内容（非 frontmatter 部分）
        if let Ok(content) = std::fs::read_to_string(&skill_md) {
            // 检查是否有空内容
            let body = content
                .split("---")
                .skip(2) // 跳过 frontmatter
                .collect::<Vec<_>>()
                .join("---")
                .trim()
                .to_string();

            if body.len() < 10 {
                warnings.push(crate::jv!({
                    "level": "warning",
                    "code": "EMPTY_SKILL_CONTENT",
                    "message": "SKILL.md 正文内容为空或过短",
                    "suggestion": "添加 skill 的使用说明、功能描述等详细内容"
                }));
            } else {
                passed.push("SKILL.md 正文内容完整".to_string());
            }
        }
    }

    // 4. 检查 package.json
    if !package_json.exists() {
        warnings.push(crate::jv!({
            "level": "warning",
            "code": "MISSING_PACKAGE_JSON",
            "message": "缺少 package.json 文件",
            "suggestion": "可选：创建 package.json 以便管理 npm 依赖"
        }));
    } else {
        passed.push("package.json 存在".to_string());

        // 5. 解析并验证 package.json
        if let Ok(pkg_content) = std::fs::read_to_string(&package_json) {
            if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&pkg_content) {
                // 检查 name 字段
                if let Some(pkg_name) = pkg.get("name").and_then(|v| v.as_str()) {
                    if pkg_name != name {
                        warnings.push(crate::jv!({
                            "level": "warning",
                            "code": "NAME_MISMATCH",
                            "message": format!("package.json 中的 name '{}' 与目录名 '{}' 不一致", pkg_name, name),
                            "suggestion": "确保 package.json 的 name 字段与 skill 目录名一致"
                        }));
                    } else {
                        passed.push("package.json.name 与目录名一致".to_string());
                    }
                }

                // 检查 dependencies 和 node_modules
                if let Some(deps) = pkg.get("dependencies").and_then(|v| v.as_object()) {
                    let deps_count = deps.len();
                    passed.push(format!("package.json 声明了 {} 个依赖", deps_count));

                    // 检查 node_modules
                    let node_modules = skill_dir.join("node_modules");
                    if node_modules.exists() {
                        let missing = detect_missing_dependencies(&deps.keys().cloned().collect::<Vec<_>>(), &skill_dir);
                        if !missing.is_empty() {
                            warnings.push(crate::jv!({
                                "level": "warning",
                                "code": "MISSING_NPM_DEPS",
                                "message": format!("缺少 {} 个 npm 依赖: {}", missing.len(), missing.join(", ")),
                                "missingDeps": missing,
                                "suggestion": "运行 npm install 安装依赖"
                            }));
                        } else {
                            passed.push("所有 npm 依赖已安装".to_string());
                        }
                    } else if deps_count > 0 {
                        issues.push(crate::jv!({
                            "level": "error",
                            "code": "NODE_MODULES_MISSING",
                            "message": "package.json 声明了依赖但 node_modules 不存在",
                            "suggestion": "运行 npm install 安装依赖"
                        }));
                    }
                }
            } else {
                issues.push(crate::jv!({
                    "level": "error",
                    "code": "INVALID_PACKAGE_JSON",
                    "message": "package.json 格式不正确",
                    "suggestion": "确保 package.json 是有效的 JSON 格式"
                }));
            }
        }
    }

    // 6. 检查常见的不应该存在的文件
    let unnecessary_files = ["README.md", "README.txt", "readme.md"];
    for file in unnecessary_files {
        let file_path = skill_dir.join(file);
        if file_path.exists() {
            warnings.push(crate::jv!({
                "level": "warning",
                "code": "UNNECESSARY_FILE",
                "message": format!("发现不必要的文件: {}", file),
                "suggestion": "Skill 文档应放在 SKILL.md 中，删除 README.md"
            }));
        }
    }

    // 汇总结果
    let has_errors = !issues.is_empty();
    let is_valid = !has_errors;

    Ok(crate::jv!({
        "name": name,
        "valid": is_valid,
        "summary": crate::jv!({
            "errors": issues.len(),
            "warnings": warnings.len(),
            "passed": passed.len()
        }),
        "issues": issues,
        "warnings": warnings,
        "passed": passed,
        "validatedAt": chrono::Utc::now().to_rfc3339()
    }))
}

// Public wrapper for extract_json, used by config.rs get_status_summary
include!("skills_modules/local_scan.rs");
