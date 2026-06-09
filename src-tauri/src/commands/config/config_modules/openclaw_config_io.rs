
fn backups_dir() -> PathBuf {
    super::openclaw_dir().join("backups")
}

#[tauri::command]
pub fn read_openclaw_config() -> Result<Value, String> {
    let path = super::openclaw_dir().join("openclaw.json");
    let raw = fs::read(&path).map_err(|e| format!("读取配置失败: {e}"))?;

    // 自愈：自动剥离 UTF-8 BOM（EF BB BF），防止 JSON 解析失败
    let content = if raw.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(&raw[3..]).into_owned()
    } else {
        String::from_utf8_lossy(&raw).into_owned()
    };

    // 解析 JSON，失败时尝试自动修复或从备份恢复
    let mut config: Value = match serde_json::from_str(&content) {
        Ok(v) => {
            // BOM 被剥离过，静默写回干净文件
            if raw.starts_with(&[0xEF, 0xBB, 0xBF]) {
                let _ = fs::write(&path, &content);
            }
            v
        }
        Err(e) => {
            // JSON 解析失败，尝试自动修复
            let fixed_content = fix_common_json_errors(&content);
            if let Ok(v) = serde_json::from_str(&fixed_content) {
                eprintln!("自动修复了配置文件的 JSON 语法错误");
                // 写回修复后的配置
                let _ = fs::write(&path, &fixed_content);
                v
            } else {
                // 自动修复失败，尝试从备份恢复
                let bak = super::openclaw_dir().join("openclaw.json.bak");
                if bak.exists() {
                    let bak_raw = fs::read(&bak).map_err(|e2| format!("备份也读取失败: {e2}"))?;
                    let bak_content = if bak_raw.starts_with(&[0xEF, 0xBB, 0xBF]) {
                        String::from_utf8_lossy(&bak_raw[3..]).into_owned()
                    } else {
                        String::from_utf8_lossy(&bak_raw).into_owned()
                    };
                    let bak_config: Value = serde_json::from_str(&bak_content)
                        .map_err(|e2| format!("配置损坏且备份也无效: 原始错误='{}', 备份错误='{}'", e, e2))?;
                    // 备份有效，恢复主文件
                    let _ = fs::write(&path, &bak_content);
                    eprintln!("从备份恢复了配置文件");
                    bak_config
                } else {
                    return Err(format!("配置 JSON 损坏且无备份: {} (行: {}, 列: {})", e, e.line(), e.column()));
                }
            }
        }
    };

    // 自动清理 UI 专属字段，防止污染配置导致 CLI 启动失败
    if has_ui_fields(&config) {
        config = strip_ui_fields(config);
        // 静默写回清理后的配置
        let bak = super::openclaw_dir().join("openclaw.json.bak");
        let _ = fs::copy(&path, &bak);
        let json = serde_json::to_string_pretty(&config).map_err(|e| format!("序列化失败: {e}"))?;
        let _ = fs::write(&path, json);
    }

    Ok(config)
}

/// 尝试自动修复常见的 JSON 语法错误
/// Issue #127: 增强配置读取容错性
fn fix_common_json_errors(content: &str) -> String {
    let mut fixed = content.to_string();

    // 修复尾随逗号（在 ] 或 } 之前的逗号）
    // 模式: ,] 或 ,}
    fixed = fixed.replace(",]", "]");
    fixed = fixed.replace(",}", "}");

    // 修复多余逗号（在键值对后面的逗号）
    while fixed.contains(",,") {
        fixed = fixed.replace(",,", ",");
    }

    // 修复单引号：在字符串外将单引号替换为双引号
    fixed = simple_fix_single_quotes(&fixed);

    // 移除 JavaScript 风格的注释（// 或 /* */）
    // 注意：必须正确处理字符串内的 // （如 URL 中的 https://）
    let lines: Vec<&str> = fixed.lines().collect();
    let cleaned_lines: Vec<&str> = lines
        .iter()
        .map(|line| {
            // 逐字符扫描，跳过字符串内部，找到字符串外的 //
            let chars: Vec<char> = line.chars().collect();
            let mut in_string = false;
            let mut i = 0;
            while i < chars.len() {
                if chars[i] == '\\' && in_string {
                    // 转义字符，跳过下一个字符
                    i += 2;
                    continue;
                }
                if chars[i] == '"' {
                    in_string = !in_string;
                }
                if !in_string && i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
                    // 找到字符串外的 //，截断该行
                    let truncated: String = chars[..i].iter().collect();
                    return Box::leak(truncated.into_boxed_str()) as &str;
                }
                i += 1;
            }
            *line
        })
        .collect();
    fixed = cleaned_lines.join("\n");

    // 移除多行注释 /* ... */
    // 简化处理：只在确认不在字符串内时移除
    static RE_MULTI_COMMENT: std::sync::LazyLock<Option<regex::Regex>> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"/\*[\s\S]*?\*/").ok());
    if let Some(re_multi_comment) = RE_MULTI_COMMENT.as_ref() {
        if re_multi_comment.is_match(&fixed) {
            fixed = re_multi_comment.replace_all(&fixed, "").to_string();
        }
    }

    fixed
}

