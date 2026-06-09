
fn build_hermes_web_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let web = root.and_then(|map| yaml_get_mapping(map, "web"));
    let web_backend = normalize_hermes_web_backend(web.and_then(|map| yaml_string_field(map, "backend")), "web.backend", false)
        .unwrap_or_default();
    let web_search_backend =
        normalize_hermes_web_backend(web.and_then(|map| yaml_string_field(map, "search_backend")), "web.search_backend", false)
            .unwrap_or_default();
    let web_extract_backend = normalize_hermes_web_backend(
        web.and_then(|map| yaml_string_field(map, "extract_backend")),
        "web.extract_backend",
        false,
    )
    .unwrap_or_default();

    crate::jv!({
        "webBackend": web_backend,
        "webSearchBackend": web_search_backend,
        "webExtractBackend": web_extract_backend,
    })
}

fn merge_hermes_web_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_web_config_values(config);
    let web_backend = normalize_hermes_web_backend(
        if form.get("webBackend").is_some() {
            form_string(form, "webBackend")
        } else {
            current["webBackend"].as_str().map(ToString::to_string)
        },
        "web.backend",
        true,
    )?;
    let web_search_backend = normalize_hermes_web_backend(
        if form.get("webSearchBackend").is_some() {
            form_string(form, "webSearchBackend")
        } else {
            current["webSearchBackend"].as_str().map(ToString::to_string)
        },
        "web.search_backend",
        true,
    )?;
    let web_extract_backend = normalize_hermes_web_backend(
        if form.get("webExtractBackend").is_some() {
            form_string(form, "webExtractBackend")
        } else {
            current["webExtractBackend"].as_str().map(ToString::to_string)
        },
        "web.extract_backend",
        true,
    )?;

    let root = ensure_yaml_object(config)?;
    let web = yaml_child_object(root, "web")?;
    set_optional_yaml_string(web, "backend", web_backend);
    set_optional_yaml_string(web, "search_backend", web_search_backend);
    set_optional_yaml_string(web, "extract_backend", web_extract_backend);
    Ok(())
}

fn build_hermes_model_catalog_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let model_catalog = root.and_then(|map| yaml_get_mapping(map, "model_catalog"));
    let enabled = model_catalog.and_then(|map| yaml_bool_field(map, "enabled")).unwrap_or(true);
    let url = normalize_hermes_http_url(
        model_catalog.and_then(|map| yaml_string_field(map, "url")),
        "model_catalog.url",
        HERMES_MODEL_CATALOG_DEFAULT_URL,
        false,
    )
    .unwrap_or_else(|_| HERMES_MODEL_CATALOG_DEFAULT_URL.to_string());
    let ttl_hours = model_catalog
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "ttl_hours"), 24, 1, 8760))
        .unwrap_or(24);
    let providers = model_catalog
        .and_then(|map| yaml_get(map, "providers"))
        .and_then(|value| serde_json::to_value(value).ok())
        .and_then(|value| validate_hermes_model_catalog_providers(&value).ok())
        .unwrap_or_default();
    crate::jv!({
        "modelCatalogEnabled": enabled,
        "modelCatalogUrl": url,
        "modelCatalogTtlHours": ttl_hours,
        "modelCatalogProvidersJson": serde_json::to_string_pretty(&Value::Object(providers)).unwrap_or_else(|_| "{}".to_string()),
    })
}

