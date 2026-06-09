fn build_hermes_tts_voice_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let tts = root.and_then(|map| yaml_get_mapping(map, "tts"));
    let edge = tts.and_then(|map| yaml_get_mapping(map, "edge"));
    let openai = tts.and_then(|map| yaml_get_mapping(map, "openai"));
    let elevenlabs = tts.and_then(|map| yaml_get_mapping(map, "elevenlabs"));
    let xai = tts.and_then(|map| yaml_get_mapping(map, "xai"));
    let mistral = tts.and_then(|map| yaml_get_mapping(map, "mistral"));
    let piper = tts.and_then(|map| yaml_get_mapping(map, "piper"));
    let voice = root.and_then(|map| yaml_get_mapping(map, "voice"));
    let tts_string = |section: Option<&serde_yaml::Mapping>, key: &str, fallback: &str| {
        section
            .and_then(|map| yaml_string_field(map, key))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| fallback.to_string())
    };
    crate::jv!({
        "ttsProvider": normalize_hermes_tts_provider(tts.and_then(|map| yaml_string_field(map, "provider")), false).unwrap_or_else(|_| "edge".to_string()),
        "ttsEdgeVoice": tts_string(edge, "voice", "en-US-AriaNeural"),
        "ttsOpenaiModel": tts_string(openai, "model", "gpt-4o-mini-tts"),
        "ttsOpenaiVoice": normalize_hermes_tts_openai_voice(openai.and_then(|map| yaml_string_field(map, "voice")), false).unwrap_or_else(|_| "alloy".to_string()),
        "ttsElevenlabsVoiceId": tts_string(elevenlabs, "voice_id", "pNInz6obpgDQGcFmaJgB"),
        "ttsElevenlabsModelId": tts_string(elevenlabs, "model_id", "eleven_multilingual_v2"),
        "ttsXaiVoiceId": tts_string(xai, "voice_id", "eve"),
        "ttsXaiLanguage": normalize_hermes_voice_language(xai.and_then(|map| yaml_string_field(map, "language")), false, "tts.xai.language").unwrap_or_else(|_| "en".to_string()),
        "ttsXaiSampleRate": xai.map(|map| bounded_hermes_i64(yaml_i64_field(map, "sample_rate"), 24000, 8000, 192000)).unwrap_or(24000),
        "ttsXaiBitRate": xai.map(|map| bounded_hermes_i64(yaml_i64_field(map, "bit_rate"), 128000, 16000, 512000)).unwrap_or(128000),
        "ttsMistralModel": tts_string(mistral, "model", "voxtral-mini-tts-2603"),
        "ttsMistralVoiceId": tts_string(mistral, "voice_id", "c69964a6-ab8b-4f8a-9465-ec0925096ec8"),
        "ttsPiperVoice": tts_string(piper, "voice", "en_US-lessac-medium"),
        "voiceRecordKey": voice.and_then(|map| yaml_string_field(map, "record_key")).map(|value| value.trim().to_string()).unwrap_or_else(|| "ctrl+b".to_string()),
        "voiceMaxRecordingSeconds": voice.map(|map| bounded_hermes_i64(yaml_i64_field(map, "max_recording_seconds"), 120, 1, 3600)).unwrap_or(120),
        "voiceAutoTts": voice.and_then(|map| yaml_bool_field(map, "auto_tts")).unwrap_or(false),
        "voiceBeepEnabled": voice.and_then(|map| yaml_bool_field(map, "beep_enabled")).unwrap_or(true),
        "voiceSilenceThreshold": voice.map(|map| bounded_hermes_i64(yaml_i64_field(map, "silence_threshold"), 200, 0, 32767)).unwrap_or(200),
        "voiceSilenceDuration": voice.map(|map| bounded_hermes_f64(yaml_f64_field(map, "silence_duration"), 3.0, 0.1, 60.0)).unwrap_or(3.0),
    })
}

