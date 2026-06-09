
fn merge_hermes_kanban_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_kanban_config_values(config);
    let dispatch_in_gateway = form_bool(form, "dispatchInGateway")
        .or_else(|| current["dispatchInGateway"].as_bool())
        .unwrap_or(true);
    let dispatch_interval_seconds = validate_hermes_i64(
        form_i64(form, "dispatchIntervalSeconds").or_else(|| current["dispatchIntervalSeconds"].as_i64()),
        "kanban.dispatch_interval_seconds",
        60,
        1,
        86400,
    )?;
    let max_spawn = validate_hermes_i64(
        form_i64(form, "maxSpawn").or_else(|| current["maxSpawn"].as_i64()),
        "kanban.max_spawn",
        0,
        0,
        1000,
    )?;
    let max_in_progress = validate_hermes_i64(
        form_i64(form, "maxInProgress").or_else(|| current["maxInProgress"].as_i64()),
        "kanban.max_in_progress",
        0,
        0,
        1000,
    )?;
    let failure_limit = validate_hermes_i64(
        form_i64(form, "failureLimit").or_else(|| current["failureLimit"].as_i64()),
        "kanban.failure_limit",
        2,
        1,
        100,
    )?;
    let auto_decompose = form_bool(form, "autoDecompose")
        .or_else(|| current["autoDecompose"].as_bool())
        .unwrap_or(true);
    let auto_decompose_per_tick = validate_hermes_i64(
        form_i64(form, "autoDecomposePerTick").or_else(|| current["autoDecomposePerTick"].as_i64()),
        "kanban.auto_decompose_per_tick",
        3,
        1,
        1000,
    )?;
    let worker_log_rotate_bytes = validate_hermes_i64(
        form_i64(form, "workerLogRotateBytes").or_else(|| current["workerLogRotateBytes"].as_i64()),
        "kanban.worker_log_rotate_bytes",
        2097152,
        1,
        1073741824,
    )?;
    let worker_log_backup_count = validate_hermes_i64(
        form_i64(form, "workerLogBackupCount").or_else(|| current["workerLogBackupCount"].as_i64()),
        "kanban.worker_log_backup_count",
        1,
        0,
        100,
    )?;
    let orchestrator_profile = if form.get("orchestratorProfile").is_some() {
        form_string(form, "orchestratorProfile")
            .ok_or_else(|| "kanban.orchestrator_profile must be a string".to_string())?
            .trim()
            .to_string()
    } else {
        current["orchestratorProfile"].as_str().unwrap_or_default().trim().to_string()
    };
    let default_assignee = if form.get("defaultAssignee").is_some() {
        form_string(form, "defaultAssignee")
            .ok_or_else(|| "kanban.default_assignee must be a string".to_string())?
            .trim()
            .to_string()
    } else {
        current["defaultAssignee"].as_str().unwrap_or_default().trim().to_string()
    };
    let stale_timeout = validate_hermes_i64(
        form_i64(form, "dispatchStaleTimeoutSeconds").or_else(|| current["dispatchStaleTimeoutSeconds"].as_i64()),
        "kanban.dispatch_stale_timeout_seconds",
        14400,
        0,
        604800,
    )?;

    let kanban = yaml_child_object(ensure_yaml_object(config)?, "kanban")?;
    kanban.insert(yaml_key("dispatch_in_gateway"), serde_yaml::Value::Bool(dispatch_in_gateway));
    kanban.insert(
        yaml_key("dispatch_interval_seconds"),
        serde_yaml::Value::Number(serde_yaml::Number::from(dispatch_interval_seconds)),
    );
    if max_spawn > 0 {
        kanban.insert(yaml_key("max_spawn"), serde_yaml::Value::Number(serde_yaml::Number::from(max_spawn)));
    } else {
        kanban.remove(yaml_key("max_spawn"));
    }
    if max_in_progress > 0 {
        kanban.insert(
            yaml_key("max_in_progress"),
            serde_yaml::Value::Number(serde_yaml::Number::from(max_in_progress)),
        );
    } else {
        kanban.remove(yaml_key("max_in_progress"));
    }
    kanban.insert(
        yaml_key("failure_limit"),
        serde_yaml::Value::Number(serde_yaml::Number::from(failure_limit)),
    );
    kanban.insert(yaml_key("auto_decompose"), serde_yaml::Value::Bool(auto_decompose));
    kanban.insert(
        yaml_key("auto_decompose_per_tick"),
        serde_yaml::Value::Number(serde_yaml::Number::from(auto_decompose_per_tick)),
    );
    kanban.insert(
        yaml_key("worker_log_rotate_bytes"),
        serde_yaml::Value::Number(serde_yaml::Number::from(worker_log_rotate_bytes)),
    );
    kanban.insert(
        yaml_key("worker_log_backup_count"),
        serde_yaml::Value::Number(serde_yaml::Number::from(worker_log_backup_count)),
    );
    set_optional_yaml_string(kanban, "orchestrator_profile", orchestrator_profile);
    set_optional_yaml_string(kanban, "default_assignee", default_assignee);
    kanban.insert(
        yaml_key("dispatch_stale_timeout_seconds"),
        serde_yaml::Value::Number(serde_yaml::Number::from(stale_timeout)),
    );
    Ok(())
}

