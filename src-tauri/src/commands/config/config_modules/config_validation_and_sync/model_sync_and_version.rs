fn sync_providers_to_agent_models(config: &Value) {
    let src_providers = config.pointer("/models/providers").and_then(|p| p.as_object());

    // 收集 openclaw.json 中所有有效的 provider/model 组合
    let mut valid_models: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Some(providers) = src_providers {
        for (pk, pv) in providers {
            if let Some(models) = pv.get("models").and_then(|m| m.as_array()) {
                for m in models {
                    let id = m.get("id").and_then(|v| v.as_str()).or_else(|| m.as_str());
                    if let Some(id) = id {
                        valid_models.insert(format!("{}/{}", pk, id));
                    }
                }
            }
        }
    }

    // 收集所有 agent ID
    let mut agent_ids = vec!["main".to_string()];
    if let Some(Value::Array(list)) = config.pointer("/agents/list") {
        for agent in list {
            if let Some(id) = agent.get("id").and_then(|v| v.as_str()) {
                if id != "main" {
                    agent_ids.push(id.to_string());
                }
            }
        }
    }

    let agents_dir = super::openclaw_dir().join("agents");
    for agent_id in &agent_ids {
        let models_path = agents_dir.join(agent_id).join("agent").join("models.json");
        if !models_path.exists() {
            continue;
        }
        let Ok(content) = fs::read_to_string(&models_path) else {
            continue;
        };
        let Ok(mut models_json) = serde_json::from_str::<Value>(&content) else {
            continue;
        };

        let mut changed = false;

        if models_json.get("providers").and_then(|p| p.as_object()).is_none() {
            if let Some(root) = models_json.as_object_mut() {
                root.insert("providers".into(), crate::jv!({}));
                changed = true;
            }
        }

        // 同步 providers
        if let Some(dst_providers) = models_json.get_mut("providers").and_then(|p| p.as_object_mut()) {
            // 1. 删除 openclaw.json 中已不存在的 provider
            if let Some(src) = src_providers {
                let to_remove: Vec<String> = dst_providers
                    .keys()
                    .filter(|k| !src.contains_key(k.as_str()))
                    .cloned()
                    .collect();
                for k in to_remove {
                    dst_providers.remove(&k);
                    changed = true;
                }

                for (provider_name, src_provider) in src.iter() {
                    if !dst_providers.contains_key(provider_name) {
                        dst_providers.insert(provider_name.clone(), src_provider.clone());
                        changed = true;
                    }
                }

                // 2. 同步存在的 provider 的 baseUrl/apiKey/api + 清理已删除的 models
                for (provider_name, src_provider) in src.iter() {
                    if let Some(dst_provider) = dst_providers.get_mut(provider_name) {
                        if let Some(dst_obj) = dst_provider.as_object_mut() {
                            // 同步连接信息
                            for field in ["baseUrl", "apiKey", "api"] {
                                if let Some(src_val) = src_provider.get(field).and_then(|v| v.as_str()) {
                                    if dst_obj.get(field).and_then(|v| v.as_str()) != Some(src_val) {
                                        dst_obj.insert(field.to_string(), Value::String(src_val.to_string()));
                                        changed = true;
                                    }
                                }
                            }
                            // 注意：不删除 agent models.json 中用户手动添加的模型。
                            // 只同步连接信息（baseUrl/apiKey/api），保留用户通过 CLI
                            // 或手动编辑添加的自定义模型。
                        }
                    }
                }
            }
        }

        if changed {
            if let Ok(new_json) = serde_json::to_string_pretty(&models_json) {
                let _ = fs::write(&models_path, new_json);
            }
        }
    }
}

