
// 合并两个配置对象，保留现有配置中的合法字段
//
// Issue #127: 修复配置合并时丢失 browser.* 等合法字段的问题
//
// 策略：对 Object 类型字段做递归合并（新值覆盖旧值，旧值中新配置没有的字段保留）。
// 这样用户通过 CLI / 手动编辑添加的自定义子字段不会被前端的部分配置所覆盖掉。
//
// 清理的字段：
/// - UI 专属字段（通过 strip_ui_fields 处理）
fn merge_configs_preserving_fields(existing: &Value, new: &Value) -> Value {
    use serde_json::Value;

    match (existing, new) {
        (Value::Object(existing_obj), Value::Object(new_obj)) => {
            let mut merged = existing_obj.clone();

            for (key, new_value) in new_obj {
                if let Some(existing_value) = existing_obj.get(key) {
                    merged.insert(key.clone(), merge_configs_preserving_fields(existing_value, new_value));
                } else {
                    // 现有配置没有此 key，使用新值
                    merged.insert(key.clone(), new_value.clone());
                }
            }

            Value::Object(merged)
        }
        // 非对象类型，直接使用新配置
        _ => new.clone(),
    }
}

/// 已知需要清理的 UI 字段列表（用于诊断报告）
const KNOWN_UI_FIELDS: &[&str] = &[
    "current",
    "latest",
    "recommended",
    "update_available",
    "latest_update_available",
    "is_recommended",
    "ahead_of_recommended",
    "panel_version",
    "source",
    // models.providers 中的 UI 字段
    "lastTestAt",
    "latency",
    "testStatus",
    "testError",
    "profiles",
];

// 已知需要保留的合法 OpenClaw 配置字段（用于诊断报告）
// 这些字段虽然不在标准列表中，但不应被警告为未知字段
/// 注意：这些字段在 `merge_configs_preserving_fields` 中会被特殊处理
#[allow(dead_code)]
const KNOWN_LEGAL_FIELDS: &[&str] = &["browser", "agents", "gateway", "logging", "mcp"];

// KNOWN_LEGAL_FIELDS 目前在诊断逻辑中使用，用于生成报告信息