fn merge_hermes_human_delay_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_human_delay_config_values(config);
    let mode = normalize_hermes_human_delay_mode(
        form_string(form, "humanDelayMode").or_else(|| current["humanDelayMode"].as_str().map(ToString::to_string)),
        true,
    )?;
    let min_ms = validate_hermes_i64(
        if form.get("humanDelayMinMs").is_some() {
            form_i64(form, "humanDelayMinMs")
        } else {
            Some(current["humanDelayMinMs"].as_i64().unwrap_or(800))
        },
        "human_delay.min_ms",
        800,
        0,
        60000,
    )?;
    let max_ms = validate_hermes_i64(
        if form.get("humanDelayMaxMs").is_some() {
            form_i64(form, "humanDelayMaxMs")
        } else {
            Some(current["humanDelayMaxMs"].as_i64().unwrap_or(2500))
        },
        "human_delay.max_ms",
        2500,
        0,
        60000,
    )?;
    if max_ms < min_ms {
        return Err("human_delay.max_ms 不能小于 min_ms".to_string());
    }

    let root = ensure_yaml_object(config)?;
    let human_delay = yaml_child_object(root, "human_delay")?;
    human_delay.insert(yaml_key("mode"), serde_yaml::Value::String(mode));
    human_delay.insert(yaml_key("min_ms"), serde_yaml::Value::Number(min_ms.into()));
    human_delay.insert(yaml_key("max_ms"), serde_yaml::Value::Number(max_ms.into()));
    Ok(())
}

fn normalize_hermes_streaming_transport(value: Option<String>, strict: bool) -> Result<String, String> {
    let transport = value.unwrap_or_default().trim().to_ascii_lowercase();
    let transport = if transport.is_empty() { "edit".to_string() } else { transport };
    if matches!(transport.as_str(), "auto" | "draft" | "edit" | "off") {
        return Ok(transport);
    }
    if strict {
        Err("streaming.transport 必须是 auto、draft、edit 或 off".to_string())
    } else {
        Ok("edit".to_string())
    }
}

fn normalize_hermes_code_execution_mode(value: Option<String>, strict: bool) -> Result<String, String> {
    let mode = value.unwrap_or_default().trim().to_ascii_lowercase();
    let mode = if mode.is_empty() { "project".to_string() } else { mode };
    if matches!(mode.as_str(), "project" | "strict") {
        return Ok(mode);
    }
    if strict {
        Err("code_execution.mode 必须是 project 或 strict".to_string())
    } else {
        Ok("project".to_string())
    }
}

