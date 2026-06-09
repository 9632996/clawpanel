
#[tauri::command]
pub fn hermes_provider_overrides_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_provider_overrides_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_provider_overrides_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_provider_overrides_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_provider_overrides_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_mcp_servers_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_mcp_servers_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_mcp_servers_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_mcp_servers_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_mcp_servers_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_agent_toolsets_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_agent_toolsets_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_agent_toolsets_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_agent_toolsets_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_agent_toolsets_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_platform_toolsets_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_platform_toolsets_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_platform_toolsets_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_platform_toolsets_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_platform_toolsets_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_agent_runtime_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_agent_runtime_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_agent_runtime_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_agent_runtime_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_agent_runtime_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_unauthorized_dm_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_unauthorized_dm_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_unauthorized_dm_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_unauthorized_dm_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_unauthorized_dm_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_security_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_security_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_security_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_security_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_security_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_display_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_display_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_display_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_display_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_display_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_kanban_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_kanban_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_kanban_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_kanban_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_kanban_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_human_delay_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_human_delay_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_human_delay_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_human_delay_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_human_delay_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_streaming_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_streaming_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_streaming_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_streaming_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_streaming_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_execution_limits_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_execution_limits_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_execution_limits_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_execution_limits_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_execution_limits_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_io_safety_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_io_safety_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_io_safety_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_io_safety_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_io_safety_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_checkpoints_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_checkpoints_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_checkpoints_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_checkpoints_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_checkpoints_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_cron_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_cron_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_cron_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_cron_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_cron_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_sessions_maintenance_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_sessions_maintenance_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_sessions_maintenance_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_sessions_maintenance_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_sessions_maintenance_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_updates_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_updates_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_updates_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_updates_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_updates_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_logging_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_logging_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_logging_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_logging_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_logging_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_approvals_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_approvals_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_approvals_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_approvals_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_approvals_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_privacy_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_privacy_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_privacy_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_privacy_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_privacy_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_browser_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_browser_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_browser_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_browser_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_browser_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_web_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_web_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_web_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_web_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_web_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_lsp_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_lsp_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_lsp_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_lsp_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_lsp_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_model_catalog_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_model_catalog_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_model_catalog_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_model_catalog_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_model_catalog_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_x_search_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_x_search_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_x_search_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_x_search_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_x_search_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_context_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_context_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_context_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_context_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_context_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_stt_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_stt_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_stt_config_save(form: Value) -> Result<Value, String> {
    let (config_path, _exists, mut config) = read_hermes_channel_yaml_config()?;
    merge_hermes_stt_config(&mut config, &form)?;
    let backup = write_hermes_yaml_config(&config_path, &config)?;
    Ok(crate::jv!({
        "ok": true,
        "configPath": config_path.to_string_lossy(),
        "backup": backup,
        "values": build_hermes_stt_config_values(&config),
    }))
}

#[tauri::command]
pub fn hermes_tts_voice_config_read() -> Result<Value, String> {
    let (config_path, exists, config) = read_hermes_channel_yaml_config()?;
    ensure_yaml_object(&mut config.clone())?;
    Ok(crate::jv!({
        "exists": exists,
        "configPath": config_path.to_string_lossy(),
        "values": build_hermes_tts_voice_config_values(&config),
    }))
}