/// 检测配置中是否包含 UI 专属字段
fn has_ui_fields(val: &Value) -> bool {
    if let Some(obj) = val.as_object() {
        for key in &[
            "current",
            "latest",
            "recommended",
            "update_available",
            "latest_update_available",
            "is_recommended",
            "ahead_of_recommended",
            "panel_version",
            "source",
            "qqbot",
            "profiles",
        ] {
            if obj.contains_key(*key) {
                return true;
            }
        }
        if obj
            .get("auth")
            .and_then(|v| v.as_object())
            .map(|auth| auth.contains_key("profiles"))
            .unwrap_or(false)
        {
            return true;
        }
        if obj
            .get("agents")
            .and_then(|v| v.as_object())
            .map(|agents| agents.contains_key("profiles"))
            .unwrap_or(false)
        {
            return true;
        }
        if let Some(models_val) = obj.get("models") {
            if let Some(models_obj) = models_val.as_object() {
                if let Some(providers_val) = models_obj.get("providers") {
                    if let Some(providers_obj) = providers_val.as_object() {
                        for (_provider_name, provider_val) in providers_obj.iter() {
                            if let Some(provider_obj) = provider_val.as_object() {
                                if let Some(Value::Array(arr)) = provider_obj.get("models") {
                                    for model in arr.iter() {
                                        if let Some(mobj) = model.as_object() {
                                            if mobj.contains_key("lastTestAt")
                                                || mobj.contains_key("latency")
                                                || mobj.contains_key("testStatus")
                                                || mobj.contains_key("testError")
                                            {
                                                return true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

/// 清理 ClawPanel 内部字段，避免污染 openclaw.json 导致 Gateway 启动失败
/// Issue #89: version info 字段被写入 openclaw.json → Unknown config keys
/// Issue #127: 增强清理逻辑，保留 OpenClaw 合法的配置字段
///
/// 保留的合法配置字段（不清理）：
/// - `browser.*` - OpenClaw browser profiles 配置（如 browser.profiles）
/// - `agents.list` - OpenClaw agent list 配置
/// - 其他 OpenClaw schema 定义的字段
///
/// 清理的 UI 专属字段：
/// - 根层级：current, latest, update_available 等版本信息
/// - models.providers 中每个 model 的测试状态：lastTestAt, latency, testStatus, testError
fn strip_ui_fields(mut val: Value) -> Value {
    if let Some(obj) = val.as_object_mut() {
        // 清理根层级 ClawPanel 内部字段（version info 等）
        // 注意：保留 browser.* 和 agents.list，这些是 OpenClaw 合法的配置字段
        for key in &[
            "current",
            "latest",
            "recommended",
            "update_available",
            "latest_update_available",
            "is_recommended",
            "ahead_of_recommended",
            "panel_version",
            "source",
            // 渠道插件别名：OpenClaw schema 不承认 qqbot 作为根键（应写在 channels.qqbot）
            "qqbot",
            "profiles",
        ] {
            obj.remove(*key);
        }
        if let Some(auth_val) = obj.get_mut("auth") {
            if let Some(auth_obj) = auth_val.as_object_mut() {
                auth_obj.remove("profiles");
            }
        }
        // 处理 models.providers.xxx.models 结构
        if let Some(models_val) = obj.get_mut("models") {
            if let Some(models_obj) = models_val.as_object_mut() {
                if let Some(providers_val) = models_obj.get_mut("providers") {
                    if let Some(providers_obj) = providers_val.as_object_mut() {
                        for (_provider_name, provider_val) in providers_obj.iter_mut() {
                            if let Some(provider_obj) = provider_val.as_object_mut() {
                                if let Some(Value::Array(arr)) = provider_obj.get_mut("models") {
                                    for model in arr.iter_mut() {
                                        if let Some(mobj) = model.as_object_mut() {
                                            mobj.remove("lastTestAt");
                                            mobj.remove("latency");
                                            mobj.remove("testStatus");
                                            mobj.remove("testError");
                                            if !mobj.contains_key("name") {
                                                if let Some(id) = mobj.get("id").and_then(|v| v.as_str()) {
                                                    mobj.insert("name".into(), Value::String(id.to_string()));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // 递归处理 agents 数组中的元素（保留 agents.list 等合法字段）
        if let Some(agents_val) = obj.get_mut("agents") {
            if let Some(agents_obj) = agents_val.as_object_mut() {
                agents_obj.remove("profiles");
                // 保留 agents 子字段不做修改
                // 只清理 agents 数组中的元素（如果有 UI 字段）
                if let Some(Value::Array(arr)) = agents_obj.get_mut("list") {
                    for agent in arr.iter_mut() {
                        if let Some(agent_obj) = agent.as_object_mut() {
                            // 清理 agent 中的 UI 字段，但保留 profiles
                            agent_obj.remove("current");
                            agent_obj.remove("latest");
                            agent_obj.remove("update_available");
                        }
                    }
                }
            }
        }
    }
    val
}

#[tauri::command]
pub fn read_mcp_config() -> Result<Value, String> {
    let path = super::openclaw_dir().join("mcp.json");
    if !path.exists() {
        return Ok(Value::Object(Default::default()));
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("读取 MCP 配置失败: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("解析 JSON 失败: {e}"))
}

#[tauri::command]
pub fn write_mcp_config(config: Value) -> Result<(), String> {
    let path = super::openclaw_dir().join("mcp.json");
    let json = serde_json::to_string_pretty(&config).map_err(|e| format!("序列化失败: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("写入失败: {e}"))
}

/// 获取本地安装的 openclaw 版本号（异步版本）
/// macOS: 优先从 npm 包的 package.json 读取（含完整后缀），fallback 到 CLI
/// Windows/Linux: 优先读文件系统，fallback 到 CLI
async fn get_local_version() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        if let Some(cli_path) = crate::utils::resolve_openclaw_cli_path() {
            let resolved = std::fs::canonicalize(&cli_path)
                .ok()
                .unwrap_or_else(|| PathBuf::from(&cli_path));
            if let Some(ver) = read_version_from_installation(&resolved)
                .or_else(|| read_version_from_installation(std::path::Path::new(&cli_path)))
            {
                return Some(ver);
            }
        }

        for brew_prefix in &["/opt/homebrew/bin", "/usr/local/bin"] {
            let openclaw_path = format!("{}/openclaw", brew_prefix);
            if let Ok(target) = fs::read_link(&openclaw_path) {
                let pkg_json = PathBuf::from(brew_prefix)
                    .join(&target)
                    .parent()
                    .map(|p| p.join("package.json"));
                if let Some(pkg_path) = pkg_json {
                    if let Ok(content) = fs::read_to_string(&pkg_path) {
                        if let Some(ver) = serde_json::from_str::<Value>(&content)
                            .ok()
                            .and_then(|v| v.get("version")?.as_str().map(String::from))
                        {
                            return Some(ver);
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // 优先从活跃 CLI 路径读取版本（与 macOS 逻辑一致）
        if let Some(cli_path) = crate::utils::resolve_openclaw_cli_path() {
            let cli_pb = PathBuf::from(&cli_path);
            let resolved = std::fs::canonicalize(&cli_pb).unwrap_or_else(|_| cli_pb.clone());
            if let Some(ver) = read_version_from_installation(&resolved).or_else(|| read_version_from_installation(&cli_pb)) {
                return Some(ver);
            }
        }

        for sa_dir in all_standalone_dirs() {
            // 仅当 CLI 二进制实际存在时才读取版本，避免残留文件误判为已安装
            if !sa_dir.join("openclaw.cmd").exists() {
                continue;
            }
            let version_file = sa_dir.join("VERSION");
            if let Ok(content) = fs::read_to_string(&version_file) {
                for line in content.lines() {
                    if let Some(ver) = line.strip_prefix("openclaw_version=") {
                        let ver = ver.trim();
                        if !ver.is_empty() {
                            return Some(ver.to_string());
                        }
                    }
                }
            }
            let sa_pkg = sa_dir
                .join("node_modules")
                .join(legacy_openclaw_zh_scope())
                .join(legacy_openclaw_zh_package())
                .join("package.json");
            if let Ok(content) = fs::read_to_string(&sa_pkg) {
                if let Some(ver) = serde_json::from_str::<Value>(&content)
                    .ok()
                    .and_then(|v| v.get("version")?.as_str().map(String::from))
                {
                    return Some(ver);
                }
            }
        }

        if let Some(npm_bin) = npm_global_bin_dir() {
            let shim_path = npm_bin.join("openclaw.cmd");
            // 仅当 npm 全局 CLI shim 存在时才读取版本
            if !shim_path.exists() {
                // npm 全局无 CLI shim，跳过
            } else {
                // 读 .cmd 内容判断活跃包，而非依赖 classify_cli_source（路径无法区分）
                let pkgs: &[&str] = &["openclaw"];
                for pkg in pkgs {
                    let pkg_json = npm_bin.join("node_modules").join(pkg).join("package.json");
                    if let Ok(content) = fs::read_to_string(&pkg_json) {
                        if let Some(ver) = serde_json::from_str::<Value>(&content)
                            .ok()
                            .and_then(|v| v.get("version")?.as_str().map(String::from))
                        {
                            return Some(ver);
                        }
                    }
                }
            }
        }
    }

    // Linux: 参照 macOS/Windows 实现，完整检测链
    #[cfg(target_os = "linux")]
    {
        // 1. 活跃 CLI 优先
        if let Some(cli_path) = crate::utils::resolve_openclaw_cli_path() {
            let cli_pb = PathBuf::from(&cli_path);
            let resolved = std::fs::canonicalize(&cli_pb).unwrap_or_else(|_| cli_pb.clone());
            if let Some(ver) = read_version_from_installation(&resolved).or_else(|| read_version_from_installation(&cli_pb)) {
                return Some(ver);
            }
        }
        // 2. standalone 目录
        for sa_dir in all_standalone_dirs() {
            if sa_dir.join("openclaw").exists() || sa_dir.join("VERSION").exists() {
                if let Some(ver) = read_version_from_installation(&sa_dir.join("openclaw")) {
                    return Some(ver);
                }
            }
        }
        // 3. symlink -> package.json
        if let Ok(target) = fs::read_link("/usr/local/bin/openclaw") {
            let pkg_json = PathBuf::from("/usr/local/bin")
                .join(&target)
                .parent()
                .map(|p| p.join("package.json"));
            if let Some(ref pkg_path) = pkg_json {
                if let Ok(content) = fs::read_to_string(pkg_path) {
                    if let Some(ver) = serde_json::from_str::<Value>(&content)
                        .ok()
                        .and_then(|v| v.get("version")?.as_str().map(String::from))
                    {
                        return Some(ver);
                    }
                }
            }
        }
    }

    if is_portable_runtime_config_dir() {
        return None;
    }

    let mut status_cmd = crate::utils::openclaw_command_async();
    status_cmd.args(["status", "--json"]);
    if let Ok(Ok(output)) = tokio::time::timeout(std::time::Duration::from_secs(2), status_cmd.output()).await {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(ver) = crate::commands::skills::extract_json_pub(&stdout)
                .and_then(|v| v.get("runtimeVersion")?.as_str().map(String::from))
            {
                return Some(ver);
            }
        }
    }

    // 所有平台通用 fallback: CLI 输出
    // Windows: 先确认 openclaw 不是第三方程序（如 CherryStudio）
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        if let Ok(o) = std::process::Command::new("where")
            .arg("openclaw")
            .creation_flags(0x08000000)
            .output()
        {
            let stdout = String::from_utf8_lossy(&o.stdout).to_lowercase();
            let all_third_party = stdout
                .lines()
                .filter(|l| !l.trim().is_empty())
                .all(|l| l.contains(".cherrystudio") || l.contains("cherry-studio"));
            if all_third_party {
                return None;
            }
        }
    }

    use crate::utils::openclaw_command_async;
    let output = openclaw_command_async().arg("--version").output().await.ok()?;
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // 输出格式: "OpenClaw 2026.3.24 (hash)" → 取第一个数字开头的词（版本号）
    raw.split_whitespace()
        .find(|w| w.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .map(String::from)
}