fn normalize_hermes_terminal_backend(value: Option<String>, strict: bool) -> Result<String, String> {
    let backend = value.unwrap_or_default().trim().to_ascii_lowercase();
    let backend = if backend.is_empty() { "local".to_string() } else { backend };
    if matches!(
        backend.as_str(),
        "local" | "ssh" | "docker" | "singularity" | "modal" | "daytona" | "vercel_sandbox"
    ) {
        return Ok(backend);
    }
    if strict {
        Err("terminal.backend 必须是 local、ssh、docker、singularity、modal、daytona 或 vercel_sandbox".to_string())
    } else {
        Ok("local".to_string())
    }
}

fn normalize_hermes_terminal_modal_mode(value: Option<String>, strict: bool) -> Result<String, String> {
    let mode = value.unwrap_or_default().trim().to_ascii_lowercase();
    let mode = if mode.is_empty() { "auto".to_string() } else { mode };
    if matches!(mode.as_str(), "auto" | "managed" | "direct") {
        return Ok(mode);
    }
    if strict {
        Err("terminal.modal_mode 必须是 auto、managed 或 direct".to_string())
    } else {
        Ok("auto".to_string())
    }
}

fn normalize_hermes_terminal_vercel_runtime(value: Option<String>, strict: bool) -> Result<String, String> {
    let runtime = value.unwrap_or_default().trim().to_ascii_lowercase();
    let runtime = if runtime.is_empty() { "node24".to_string() } else { runtime };
    if matches!(runtime.as_str(), "node24" | "node22" | "python3.13") {
        return Ok(runtime);
    }
    if strict {
        Err("terminal.vercel_runtime 必须是 node24、node22 或 python3.13".to_string())
    } else {
        Ok("node24".to_string())
    }
}

fn normalize_hermes_browser_engine(value: Option<String>, strict: bool) -> Result<String, String> {
    let engine = value.unwrap_or_default().trim().to_ascii_lowercase();
    let engine = if engine.is_empty() { "auto".to_string() } else { engine };
    if matches!(engine.as_str(), "auto" | "lightpanda" | "chrome") {
        return Ok(engine);
    }
    if strict {
        Err("browser.engine 必须是 auto、lightpanda 或 chrome".to_string())
    } else {
        Ok("auto".to_string())
    }
}

fn normalize_hermes_browser_dialog_policy(value: Option<String>, strict: bool) -> Result<String, String> {
    let policy = value.unwrap_or_default().trim().to_ascii_lowercase();
    let policy = if policy.is_empty() {
        "must_respond".to_string()
    } else {
        policy
    };
    if matches!(policy.as_str(), "must_respond" | "auto_dismiss" | "auto_accept") {
        return Ok(policy);
    }
    if strict {
        Err("browser.dialog_policy 必须是 must_respond、auto_dismiss 或 auto_accept".to_string())
    } else {
        Ok("must_respond".to_string())
    }
}

fn normalize_hermes_web_backend(value: Option<String>, key: &str, strict: bool) -> Result<String, String> {
    let backend = value.unwrap_or_default().trim().to_ascii_lowercase();
    if backend.is_empty() {
        return Ok(String::new());
    }
    if matches!(
        backend.as_str(),
        "tavily" | "firecrawl" | "parallel" | "exa" | "searxng" | "brave" | "brave_free" | "ddgs" | "xai" | "native"
    ) {
        return Ok(backend);
    }
    if strict {
        Err(format!(
            "{key} 必须为空或 tavily、firecrawl、parallel、exa、searxng、brave、brave_free、ddgs、xai、native"
        ))
    } else {
        Ok(String::new())
    }
}