fn merge_hermes_model_catalog_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_model_catalog_config_values(config);
    let enabled =
        form_bool(form, "modelCatalogEnabled").unwrap_or_else(|| current["modelCatalogEnabled"].as_bool().unwrap_or(true));
    let url = normalize_hermes_http_url(
        if form.get("modelCatalogUrl").is_some() {
            form_string(form, "modelCatalogUrl")
        } else {
            current["modelCatalogUrl"].as_str().map(ToString::to_string)
        },
        "model_catalog.url",
        HERMES_MODEL_CATALOG_DEFAULT_URL,
        true,
    )?;
    let ttl_hours = validate_hermes_i64(
        if form.get("modelCatalogTtlHours").is_some() {
            form_i64(form, "modelCatalogTtlHours")
        } else {
            current["modelCatalogTtlHours"].as_i64()
        },
        "model_catalog.ttl_hours",
        24,
        1,
        8760,
    )?;
    let providers = parse_hermes_model_catalog_providers_json(if form.get("modelCatalogProvidersJson").is_some() {
        form_string(form, "modelCatalogProvidersJson")
    } else {
        current["modelCatalogProvidersJson"].as_str().map(ToString::to_string)
    })?;

    let root = ensure_yaml_object(config)?;
    let model_catalog = yaml_child_object(root, "model_catalog")?;
    model_catalog.insert(yaml_key("enabled"), serde_yaml::Value::Bool(enabled));
    model_catalog.insert(yaml_key("url"), serde_yaml::Value::String(url));
    model_catalog.insert(yaml_key("ttl_hours"), serde_yaml::Value::Number(serde_yaml::Number::from(ttl_hours)));
    if providers.is_empty() {
        model_catalog.remove(yaml_key("providers"));
    } else {
        let yaml_value =
            serde_yaml::to_value(Value::Object(providers)).map_err(|err| format!("model_catalog.providers 序列化失败: {err}"))?;
        model_catalog.insert(yaml_key("providers"), yaml_value);
    }
    Ok(())
}

fn normalize_hermes_x_search_model(value: Option<String>, strict: bool) -> Result<String, String> {
    let text = value.unwrap_or_default().trim().to_string();
    if text.is_empty() {
        if strict {
            return Err("x_search.model 不能为空".to_string());
        }
        return Ok("grok-4.20-reasoning".to_string());
    }
    if text
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | ':' | '/' | '-'))
    {
        return Ok(text);
    }
    if strict {
        return Err("x_search.model 只能包含字母、数字、下划线、点、斜杠、冒号和短横线".to_string());
    }
    Ok("grok-4.20-reasoning".to_string())
}

fn build_hermes_x_search_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let x_search = root.and_then(|map| yaml_get_mapping(map, "x_search"));
    let model = normalize_hermes_x_search_model(x_search.and_then(|map| yaml_string_field(map, "model")), false)
        .unwrap_or_else(|_| "grok-4.20-reasoning".to_string());
    let timeout_seconds = x_search
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "timeout_seconds"), 180, 30, 3600))
        .unwrap_or(180);
    let retries = x_search
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "retries"), 2, 0, 20))
        .unwrap_or(2);

    crate::jv!({
        "xSearchModel": model,
        "xSearchTimeoutSeconds": timeout_seconds,
        "xSearchRetries": retries,
    })
}

fn merge_hermes_x_search_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_x_search_config_values(config);
    let model = normalize_hermes_x_search_model(
        if form.get("xSearchModel").is_some() {
            form_string(form, "xSearchModel")
        } else {
            current["xSearchModel"].as_str().map(ToString::to_string)
        },
        true,
    )?;
    let timeout_seconds = validate_hermes_i64(
        if form.get("xSearchTimeoutSeconds").is_some() {
            form_i64(form, "xSearchTimeoutSeconds")
        } else {
            current["xSearchTimeoutSeconds"].as_i64()
        },
        "x_search.timeout_seconds",
        180,
        30,
        3600,
    )?;
    let retries = validate_hermes_i64(
        if form.get("xSearchRetries").is_some() {
            form_i64(form, "xSearchRetries")
        } else {
            current["xSearchRetries"].as_i64()
        },
        "x_search.retries",
        2,
        0,
        20,
    )?;

    let root = ensure_yaml_object(config)?;
    let x_search = yaml_child_object(root, "x_search")?;
    x_search.insert(yaml_key("model"), serde_yaml::Value::String(model));
    x_search.insert(
        yaml_key("timeout_seconds"),
        serde_yaml::Value::Number(serde_yaml::Number::from(timeout_seconds)),
    );
    x_search.insert(yaml_key("retries"), serde_yaml::Value::Number(serde_yaml::Number::from(retries)));
    Ok(())
}

