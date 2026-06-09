/// Agent 管理命令 — 列表/改名直接读写 openclaw.json；创建/删除走 CLI（需要创建 workspace 等文件）
use crate::utils::openclaw_command_async;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::Write;

use super::agent_workspace::{
    is_workspace_previewable_file, is_workspace_text_file, looks_binary_bytes, normalize_workspace_relative_path,
    resolve_agent_workspace_path, resolve_workspace_target_path, to_workspace_relative_path,
};

const AGENT_FILE_ALLOWLIST: &[&str] = &[
    "AGENTS.md",
    "SOUL.md",
    "TOOLS.md",
    "IDENTITY.md",
    "USER.md",
    "HEARTBEAT.md",
    "BOOTSTRAP.md",
    "MEMORY.md",
];

const MAX_WORKSPACE_FILE_SIZE: u64 = 1024 * 1024;

/// Workspace 状态信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStatus {
    /// 路径是否存在
    pub exists: bool,
    /// 是否为软链接
    pub is_symlink: bool,
    /// 软链接指向的目标路径（如果是软链接）
    pub symlink_target: Option<String>,
    /// 软链接目标是否有效（仅当 is_symlink=true 时有意义）
    pub symlink_valid: bool,
    /// 是否有读取权限
    pub readable: bool,
}

/// Workspace 状态检测结果（包含状态和警告信息）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceCheckResult {
    pub status: WorkspaceStatus,
    pub warning: Option<String>,
}