fn normalize_hermes_lsp_wait_mode(value: Option<String>, strict: bool) -> Result<String, String> {
    let mode = value.unwrap_or_default().trim().to_ascii_lowercase();
    let mode = if mode.is_empty() { "document".to_string() } else { mode };
    if matches!(mode.as_str(), "document" | "full") {
        return Ok(mode);
    }
    if strict {
        Err("lsp.wait_mode 必须是 document 或 full".to_string())
    } else {
        Ok("document".to_string())
    }
}

fn normalize_hermes_lsp_install_strategy(value: Option<String>, strict: bool) -> Result<String, String> {
    let strategy = value.unwrap_or_default().trim().to_ascii_lowercase();
    let strategy = if strategy.is_empty() { "auto".to_string() } else { strategy };
    if matches!(strategy.as_str(), "auto" | "manual" | "off") {
        return Ok(strategy);
    }
    if strict {
        Err("lsp.install_strategy 必须是 auto、manual 或 off".to_string())
    } else {
        Ok("auto".to_string())
    }
}

fn normalize_hermes_stt_provider(value: Option<String>, strict: bool) -> Result<String, String> {
    let provider = value.unwrap_or_default().trim().to_ascii_lowercase();
    let provider = if provider.is_empty() { "auto".to_string() } else { provider };
    if matches!(provider.as_str(), "auto" | "local" | "groq" | "openai" | "mistral") {
        return Ok(provider);
    }
    if strict {
        Err("stt.provider 必须是 auto、local、groq、openai 或 mistral".to_string())
    } else {
        Ok("auto".to_string())
    }
}

fn normalize_hermes_stt_local_model(value: Option<String>, strict: bool) -> Result<String, String> {
    let model = value.unwrap_or_default().trim().to_ascii_lowercase();
    let model = if model.is_empty() { "base".to_string() } else { model };
    if matches!(model.as_str(), "tiny" | "base" | "small" | "medium" | "large-v3" | "turbo") {
        return Ok(model);
    }
    if strict {
        Err("stt.local.model 必须是 tiny、base、small、medium、large-v3 或 turbo".to_string())
    } else {
        Ok("base".to_string())
    }
}

fn normalize_hermes_stt_openai_model(value: Option<String>, strict: bool) -> Result<String, String> {
    let model = value.unwrap_or_default().trim().to_string();
    let model = if model.is_empty() { "whisper-1".to_string() } else { model };
    if matches!(model.as_str(), "whisper-1" | "gpt-4o-mini-transcribe" | "gpt-4o-transcribe") {
        return Ok(model);
    }
    if strict {
        Err("stt.openai.model 必须是 whisper-1、gpt-4o-mini-transcribe 或 gpt-4o-transcribe".to_string())
    } else {
        Ok("whisper-1".to_string())
    }
}

fn normalize_hermes_stt_mistral_model(value: Option<String>, strict: bool) -> Result<String, String> {
    let model = value.unwrap_or_default().trim().to_string();
    let model = if model.is_empty() {
        "voxtral-mini-latest".to_string()
    } else {
        model
    };
    if matches!(model.as_str(), "voxtral-mini-latest" | "voxtral-mini-2602") {
        return Ok(model);
    }
    if strict {
        Err("stt.mistral.model 必须是 voxtral-mini-latest 或 voxtral-mini-2602".to_string())
    } else {
        Ok("voxtral-mini-latest".to_string())
    }
}

fn normalize_hermes_stt_language(value: Option<String>, strict: bool) -> Result<String, String> {
    let language = value.unwrap_or_default().trim().to_string();
    if language.is_empty() {
        return Ok(String::new());
    }
    let mut parts = language.split('-');
    let Some(first) = parts.next() else {
        return Ok(String::new());
    };
    let first_valid = (2..=3).contains(&first.len()) && first.chars().all(|ch| ch.is_ascii_lowercase());
    let rest_valid = parts.all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_alphanumeric()));
    if first_valid && rest_valid {
        return Ok(language);
    }
    if strict {
        Err("stt.local.language 必须为空或合法语言标签，例如 zh、en、pt-BR".to_string())
    } else {
        Ok(String::new())
    }
}

