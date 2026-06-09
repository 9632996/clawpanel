
/// 在 PATH 中查找 node 可执行文件的实际路径
fn find_node_path(enhanced_path: &str) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        // Windows: 使用 where 命令
        let mut cmd = Command::new("where");
        cmd.arg("node");
        cmd.creation_flags(0x08000000);
        // 设置 PATH 为 enhanced_path，优先查找 node
        if std::env::var("PATH").is_ok() {
            cmd.env("PATH", enhanced_path);
            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    // where 输出可能有多行，取第一行
                    if let Some(first_line) = stdout.lines().next() {
                        let path = first_line.trim().to_string();
                        if !path.is_empty() && std::path::Path::new(&path).exists() {
                            return Some(path);
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Unix: 使用 which 命令
        let mut cmd = Command::new("which");
        cmd.arg("node");
        if let Ok(_current_path) = std::env::var("PATH") {
            cmd.env("PATH", enhanced_path);
            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() && std::path::Path::new(&path).exists() {
                        return Some(path);
                    }
                }
            }
        }
    }

    None
}

/// 根据 node 路径推断其来源
fn detect_node_source(node_path: &str) -> String {
    let path_lower = node_path.to_lowercase();
    let path_obj = std::path::Path::new(node_path);

    // 检查父目录
    if let Some(parent) = path_obj.parent() {
        let parent_str = parent.to_string_lossy().to_lowercase();

        // nvm-windows 符号链接路径
        if parent_str.contains("nvm") || parent_str.contains(".nvm") {
            // 检查是否是 nvm-windows 的当前版本符号链接
            if let Ok(nvm_symlink) = std::env::var("NVM_SYMLINK") {
                if path_lower.contains(&nvm_symlink.to_lowercase()) {
                    return "NVM_SYMLINK".to_string();
                }
            }
            return "NVM".to_string();
        }

        // Volta
        if parent_str.contains(".volta") || parent_str.contains("volta") {
            return "VOLTA".to_string();
        }

        // fnm
        if parent_str.contains("fnm") || parent_str.contains("fnm_multishells") {
            return "FNM".to_string();
        }

        // nodenv
        if parent_str.contains("nodenv") {
            return "NODENV".to_string();
        }

        // n (node version manager)
        if parent_str.contains("/n/bin") || parent_str.contains("\\n\\bin") {
            return "N".to_string();
        }

        // npm 全局
        if parent_str.contains("npm") && parent_str.contains("appdata") {
            return "NPM_GLOBAL".to_string();
        }

        // 系统默认安装位置
        if parent_str.contains("program files") || parent_str.contains("programs\\nodejs") {
            return "SYSTEM".to_string();
        }
    }

    // 检查环境变量
    #[cfg(target_os = "windows")]
    {
        if let Ok(nvm_symlink) = std::env::var("NVM_SYMLINK") {
            if path_lower.contains(&nvm_symlink.to_lowercase()) {
                return "NVM_SYMLINK".to_string();
            }
        }
    }

    "PATH".to_string()
}

/// 在指定路径下检测 node 是否存在
#[tauri::command]
pub fn check_node_at_path(node_dir: String) -> Result<Value, String> {
    let dir = std::path::PathBuf::from(&node_dir);
    #[cfg(target_os = "windows")]
    let node_bin = dir.join("node.exe");
    #[cfg(not(target_os = "windows"))]
    let node_bin = dir.join("node");

    let mut result = serde_json::Map::new();
    if !node_bin.exists() {
        result.insert("installed".into(), Value::Bool(false));
        result.insert("version".into(), Value::Null);
        return Ok(Value::Object(result));
    }

    let mut cmd = Command::new(&node_bin);
    cmd.arg("--version");
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);
    match cmd.output() {
        Ok(o) if o.status.success() => {
            let ver = String::from_utf8_lossy(&o.stdout).trim().to_string();
            result.insert("installed".into(), Value::Bool(true));
            result.insert("version".into(), Value::String(ver));
            result.insert("path".into(), Value::String(node_dir));
        }
        _ => {
            result.insert("installed".into(), Value::Bool(false));
            result.insert("version".into(), Value::Null);
        }
    }
    Ok(Value::Object(result))
}

