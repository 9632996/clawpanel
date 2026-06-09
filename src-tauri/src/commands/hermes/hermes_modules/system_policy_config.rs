
fn merge_hermes_io_safety_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_io_safety_config_values(config);
    let file_read_max_chars = validate_hermes_i64(
        if form.get("fileReadMaxChars").is_some() {
            form_i64(form, "fileReadMaxChars")
        } else {
            Some(current["fileReadMaxChars"].as_i64().unwrap_or(100000))
        },
        "file_read_max_chars",
        100000,
        1000,
        1000000,
    )?;
    let tool_output_max_bytes = validate_hermes_i64(
        if form.get("toolOutputMaxBytes").is_some() {
            form_i64(form, "toolOutputMaxBytes")
        } else {
            Some(current["toolOutputMaxBytes"].as_i64().unwrap_or(50000))
        },
        "tool_output.max_bytes",
        50000,
        1000,
        1000000,
    )?;
    let tool_output_max_lines = validate_hermes_i64(
        if form.get("toolOutputMaxLines").is_some() {
            form_i64(form, "toolOutputMaxLines")
        } else {
            Some(current["toolOutputMaxLines"].as_i64().unwrap_or(2000))
        },
        "tool_output.max_lines",
        2000,
        1,
        100000,
    )?;
    let tool_output_max_line_length = validate_hermes_i64(
        if form.get("toolOutputMaxLineLength").is_some() {
            form_i64(form, "toolOutputMaxLineLength")
        } else {
            Some(current["toolOutputMaxLineLength"].as_i64().unwrap_or(2000))
        },
        "tool_output.max_line_length",
        2000,
        1,
        100000,
    )?;

    let root = ensure_yaml_object(config)?;
    root.insert(yaml_key("file_read_max_chars"), serde_yaml::Value::Number(file_read_max_chars.into()));
    let tool_output = yaml_child_object(root, "tool_output")?;
    tool_output.insert(yaml_key("max_bytes"), serde_yaml::Value::Number(tool_output_max_bytes.into()));
    tool_output.insert(yaml_key("max_lines"), serde_yaml::Value::Number(tool_output_max_lines.into()));
    tool_output.insert(yaml_key("max_line_length"), serde_yaml::Value::Number(tool_output_max_line_length.into()));
    Ok(())
}

fn build_hermes_checkpoints_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let checkpoints = root.and_then(|map| yaml_get_mapping(map, "checkpoints"));
    let checkpoints_enabled = checkpoints.and_then(|map| yaml_bool_field(map, "enabled")).unwrap_or(false);
    let checkpoint_max_snapshots = checkpoints
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "max_snapshots"), 20, 1, 10000))
        .unwrap_or(20);
    let checkpoint_max_total_size_mb = checkpoints
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "max_total_size_mb"), 500, 0, 10485760))
        .unwrap_or(500);
    let checkpoint_max_file_size_mb = checkpoints
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "max_file_size_mb"), 10, 0, 1048576))
        .unwrap_or(10);
    let checkpoint_auto_prune = checkpoints.and_then(|map| yaml_bool_field(map, "auto_prune")).unwrap_or(true);
    let checkpoint_retention_days = checkpoints
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "retention_days"), 7, 1, 3650))
        .unwrap_or(7);
    let checkpoint_delete_orphans = checkpoints
        .and_then(|map| yaml_bool_field(map, "delete_orphans"))
        .unwrap_or(true);
    let checkpoint_min_interval_hours = checkpoints
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "min_interval_hours"), 24, 0, 8760))
        .unwrap_or(24);

    crate::jv!({
        "checkpointsEnabled": checkpoints_enabled,
        "checkpointMaxSnapshots": checkpoint_max_snapshots,
        "checkpointMaxTotalSizeMb": checkpoint_max_total_size_mb,
        "checkpointMaxFileSizeMb": checkpoint_max_file_size_mb,
        "checkpointAutoPrune": checkpoint_auto_prune,
        "checkpointRetentionDays": checkpoint_retention_days,
        "checkpointDeleteOrphans": checkpoint_delete_orphans,
        "checkpointMinIntervalHours": checkpoint_min_interval_hours,
    })
}