// 检测 workspace 路径的状态
/// 使用 symlink_metadata 而非 metadata，避免跟随软链接
fn check_workspace_status(path: &std::path::Path) -> WorkspaceCheckResult {
    let mut status = WorkspaceStatus {
        exists: false,
        is_symlink: false,
        symlink_target: None,
        symlink_valid: false,
        readable: true,
    };
    let mut warning = None;

    // 使用 symlink_metadata 不会跟随软链接，能正确检测软链接本身的状态
    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            status.exists = true;
            status.is_symlink = meta.file_type().is_symlink();

            if status.is_symlink {
                // 软链接：获取目标路径
                match std::fs::read_link(path) {
                    Ok(target) => {
                        status.symlink_target = Some(target.to_string_lossy().to_string());
                        // 检查软链接目标是否存在
                        match std::fs::metadata(path) {
                            Ok(_) => status.symlink_valid = true,
                            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                                status.symlink_valid = false;
                                warning = Some("软链接目标不存在".to_string());
                            }
                            Err(e) => {
                                status.symlink_valid = false;
                                warning = Some(format!("无法访问软链接目标: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        warning = Some(format!("无法读取软链接目标: {}", e));
                    }
                }
            } else {
                // 普通目录：验证读取权限
                match std::fs::read_dir(path) {
                    Ok(_) => status.readable = true,
                    Err(e) => {
                        status.readable = false;
                        warning = Some(format!("权限不足: {}", e));
                    }
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            warning = Some("工作目录不存在".to_string());
        }
        Err(e) => {
            status.readable = false;
            warning = Some(format!("无法访问路径: {}", e));
        }
    }

    WorkspaceCheckResult { status, warning }
}

/// 获取 agent 列表（直接读 openclaw.json，不走 CLI，毫秒级响应）
#[tauri::command]
pub async fn list_agents() -> Result<Value, String> {
    let config = super::config::load_openclaw_json()?;

    let agents_list = config
        .get("agents")
        .and_then(|a| a.get("list"))
        .and_then(|l| l.as_array())
        .cloned()
        .unwrap_or_default();

    // 补全 main agent 的 workspace（config 中可能没有显式指定）
    let default_workspace = config
        .get("agents")
        .and_then(|a| a.get("defaults"))
        .and_then(|d| d.get("workspace"))
        .and_then(|w| w.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| super::openclaw_dir().join("workspace").to_string_lossy().to_string());

    // main agent 是隐式的（不在 agents.list 中），始终插入
    let has_main = agents_list
        .iter()
        .any(|a| a.get("id").and_then(|v| v.as_str()) == Some("main"));
    let all_agents = if has_main {
        agents_list
    } else {
        let mut v = vec![crate::jv!({
            "id": "main",
            "isDefault": true,
            "workspace": default_workspace.clone(),
        })];
        v.extend(agents_list);
        v
    };

    let enriched: Vec<Value> = all_agents
        .into_iter()
        .map(|mut agent| {
            let id = agent.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            // 补全 workspace 路径
            if agent.get("workspace").and_then(|w| w.as_str()).is_none()
                || agent.get("workspace").and_then(|w| w.as_str()) == Some("")
            {
                if id == "main" {
                    agent
                        .as_object_mut()
                        .map(|o| o.insert("workspace".to_string(), Value::String(default_workspace.clone())));
                } else {
                    let ws = super::openclaw_dir()
                        .join("agents")
                        .join(&id)
                        .join("workspace")
                        .to_string_lossy()
                        .to_string();
                    agent
                        .as_object_mut()
                        .map(|o| o.insert("workspace".to_string(), Value::String(ws)));
                }
            }

            // 检测 workspace 状态
            if let Some(ws_str) = agent.get("workspace").and_then(|w| w.as_str()) {
                let ws_path = std::path::Path::new(ws_str);
                let check_result = check_workspace_status(ws_path);

                // 添加 workspaceStatus 字段
                agent.as_object_mut().map(|o| {
                    o.insert(
                        "workspaceStatus".to_string(),
                        serde_json::to_value(&check_result.status).unwrap_or(Value::Null),
                    )
                });

                // 添加警告信息
                if let Some(w) = check_result.warning {
                    agent
                        .as_object_mut()
                        .map(|o| o.insert("workspaceWarning".to_string(), Value::String(w)));
                }
            }

            // 补全 identityName 用于前端显示
            let identity_name = agent
                .get("identity")
                .and_then(|i| i.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();
            if !identity_name.is_empty() {
                agent
                    .as_object_mut()
                    .map(|o| o.insert("identityName".to_string(), Value::String(identity_name)));
            }
            agent
        })
        .collect();

    Ok(Value::Array(enriched))
}

#[tauri::command]
pub async fn get_agent_detail(id: String) -> Result<Value, String> {
    let config = super::config::load_openclaw_json()?;

    let defaults = config
        .get("agents")
        .and_then(|a| a.get("defaults"))
        .cloned()
        .unwrap_or(Value::Null);
    let bindings = config.get("bindings").and_then(|b| b.as_array()).cloned().unwrap_or_default();

    let mut agent = config
        .get("agents")
        .and_then(|a| a.get("list"))
        .and_then(|l| l.as_array())
        .and_then(|list| {
            list.iter()
                .find(|a| a.get("id").and_then(|v| v.as_str()) == Some(id.as_str()))
                .cloned()
        })
        .unwrap_or_else(|| crate::jv!({ "id": id.clone(), "default": id == "main" }));

    let workspace = resolve_agent_workspace_path(&id, &config).to_string_lossy().to_string();

    let agent_bindings: Vec<Value> = bindings
        .into_iter()
        .filter(|b| b.get("agentId").and_then(|v| v.as_str()).unwrap_or("main") == id)
        .collect();

    let is_default = agent.get("default").and_then(|v| v.as_bool()).unwrap_or(id == "main");

    if let Some(obj) = agent.as_object_mut() {
        obj.insert("workspace".to_string(), Value::String(workspace));
        obj.insert("bindings".to_string(), Value::Array(agent_bindings));
        obj.insert("isDefault".to_string(), Value::Bool(is_default));
        obj.insert("defaults".to_string(), defaults);
    }

    Ok(agent)
}

#[tauri::command]
pub async fn list_agent_files(id: String) -> Result<Value, String> {
    let config = super::config::load_openclaw_json()?;
    let workspace_dir = resolve_agent_workspace_path(&id, &config);
    let files: Vec<Value> = AGENT_FILE_ALLOWLIST
        .iter()
        .map(|name| {
            let path = workspace_dir.join(name);
            let meta = fs::metadata(&path).ok();
            crate::jv!({
                "name": name,
                "desc": bootstrap_file_desc(name),
                "exists": path.exists(),
                "size": meta.as_ref().map(|m| m.len()).unwrap_or(0),
                "mtime": meta.and_then(|m| m.modified().ok()).and_then(|m| chrono::DateTime::<chrono::Utc>::from(m).to_rfc3339().into()),
                "path": path.to_string_lossy().to_string(),
            })
        })
        .collect();
    Ok(Value::Array(files))
}

#[tauri::command]
pub async fn read_agent_file(id: String, name: String) -> Result<Value, String> {
    ensure_allowed_agent_file(&name)?;
    let config = super::config::load_openclaw_json()?;
    let path = resolve_agent_workspace_path(&id, &config).join(&name);
    if !path.exists() {
        return Ok(crate::jv!({ "exists": false, "content": "" }));
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("读取文件失败: {e}"))?;
    Ok(crate::jv!({ "exists": true, "content": content }))
}

#[tauri::command]
pub async fn write_agent_file(id: String, name: String, content: String) -> Result<Value, String> {
    ensure_allowed_agent_file(&name)?;
    let config = super::config::load_openclaw_json()?;
    let dir = resolve_agent_workspace_path(&id, &config);
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败: {e}"))?;
    }
    fs::write(dir.join(&name), content).map_err(|e| format!("写入文件失败: {e}"))?;
    Ok(crate::jv!({ "ok": true }))
}

#[tauri::command]
pub async fn get_agent_workspace_info(id: String) -> Result<Value, String> {
    let config = super::config::load_openclaw_json()?;
    let workspace_dir = resolve_agent_workspace_path(&id, &config);
    Ok(crate::jv!({
        "agentId": id,
        "workspacePath": workspace_dir.to_string_lossy().to_string(),
        "exists": workspace_dir.exists(),
        "isDefault": id == "main",
    }))
}

#[tauri::command]
pub async fn list_agent_workspace_entries(id: String, relative_path: Option<String>) -> Result<Value, String> {
    let config = super::config::load_openclaw_json()?;
    let workspace_dir = resolve_agent_workspace_path(&id, &config);
    if !workspace_dir.exists() {
        return Ok(Value::Array(Vec::new()));
    }

    let target_dir = resolve_workspace_target_path(&workspace_dir, relative_path.as_deref())?;
    if !target_dir.exists() {
        return Err("目录不存在".to_string());
    }
    if !target_dir.is_dir() {
        return Err("目标不是目录".to_string());
    }

    let mut items: Vec<(u8, String, Value)> = fs::read_dir(&target_dir)
        .map_err(|e| format!("读取目录失败: {e}"))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let meta = entry.metadata().ok()?;
            let is_dir = meta.is_dir();
            let name = entry.file_name().to_string_lossy().to_string();
            let relative = to_workspace_relative_path(&workspace_dir, &path);
            let mtime = meta
                .modified()
                .ok()
                .map(|m| chrono::DateTime::<chrono::Utc>::from(m).to_rfc3339());

            Some((
                if is_dir { 0 } else { 1 },
                name.to_lowercase(),
                crate::jv!({
                    "name": name,
                    "relativePath": relative,
                    "type": if is_dir { "dir" } else { "file" },
                    "size": if is_dir { 0 } else { meta.len() },
                    "mtime": mtime,
                    "editable": !is_dir && is_workspace_text_file(&path),
                    "previewable": !is_dir && is_workspace_previewable_file(&path),
                }),
            ))
        })
        .collect();

    items.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    Ok(Value::Array(items.into_iter().map(|(_, _, item)| item).collect()))
}

#[tauri::command]
pub async fn read_agent_workspace_file(id: String, relative_path: String) -> Result<Value, String> {
    let config = super::config::load_openclaw_json()?;
    let workspace_dir = resolve_agent_workspace_path(&id, &config);
    let normalized = normalize_workspace_relative_path(&relative_path)?;
    if normalized.as_os_str().is_empty() {
        return Err("文件路径不能为空".to_string());
    }

    let file_path = workspace_dir.join(&normalized);
    if !file_path.exists() {
        return Err("文件不存在".to_string());
    }
    if !file_path.is_file() {
        return Err("目标不是文件".to_string());
    }

    let meta = fs::metadata(&file_path).map_err(|e| format!("读取文件信息失败: {e}"))?;
    if meta.len() > MAX_WORKSPACE_FILE_SIZE {
        return Err("文件过大，暂不支持在面板中打开".to_string());
    }

    let mtime = meta
        .modified()
        .ok()
        .map(|m| chrono::DateTime::<chrono::Utc>::from(m).to_rfc3339());

    let bytes = fs::read(&file_path).map_err(|e| format!("读取文件失败: {e}"))?;
    if looks_binary_bytes(&bytes) {
        return Err("暂不支持在面板中打开二进制文件".to_string());
    }

    let content = String::from_utf8(bytes).map_err(|_| "暂不支持在面板中打开非 UTF-8 文本文件".to_string())?;

    Ok(crate::jv!({
        "relativePath": normalized.to_string_lossy().replace('\\', "/"),
        "path": file_path.to_string_lossy().to_string(),
        "size": meta.len(),
        "mtime": mtime,
        "editable": true,
        "previewable": is_workspace_previewable_file(&file_path),
        "content": content,
    }))
}

#[tauri::command]
pub async fn write_agent_workspace_file(id: String, relative_path: String, content: String) -> Result<Value, String> {
    let config = super::config::load_openclaw_json()?;
    let workspace_dir = resolve_agent_workspace_path(&id, &config);
    let normalized = normalize_workspace_relative_path(&relative_path)?;
    if normalized.as_os_str().is_empty() {
        return Err("文件路径不能为空".to_string());
    }

    let file_path = workspace_dir.join(&normalized);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
    }

    fs::write(&file_path, content.as_bytes()).map_err(|e| format!("写入文件失败: {e}"))?;

    Ok(crate::jv!({
        "ok": true,
        "relativePath": normalized.to_string_lossy().replace('\\', "/"),
        "size": content.len(),
    }))
}