/// 简单的单引号修复（fallback 方案）
fn simple_fix_single_quotes(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut in_string = false;
    let chars: Vec<char> = content.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let prev_char = if i > 0 { Some(chars[i - 1]) } else { None };

        if c == '"' && prev_char != Some('\\') {
            in_string = !in_string;
            result.push(c);
        } else if !in_string && c == '\'' {
            // 在字符串外，将单引号替换为双引号
            result.push('"');
        } else {
            result.push(c);
        }
        i += 1;
    }

    result
}

/// 供其他模块复用：读取 openclaw.json 为 JSON Value
pub fn load_openclaw_json() -> Result<Value, String> {
    read_openclaw_config()
}

/// 供其他模块复用：将 JSON Value 写回 openclaw.json（含备份和清理）
pub fn save_openclaw_json(config: &Value) -> Result<(), String> {
    write_openclaw_config(config.clone())
}

/// 供其他模块复用：触发 Gateway 重载
pub async fn do_reload_gateway(app: &tauri::AppHandle) -> Result<String, String> {
    reload_gateway_internal(Some(app)).await
}

#[tauri::command]
pub fn write_openclaw_config(config: Value) -> Result<(), String> {
    let path = super::openclaw_dir().join("openclaw.json");

    // Issue #127 修复：先读取现有配置，合并后写入
    // 这样可以保留用户手动添加的合法字段（如 browser.profiles）
    // 即使这些字段不在前端传入的配置对象中
    let existing_config = fs::read_to_string(&path)
        .ok()
        .and_then(|c| serde_json::from_str::<Value>(&c).ok());

    // 备份
    let bak = super::openclaw_dir().join("openclaw.json.bak");
    let _ = fs::copy(&path, &bak);

    // 合并配置：现有配置 + 新配置
    // 策略：遍历现有配置，保留所有非 UI 字段
    // 然后将新配置的值覆盖到合并结果中
    let merged = if let Some(existing) = existing_config {
        merge_configs_preserving_fields(&existing, &config)
    } else {
        config.clone()
    };

    // 清理 UI 专属字段，避免 CLI schema 校验失败
    let cleaned = strip_ui_fields(merged);

    // 写入
    let json = serde_json::to_string_pretty(&cleaned).map_err(|e| format!("序列化失败: {e}"))?;
    fs::write(&path, &json).map_err(|e| format!("写入失败: {e}"))?;

    // 同步 provider 配置到所有 agent 的 models.json（运行时注册表）
    // 必须使用与磁盘一致的 merged+strip 结果，而非前端原始 payload：
    // 否则 partial 写入时 merge 保留了其它 provider，但 sync 按 payload 会把
    // agents/*/agent/models.json 里多出的 provider 整棵删掉，造成与 openclaw.json 不一致。
    sync_providers_to_agent_models(&cleaned);

    Ok(())
}

const CALIBRATION_RESET_INHERIT_KEYS: &[&str] = &[
    "agents", "auth", "bindings", "browser", "channels", "commands", "env", "hooks", "models", "plugins", "session", "skills",
    "wizard",
];

fn calibration_required_origins() -> Vec<String> {
    vec![
        "tauri://localhost".into(),
        "https://tauri.localhost".into(),
        "http://tauri.localhost".into(),
        "http://localhost".into(),
        "http://localhost:1420".into(),
        "http://127.0.0.1:1420".into(),
        "http://localhost:18789".into(),
        "http://127.0.0.1:18789".into(),
        "http://localhost:18777".into(),
        "http://127.0.0.1:18777".into(),
    ]
}