fn normalize_hermes_context_engine(value: Option<String>, strict: bool) -> Result<String, String> {
    let text = value.unwrap_or_default().trim().to_string();
    if text.is_empty() {
        if strict {
            return Err("context.engine 不能为空".to_string());
        }
        return Ok("compressor".to_string());
    }
    if text
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
    {
        return Ok(text);
    }
    if strict {
        return Err("context.engine 只能包含字母、数字、下划线、点和短横线".to_string());
    }
    Ok("compressor".to_string())
}

fn build_hermes_context_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let context = root.and_then(|map| yaml_get_mapping(map, "context"));
    let engine = normalize_hermes_context_engine(context.and_then(|map| yaml_string_field(map, "engine")), false)
        .unwrap_or_else(|_| "compressor".to_string());

    crate::jv!({
        "contextEngine": engine,
    })
}

fn merge_hermes_context_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_context_config_values(config);
    let engine = normalize_hermes_context_engine(
        if form.get("contextEngine").is_some() {
            form_string(form, "contextEngine")
        } else {
            current["contextEngine"].as_str().map(ToString::to_string)
        },
        true,
    )?;

    let root = ensure_yaml_object(config)?;
    let context = yaml_child_object(root, "context")?;
    context.insert(yaml_key("engine"), serde_yaml::Value::String(engine));
    Ok(())
}

fn build_hermes_lsp_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let lsp = root.and_then(|map| yaml_get_mapping(map, "lsp"));
    let lsp_enabled = lsp.and_then(|map| yaml_bool_field(map, "enabled")).unwrap_or(true);
    let lsp_wait_mode = normalize_hermes_lsp_wait_mode(lsp.and_then(|map| yaml_string_field(map, "wait_mode")), false)
        .unwrap_or_else(|_| "document".to_string());
    let lsp_wait_timeout = lsp
        .map(|map| bounded_hermes_f64(yaml_f64_field(map, "wait_timeout"), 5.0, 0.1, 120.0))
        .unwrap_or(5.0);
    let lsp_install_strategy =
        normalize_hermes_lsp_install_strategy(lsp.and_then(|map| yaml_string_field(map, "install_strategy")), false)
            .unwrap_or_else(|_| "auto".to_string());

    crate::jv!({
        "lspEnabled": lsp_enabled,
        "lspWaitMode": lsp_wait_mode,
        "lspWaitTimeout": lsp_wait_timeout,
        "lspInstallStrategy": lsp_install_strategy,
    })
}

fn merge_hermes_lsp_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_lsp_config_values(config);
    let lsp_enabled = form_bool(form, "lspEnabled").unwrap_or_else(|| current["lspEnabled"].as_bool().unwrap_or(true));
    let lsp_wait_mode = normalize_hermes_lsp_wait_mode(
        if form.get("lspWaitMode").is_some() {
            form_string(form, "lspWaitMode")
        } else {
            current["lspWaitMode"].as_str().map(ToString::to_string)
        },
        true,
    )?;
    let lsp_wait_timeout = validate_hermes_f64(
        if form.get("lspWaitTimeout").is_some() {
            form_f64(form, "lspWaitTimeout")
        } else {
            current["lspWaitTimeout"].as_f64()
        },
        "lsp.wait_timeout",
        5.0,
        0.1,
        120.0,
    )?;
    let lsp_install_strategy = normalize_hermes_lsp_install_strategy(
        if form.get("lspInstallStrategy").is_some() {
            form_string(form, "lspInstallStrategy")
        } else {
            current["lspInstallStrategy"].as_str().map(ToString::to_string)
        },
        true,
    )?;

    let root = ensure_yaml_object(config)?;
    let lsp = yaml_child_object(root, "lsp")?;
    lsp.insert(yaml_key("enabled"), serde_yaml::Value::Bool(lsp_enabled));
    lsp.insert(yaml_key("wait_mode"), serde_yaml::Value::String(lsp_wait_mode));
    lsp.insert(
        yaml_key("wait_timeout"),
        serde_yaml::Value::Number(serde_yaml::Number::from(lsp_wait_timeout)),
    );
    lsp.insert(yaml_key("install_strategy"), serde_yaml::Value::String(lsp_install_strategy));
    Ok(())
}