/// 扫描常见路径，返回所有找到的 Node.js 安装，包含来源说明
#[tauri::command]
pub fn scan_node_paths() -> Result<Value, String> {
    let mut found: Vec<Value> = vec![];
    let home = dirs::home_dir().unwrap_or_default();

    let mut candidates: Vec<(String, String)> = vec![]; // (path, source)

    #[cfg(target_os = "windows")]
    {
        let pf = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".into());
        let pf86 = std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| r"C:\Program Files (x86)".into());
        let localappdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
        let appdata = std::env::var("APPDATA").unwrap_or_default();

        // NVM_SYMLINK - nvm-windows 活跃版本
        if let Ok(nvm_symlink) = std::env::var("NVM_SYMLINK") {
            if std::path::Path::new(&nvm_symlink).is_dir() {
                candidates.push((nvm_symlink, "NVM_SYMLINK".to_string()));
            }
        }

        // NVM_HOME - 用户自定义 nvm 目录
        if let Ok(nvm_home) = std::env::var("NVM_HOME") {
            if std::path::Path::new(&nvm_home).is_dir() {
                if let Ok(entries) = std::fs::read_dir(&nvm_home) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.is_dir() && p.join("node.exe").exists() {
                            // 检查是否是当前激活版本（通过 settings.json）
                            let is_active = is_nvm_active_version(&nvm_home, &p);
                            let source = if is_active { "NVM_ACTIVE" } else { "NVM" };
                            candidates.push((p.to_string_lossy().to_string(), source.to_string()));
                        }
                    }
                }
            }
        }

        // %APPDATA%\nvm - nvm-windows 默认目录
        if !appdata.is_empty() {
            let nvm_dir = std::path::Path::new(&appdata).join("nvm");
            if nvm_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&nvm_dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.is_dir() && p.join("node.exe").exists() {
                            let is_active = is_nvm_active_version(nvm_dir.to_string_lossy().as_ref(), &p);
                            let source = if is_active { "NVM_ACTIVE" } else { "NVM" };
                            candidates.push((p.to_string_lossy().to_string(), source.to_string()));
                        }
                    }
                }
            }
        }

        // Volta
        let volta_bin = format!(r"{}\.volta\bin", home.display());
        candidates.push((volta_bin.clone(), "VOLTA".to_string()));
        // 检查 volta 当前激活版本
        if let Ok(volta_home) = std::env::var("VOLTA_HOME") {
            let volta_current = std::path::Path::new(&volta_home).join("current/bin");
            if volta_current.exists() {
                candidates.push((volta_current.to_string_lossy().to_string(), "VOLTA_ACTIVE".to_string()));
            }
        }

        // fnm
        if !localappdata.is_empty() {
            candidates.push((format!(r"{}\fnm_multishells", localappdata), "FNM_TEMP".to_string()));
        }
        let fnm_base = std::env::var("FNM_DIR")
            .ok()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::Path::new(&appdata).join("fnm"));
        // fnm current
        let fnm_current = fnm_base.join("current/installation");
        if fnm_current.is_dir() && fnm_current.join("node.exe").exists() {
            candidates.push((fnm_current.to_string_lossy().to_string(), "FNM_ACTIVE".to_string()));
        }
        // fnm versions
        let fnm_versions = fnm_base.join("node-versions");
        if fnm_versions.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&fnm_versions) {
                for entry in entries.flatten() {
                    let inst = entry.path().join("installation");
                    if inst.is_dir() && inst.join("node.exe").exists() {
                        let source = if inst == fnm_current { "FNM_ACTIVE" } else { "FNM" };
                        candidates.push((inst.to_string_lossy().to_string(), source.to_string()));
                    }
                }
            }
        }

        // npm 全局
        if !appdata.is_empty() {
            candidates.push((format!(r"{}\npm", appdata), "NPM_GLOBAL".to_string()));
        }
        if let Some(prefix) = super::windows_npm_global_prefix() {
            candidates.push((prefix, "NPM_GLOBAL".to_string()));
        }

        // 系统默认
        candidates.push((format!(r"{}\nodejs", pf), "SYSTEM".to_string()));
        candidates.push((format!(r"{}\nodejs", pf86), "SYSTEM".to_string()));
        if !localappdata.is_empty() {
            candidates.push((format!(r"{}\Programs\nodejs", localappdata), "SYSTEM".to_string()));
        }

        // 常见盘符
        for drive in &["C", "D", "E", "F", "G"] {
            candidates.push((format!(r"{}:\nodejs", drive), "MANUAL".to_string()));
            candidates.push((format!(r"{}:\Node", drive), "MANUAL".to_string()));
            candidates.push((format!(r"{}:\Node.js", drive), "MANUAL".to_string()));
            candidates.push((format!(r"{}:\Program Files\nodejs", drive), "SYSTEM".to_string()));
            // AI/Dev 工具目录
            candidates.push((format!(r"{}:\AI\Node", drive), "MANUAL".to_string()));
            candidates.push((format!(r"{}:\AI\nodejs", drive), "MANUAL".to_string()));
            candidates.push((format!(r"{}:\Dev\nodejs", drive), "MANUAL".to_string()));
            candidates.push((format!(r"{}:\Tools\nodejs", drive), "MANUAL".to_string()));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        candidates.push(("/usr/local/bin".into(), "SYSTEM".to_string()));
        candidates.push(("/opt/homebrew/bin".into(), "BREW".to_string()));
        candidates.push((format!("{}/.nvm/current/bin", home.display()), "NVM_ACTIVE".to_string()));
        candidates.push((format!("{}/.volta/bin", home.display()), "VOLTA".to_string()));
        candidates.push((format!("{}/.nodenv/shims", home.display()), "NODENV".to_string()));
        candidates.push((format!("{}/.fnm/current/bin", home.display()), "FNM_ACTIVE".to_string()));
        candidates.push((format!("{}/n/bin", home.display()), "N".to_string()));
        candidates.push((format!("{}/.npm-global/bin", home.display()), "NPM_GLOBAL".to_string()));
    }

    // 去重并检测 node
    let mut seen_paths: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (dir, source) in &candidates {
        let path = std::path::Path::new(dir);
        #[cfg(target_os = "windows")]
        let node_bin = path.join("node.exe");
        #[cfg(not(target_os = "windows"))]
        let node_bin = path.join("node");

        if node_bin.exists() {
            let node_path_str = node_bin.to_string_lossy().to_string();
            // 去重
            if seen_paths.contains(&node_path_str) {
                continue;
            }
            seen_paths.insert(node_path_str.clone());

            let mut cmd = Command::new(&node_bin);
            cmd.arg("--version");
            #[cfg(target_os = "windows")]
            cmd.creation_flags(0x08000000);
            if let Ok(o) = cmd.output() {
                if o.status.success() {
                    let ver = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    let mut entry = serde_json::Map::new();
                    entry.insert("path".into(), Value::String(node_path_str));
                    entry.insert("version".into(), Value::String(ver));
                    entry.insert("source".into(), Value::String(source.clone()));
                    // 标记是否激活
                    let is_active = source.contains("ACTIVE");
                    entry.insert("active".into(), Value::Bool(is_active));
                    found.push(Value::Object(entry));
                }
            }
        }
    }

    // 按激活状态排序（激活的版本排在前面）
    found.sort_by(|a, b| {
        let a_active = a.get("active").and_then(|v| v.as_bool()).unwrap_or(false);
        let b_active = b.get("active").and_then(|v| v.as_bool()).unwrap_or(false);
        b_active.cmp(&a_active)
    });

    Ok(Value::Array(found))
}

