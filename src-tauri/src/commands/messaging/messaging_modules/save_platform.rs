
/// 保存平台配置到 openclaw.json
/// 前端传入的是表单字段，后端负责转换成 OpenClaw 要求的结构
/// account_id: 可选，指定时写入 channels.<platform>.accounts.<account_id>（多账号模式）
/// agent_id: 可选，指定时同时创建 bindings 配置将渠道绑定到 Agent
#[tauri::command]
pub async fn save_messaging_platform(
    platform: String,
    form: Value,
    account_id: Option<String>,
    agent_id: Option<String>,
    app: tauri::AppHandle,
) -> Result<Value, String> {
    let mut cfg = super::config::load_openclaw_json()?;
    let storage_key = platform_storage_key(&platform).to_string();

    let channels = cfg
        .as_object_mut()
        .ok_or("配置格式错误")?
        .entry("channels")
        .or_insert_with(|| crate::jv!({}));
    let channels_map = channels.as_object_mut().ok_or("channels 节点格式错误")?;

    let raw_form_obj = form.as_object().ok_or("表单数据格式错误")?;
    let normalized_form = normalize_messaging_platform_form(&platform, raw_form_obj);
    let form_obj = &normalized_form;
    let current_saved = resolve_platform_config_entry(channels_map.get(storage_key.as_str()), &platform, account_id.as_deref())
        .unwrap_or(Value::Null);

    // 用于后续创建 bindings 的平台信息
    let saved_account_id = account_id.clone();

    save_messaging_platform_entry(
        &mut cfg,
        &storage_key,
        &platform,
        form_obj,
        &current_saved,
        account_id.as_deref(),
    )?;

    // 如果指定了 agent_id，同时创建 bindings 配置
    if let Some(ref agent) = agent_id {
        if !agent.is_empty() {
            create_agent_binding(&mut cfg, agent, &platform, saved_account_id)?;
        }
    }

    // 写回配置并重载 Gateway
    super::config::save_openclaw_json(&cfg)?;

    // Gateway 重载在后台进行，不阻塞 UI 响应
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = super::config::do_reload_gateway(&app2).await;
    });

    Ok(crate::jv!({ "ok": true }))
}

fn messaging_channels_map(cfg: &mut Value) -> Result<&mut Map<String, Value>, String> {
    let channels = cfg
        .as_object_mut()
        .ok_or("??????")?
        .entry("channels")
        .or_insert_with(|| crate::jv!({}));
    channels.as_object_mut().ok_or_else(|| "channels ??????".to_string())
}

fn save_messaging_platform_entry(
    cfg: &mut Value,
    storage_key: &str,
    platform: &str,
    form_obj: &Map<String, Value>,
    current_saved: &Value,
    account_id: Option<&str>,
) -> Result<(), String> {
    match platform {
        "discord" => save_discord_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "telegram" => save_telegram_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "zalo" => save_zalo_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "zalouser" => save_zalouser_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "qqbot" => save_qqbot_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "feishu" => save_feishu_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "dingtalk" | "dingtalk-connector" => save_dingtalk_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "slack" => save_slack_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "whatsapp" => save_whatsapp_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "signal" => save_signal_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "imessage" => save_imessage_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "matrix" => save_matrix_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "msteams" => save_msteams_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "line" => save_line_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "mattermost" => save_mattermost_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "clickclack" => save_clickclack_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "nextcloud-talk" => save_nextcloud_talk_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "twitch" => save_twitch_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "nostr" => save_nostr_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "irc" => save_irc_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "tlon" => save_tlon_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "synology-chat" => save_synology_chat_platform(cfg, storage_key, form_obj, current_saved, account_id),
        "googlechat" => save_googlechat_platform(cfg, storage_key, form_obj, current_saved, account_id),
        _ => save_generic_messaging_platform(cfg, storage_key, form_obj, current_saved, account_id),
    }
}

include!("save_handlers_primary.rs");
include!("save_handlers_extended.rs");