#[tauri::command]
pub async fn update_agent_config(app: tauri::AppHandle, id: String, config: Value) -> Result<Value, String> {
    let mut root = super::config::load_openclaw_json()?;
    if root.get("agents").is_none() {
        root.as_object_mut()
            .ok_or("配置格式错误")?
            .insert("agents".to_string(), crate::jv!({}));
    }
    if root["agents"].get("list").is_none() {
        root["agents"]
            .as_object_mut()
            .ok_or("agents 格式错误")?
            .insert("list".to_string(), crate::jv!([]));
    }

    let list = root["agents"]["list"].as_array_mut().ok_or("agents.list 格式错误")?;

    let index = list
        .iter()
        .position(|agent| agent.get("id").and_then(|v| v.as_str()) == Some(id.as_str()));

    let idx = match index {
        Some(idx) => idx,
        None if id == "main" => {
            list.insert(0, crate::jv!({ "id": "main" }));
            0
        }
        None => return Err(format!("Agent「{id}」不存在")),
    };

    let agent = list[idx].as_object_mut().ok_or("Agent 格式错误")?;

    if let Some(identity) = config.get("identity").and_then(|v| v.as_object()) {
        let identity_obj = agent.entry("identity".to_string()).or_insert_with(|| crate::jv!({}));
        let identity_obj = identity_obj.as_object_mut().ok_or("identity 格式错误")?;
        if let Some(name) = identity.get("name") {
            if name.is_null() {
                identity_obj.remove("name");
            } else {
                identity_obj.insert("name".to_string(), name.clone());
            }
        }
        if let Some(emoji) = identity.get("emoji") {
            if emoji.is_null() {
                identity_obj.remove("emoji");
            } else {
                identity_obj.insert("emoji".to_string(), emoji.clone());
            }
        }
    }
    if let Some(model) = config.get("model") {
        if model.is_null() {
            agent.remove("model");
        } else {
            agent.insert("model".to_string(), model.clone());
        }
    }
    if let Some(thinking) = config.get("thinkingDefault") {
        if thinking.is_null() {
            agent.remove("thinkingDefault");
        } else {
            agent.insert("thinkingDefault".to_string(), thinking.clone());
        }
    }
    if let Some(skills) = config.get("skills") {
        if skills.is_null() {
            agent.remove("skills");
        } else {
            agent.insert("skills".to_string(), skills.clone());
        }
    }
    if let Some(tools) = config.get("tools") {
        if tools.is_null() {
            agent.remove("tools");
        } else {
            agent.insert("tools".to_string(), tools.clone());
        }
    }

    super::config::save_openclaw_json(&root)?;
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = super::config::do_reload_gateway(&app2).await;
    });
    Ok(crate::jv!({ "ok": true }))
}