/// 检查给定版本目录是否是 nvm-windows 的当前激活版本
#[allow(dead_code)]
fn is_nvm_active_version(nvm_dir: &str, version_dir: &std::path::Path) -> bool {
    let settings_path = std::path::Path::new(nvm_dir).join("settings.json");
    if !settings_path.exists() {
        return false;
    }

    if let Ok(content) = std::fs::read_to_string(&settings_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(current_path) = json.get("path").and_then(|v| v.as_str()) {
                // settings.json 中的 path 可能是绝对路径或相对路径
                let expected_path: std::path::PathBuf = if current_path.starts_with('/') || current_path.contains(':') {
                    // 绝对路径
                    std::path::Path::new(current_path).to_path_buf()
                } else {
                    // 相对路径
                    std::path::Path::new(nvm_dir).join(current_path)
                };
                return version_dir == expected_path.as_path();
            }
        }
    }
    false
}

/// 保存用户自定义的 Node.js 路径到 ~/.openclaw/clawpanel.json
#[tauri::command]
pub fn save_custom_node_path(node_dir: String) -> Result<(), String> {
    let config_path = super::panel_config_path();
    if let Some(parent) = config_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut config: serde_json::Map<String, Value> = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).map_err(|e| format!("读取配置失败: {e}"))?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        serde_json::Map::new()
    };
    config.insert("nodePath".into(), Value::String(node_dir));
    let json = serde_json::to_string_pretty(&Value::Object(config)).map_err(|e| format!("序列化失败: {e}"))?;
    std::fs::write(&config_path, json).map_err(|e| format!("写入配置失败: {e}"))?;
    // 立即刷新 PATH 缓存，使新路径生效（无需重启应用）
    super::refresh_enhanced_path();
    crate::commands::service::invalidate_cli_detection_cache();
    Ok(())
}