fn merge_hermes_checkpoints_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_checkpoints_config_values(config);
    let checkpoints_enabled =
        form_bool(form, "checkpointsEnabled").unwrap_or_else(|| current["checkpointsEnabled"].as_bool().unwrap_or(false));
    let checkpoint_max_snapshots = validate_hermes_i64(
        if form.get("checkpointMaxSnapshots").is_some() {
            form_i64(form, "checkpointMaxSnapshots")
        } else {
            Some(current["checkpointMaxSnapshots"].as_i64().unwrap_or(20))
        },
        "checkpoints.max_snapshots",
        20,
        1,
        10000,
    )?;
    let checkpoint_max_total_size_mb = validate_hermes_i64(
        if form.get("checkpointMaxTotalSizeMb").is_some() {
            form_i64(form, "checkpointMaxTotalSizeMb")
        } else {
            Some(current["checkpointMaxTotalSizeMb"].as_i64().unwrap_or(500))
        },
        "checkpoints.max_total_size_mb",
        500,
        0,
        10485760,
    )?;
    let checkpoint_max_file_size_mb = validate_hermes_i64(
        if form.get("checkpointMaxFileSizeMb").is_some() {
            form_i64(form, "checkpointMaxFileSizeMb")
        } else {
            Some(current["checkpointMaxFileSizeMb"].as_i64().unwrap_or(10))
        },
        "checkpoints.max_file_size_mb",
        10,
        0,
        1048576,
    )?;
    let checkpoint_auto_prune =
        form_bool(form, "checkpointAutoPrune").unwrap_or_else(|| current["checkpointAutoPrune"].as_bool().unwrap_or(true));
    let checkpoint_retention_days = validate_hermes_i64(
        if form.get("checkpointRetentionDays").is_some() {
            form_i64(form, "checkpointRetentionDays")
        } else {
            Some(current["checkpointRetentionDays"].as_i64().unwrap_or(7))
        },
        "checkpoints.retention_days",
        7,
        1,
        3650,
    )?;
    let checkpoint_delete_orphans = form_bool(form, "checkpointDeleteOrphans")
        .unwrap_or_else(|| current["checkpointDeleteOrphans"].as_bool().unwrap_or(true));
    let checkpoint_min_interval_hours = validate_hermes_i64(
        if form.get("checkpointMinIntervalHours").is_some() {
            form_i64(form, "checkpointMinIntervalHours")
        } else {
            Some(current["checkpointMinIntervalHours"].as_i64().unwrap_or(24))
        },
        "checkpoints.min_interval_hours",
        24,
        0,
        8760,
    )?;

    let root = ensure_yaml_object(config)?;
    let checkpoints = yaml_child_object(root, "checkpoints")?;
    checkpoints.insert(yaml_key("enabled"), serde_yaml::Value::Bool(checkpoints_enabled));
    checkpoints.insert(yaml_key("max_snapshots"), serde_yaml::Value::Number(checkpoint_max_snapshots.into()));
    checkpoints.insert(
        yaml_key("max_total_size_mb"),
        serde_yaml::Value::Number(checkpoint_max_total_size_mb.into()),
    );
    checkpoints.insert(
        yaml_key("max_file_size_mb"),
        serde_yaml::Value::Number(checkpoint_max_file_size_mb.into()),
    );
    checkpoints.insert(yaml_key("auto_prune"), serde_yaml::Value::Bool(checkpoint_auto_prune));
    checkpoints.insert(yaml_key("retention_days"), serde_yaml::Value::Number(checkpoint_retention_days.into()));
    checkpoints.insert(yaml_key("delete_orphans"), serde_yaml::Value::Bool(checkpoint_delete_orphans));
    checkpoints.insert(
        yaml_key("min_interval_hours"),
        serde_yaml::Value::Number(checkpoint_min_interval_hours.into()),
    );
    Ok(())
}

fn build_hermes_cron_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let cron = root.and_then(|map| yaml_get_mapping(map, "cron"));
    let cron_wrap_response = cron.and_then(|map| yaml_bool_field(map, "wrap_response")).unwrap_or(true);
    let cron_max_parallel_jobs = cron
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "max_parallel_jobs"), 0, 0, 10000))
        .unwrap_or(0);

    crate::jv!({
        "cronWrapResponse": cron_wrap_response,
        "cronMaxParallelJobs": cron_max_parallel_jobs,
    })
}

fn merge_hermes_cron_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_cron_config_values(config);
    let cron_wrap_response =
        form_bool(form, "cronWrapResponse").unwrap_or_else(|| current["cronWrapResponse"].as_bool().unwrap_or(true));
    let cron_max_parallel_jobs = validate_hermes_i64(
        if form.get("cronMaxParallelJobs").is_some() {
            form_i64(form, "cronMaxParallelJobs")
        } else {
            Some(current["cronMaxParallelJobs"].as_i64().unwrap_or(0))
        },
        "cron.max_parallel_jobs",
        0,
        0,
        10000,
    )?;

    let root = ensure_yaml_object(config)?;
    let cron = yaml_child_object(root, "cron")?;
    cron.insert(yaml_key("wrap_response"), serde_yaml::Value::Bool(cron_wrap_response));
    cron.insert(
        yaml_key("max_parallel_jobs"),
        if cron_max_parallel_jobs == 0 {
            serde_yaml::Value::Null
        } else {
            serde_yaml::Value::Number(cron_max_parallel_jobs.into())
        },
    );
    Ok(())
}