fn calibration_last_touched_version() -> String {
    offline_recommended_version_for("chinese").unwrap_or_else(|| "2026.1.1".to_string())
}

fn calibration_default_workspace() -> String {
    super::openclaw_dir().join("workspace").to_string_lossy().to_string()
}

fn generate_calibration_token() -> String {
    format!("cp-{:016x}{:016x}", rand::random::<u64>(), rand::random::<u64>())
}

fn decode_json_bytes(raw: &[u8]) -> String {
    if raw.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(&raw[3..]).into_owned()
    } else {
        String::from_utf8_lossy(raw).into_owned()
    }
}

fn parse_json_relaxed(content: &str) -> Option<Value> {
    serde_json::from_str(content)
        .ok()
        .or_else(|| serde_json::from_str(&fix_common_json_errors(content)).ok())
}

fn read_json_file_relaxed(path: &PathBuf) -> Option<Value> {
    let raw = fs::read(path).ok()?;
    let content = decode_json_bytes(&raw);
    parse_json_relaxed(&content)
}

fn calibration_has_usable_gateway_auth(auth: &Value) -> bool {
    let mode = auth.get("mode").and_then(|v| v.as_str()).unwrap_or("");
    match mode {
        "token" => auth
            .get("token")
            .and_then(|v| v.as_str())
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false),
        "password" => auth
            .get("password")
            .and_then(|v| v.as_str())
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false),
        _ => false,
    }
}

fn calibration_richness_score(config: &Value) -> usize {
    let mut score = 0;
    if config
        .pointer("/models/providers")
        .and_then(|v| v.as_object())
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        score += 4;
    }
    if config.pointer("/agents/defaults").is_some() {
        score += 2;
    }
    if config
        .pointer("/agents/list")
        .and_then(|v| v.as_array())
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        score += 3;
    }
    if config
        .get("channels")
        .and_then(|v| v.as_object())
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        score += 2;
    }
    if config
        .get("bindings")
        .and_then(|v| v.as_array())
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        score += 2;
    }
    if config
        .pointer("/plugins/entries")
        .and_then(|v| v.as_object())
        .map(|v| !v.is_empty())
        .unwrap_or(false)
        || config
            .pointer("/plugins/installs")
            .and_then(|v| v.as_object())
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    {
        score += 2;
    }
    if config
        .get("env")
        .and_then(|v| v.as_object())
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        score += 1;
    }
    if config
        .pointer("/gateway/auth")
        .map(calibration_has_usable_gateway_auth)
        .unwrap_or(false)
    {
        score += 3;
    }
    if config
        .pointer("/gateway/controlUi/allowedOrigins")
        .and_then(|v| v.as_array())
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        score += 1;
    }
    score
}

fn select_calibration_source(current: Option<Value>, backup: Option<Value>) -> (String, Value) {
    match (current, backup) {
        (Some(current), Some(backup)) => {
            let current_score = calibration_richness_score(&current);
            let backup_score = calibration_richness_score(&backup);
            if backup_score > current_score {
                ("backup".into(), backup)
            } else {
                ("current".into(), current)
            }
        }
        (Some(current), None) => ("current".into(), current),
        (None, Some(backup)) => ("backup".into(), backup),
        (None, None) => ("empty".into(), crate::jv!({})),
    }
}

fn build_calibration_baseline() -> Value {
    crate::jv!({
        "$schema": "https://openclaw.ai/schema/config.json",
        "meta": {
            "lastTouchedVersion": calibration_last_touched_version(),
        },
        "models": { "providers": {} },
        "agents": {
            "defaults": {
                "workspace": calibration_default_workspace(),
            },
            "list": [],
        },
        "bindings": [],
        "channels": {},
        "commands": {
            "native": "auto",
            "nativeSkills": "auto",
            "ownerDisplay": "raw",
            "restart": true,
        },
        "plugins": {},
        "session": { "dmScope": "per-channel-peer" },
        "skills": { "entries": {} },
        "tools": {
            "profile": "full",
            "sessions": { "visibility": "all" },
        },
        "gateway": {
            "mode": "local",
            "bind": "loopback",
            "port": 18789,
            "auth": {
                "mode": "token",
                "token": generate_calibration_token(),
            },
            "controlUi": {
                "enabled": true,
                "allowedOrigins": calibration_required_origins(),
                "allowInsecureAuth": true,
            },
        },
    })
}