fn merge_hermes_tts_voice_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_tts_voice_config_values(config);
    let form_or_current_string = |key: &str| {
        if form.get(key).is_some() {
            form_string(form, key)
        } else {
            current[key].as_str().map(ToString::to_string)
        }
    };
    let tts_provider = normalize_hermes_tts_provider(form_or_current_string("ttsProvider"), true)?;
    let tts_edge_voice = form_or_current_string("ttsEdgeVoice").unwrap_or_default().trim().to_string();
    let tts_openai_model = form_or_current_string("ttsOpenaiModel")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "gpt-4o-mini-tts".to_string());
    let tts_openai_voice = normalize_hermes_tts_openai_voice(form_or_current_string("ttsOpenaiVoice"), true)?;
    let tts_elevenlabs_voice_id = form_or_current_string("ttsElevenlabsVoiceId")
        .unwrap_or_default()
        .trim()
        .to_string();
    let tts_elevenlabs_model_id = form_or_current_string("ttsElevenlabsModelId")
        .unwrap_or_default()
        .trim()
        .to_string();
    let tts_xai_voice_id = form_or_current_string("ttsXaiVoiceId")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "eve".to_string());
    let tts_xai_language = normalize_hermes_voice_language(form_or_current_string("ttsXaiLanguage"), true, "tts.xai.language")?;
    let tts_xai_sample_rate = validate_hermes_i64(
        if form.get("ttsXaiSampleRate").is_some() {
            form_i64(form, "ttsXaiSampleRate")
        } else {
            current["ttsXaiSampleRate"].as_i64()
        },
        "tts.xai.sample_rate",
        24000,
        8000,
        192000,
    )?;
    let tts_xai_bit_rate = validate_hermes_i64(
        if form.get("ttsXaiBitRate").is_some() {
            form_i64(form, "ttsXaiBitRate")
        } else {
            current["ttsXaiBitRate"].as_i64()
        },
        "tts.xai.bit_rate",
        128000,
        16000,
        512000,
    )?;
    let tts_mistral_model = form_or_current_string("ttsMistralModel")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "voxtral-mini-tts-2603".to_string());
    let tts_mistral_voice_id = form_or_current_string("ttsMistralVoiceId")
        .unwrap_or_default()
        .trim()
        .to_string();
    let tts_piper_voice = form_or_current_string("ttsPiperVoice").unwrap_or_default().trim().to_string();
    let voice_record_key = form_or_current_string("voiceRecordKey")
        .unwrap_or_default()
        .trim()
        .to_string();
    let voice_max_recording_seconds = validate_hermes_i64(
        if form.get("voiceMaxRecordingSeconds").is_some() {
            form_i64(form, "voiceMaxRecordingSeconds")
        } else {
            current["voiceMaxRecordingSeconds"].as_i64()
        },
        "voice.max_recording_seconds",
        120,
        1,
        3600,
    )?;
    let voice_auto_tts = form_bool(form, "voiceAutoTts").unwrap_or_else(|| current["voiceAutoTts"].as_bool().unwrap_or(false));
    let voice_beep_enabled =
        form_bool(form, "voiceBeepEnabled").unwrap_or_else(|| current["voiceBeepEnabled"].as_bool().unwrap_or(true));
    let voice_silence_threshold = validate_hermes_i64(
        if form.get("voiceSilenceThreshold").is_some() {
            form_i64(form, "voiceSilenceThreshold")
        } else {
            current["voiceSilenceThreshold"].as_i64()
        },
        "voice.silence_threshold",
        200,
        0,
        32767,
    )?;
    let voice_silence_duration = validate_hermes_f64(
        if form.get("voiceSilenceDuration").is_some() {
            form_f64(form, "voiceSilenceDuration")
        } else {
            current["voiceSilenceDuration"].as_f64()
        },
        "voice.silence_duration",
        3.0,
        0.1,
        60.0,
    )?;

    let root = ensure_yaml_object(config)?;
    let tts = yaml_child_object(root, "tts")?;
    tts.insert(yaml_key("provider"), serde_yaml::Value::String(tts_provider));
    let edge = yaml_child_object(tts, "edge")?;
    set_optional_yaml_string(edge, "voice", tts_edge_voice);
    let openai = yaml_child_object(tts, "openai")?;
    openai.insert(yaml_key("model"), serde_yaml::Value::String(tts_openai_model));
    openai.insert(yaml_key("voice"), serde_yaml::Value::String(tts_openai_voice));
    let elevenlabs = yaml_child_object(tts, "elevenlabs")?;
    set_optional_yaml_string(elevenlabs, "voice_id", tts_elevenlabs_voice_id);
    set_optional_yaml_string(elevenlabs, "model_id", tts_elevenlabs_model_id);
    let xai = yaml_child_object(tts, "xai")?;
    xai.insert(yaml_key("voice_id"), serde_yaml::Value::String(tts_xai_voice_id));
    xai.insert(yaml_key("language"), serde_yaml::Value::String(tts_xai_language));
    xai.insert(yaml_key("sample_rate"), serde_yaml::Value::Number(tts_xai_sample_rate.into()));
    xai.insert(yaml_key("bit_rate"), serde_yaml::Value::Number(tts_xai_bit_rate.into()));
    let mistral = yaml_child_object(tts, "mistral")?;
    mistral.insert(yaml_key("model"), serde_yaml::Value::String(tts_mistral_model));
    set_optional_yaml_string(mistral, "voice_id", tts_mistral_voice_id);
    let piper = yaml_child_object(tts, "piper")?;
    set_optional_yaml_string(piper, "voice", tts_piper_voice);

    let voice = yaml_child_object(root, "voice")?;
    set_optional_yaml_string(voice, "record_key", voice_record_key);
    voice.insert(
        yaml_key("max_recording_seconds"),
        serde_yaml::Value::Number(voice_max_recording_seconds.into()),
    );
    voice.insert(yaml_key("auto_tts"), serde_yaml::Value::Bool(voice_auto_tts));
    voice.insert(yaml_key("beep_enabled"), serde_yaml::Value::Bool(voice_beep_enabled));
    voice.insert(yaml_key("silence_threshold"), serde_yaml::Value::Number(voice_silence_threshold.into()));
    voice.insert(
        yaml_key("silence_duration"),
        serde_yaml::to_value(voice_silence_duration).map_err(|err| err.to_string())?,
    );
    Ok(())
}