/// 创建新 agent（优先走 CLI，失败则直接写 openclaw.json 兜底）
#[tauri::command]
pub async fn add_agent(app: tauri::AppHandle, name: String, model: String, workspace: Option<String>) -> Result<Value, String> {
    let ws = match workspace {
        Some(ref w) if !w.is_empty() => std::path::PathBuf::from(w),
        _ => super::openclaw_dir().join("agents").join(&name).join("workspace"),
    };

    // 验证 workspace 路径有效性
    let ws_check = check_workspace_status(&ws);
    if let Some(ref warning) = ws_check.warning {
        eprintln!("[agent] Workspace 警告: {}", warning);
    }
    if ws_check.status.is_symlink && !ws_check.status.symlink_valid {
        return Err(format!(
            "指定的 workspace 是软链接，但目标不存在: {}",
            ws_check.status.symlink_target.as_deref().unwrap_or("未知")
        ));
    }

    let mut args = vec![
        "agents".to_string(),
        "add".to_string(),
        name.clone(),
        "--non-interactive".to_string(),
        "--workspace".to_string(),
        ws.to_string_lossy().to_string(),
    ];

    if !model.is_empty() {
        args.push("--model".to_string());
        args.push(model.clone());
    }

    // 尝试 CLI（15s 超时），失败则直接写配置兜底
    let cli_ok =
        match tokio::time::timeout(std::time::Duration::from_secs(15), openclaw_command_async().args(&args).output()).await {
            Ok(Ok(o)) if o.status.success() => true,
            Ok(Ok(o)) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                eprintln!("[agent] CLI 创建失败: {}", stderr.chars().take(200).collect::<String>());
                false
            }
            Ok(Err(e)) => {
                eprintln!("[agent] CLI 执行错误: {e}");
                false
            }
            Err(_) => {
                eprintln!("[agent] CLI 超时 (15s)，可能是 OpenClaw 未响应");
                false
            }
        };

    if !cli_ok {
        // 兜底：直接写 openclaw.json
        if let Err(e) = add_agent_to_config(&name, &model, &ws) {
            return Err(format!(
                "CLI 创建超时且配置写入失败: {}\n请尝试手动运行: openclaw agents add {} --workspace {}",
                e,
                name,
                ws.to_string_lossy()
            ));
        }
    }

    // 确保 workspace 目录存在
    if !ws.exists() {
        if let Err(e) = fs::create_dir_all(&ws) {
            eprintln!("[agent] 创建 workspace 目录失败: {e}");
        }
    }

    // 验证步骤
    let agents = list_agents().await?;
    let created = agents
        .as_array()
        .and_then(|arr| arr.iter().find(|a| a.get("id").and_then(|v| v.as_str()) == Some(&name)));

    if created.is_none() {
        eprintln!("[agent] 警告: Agent 创建后未在列表中出现");
    }

    if !ws.exists() {
        eprintln!("[agent] 警告: Agent workspace 目录未创建");
    }

    // 触发 Gateway 重载使新 agent 生效
    let _ = super::config::do_reload_gateway(&app).await;

    list_agents().await
}

// 直接写 openclaw.json 创建 agent（CLI 不可用时的兜底方案）
include!("agent_modules/mutation.rs");