fn build_hermes_stt_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let stt = root.and_then(|map| yaml_get_mapping(map, "stt"));
    let local = stt.and_then(|map| yaml_get_mapping(map, "local"));
    let openai = stt.and_then(|map| yaml_get_mapping(map, "openai"));
    let mistral = stt.and_then(|map| yaml_get_mapping(map, "mistral"));
    let stt_enabled = stt.and_then(|map| yaml_bool_field(map, "enabled")).unwrap_or(true);
    let stt_provider = normalize_hermes_stt_provider(stt.and_then(|map| yaml_string_field(map, "provider")), false)
        .unwrap_or_else(|_| "auto".to_string());
    let stt_local_model = normalize_hermes_stt_local_model(local.and_then(|map| yaml_string_field(map, "model")), false)
        .unwrap_or_else(|_| "base".to_string());
    let stt_local_language = normalize_hermes_stt_language(local.and_then(|map| yaml_string_field(map, "language")), false)
        .unwrap_or_else(|_| String::new());
    let stt_openai_model = normalize_hermes_stt_openai_model(openai.and_then(|map| yaml_string_field(map, "model")), false)
        .unwrap_or_else(|_| "whisper-1".to_string());
    let stt_mistral_model = normalize_hermes_stt_mistral_model(mistral.and_then(|map| yaml_string_field(map, "model")), false)
        .unwrap_or_else(|_| "voxtral-mini-latest".to_string());

    crate::jv!({
        "sttEnabled": stt_enabled,
        "sttProvider": stt_provider,
        "sttLocalModel": stt_local_model,
        "sttLocalLanguage": stt_local_language,
        "sttOpenaiModel": stt_openai_model,
        "sttMistralModel": stt_mistral_model,
    })
}

fn merge_hermes_stt_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_stt_config_values(config);
    let stt_enabled = form_bool(form, "sttEnabled").unwrap_or_else(|| current["sttEnabled"].as_bool().unwrap_or(true));
    let stt_provider = normalize_hermes_stt_provider(
        if form.get("sttProvider").is_some() {
            form_string(form, "sttProvider")
        } else {
            current["sttProvider"].as_str().map(ToString::to_string)
        },
        true,
    )?;
    let stt_local_model = normalize_hermes_stt_local_model(
        if form.get("sttLocalModel").is_some() {
            form_string(form, "sttLocalModel")
        } else {
            current["sttLocalModel"].as_str().map(ToString::to_string)
        },
        true,
    )?;
    let stt_local_language = normalize_hermes_stt_language(
        if form.get("sttLocalLanguage").is_some() {
            form_string(form, "sttLocalLanguage")
        } else {
            current["sttLocalLanguage"].as_str().map(ToString::to_string)
        },
        true,
    )?;
    let stt_openai_model = normalize_hermes_stt_openai_model(
        if form.get("sttOpenaiModel").is_some() {
            form_string(form, "sttOpenaiModel")
        } else {
            current["sttOpenaiModel"].as_str().map(ToString::to_string)
        },
        true,
    )?;
    let stt_mistral_model = normalize_hermes_stt_mistral_model(
        if form.get("sttMistralModel").is_some() {
            form_string(form, "sttMistralModel")
        } else {
            current["sttMistralModel"].as_str().map(ToString::to_string)
        },
        true,
    )?;

    let root = ensure_yaml_object(config)?;
    let stt = yaml_child_object(root, "stt")?;
    stt.insert(yaml_key("enabled"), serde_yaml::Value::Bool(stt_enabled));
    stt.insert(yaml_key("provider"), serde_yaml::Value::String(stt_provider));

    let local = yaml_child_object(stt, "local")?;
    local.insert(yaml_key("model"), serde_yaml::Value::String(stt_local_model));
    local.insert(yaml_key("language"), serde_yaml::Value::String(stt_local_language));

    let openai = yaml_child_object(stt, "openai")?;
    openai.insert(yaml_key("model"), serde_yaml::Value::String(stt_openai_model));

    let mistral = yaml_child_object(stt, "mistral")?;
    mistral.insert(yaml_key("model"), serde_yaml::Value::String(stt_mistral_model));
    Ok(())
}

include!("integration_config/tts_execution_config.rs");