#[tauri::command]
pub fn write_env_file(path: String, config: String) -> Result<(), String> {
    let expanded = if let Some(stripped) = path.strip_prefix("~/") {
        dirs::home_dir().unwrap_or_default().join(stripped)
    } else {
        PathBuf::from(&path)
    };

    // 安全限制：只允许写入 ~/.openclaw/ 目录下的文件
    let openclaw_base = super::openclaw_dir();
    if !expanded.starts_with(&openclaw_base) {
        return Err(format!("只允许写入 {} 目录下的文件", openclaw_base.display()));
    }

    if let Some(parent) = expanded.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&expanded, &config).map_err(|e| format!("写入 .env 失败: {e}"))
}

// ===== 备份管理 =====

#[tauri::command]
pub fn list_backups() -> Result<Value, String> {
    let dir = backups_dir();
    if !dir.exists() {
        return Ok(Value::Array(vec![]));
    }
    let mut backups: Vec<Value> = vec![];
    let entries = fs::read_dir(&dir).map_err(|e| format!("读取备份目录失败: {e}"))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let meta = fs::metadata(&path).ok();
        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        // macOS 支持 created()，fallback 到 modified()
        let created = meta
            .and_then(|m| m.created().ok().or_else(|| m.modified().ok()))
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut obj = serde_json::Map::new();
        obj.insert("name".into(), Value::String(name));
        obj.insert("size".into(), Value::Number(size.into()));
        obj.insert("created_at".into(), Value::Number(created.into()));
        backups.push(Value::Object(obj));
    }
    // 按时间倒序
    backups.sort_by(|a, b| {
        let ta = a.get("created_at").and_then(|v| v.as_u64()).unwrap_or(0);
        let tb = b.get("created_at").and_then(|v| v.as_u64()).unwrap_or(0);
        tb.cmp(&ta)
    });
    Ok(Value::Array(backups))
}

#[tauri::command]
pub fn create_backup() -> Result<Value, String> {
    let dir = backups_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("创建备份目录失败: {e}"))?;

    let src = super::openclaw_dir().join("openclaw.json");
    if !src.exists() {
        return Err("openclaw.json 不存在".into());
    }

    let now = chrono::Local::now();
    let name = format!("openclaw-{}.json", now.format("%Y%m%d-%H%M%S"));
    let dest = dir.join(&name);
    fs::copy(&src, &dest).map_err(|e| format!("备份失败: {e}"))?;

    let size = fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    let mut obj = serde_json::Map::new();
    obj.insert("name".into(), Value::String(name));
    obj.insert("size".into(), Value::Number(size.into()));
    Ok(Value::Object(obj))
}

/// 检查备份文件名是否安全
fn is_unsafe_backup_name(name: &str) -> bool {
    name.contains("..") || name.contains('/') || name.contains('\\')
}

#[tauri::command]
pub fn restore_backup(name: String) -> Result<(), String> {
    if is_unsafe_backup_name(&name) {
        return Err("非法文件名".into());
    }
    let backup_path = backups_dir().join(&name);
    if !backup_path.exists() {
        return Err(format!("备份文件不存在: {name}"));
    }
    let target = super::openclaw_dir().join("openclaw.json");

    // 恢复前先自动备份当前配置
    if target.exists() {
        let _ = create_backup();
    }

    fs::copy(&backup_path, &target).map_err(|e| format!("恢复失败: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn delete_backup(name: String) -> Result<(), String> {
    if is_unsafe_backup_name(&name) {
        return Err("非法文件名".into());
    }
    let path = backups_dir().join(&name);
    if !path.exists() {
        return Err(format!("备份文件不存在: {name}"));
    }
    fs::remove_file(&path).map_err(|e| format!("删除失败: {e}"))
}

/// 获取当前用户 UID（macOS/Linux 用 id -u，Windows 返回 0）
#[allow(dead_code)]
fn get_uid() -> Result<u32, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(0)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("id")
            .arg("-u")
            .output()
            .map_err(|e| format!("获取 UID 失败: {e}"))?;
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<u32>()
            .map_err(|e| format!("解析 UID 失败: {e}"))
    }
}