fn normalize_hermes_tts_provider(value: Option<String>, strict: bool) -> Result<String, String> {
    let provider = value.unwrap_or_default().trim().to_ascii_lowercase();
    let provider = if provider.is_empty() { "edge".to_string() } else { provider };
    if matches!(
        provider.as_str(),
        "edge" | "elevenlabs" | "openai" | "xai" | "minimax" | "mistral" | "gemini" | "neutts" | "kittentts" | "piper"
    ) {
        return Ok(provider);
    }
    if strict {
        Err("tts.provider 必须是 edge、elevenlabs、openai、xai、minimax、mistral、gemini、neutts、kittentts 或 piper".to_string())
    } else {
        Ok("edge".to_string())
    }
}

fn normalize_hermes_tts_openai_voice(value: Option<String>, strict: bool) -> Result<String, String> {
    let voice = value.unwrap_or_default().trim().to_ascii_lowercase();
    let voice = if voice.is_empty() { "alloy".to_string() } else { voice };
    if matches!(voice.as_str(), "alloy" | "echo" | "fable" | "onyx" | "nova" | "shimmer") {
        return Ok(voice);
    }
    if strict {
        Err("tts.openai.voice 必须是 alloy、echo、fable、onyx、nova 或 shimmer".to_string())
    } else {
        Ok("alloy".to_string())
    }
}

fn normalize_hermes_voice_language(value: Option<String>, strict: bool, key: &str) -> Result<String, String> {
    let language = value.unwrap_or_default().trim().to_string();
    if language.is_empty() {
        return Ok("en".to_string());
    }
    let mut parts = language.split('-');
    let Some(first) = parts.next() else {
        return Ok("en".to_string());
    };
    let first_valid = (2..=3).contains(&first.len()) && first.chars().all(|ch| ch.is_ascii_lowercase());
    let rest_valid = parts.all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_alphanumeric()));
    if first_valid && rest_valid {
        return Ok(language);
    }
    if strict {
        Err(format!("{key} 必须是合法语言标签，例如 en、zh、pt-BR"))
    } else {
        Ok("en".to_string())
    }
}

fn normalize_hermes_approval_mode(value: Option<String>, strict: bool) -> Result<String, String> {
    let mode = value.unwrap_or_default().trim().to_ascii_lowercase();
    let mode = if mode.is_empty() { "manual".to_string() } else { mode };
    if matches!(mode.as_str(), "manual" | "smart" | "off") {
        return Ok(mode);
    }
    if strict {
        Err("approvals.mode 必须是 manual、smart 或 off".to_string())
    } else {
        Ok("manual".to_string())
    }
}

fn normalize_hermes_approval_cron_mode(value: Option<String>, strict: bool) -> Result<String, String> {
    let mode = value.unwrap_or_default().trim().to_ascii_lowercase();
    let mode = if mode.is_empty() { "deny".to_string() } else { mode };
    if matches!(mode.as_str(), "deny" | "approve") {
        return Ok(mode);
    }
    if strict {
        Err("approvals.cron_mode 必须是 deny 或 approve".to_string())
    } else {
        Ok("deny".to_string())
    }
}

fn normalize_hermes_logging_level(value: Option<String>, strict: bool) -> Result<String, String> {
    let level = value.unwrap_or_default().trim().to_ascii_uppercase();
    let level = if level.is_empty() { "INFO".to_string() } else { level };
    if matches!(level.as_str(), "DEBUG" | "INFO" | "WARNING") {
        return Ok(level);
    }
    if strict {
        Err("logging.level 必须是 DEBUG、INFO 或 WARNING".to_string())
    } else {
        Ok("INFO".to_string())
    }
}