fn apply_reset_inheritance(mut config: Value, seed: &Value) -> (Value, Vec<String>) {
    let mut inherited = Vec::new();
    let Some(root) = config.as_object_mut() else {
        return (config, inherited);
    };

    for key in CALIBRATION_RESET_INHERIT_KEYS {
        if let Some(value) = seed.get(*key) {
            root.insert((*key).to_string(), value.clone());
            inherited.push((*key).to_string());
        }
    }

    if let Some(web) = seed.pointer("/tools/web").cloned() {
        let tools = root.entry("tools").or_insert_with(|| crate::jv!({}));
        if !tools.is_object() {
            *tools = crate::jv!({});
        }
        if let Some(tools_obj) = tools.as_object_mut() {
            tools_obj.insert("web".into(), web);
            inherited.push("tools.web".into());
        }
    }

    (config, inherited)
}

fn normalize_calibrated_config(mut config: Value) -> Value {
    let required_origins = calibration_required_origins();
    let last_touched_version = calibration_last_touched_version();
    let default_workspace = calibration_default_workspace();

    let Some(root) = config.as_object_mut() else {
        return build_calibration_baseline();
    };

    root.insert("$schema".into(), Value::String("https://openclaw.ai/schema/config.json".into()));

    let meta = root.entry("meta").or_insert_with(|| crate::jv!({}));
    if !meta.is_object() {
        *meta = crate::jv!({});
    }
    if let Some(meta_obj) = meta.as_object_mut() {
        meta_obj.insert("lastTouchedVersion".into(), Value::String(last_touched_version));
        meta_obj.insert("lastTouchedAt".into(), Value::String(chrono::Utc::now().to_rfc3339()));
    }

    let models = root.entry("models").or_insert_with(|| crate::jv!({}));
    if !models.is_object() {
        *models = crate::jv!({});
    }
    if let Some(models_obj) = models.as_object_mut() {
        let providers = models_obj.entry("providers").or_insert_with(|| crate::jv!({}));
        if !providers.is_object() {
            *providers = crate::jv!({});
        }
    }

    let agents = root.entry("agents").or_insert_with(|| crate::jv!({}));
    if !agents.is_object() {
        *agents = crate::jv!({});
    }
    if let Some(agents_obj) = agents.as_object_mut() {
        let defaults = agents_obj.entry("defaults").or_insert_with(|| crate::jv!({}));
        if !defaults.is_object() {
            *defaults = crate::jv!({});
        }
        if let Some(defaults_obj) = defaults.as_object_mut() {
            if !defaults_obj
                .get("workspace")
                .and_then(|v| v.as_str())
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
            {
                defaults_obj.insert("workspace".into(), Value::String(default_workspace));
            }
        }
        let list = agents_obj.entry("list").or_insert_with(|| crate::jv!([]));
        if !list.is_array() {
            *list = crate::jv!([]);
        }
    }

    let bindings = root.entry("bindings").or_insert_with(|| crate::jv!([]));
    if !bindings.is_array() {
        *bindings = crate::jv!([]);
    }

    let channels = root.entry("channels").or_insert_with(|| crate::jv!({}));
    if !channels.is_object() {
        *channels = crate::jv!({});
    }

    let plugins = root.entry("plugins").or_insert_with(|| crate::jv!({}));
    if !plugins.is_object() {
        *plugins = crate::jv!({});
    }

    let tools = root.entry("tools").or_insert_with(|| crate::jv!({}));
    if !tools.is_object() {
        *tools = crate::jv!({});
    }
    if let Some(tools_obj) = tools.as_object_mut() {
        if !tools_obj
            .get("profile")
            .and_then(|v| v.as_str())
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
        {
            tools_obj.insert("profile".into(), Value::String("full".into()));
        }
        let sessions = tools_obj.entry("sessions").or_insert_with(|| crate::jv!({}));
        if !sessions.is_object() {
            *sessions = crate::jv!({});
        }
        if let Some(sessions_obj) = sessions.as_object_mut() {
            if !sessions_obj
                .get("visibility")
                .and_then(|v| v.as_str())
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
            {
                sessions_obj.insert("visibility".into(), Value::String("all".into()));
            }
        }
    }

    let gateway = root.entry("gateway").or_insert_with(|| crate::jv!({}));
    if !gateway.is_object() {
        *gateway = crate::jv!({});
    }
    if let Some(gateway_obj) = gateway.as_object_mut() {
        if !gateway_obj
            .get("mode")
            .and_then(|v| v.as_str())
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
        {
            gateway_obj.insert("mode".into(), Value::String("local".into()));
        }

        let port_valid = gateway_obj
            .get("port")
            .and_then(|v| v.as_u64())
            .map(|port| (1..=65535).contains(&port))
            .unwrap_or(false);
        if !port_valid {
            gateway_obj.insert("port".into(), crate::jv!(18789));
        }

        if !gateway_obj
            .get("bind")
            .and_then(|v| v.as_str())
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
        {
            gateway_obj.insert("bind".into(), Value::String("loopback".into()));
        }

        let auth_valid = gateway_obj
            .get("auth")
            .map(calibration_has_usable_gateway_auth)
            .unwrap_or(false);
        if !auth_valid {
            gateway_obj.insert(
                "auth".into(),
                crate::jv!({
                    "mode": "token",
                    "token": generate_calibration_token(),
                }),
            );
        }

        let control_ui = gateway_obj.entry("controlUi").or_insert_with(|| crate::jv!({}));
        if !control_ui.is_object() {
            *control_ui = crate::jv!({});
        }
        if let Some(control_ui_obj) = control_ui.as_object_mut() {
            let existing: Vec<String> = control_ui_obj
                .get("allowedOrigins")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|value| value.as_str().map(|value| value.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let mut merged = existing;
            for origin in required_origins {
                if !merged.iter().any(|existing| existing == &origin) {
                    merged.push(origin);
                }
            }
            control_ui_obj.insert("allowedOrigins".into(), crate::jv!(merged));
            control_ui_obj.insert("enabled".into(), Value::Bool(true));
            control_ui_obj.insert("allowInsecureAuth".into(), Value::Bool(true));
        }
    }

    config
}

#[tauri::command]
pub fn calibrate_openclaw_config(mode: String) -> Result<Value, String> {
    let normalized_mode = match mode.trim() {
        "inherit" => "inherit",
        "reset" | "reinitialize" => "reset",
        _ => return Err("mode 必须是 inherit 或 reset".into()),
    };

    let dir = super::openclaw_dir();
    let config_path = dir.join("openclaw.json");
    let backup_path = dir.join("openclaw.json.bak");
    fs::create_dir_all(&dir).map_err(|e| format!("创建配置目录失败: {e}"))?;

    let mut warnings: Vec<String> = vec![];
    let pre_backup = if config_path.exists() {
        match create_backup() {
            Ok(result) => result
                .get("name")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string()),
            Err(err) => {
                warnings.push(format!("修复前备份失败: {err}"));
                None
            }
        }
    } else {
        None
    };

    let current = read_json_file_relaxed(&config_path);
    let backup = read_json_file_relaxed(&backup_path);
    let (source, seed) = select_calibration_source(current, backup);

    let (calibrated, mut inherited_keys) = if normalized_mode == "inherit" {
        let inherited = seed
            .as_object()
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_else(Vec::new);
        (merge_configs_preserving_fields(&build_calibration_baseline(), &seed), inherited)
    } else {
        apply_reset_inheritance(build_calibration_baseline(), &seed)
    };

    inherited_keys.sort();
    inherited_keys.dedup();

    let calibrated = strip_ui_fields(normalize_calibrated_config(calibrated));
    let json = serde_json::to_string_pretty(&calibrated).map_err(|e| format!("序列化校准配置失败: {e}"))?;

    fs::write(&config_path, &json).map_err(|e| format!("写入校准配置失败: {e}"))?;
    fs::write(&backup_path, &json).map_err(|e| format!("写入配置备份失败: {e}"))?;

    sync_providers_to_agent_models(&calibrated);

    Ok(crate::jv!({
        "mode": normalized_mode,
        "source": source,
        "backup": pre_backup,
        "inheritedKeys": inherited_keys,
        "warnings": warnings,
        "message": if normalized_mode == "inherit" {
            "配置已按继承模式校准"
        } else {
            "配置已按完全初始化修复模式校准"
        }
    }))
}