/// 重载 Gateway 配置（热重载，不重启进程）
/// 通过 HTTP POST 向 Gateway 发送 reload 信号，避免触发完整的服务重启循环
#[allow(dead_code)]
async fn reload_gateway_via_http() -> Result<String, String> {
    // 读取 gateway 端口和 token
    let config_path = crate::commands::openclaw_dir().join("openclaw.json");
    let content = std::fs::read_to_string(&config_path).map_err(|e| format!("读取配置失败: {e}"))?;
    let config: serde_json::Value = serde_json::from_str(&content).map_err(|e| format!("解析配置失败: {e}"))?;

    let gw_port = config
        .get("gateway")
        .and_then(|g| g.get("port"))
        .and_then(|p| p.as_u64())
        .unwrap_or(18789) as u16;

    let token = config
        .get("gateway")
        .and_then(|g| g.get("auth"))
        .and_then(|a| a.get("token"))
        .and_then(|t| t.as_str())
        .unwrap_or("");

    // 尝试两个可能的 control UI 端口
    let control_ports = [gw_port + 2, 18792];

    for ctrl_port in control_ports {
        let url = format!("http://127.0.0.1:{}/__api/reload", ctrl_port);
        let client = crate::commands::build_http_client(std::time::Duration::from_secs(5), Some("Workbench"))?;

        let mut req = client.post(&url);
        if !token.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        match req.send().await {
            Ok(resp) if resp.status().is_success() => {
                return Ok("Gateway 配置已热重载".to_string());
            }
            Ok(resp) => {
                eprintln!("[reload_gateway] 端口 {ctrl_port} 返回状态: {}", resp.status());
            }
            Err(e) => {
                eprintln!("[reload_gateway] 端口 {ctrl_port} 请求失败: {e}");
            }
        }
    }

    eprintln!("[reload_gateway] HTTP 热重载不可用");
    Err("Gateway HTTP 重载不可用".to_string())
}

/// 重载 Gateway 服务
/// Windows/Linux: 只尝试 HTTP 热重载，不隐式重启进程。
/// 需要完整重启时必须调用 restart_gateway，让前端 Gateway 状态机统一管理。
#[allow(unused_variables)]
async fn reload_gateway_internal(app: Option<&tauri::AppHandle>) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let uid = get_uid()?;
        let target = format!("gui/{uid}/ai.openclaw.gateway");
        let output = tokio::process::Command::new("launchctl")
            .args(["kickstart", "-k", &target])
            .output()
            .await
            .map_err(|e| format!("重载失败: {e}"))?;
        if output.status.success() {
            Ok("Gateway 已重载".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("重载失败: {stderr}"))
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let (running, _) = crate::commands::service::current_gateway_runtime("ai.openclaw.gateway").await;
        if !running {
            return Ok("Gateway 当前未运行，配置将在下次启动时生效".to_string());
        }
        reload_gateway_via_http().await
    }
}

async fn restart_gateway_internal(app: Option<&tauri::AppHandle>) -> Result<String, String> {
    let (running, _) = crate::commands::service::current_gateway_runtime("ai.openclaw.gateway").await;
    if !running {
        return Ok("Gateway 当前未运行，配置将在下次启动时生效".to_string());
    }
    crate::commands::service::restart_service(
        app.cloned().ok_or_else(|| "缺少 AppHandle，无法重启 Gateway".to_string())?,
        "ai.openclaw.gateway".into(),
    )
    .await
    .map(|_| "Gateway 已重启".to_string())
}

/// 全局 Gateway 重启 mutex（单飞行锁）
/// 保证同时只有一个重启操作在运行，彻底避免僵尸进程堆积（issue #243）
static RESTART_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
/// 上一次重启完成的时间戳（用于 2 秒冷却，防止穿透式重复调用）
static LAST_RESTART_FINISHED_AT: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);

const RESTART_COOLDOWN: std::time::Duration = std::time::Duration::from_secs(2);

/// 带单飞行锁和 2s 冷却的 restart 入口
/// 即使前端穿透节流发来多个请求，后端也只串行执行，且 2s 内不重复
async fn restart_gateway_guarded(app: Option<&tauri::AppHandle>, full_restart: bool) -> Result<String, String> {
    // 获取 mutex：并发调用时串行化
    let _guard = RESTART_MUTEX.lock().await;

    // 2 秒冷却：如果刚刚才完成一次重启，跳过本次（配置已被前一次生效）
    let last_finished = {
        let guard = LAST_RESTART_FINISHED_AT
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard
    };
    if let Some(last) = last_finished {
        if last.elapsed() < RESTART_COOLDOWN {
            return Ok("Gateway 刚重启过，本次请求已合并（冷却中）".to_string());
        }
    }

    let result = if full_restart {
        restart_gateway_internal(app).await
    } else {
        reload_gateway_internal(app).await
    };

    // 无论成功失败都记录时间，避免失败后被重试风暴压爆
    {
        let mut guard = LAST_RESTART_FINISHED_AT
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = Some(std::time::Instant::now());
    }

    result
}
