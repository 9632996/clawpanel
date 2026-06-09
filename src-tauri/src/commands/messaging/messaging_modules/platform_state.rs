
/// 删除指定平台配置
/// account_id: 可选，指定时仅删除 channels.<platform>.accounts.<account_id>（多账号模式）
///             未指定时删除整个平台配置
#[tauri::command]
pub async fn remove_messaging_platform(
    platform: String,
    account_id: Option<String>,
    app: tauri::AppHandle,
) -> Result<Value, String> {
    let mut cfg = super::config::load_openclaw_json()?;
    let storage_key = platform_storage_key(&platform);

    match &account_id {
        Some(acct) if !acct.is_empty() => {
            // 多账号模式：仅删除指定账号
            if let Some(channel) = cfg.get_mut("channels").and_then(|c| c.get_mut(storage_key)) {
                if let Some(accounts) = channel.get_mut("accounts").and_then(|a| a.as_object_mut()) {
                    accounts.remove(acct.as_str());
                }
            }
        }
        _ => {
            // 整平台删除
            if let Some(channels) = cfg.get_mut("channels").and_then(|c| c.as_object_mut()) {
                channels.remove(storage_key);
            }
        }
    }

    // 清理对应的 bindings 条目
    let binding_channel = platform_list_id(&platform);
    if let Some(bindings) = cfg.get_mut("bindings").and_then(|b| b.as_array_mut()) {
        bindings.retain(|b| {
            let m = match b.get("match") {
                Some(m) => m,
                None => return true,
            };
            if m.get("channel").and_then(|v| v.as_str()) != Some(binding_channel) {
                return true; // 不同渠道，保留
            }
            match &account_id {
                Some(acct) if !acct.is_empty() => m.get("accountId").and_then(|v| v.as_str()) != Some(acct.as_str()),
                _ => false, // 整平台删除，移除该渠道所有 binding
            }
        });
    }

    super::config::save_openclaw_json(&cfg)?;
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = super::config::do_reload_gateway(&app2).await;
    });

    Ok(crate::jv!({ "ok": true }))
}

/// 切换平台启用/禁用
#[tauri::command]
pub async fn toggle_messaging_platform(platform: String, enabled: bool, app: tauri::AppHandle) -> Result<Value, String> {
    let mut cfg = super::config::load_openclaw_json()?;
    let storage_key = platform_storage_key(&platform);

    if let Some(entry) = cfg
        .get_mut("channels")
        .and_then(|c| c.get_mut(storage_key))
        .and_then(|v| v.as_object_mut())
    {
        entry.insert("enabled".into(), Value::Bool(enabled));
    } else {
        return Err(format!("平台 {} 未配置", platform));
    }

    super::config::save_openclaw_json(&cfg)?;
    // Gateway 重载在后台进行，不阻塞 UI 响应
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = super::config::do_reload_gateway(&app2).await;
    });

    Ok(crate::jv!({ "ok": true }))
}