fn build_hermes_sessions_maintenance_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let sessions = root.and_then(|map| yaml_get_mapping(map, "sessions"));
    let sessions_auto_prune = sessions.and_then(|map| yaml_bool_field(map, "auto_prune")).unwrap_or(false);
    let sessions_retention_days = sessions
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "retention_days"), 90, 1, 36500))
        .unwrap_or(90);
    let sessions_vacuum_after_prune = sessions
        .and_then(|map| yaml_bool_field(map, "vacuum_after_prune"))
        .unwrap_or(true);
    let sessions_min_interval_hours = sessions
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "min_interval_hours"), 24, 0, 87600))
        .unwrap_or(24);
    let sessions_write_json_snapshots = sessions
        .and_then(|map| yaml_bool_field(map, "write_json_snapshots"))
        .unwrap_or(false);

    crate::jv!({
        "sessionsAutoPrune": sessions_auto_prune,
        "sessionsRetentionDays": sessions_retention_days,
        "sessionsVacuumAfterPrune": sessions_vacuum_after_prune,
        "sessionsMinIntervalHours": sessions_min_interval_hours,
        "sessionsWriteJsonSnapshots": sessions_write_json_snapshots,
    })
}

fn merge_hermes_sessions_maintenance_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_sessions_maintenance_config_values(config);
    let sessions_retention_days = validate_hermes_i64(
        if form.get("sessionsRetentionDays").is_some() {
            form_i64(form, "sessionsRetentionDays")
        } else {
            Some(current["sessionsRetentionDays"].as_i64().unwrap_or(90))
        },
        "sessions.retention_days",
        90,
        1,
        36500,
    )?;
    let sessions_min_interval_hours = validate_hermes_i64(
        if form.get("sessionsMinIntervalHours").is_some() {
            form_i64(form, "sessionsMinIntervalHours")
        } else {
            Some(current["sessionsMinIntervalHours"].as_i64().unwrap_or(24))
        },
        "sessions.min_interval_hours",
        24,
        0,
        87600,
    )?;

    let root = ensure_yaml_object(config)?;
    let sessions = yaml_child_object(root, "sessions")?;
    sessions.insert(
        yaml_key("auto_prune"),
        serde_yaml::Value::Bool(
            form_bool(form, "sessionsAutoPrune").unwrap_or_else(|| current["sessionsAutoPrune"].as_bool().unwrap_or(false)),
        ),
    );
    sessions.insert(yaml_key("retention_days"), serde_yaml::Value::Number(sessions_retention_days.into()));
    sessions.insert(
        yaml_key("vacuum_after_prune"),
        serde_yaml::Value::Bool(
            form_bool(form, "sessionsVacuumAfterPrune")
                .unwrap_or_else(|| current["sessionsVacuumAfterPrune"].as_bool().unwrap_or(true)),
        ),
    );
    sessions.insert(
        yaml_key("min_interval_hours"),
        serde_yaml::Value::Number(sessions_min_interval_hours.into()),
    );
    sessions.insert(
        yaml_key("write_json_snapshots"),
        serde_yaml::Value::Bool(
            form_bool(form, "sessionsWriteJsonSnapshots")
                .unwrap_or_else(|| current["sessionsWriteJsonSnapshots"].as_bool().unwrap_or(false)),
        ),
    );
    Ok(())
}

fn build_hermes_updates_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let updates = root.and_then(|map| yaml_get_mapping(map, "updates"));
    let updates_pre_update_backup = updates
        .and_then(|map| yaml_bool_field(map, "pre_update_backup"))
        .unwrap_or(false);
    let updates_backup_keep = updates
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "backup_keep"), 5, 1, 1000))
        .unwrap_or(5);

    crate::jv!({
        "updatesPreUpdateBackup": updates_pre_update_backup,
        "updatesBackupKeep": updates_backup_keep,
    })
}