fn merge_hermes_execution_limits_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_execution_limits_config_values(config);
    let code_execution_mode = normalize_hermes_code_execution_mode(
        if form.get("codeExecutionMode").is_some() {
            form_string(form, "codeExecutionMode")
        } else {
            current["codeExecutionMode"].as_str().map(ToString::to_string)
        },
        true,
    )?;
    let code_execution_timeout = validate_hermes_i64(
        if form.get("codeExecutionTimeout").is_some() {
            form_i64(form, "codeExecutionTimeout")
        } else {
            Some(current["codeExecutionTimeout"].as_i64().unwrap_or(300))
        },
        "code_execution.timeout",
        300,
        1,
        86400,
    )?;
    let code_execution_max_tool_calls = validate_hermes_i64(
        if form.get("codeExecutionMaxToolCalls").is_some() {
            form_i64(form, "codeExecutionMaxToolCalls")
        } else {
            Some(current["codeExecutionMaxToolCalls"].as_i64().unwrap_or(50))
        },
        "code_execution.max_tool_calls",
        50,
        1,
        10000,
    )?;
    let delegation_max_iterations = validate_hermes_i64(
        if form.get("delegationMaxIterations").is_some() {
            form_i64(form, "delegationMaxIterations")
        } else {
            Some(current["delegationMaxIterations"].as_i64().unwrap_or(50))
        },
        "delegation.max_iterations",
        50,
        1,
        1000,
    )?;
    let delegation_child_timeout_seconds = validate_hermes_i64(
        if form.get("delegationChildTimeoutSeconds").is_some() {
            form_i64(form, "delegationChildTimeoutSeconds")
        } else {
            Some(current["delegationChildTimeoutSeconds"].as_i64().unwrap_or(600))
        },
        "delegation.child_timeout_seconds",
        600,
        30,
        86400,
    )?;
    let delegation_max_concurrent_children = validate_hermes_i64(
        if form.get("delegationMaxConcurrentChildren").is_some() {
            form_i64(form, "delegationMaxConcurrentChildren")
        } else {
            Some(current["delegationMaxConcurrentChildren"].as_i64().unwrap_or(3))
        },
        "delegation.max_concurrent_children",
        3,
        1,
        100,
    )?;
    let delegation_max_spawn_depth = validate_hermes_i64(
        if form.get("delegationMaxSpawnDepth").is_some() {
            form_i64(form, "delegationMaxSpawnDepth")
        } else {
            Some(current["delegationMaxSpawnDepth"].as_i64().unwrap_or(1))
        },
        "delegation.max_spawn_depth",
        1,
        1,
        3,
    )?;
    let delegation_orchestrator_enabled = form_bool(form, "delegationOrchestratorEnabled")
        .unwrap_or_else(|| current["delegationOrchestratorEnabled"].as_bool().unwrap_or(true));
    let delegation_subagent_auto_approve = form_bool(form, "delegationSubagentAutoApprove")
        .unwrap_or_else(|| current["delegationSubagentAutoApprove"].as_bool().unwrap_or(false));
    let delegation_inherit_mcp_toolsets = form_bool(form, "delegationInheritMcpToolsets")
        .unwrap_or_else(|| current["delegationInheritMcpToolsets"].as_bool().unwrap_or(true));
    let delegation_model = form_string(form, "delegationModel")
        .or_else(|| current["delegationModel"].as_str().map(ToString::to_string))
        .unwrap_or_default()
        .trim()
        .to_string();
    let delegation_provider = form_string(form, "delegationProvider")
        .or_else(|| current["delegationProvider"].as_str().map(ToString::to_string))
        .unwrap_or_default()
        .trim()
        .to_string();

    let root = ensure_yaml_object(config)?;
    let code_execution = yaml_child_object(root, "code_execution")?;
    code_execution.insert(yaml_key("mode"), serde_yaml::Value::String(code_execution_mode));
    code_execution.insert(yaml_key("timeout"), serde_yaml::Value::Number(code_execution_timeout.into()));
    code_execution.insert(
        yaml_key("max_tool_calls"),
        serde_yaml::Value::Number(code_execution_max_tool_calls.into()),
    );

    let delegation = yaml_child_object(root, "delegation")?;
    delegation.insert(yaml_key("max_iterations"), serde_yaml::Value::Number(delegation_max_iterations.into()));
    delegation.insert(
        yaml_key("child_timeout_seconds"),
        serde_yaml::Value::Number(delegation_child_timeout_seconds.into()),
    );
    delegation.insert(
        yaml_key("max_concurrent_children"),
        serde_yaml::Value::Number(delegation_max_concurrent_children.into()),
    );
    delegation.insert(yaml_key("max_spawn_depth"), serde_yaml::Value::Number(delegation_max_spawn_depth.into()));
    delegation.insert(yaml_key("orchestrator_enabled"), serde_yaml::Value::Bool(delegation_orchestrator_enabled));
    delegation.insert(
        yaml_key("subagent_auto_approve"),
        serde_yaml::Value::Bool(delegation_subagent_auto_approve),
    );
    delegation.insert(yaml_key("inherit_mcp_toolsets"), serde_yaml::Value::Bool(delegation_inherit_mcp_toolsets));
    if delegation_model.is_empty() {
        delegation.remove(yaml_key("model"));
    } else {
        delegation.insert(yaml_key("model"), serde_yaml::Value::String(delegation_model));
    }
    if delegation_provider.is_empty() {
        delegation.remove(yaml_key("provider"));
    } else {
        delegation.insert(yaml_key("provider"), serde_yaml::Value::String(delegation_provider));
    }
    Ok(())
}