fn hermes_streaming_config_source(config: &serde_yaml::Value) -> Option<&serde_yaml::Mapping> {
    let root = config.as_mapping()?;
    if let Some(streaming) = yaml_get_mapping(root, "streaming") {
        return Some(streaming);
    }
    let gateway = yaml_get_mapping(root, "gateway")?;
    yaml_get_mapping(gateway, "streaming")
}

fn build_hermes_streaming_config_values(config: &serde_yaml::Value) -> Value {
    let streaming = hermes_streaming_config_source(config);
    let enabled = streaming.and_then(|map| yaml_bool_field(map, "enabled")).unwrap_or(false);
    let transport = normalize_hermes_streaming_transport(streaming.and_then(|map| yaml_string_field(map, "transport")), false)
        .unwrap_or_else(|_| "edit".to_string());
    let edit_interval = streaming
        .map(|map| bounded_hermes_f64(yaml_f64_field(map, "edit_interval"), 0.8, 0.05, 60.0))
        .unwrap_or(0.8);
    let buffer_threshold = streaming
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "buffer_threshold"), 24, 1, 5000))
        .unwrap_or(24);
    let cursor = streaming
        .and_then(|map| yaml_string_field(map, "cursor"))
        .unwrap_or_else(|| " ▉".to_string());
    let fresh_final_after_seconds = streaming
        .map(|map| bounded_hermes_f64(yaml_f64_field(map, "fresh_final_after_seconds"), 60.0, 0.0, 86400.0))
        .unwrap_or(60.0);

    crate::jv!({
        "enabled": enabled,
        "transport": transport,
        "editInterval": edit_interval,
        "bufferThreshold": buffer_threshold,
        "cursor": cursor,
        "freshFinalAfterSeconds": fresh_final_after_seconds,
    })
}

fn merge_hermes_streaming_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_streaming_config_values(config);
    let enabled = form_bool(form, "enabled").unwrap_or_else(|| current["enabled"].as_bool().unwrap_or(false));
    let transport = normalize_hermes_streaming_transport(
        if form.get("transport").is_some() {
            form_string(form, "transport")
        } else {
            current["transport"].as_str().map(ToString::to_string)
        },
        true,
    )?;
    let edit_interval = validate_hermes_f64(
        if form.get("editInterval").is_some() {
            form_f64(form, "editInterval")
        } else {
            Some(current["editInterval"].as_f64().unwrap_or(0.8))
        },
        "streaming.edit_interval",
        0.8,
        0.05,
        60.0,
    )?;
    let buffer_threshold = validate_hermes_i64(
        if form.get("bufferThreshold").is_some() {
            form_i64(form, "bufferThreshold")
        } else {
            Some(current["bufferThreshold"].as_i64().unwrap_or(24))
        },
        "streaming.buffer_threshold",
        24,
        1,
        5000,
    )?;
    let cursor = if form.get("cursor").is_some() {
        form_string(form, "cursor").unwrap_or_default()
    } else {
        current["cursor"].as_str().unwrap_or(" ▉").to_string()
    };
    let fresh_final_after_seconds = validate_hermes_f64(
        if form.get("freshFinalAfterSeconds").is_some() {
            form_f64(form, "freshFinalAfterSeconds")
        } else {
            Some(current["freshFinalAfterSeconds"].as_f64().unwrap_or(60.0))
        },
        "streaming.fresh_final_after_seconds",
        60.0,
        0.0,
        86400.0,
    )?;

    let root = ensure_yaml_object(config)?;
    let streaming = yaml_child_object(root, "streaming")?;
    streaming.insert(yaml_key("enabled"), serde_yaml::Value::Bool(enabled));
    streaming.insert(yaml_key("transport"), serde_yaml::Value::String(transport));
    streaming.insert(yaml_key("edit_interval"), serde_yaml::Value::Number(edit_interval.into()));
    streaming.insert(yaml_key("buffer_threshold"), serde_yaml::Value::Number(buffer_threshold.into()));
    streaming.insert(yaml_key("cursor"), serde_yaml::Value::String(cursor));
    streaming.insert(
        yaml_key("fresh_final_after_seconds"),
        serde_yaml::Value::Number(fresh_final_after_seconds.into()),
    );
    Ok(())
}