fn merge_hermes_updates_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_updates_config_values(config);
    let updates_pre_update_backup =
        form_bool(form, "updatesPreUpdateBackup").unwrap_or_else(|| current["updatesPreUpdateBackup"].as_bool().unwrap_or(false));
    let updates_backup_keep = validate_hermes_i64(
        if form.get("updatesBackupKeep").is_some() {
            form_i64(form, "updatesBackupKeep")
        } else {
            Some(current["updatesBackupKeep"].as_i64().unwrap_or(5))
        },
        "updates.backup_keep",
        5,
        1,
        1000,
    )?;

    let root = ensure_yaml_object(config)?;
    let updates = yaml_child_object(root, "updates")?;
    updates.insert(yaml_key("pre_update_backup"), serde_yaml::Value::Bool(updates_pre_update_backup));
    updates.insert(yaml_key("backup_keep"), serde_yaml::Value::Number(updates_backup_keep.into()));
    Ok(())
}

fn build_hermes_logging_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let logging = root.and_then(|map| yaml_get_mapping(map, "logging"));
    let memory_monitor = logging.and_then(|map| yaml_get_mapping(map, "memory_monitor"));
    let logging_level = normalize_hermes_logging_level(logging.and_then(|map| yaml_string_field(map, "level")), false)
        .unwrap_or_else(|_| "INFO".to_string());
    let logging_max_size_mb = logging
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "max_size_mb"), 5, 1, 102400))
        .unwrap_or(5);
    let logging_backup_count = logging
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "backup_count"), 3, 0, 1000))
        .unwrap_or(3);
    let logging_memory_monitor_enabled = memory_monitor.and_then(|map| yaml_bool_field(map, "enabled")).unwrap_or(true);
    let logging_memory_monitor_interval_seconds = memory_monitor
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "interval_seconds"), 300, 1, 86400))
        .unwrap_or(300);

    crate::jv!({
        "loggingLevel": logging_level,
        "loggingMaxSizeMb": logging_max_size_mb,
        "loggingBackupCount": logging_backup_count,
        "loggingMemoryMonitorEnabled": logging_memory_monitor_enabled,
        "loggingMemoryMonitorIntervalSeconds": logging_memory_monitor_interval_seconds,
    })
}

fn merge_hermes_logging_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_logging_config_values(config);
    let logging_level = normalize_hermes_logging_level(
        if form.get("loggingLevel").is_some() {
            form_string(form, "loggingLevel")
        } else {
            current["loggingLevel"].as_str().map(ToString::to_string)
        },
        true,
    )?;
    let logging_max_size_mb = validate_hermes_i64(
        if form.get("loggingMaxSizeMb").is_some() {
            form_i64(form, "loggingMaxSizeMb")
        } else {
            Some(current["loggingMaxSizeMb"].as_i64().unwrap_or(5))
        },
        "logging.max_size_mb",
        5,
        1,
        102400,
    )?;
    let logging_backup_count = validate_hermes_i64(
        if form.get("loggingBackupCount").is_some() {
            form_i64(form, "loggingBackupCount")
        } else {
            Some(current["loggingBackupCount"].as_i64().unwrap_or(3))
        },
        "logging.backup_count",
        3,
        0,
        1000,
    )?;
    let logging_memory_monitor_enabled = form_bool(form, "loggingMemoryMonitorEnabled")
        .unwrap_or_else(|| current["loggingMemoryMonitorEnabled"].as_bool().unwrap_or(true));
    let logging_memory_monitor_interval_seconds = validate_hermes_i64(
        if form.get("loggingMemoryMonitorIntervalSeconds").is_some() {
            form_i64(form, "loggingMemoryMonitorIntervalSeconds")
        } else {
            Some(current["loggingMemoryMonitorIntervalSeconds"].as_i64().unwrap_or(300))
        },
        "logging.memory_monitor.interval_seconds",
        300,
        1,
        86400,
    )?;

    let root = ensure_yaml_object(config)?;
    let logging = yaml_child_object(root, "logging")?;
    logging.insert(yaml_key("level"), serde_yaml::Value::String(logging_level));
    logging.insert(yaml_key("max_size_mb"), serde_yaml::Value::Number(logging_max_size_mb.into()));
    logging.insert(yaml_key("backup_count"), serde_yaml::Value::Number(logging_backup_count.into()));
    let memory_monitor = yaml_child_object(logging, "memory_monitor")?;
    memory_monitor.insert(yaml_key("enabled"), serde_yaml::Value::Bool(logging_memory_monitor_enabled));
    memory_monitor.insert(
        yaml_key("interval_seconds"),
        serde_yaml::Value::Number(logging_memory_monitor_interval_seconds.into()),
    );
    Ok(())
}

include!("system_policy_config/approval_browser_config.rs");