// 验证 openclaw.json 配置，报告潜在问题
//
// Issue #127: 新增诊断命令，帮助用户识别配置问题
//
// 返回内容：
// - config_valid: 配置是否可以正常读取
// - ui_fields_found: 发现的 UI 专属字段（会被自动清理）
// - unknown_fields: 未知的字段（可能是用户手动添加或 OpenClaw 新增）
/// - warnings: 警告信息和建议
#[tauri::command]
pub fn validate_openclaw_config() -> Result<Value, String> {
    let path = super::openclaw_dir().join("openclaw.json");

    // 读取原始内容（不经过自愈逻辑）
    let raw = fs::read(&path).map_err(|e| format!("读取配置失败: {e}"))?;
    let content = if raw.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(&raw[3..]).into_owned()
    } else {
        String::from_utf8_lossy(&raw).into_owned()
    };

    // 尝试解析 JSON
    let config: Value = match serde_json::from_str(&content) {
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
                    if let Ok(bak_content) = fs::read_to_string(&bak) {
                        if serde_json::from_str::<Value>(&bak_content).is_ok() {
                            return Ok(crate::jv!({
                                "config_valid": false,
                                "json_error": format!("JSON 解析失败 (行: {}, 列: {}), 建议从备份恢复", e.line(), e.column()),
                                "backup_exists": true,
                                "warnings": [
                                    "配置文件损坏，建议使用备份恢复",
                                    "备份文件：openclaw.json.bak"
                                ]
                            }));
                        }
                    }
                }
                return Ok(crate::jv!({
                    "config_valid": false,
                    "json_error": format!("JSON 解析失败 (行: {}, 列: {}): {}", e.line(), e.column(), e),
                    "warnings": [
                        "配置文件严重损坏且无有效备份",
                        "建议：手动检查或重新创建配置文件"
                    ]
                }));
            }
        }
    };

    // 分析配置内容
    let mut ui_fields_found: Vec<String> = Vec::new();
    let mut unknown_fields: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // 检查根层级的 UI 字段
    if let Some(obj) = config.as_object() {
        for key in obj.keys() {
            if KNOWN_UI_FIELDS.contains(&key.as_str()) {
                ui_fields_found.push(format!("根层级.{}", key));
            }
        }

        // 检查 browser 字段是否存在
        if obj.contains_key("browser") {
            if let Some(browser) = obj.get("browser") {
                if let Some(browser_obj) = browser.as_object() {
                    // 检查 browser.profiles
                    if browser_obj.contains_key("profiles") {
                        warnings.push("发现 browser.profiles 字段，这是 OpenClaw 合法的配置字段，将被保留".to_string());
                    }
                    // 报告 browser 中的其他未知字段
                    for key in browser_obj.keys() {
                        if key != "profiles" {
                            unknown_fields.push(format!("browser.{}", key));
                        }
                    }
                }
            }
        }

        // 检查 agents 字段
        if obj.contains_key("agents") {
            if let Some(agents) = obj.get("agents") {
                if let Some(agents_obj) = agents.as_object() {
                    // 检查 agents 子字段（上游 schema 只定义 agents.list）
                    if agents_obj.contains_key("profiles") {
                        warnings.push("发现 agents.profiles 字段，上游 schema 未定义此字段，当前工作台会自动清理".to_string());
                    }
                    // 检查 agents.list 中的元素
                    if let Some(Value::Array(list)) = agents_obj.get("list") {
                        for (idx, agent) in list.iter().enumerate() {
                            if let Some(agent_obj) = agent.as_object() {
                                for key in agent_obj.keys() {
                                    if KNOWN_UI_FIELDS.contains(&key.as_str()) {
                                        ui_fields_found.push(format!("agents.list[{}].{}", idx, key));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 检查 models.providers 中的测试状态字段
        if let Some(models) = obj.get("models") {
            if let Some(models_obj) = models.as_object() {
                if let Some(providers) = models_obj.get("providers") {
                    if let Some(providers_obj) = providers.as_object() {
                        for (provider_name, provider_val) in providers_obj {
                            if let Some(provider_obj) = provider_val.as_object() {
                                if let Some(Value::Array(models_arr)) = provider_obj.get("models") {
                                    for (model_idx, model) in models_arr.iter().enumerate() {
                                        if let Some(model_obj) = model.as_object() {
                                            for field in ["lastTestAt", "latency", "testStatus", "testError"] {
                                                if model_obj.contains_key(field) {
                                                    ui_fields_found.push(format!(
                                                        "models.providers.{}.models[{}].{}",
                                                        provider_name, model_idx, field
                                                    ));
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

        // 生成警告信息
        if !ui_fields_found.is_empty() {
            warnings.push(format!("发现 {} 个 UI 专属字段，将被自动清理", ui_fields_found.len()));
        }
    }

    Ok(crate::jv!({
        "config_valid": true,
        "ui_fields_found": ui_fields_found,
        "unknown_fields": unknown_fields,
        "warnings": warnings,
        "suggestions": if !ui_fields_found.is_empty() || !unknown_fields.is_empty() {
            vec![
                "UI 专属字段会被当前工作台自动清理，不影响 OpenClaw 运行".to_string(),
                "未知字段如果是用户手动添加的，请确保符合 OpenClaw schema".to_string(),
                "如果遇到 'Unrecognized key' 错误，请检查配置文件是否包含 OpenClaw 不支持的字段".to_string(),
            ]
        } else {
            vec!["配置文件看起来正常，没有发现已知问题".to_string()]
        }
    }))
}

// 将 openclaw.json 的 models.providers 完整同步到每个 agent 的 models.json
// 包括：同步 baseUrl/apiKey/api + 清理已删除的 models
// 确保 Gateway 运行时不会引用 openclaw.json 中已不存在的模型
include!("config_validation_and_sync/model_sync_and_version.rs");