fn build_hermes_execution_limits_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let code_execution = root.and_then(|map| yaml_get_mapping(map, "code_execution"));
    let delegation = root.and_then(|map| yaml_get_mapping(map, "delegation"));
    let code_execution_mode =
        normalize_hermes_code_execution_mode(code_execution.and_then(|map| yaml_string_field(map, "mode")), false)
            .unwrap_or_else(|_| "project".to_string());
    let code_execution_timeout = code_execution
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "timeout"), 300, 1, 86400))
        .unwrap_or(300);
    let code_execution_max_tool_calls = code_execution
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "max_tool_calls"), 50, 1, 10000))
        .unwrap_or(50);
    let delegation_max_iterations = delegation
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "max_iterations"), 50, 1, 1000))
        .unwrap_or(50);
    let delegation_child_timeout_seconds = delegation
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "child_timeout_seconds"), 600, 30, 86400))
        .unwrap_or(600);
    let delegation_max_concurrent_children = delegation
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "max_concurrent_children"), 3, 1, 100))
        .unwrap_or(3);
    let delegation_max_spawn_depth = delegation
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "max_spawn_depth"), 1, 1, 3))
        .unwrap_or(1);
    let delegation_orchestrator_enabled = delegation
        .and_then(|map| yaml_bool_field(map, "orchestrator_enabled"))
        .unwrap_or(true);
    let delegation_subagent_auto_approve = delegation
        .and_then(|map| yaml_bool_field(map, "subagent_auto_approve"))
        .unwrap_or(false);
    let delegation_inherit_mcp_toolsets = delegation
        .and_then(|map| yaml_bool_field(map, "inherit_mcp_toolsets"))
        .unwrap_or(true);
    let delegation_model = delegation
        .and_then(|map| yaml_string_field(map, "model"))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_default();
    let delegation_provider = delegation
        .and_then(|map| yaml_string_field(map, "provider"))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_default();

    crate::jv!({
        "codeExecutionMode": code_execution_mode,
        "codeExecutionTimeout": code_execution_timeout,
        "codeExecutionMaxToolCalls": code_execution_max_tool_calls,
        "delegationMaxIterations": delegation_max_iterations,
        "delegationChildTimeoutSeconds": delegation_child_timeout_seconds,
        "delegationMaxConcurrentChildren": delegation_max_concurrent_children,
        "delegationMaxSpawnDepth": delegation_max_spawn_depth,
        "delegationOrchestratorEnabled": delegation_orchestrator_enabled,
        "delegationSubagentAutoApprove": delegation_subagent_auto_approve,
        "delegationInheritMcpToolsets": delegation_inherit_mcp_toolsets,
        "delegationModel": delegation_model,
        "delegationProvider": delegation_provider,
    })
}

fn build_hermes_io_safety_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let tool_output = root.and_then(|map| yaml_get_mapping(map, "tool_output"));
    let file_read_max_chars = root
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "file_read_max_chars"), 100000, 1000, 1000000))
        .unwrap_or(100000);
    let tool_output_max_bytes = tool_output
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "max_bytes"), 50000, 1000, 1000000))
        .unwrap_or(50000);
    let tool_output_max_lines = tool_output
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "max_lines"), 2000, 1, 100000))
        .unwrap_or(2000);
    let tool_output_max_line_length = tool_output
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "max_line_length"), 2000, 1, 100000))
        .unwrap_or(2000);

    crate::jv!({
        "fileReadMaxChars": file_read_max_chars,
        "toolOutputMaxBytes": tool_output_max_bytes,
        "toolOutputMaxLines": tool_output_max_lines,
        "toolOutputMaxLineLength": tool_output_max_line_length,
    })
}
