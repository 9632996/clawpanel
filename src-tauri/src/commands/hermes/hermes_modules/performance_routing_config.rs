
fn parse_hermes_model_catalog_providers_json(raw: Option<String>) -> Result<serde_json::Map<String, Value>, String> {
    let text = raw.unwrap_or_default().trim().to_string();
    if text.is_empty() {
        return Ok(serde_json::Map::new());
    }
    let value: Value = serde_json::from_str(&text).map_err(|err| format!("model_catalog.providers JSON 格式错误: {err}"))?;
    validate_hermes_model_catalog_providers(&value)
}

fn build_hermes_compression_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let compression = root.and_then(|map| yaml_get_mapping(map, "compression"));
    let enabled = compression.and_then(|map| yaml_bool_field(map, "enabled")).unwrap_or(true);
    let threshold = compression
        .map(|map| bounded_hermes_f64(yaml_f64_field(map, "threshold"), 0.5, 0.1, 0.95))
        .unwrap_or(0.5);
    let target_ratio = compression
        .map(|map| bounded_hermes_f64(yaml_f64_field(map, "target_ratio"), 0.2, 0.1, 0.8))
        .unwrap_or(0.2);
    let protect_last_n = compression
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "protect_last_n"), 20, 1, 500))
        .unwrap_or(20);
    let protect_first_n = compression
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "protect_first_n"), 3, 0, 100))
        .unwrap_or(3);
    let abort_on_summary_failure = compression
        .and_then(|map| yaml_bool_field(map, "abort_on_summary_failure"))
        .unwrap_or(false);

    crate::jv!({
        "enabled": enabled,
        "threshold": threshold,
        "targetRatio": target_ratio,
        "protectLastN": protect_last_n,
        "protectFirstN": protect_first_n,
        "abortOnSummaryFailure": abort_on_summary_failure,
    })
}

fn merge_hermes_compression_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_compression_config_values(config);
    let enabled = form_bool(form, "enabled").unwrap_or_else(|| current["enabled"].as_bool().unwrap_or(true));
    let threshold = validate_hermes_f64(
        if form.get("threshold").is_some() {
            form_f64(form, "threshold")
        } else {
            Some(current["threshold"].as_f64().unwrap_or(0.5))
        },
        "compression.threshold",
        0.5,
        0.1,
        0.95,
    )?;
    let target_ratio = validate_hermes_f64(
        if form.get("targetRatio").is_some() {
            form_f64(form, "targetRatio")
        } else {
            Some(current["targetRatio"].as_f64().unwrap_or(0.2))
        },
        "compression.target_ratio",
        0.2,
        0.1,
        0.8,
    )?;
    let protect_last_n = validate_hermes_i64(
        if form.get("protectLastN").is_some() {
            form_i64(form, "protectLastN")
        } else {
            Some(current["protectLastN"].as_i64().unwrap_or(20))
        },
        "compression.protect_last_n",
        20,
        1,
        500,
    )?;
    let protect_first_n = validate_hermes_i64(
        if form.get("protectFirstN").is_some() {
            form_i64(form, "protectFirstN")
        } else {
            Some(current["protectFirstN"].as_i64().unwrap_or(3))
        },
        "compression.protect_first_n",
        3,
        0,
        100,
    )?;
    let abort_on_summary_failure =
        form_bool(form, "abortOnSummaryFailure").unwrap_or_else(|| current["abortOnSummaryFailure"].as_bool().unwrap_or(false));

    let root = ensure_yaml_object(config)?;
    let compression = yaml_child_object(root, "compression")?;
    compression.insert(yaml_key("enabled"), serde_yaml::Value::Bool(enabled));
    compression.insert(yaml_key("threshold"), serde_yaml::Value::Number(threshold.into()));
    compression.insert(yaml_key("target_ratio"), serde_yaml::Value::Number(target_ratio.into()));
    compression.insert(yaml_key("protect_last_n"), serde_yaml::Value::Number(protect_last_n.into()));
    compression.insert(yaml_key("protect_first_n"), serde_yaml::Value::Number(protect_first_n.into()));
    compression.insert(yaml_key("abort_on_summary_failure"), serde_yaml::Value::Bool(abort_on_summary_failure));
    Ok(())
}

fn build_hermes_prompt_caching_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let prompt_caching = root.and_then(|map| yaml_get_mapping(map, "prompt_caching"));
    crate::jv!({
        "promptCacheTtl": normalize_hermes_prompt_cache_ttl(
            prompt_caching.and_then(|map| yaml_string_field(map, "cache_ttl")),
            false,
        ).unwrap_or_else(|_| "5m".to_string()),
    })
}

fn merge_hermes_prompt_caching_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_prompt_caching_config_values(config);
    let cache_ttl = normalize_hermes_prompt_cache_ttl(
        form_string(form, "promptCacheTtl").or_else(|| current["promptCacheTtl"].as_str().map(ToString::to_string)),
        true,
    )?;

    let root = ensure_yaml_object(config)?;
    let prompt_caching = yaml_child_object(root, "prompt_caching")?;
    prompt_caching.insert(yaml_key("cache_ttl"), serde_yaml::Value::String(cache_ttl));
    Ok(())
}

fn build_hermes_openrouter_cache_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let openrouter = root.and_then(|map| yaml_get_mapping(map, "openrouter"));
    crate::jv!({
        "openrouterResponseCache": openrouter.and_then(|map| yaml_bool_field(map, "response_cache")).unwrap_or(true),
        "openrouterResponseCacheTtl": openrouter.map(|map| bounded_hermes_i64(yaml_i64_field(map, "response_cache_ttl"), 300, 1, 86400)).unwrap_or(300),
    })
}

fn merge_hermes_openrouter_cache_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_openrouter_cache_config_values(config);
    let response_cache = form_bool(form, "openrouterResponseCache")
        .unwrap_or_else(|| current["openrouterResponseCache"].as_bool().unwrap_or(true));
    let response_cache_ttl_input = if form.get("openrouterResponseCacheTtl").is_some() {
        Some(form_i64(form, "openrouterResponseCacheTtl").ok_or_else(|| "openrouter.response_cache_ttl 必须是整数".to_string())?)
    } else {
        Some(current["openrouterResponseCacheTtl"].as_i64().unwrap_or(300))
    };
    let response_cache_ttl = validate_hermes_i64(response_cache_ttl_input, "openrouter.response_cache_ttl", 300, 1, 86400)?;

    let root = ensure_yaml_object(config)?;
    let openrouter = yaml_child_object(root, "openrouter")?;
    openrouter.insert(yaml_key("response_cache"), serde_yaml::Value::Bool(response_cache));
    openrouter.insert(yaml_key("response_cache_ttl"), serde_yaml::Value::Number(response_cache_ttl.into()));
    Ok(())
}

fn provider_routing_list_from_yaml(map: Option<&serde_yaml::Mapping>, key: &str) -> Result<Vec<String>, String> {
    let raw = map
        .map(|map| yaml_string_sequence_field(map, key).join("\n"))
        .unwrap_or_default();
    normalize_hermes_provider_routing_list(Some(raw), &format!("provider_routing.{key}"))
}

fn build_hermes_provider_routing_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let provider_routing = root.and_then(|map| yaml_get_mapping(map, "provider_routing"));
    let sort = normalize_hermes_provider_routing_sort(provider_routing.and_then(|map| yaml_string_field(map, "sort")), false)
        .unwrap_or_else(|_| "price".to_string());
    let data_collection = normalize_hermes_provider_routing_data_collection(
        provider_routing.and_then(|map| yaml_string_field(map, "data_collection")),
        false,
    )
    .unwrap_or_else(|_| "allow".to_string());
    let only = provider_routing_list_from_yaml(provider_routing, "only").unwrap_or_default();
    let ignore = provider_routing_list_from_yaml(provider_routing, "ignore").unwrap_or_default();
    let order = provider_routing_list_from_yaml(provider_routing, "order").unwrap_or_default();

    crate::jv!({
        "providerRoutingSort": sort,
        "providerRoutingOnly": only.join("\n"),
        "providerRoutingIgnore": ignore.join("\n"),
        "providerRoutingOrder": order.join("\n"),
        "providerRoutingRequireParameters": provider_routing.and_then(|map| yaml_bool_field(map, "require_parameters")).unwrap_or(false),
        "providerRoutingDataCollection": data_collection,
    })
}

fn merge_hermes_provider_routing_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_provider_routing_config_values(config);
    let sort = normalize_hermes_provider_routing_sort(
        if form.get("providerRoutingSort").is_some() {
            form_string(form, "providerRoutingSort")
        } else {
            current["providerRoutingSort"].as_str().map(ToString::to_string)
        },
        true,
    )?;
    let data_collection = normalize_hermes_provider_routing_data_collection(
        if form.get("providerRoutingDataCollection").is_some() {
            form_string(form, "providerRoutingDataCollection")
        } else {
            current["providerRoutingDataCollection"].as_str().map(ToString::to_string)
        },
        true,
    )?;
    let require_parameters = form_bool(form, "providerRoutingRequireParameters")
        .unwrap_or_else(|| current["providerRoutingRequireParameters"].as_bool().unwrap_or(false));

    let only = normalize_hermes_provider_routing_list(
        form_string(form, "providerRoutingOnly").or_else(|| current["providerRoutingOnly"].as_str().map(ToString::to_string)),
        "provider_routing.only",
    )?;
    let ignore = normalize_hermes_provider_routing_list(
        form_string(form, "providerRoutingIgnore").or_else(|| current["providerRoutingIgnore"].as_str().map(ToString::to_string)),
        "provider_routing.ignore",
    )?;
    let order = normalize_hermes_provider_routing_list(
        form_string(form, "providerRoutingOrder").or_else(|| current["providerRoutingOrder"].as_str().map(ToString::to_string)),
        "provider_routing.order",
    )?;

    let root = ensure_yaml_object(config)?;
    let provider_routing = yaml_child_object(root, "provider_routing")?;
    provider_routing.insert(yaml_key("sort"), serde_yaml::Value::String(sort));
    provider_routing.insert(yaml_key("require_parameters"), serde_yaml::Value::Bool(require_parameters));
    provider_routing.insert(yaml_key("data_collection"), serde_yaml::Value::String(data_collection));

    for (key, values) in [("only", only), ("ignore", ignore), ("order", order)] {
        if values.is_empty() {
            provider_routing.remove(yaml_key(key));
        } else {
            provider_routing.insert(
                yaml_key(key),
                serde_yaml::Value::Sequence(values.into_iter().map(serde_yaml::Value::String).collect()),
            );
        }
    }
    Ok(())
}

fn hermes_auxiliary_task<'a>(root: Option<&'a serde_yaml::Mapping>, key: &str) -> Option<&'a serde_yaml::Mapping> {
    root.and_then(|map| yaml_get_mapping(map, "auxiliary"))
        .and_then(|map| yaml_get_mapping(map, key))
}

fn build_hermes_auxiliary_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let vision = hermes_auxiliary_task(root, "vision");
    let web_extract = hermes_auxiliary_task(root, "web_extract");
    let session_search = hermes_auxiliary_task(root, "session_search");

    crate::jv!({
        "auxiliaryVisionProvider": normalize_hermes_auxiliary_provider(
            vision.and_then(|map| yaml_string_field(map, "provider")),
            "auxiliary.vision.provider",
            false,
        ).unwrap_or_else(|_| "auto".to_string()),
        "auxiliaryVisionModel": normalize_hermes_auxiliary_model(
            vision.and_then(|map| yaml_string_field(map, "model")),
            "auxiliary.vision.model",
            false,
        ).unwrap_or_default(),
        "auxiliaryVisionTimeout": vision.map(|map| bounded_hermes_i64(yaml_i64_field(map, "timeout"), 30, 1, 3600)).unwrap_or(30),
        "auxiliaryVisionDownloadTimeout": vision.map(|map| bounded_hermes_i64(yaml_i64_field(map, "download_timeout"), 30, 1, 3600)).unwrap_or(30),
        "auxiliaryWebExtractProvider": normalize_hermes_auxiliary_provider(
            web_extract.and_then(|map| yaml_string_field(map, "provider")),
            "auxiliary.web_extract.provider",
            false,
        ).unwrap_or_else(|_| "auto".to_string()),
        "auxiliaryWebExtractModel": normalize_hermes_auxiliary_model(
            web_extract.and_then(|map| yaml_string_field(map, "model")),
            "auxiliary.web_extract.model",
            false,
        ).unwrap_or_default(),
        "auxiliarySessionSearchProvider": normalize_hermes_auxiliary_provider(
            session_search.and_then(|map| yaml_string_field(map, "provider")),
            "auxiliary.session_search.provider",
            false,
        ).unwrap_or_else(|_| "auto".to_string()),
        "auxiliarySessionSearchModel": normalize_hermes_auxiliary_model(
            session_search.and_then(|map| yaml_string_field(map, "model")),
            "auxiliary.session_search.model",
            false,
        ).unwrap_or_default(),
        "auxiliarySessionSearchTimeout": session_search.map(|map| bounded_hermes_i64(yaml_i64_field(map, "timeout"), 30, 1, 3600)).unwrap_or(30),
        "auxiliarySessionSearchMaxConcurrency": session_search.map(|map| bounded_hermes_i64(yaml_i64_field(map, "max_concurrency"), 3, 1, 100)).unwrap_or(3),
    })
}

fn merge_hermes_auxiliary_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_auxiliary_config_values(config);
    let vision_provider = normalize_hermes_auxiliary_provider(
        form_string(form, "auxiliaryVisionProvider")
            .or_else(|| current["auxiliaryVisionProvider"].as_str().map(ToString::to_string)),
        "auxiliary.vision.provider",
        true,
    )?;
    let vision_model = normalize_hermes_auxiliary_model(
        form_string(form, "auxiliaryVisionModel").or_else(|| current["auxiliaryVisionModel"].as_str().map(ToString::to_string)),
        "auxiliary.vision.model",
        true,
    )?;
    let vision_timeout = validate_hermes_i64(
        if form.get("auxiliaryVisionTimeout").is_some() {
            form_i64(form, "auxiliaryVisionTimeout")
        } else {
            Some(current["auxiliaryVisionTimeout"].as_i64().unwrap_or(30))
        },
        "auxiliary.vision.timeout",
        30,
        1,
        3600,
    )?;
    let vision_download_timeout = validate_hermes_i64(
        if form.get("auxiliaryVisionDownloadTimeout").is_some() {
            form_i64(form, "auxiliaryVisionDownloadTimeout")
        } else {
            Some(current["auxiliaryVisionDownloadTimeout"].as_i64().unwrap_or(30))
        },
        "auxiliary.vision.download_timeout",
        30,
        1,
        3600,
    )?;
    let web_extract_provider = normalize_hermes_auxiliary_provider(
        form_string(form, "auxiliaryWebExtractProvider")
            .or_else(|| current["auxiliaryWebExtractProvider"].as_str().map(ToString::to_string)),
        "auxiliary.web_extract.provider",
        true,
    )?;
    let web_extract_model = normalize_hermes_auxiliary_model(
        form_string(form, "auxiliaryWebExtractModel")
            .or_else(|| current["auxiliaryWebExtractModel"].as_str().map(ToString::to_string)),
        "auxiliary.web_extract.model",
        true,
    )?;
    let session_search_provider = normalize_hermes_auxiliary_provider(
        form_string(form, "auxiliarySessionSearchProvider")
            .or_else(|| current["auxiliarySessionSearchProvider"].as_str().map(ToString::to_string)),
        "auxiliary.session_search.provider",
        true,
    )?;
    let session_search_model = normalize_hermes_auxiliary_model(
        form_string(form, "auxiliarySessionSearchModel")
            .or_else(|| current["auxiliarySessionSearchModel"].as_str().map(ToString::to_string)),
        "auxiliary.session_search.model",
        true,
    )?;
    let session_search_timeout = validate_hermes_i64(
        if form.get("auxiliarySessionSearchTimeout").is_some() {
            form_i64(form, "auxiliarySessionSearchTimeout")
        } else {
            Some(current["auxiliarySessionSearchTimeout"].as_i64().unwrap_or(30))
        },
        "auxiliary.session_search.timeout",
        30,
        1,
        3600,
    )?;
    let session_search_max_concurrency = validate_hermes_i64(
        if form.get("auxiliarySessionSearchMaxConcurrency").is_some() {
            form_i64(form, "auxiliarySessionSearchMaxConcurrency")
        } else {
            Some(current["auxiliarySessionSearchMaxConcurrency"].as_i64().unwrap_or(3))
        },
        "auxiliary.session_search.max_concurrency",
        3,
        1,
        100,
    )?;

    let root = ensure_yaml_object(config)?;
    let auxiliary = yaml_child_object(root, "auxiliary")?;
    let vision = yaml_child_object(auxiliary, "vision")?;
    vision.insert(yaml_key("provider"), serde_yaml::Value::String(vision_provider));
    vision.insert(yaml_key("model"), serde_yaml::Value::String(vision_model));
    vision.insert(yaml_key("timeout"), serde_yaml::Value::Number(vision_timeout.into()));
    vision.insert(yaml_key("download_timeout"), serde_yaml::Value::Number(vision_download_timeout.into()));

    let web_extract = yaml_child_object(auxiliary, "web_extract")?;
    web_extract.insert(yaml_key("provider"), serde_yaml::Value::String(web_extract_provider));
    web_extract.insert(yaml_key("model"), serde_yaml::Value::String(web_extract_model));

    let session_search = yaml_child_object(auxiliary, "session_search")?;
    session_search.insert(yaml_key("provider"), serde_yaml::Value::String(session_search_provider));
    session_search.insert(yaml_key("model"), serde_yaml::Value::String(session_search_model));
    session_search.insert(yaml_key("timeout"), serde_yaml::Value::Number(session_search_timeout.into()));
    session_search.insert(
        yaml_key("max_concurrency"),
        serde_yaml::Value::Number(session_search_max_concurrency.into()),
    );
    Ok(())
}

fn build_hermes_tool_loop_guardrails_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let guardrails = root.and_then(|map| yaml_get_mapping(map, "tool_loop_guardrails"));
    let warn_after = guardrails.and_then(|map| yaml_get_mapping(map, "warn_after"));
    let hard_stop_after = guardrails.and_then(|map| yaml_get_mapping(map, "hard_stop_after"));

    let warnings_enabled = guardrails
        .and_then(|map| yaml_bool_field(map, "warnings_enabled"))
        .unwrap_or(true);
    let hard_stop_enabled = guardrails
        .and_then(|map| yaml_bool_field(map, "hard_stop_enabled"))
        .unwrap_or(false);
    let warn_exact_failure = warn_after
        .and_then(|map| yaml_i64_field(map, "exact_failure"))
        .or_else(|| guardrails.and_then(|map| yaml_i64_field(map, "exact_failure_warn_after")));
    let warn_same_tool_failure = warn_after
        .and_then(|map| yaml_i64_field(map, "same_tool_failure"))
        .or_else(|| guardrails.and_then(|map| yaml_i64_field(map, "same_tool_failure_warn_after")));
    let warn_no_progress = warn_after
        .and_then(|map| yaml_i64_field(map, "idempotent_no_progress"))
        .or_else(|| guardrails.and_then(|map| yaml_i64_field(map, "no_progress_warn_after")));
    let hard_stop_exact_failure = hard_stop_after
        .and_then(|map| yaml_i64_field(map, "exact_failure"))
        .or_else(|| guardrails.and_then(|map| yaml_i64_field(map, "exact_failure_block_after")));
    let hard_stop_same_tool_failure = hard_stop_after
        .and_then(|map| yaml_i64_field(map, "same_tool_failure"))
        .or_else(|| guardrails.and_then(|map| yaml_i64_field(map, "same_tool_failure_halt_after")));
    let hard_stop_no_progress = hard_stop_after
        .and_then(|map| yaml_i64_field(map, "idempotent_no_progress"))
        .or_else(|| guardrails.and_then(|map| yaml_i64_field(map, "no_progress_block_after")));

    crate::jv!({
        "warningsEnabled": warnings_enabled,
        "hardStopEnabled": hard_stop_enabled,
        "warnExactFailure": bounded_hermes_i64(warn_exact_failure, 2, 1, 100),
        "warnSameToolFailure": bounded_hermes_i64(warn_same_tool_failure, 3, 1, 100),
        "warnNoProgress": bounded_hermes_i64(warn_no_progress, 2, 1, 100),
        "hardStopExactFailure": bounded_hermes_i64(hard_stop_exact_failure, 5, 1, 100),
        "hardStopSameToolFailure": bounded_hermes_i64(hard_stop_same_tool_failure, 8, 1, 100),
        "hardStopNoProgress": bounded_hermes_i64(hard_stop_no_progress, 5, 1, 100),
    })
}

fn merge_hermes_tool_loop_guardrails_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_tool_loop_guardrails_config_values(config);
    let warnings_enabled =
        form_bool(form, "warningsEnabled").unwrap_or_else(|| current["warningsEnabled"].as_bool().unwrap_or(true));
    let hard_stop_enabled =
        form_bool(form, "hardStopEnabled").unwrap_or_else(|| current["hardStopEnabled"].as_bool().unwrap_or(false));
    let warn_exact_failure = validate_hermes_i64(
        if form.get("warnExactFailure").is_some() {
            form_i64(form, "warnExactFailure")
        } else {
            Some(current["warnExactFailure"].as_i64().unwrap_or(2))
        },
        "tool_loop_guardrails.warn_after.exact_failure",
        2,
        1,
        100,
    )?;
    let warn_same_tool_failure = validate_hermes_i64(
        if form.get("warnSameToolFailure").is_some() {
            form_i64(form, "warnSameToolFailure")
        } else {
            Some(current["warnSameToolFailure"].as_i64().unwrap_or(3))
        },
        "tool_loop_guardrails.warn_after.same_tool_failure",
        3,
        1,
        100,
    )?;
    let warn_no_progress = validate_hermes_i64(
        if form.get("warnNoProgress").is_some() {
            form_i64(form, "warnNoProgress")
        } else {
            Some(current["warnNoProgress"].as_i64().unwrap_or(2))
        },
        "tool_loop_guardrails.warn_after.idempotent_no_progress",
        2,
        1,
        100,
    )?;
    let hard_stop_exact_failure = validate_hermes_i64(
        if form.get("hardStopExactFailure").is_some() {
            form_i64(form, "hardStopExactFailure")
        } else {
            Some(current["hardStopExactFailure"].as_i64().unwrap_or(5))
        },
        "tool_loop_guardrails.hard_stop_after.exact_failure",
        5,
        1,
        100,
    )?;
    let hard_stop_same_tool_failure = validate_hermes_i64(
        if form.get("hardStopSameToolFailure").is_some() {
            form_i64(form, "hardStopSameToolFailure")
        } else {
            Some(current["hardStopSameToolFailure"].as_i64().unwrap_or(8))
        },
        "tool_loop_guardrails.hard_stop_after.same_tool_failure",
        8,
        1,
        100,
    )?;
    let hard_stop_no_progress = validate_hermes_i64(
        if form.get("hardStopNoProgress").is_some() {
            form_i64(form, "hardStopNoProgress")
        } else {
            Some(current["hardStopNoProgress"].as_i64().unwrap_or(5))
        },
        "tool_loop_guardrails.hard_stop_after.idempotent_no_progress",
        5,
        1,
        100,
    )?;

    let root = ensure_yaml_object(config)?;
    let guardrails = yaml_child_object(root, "tool_loop_guardrails")?;
    guardrails.insert(yaml_key("warnings_enabled"), serde_yaml::Value::Bool(warnings_enabled));
    guardrails.insert(yaml_key("hard_stop_enabled"), serde_yaml::Value::Bool(hard_stop_enabled));
    let warn_after = yaml_child_object(guardrails, "warn_after")?;
    warn_after.insert(yaml_key("exact_failure"), serde_yaml::Value::Number(warn_exact_failure.into()));
    warn_after.insert(yaml_key("same_tool_failure"), serde_yaml::Value::Number(warn_same_tool_failure.into()));
    warn_after.insert(yaml_key("idempotent_no_progress"), serde_yaml::Value::Number(warn_no_progress.into()));
    let hard_stop_after = yaml_child_object(guardrails, "hard_stop_after")?;
    hard_stop_after.insert(yaml_key("exact_failure"), serde_yaml::Value::Number(hard_stop_exact_failure.into()));
    hard_stop_after.insert(
        yaml_key("same_tool_failure"),
        serde_yaml::Value::Number(hard_stop_same_tool_failure.into()),
    );
    hard_stop_after.insert(
        yaml_key("idempotent_no_progress"),
        serde_yaml::Value::Number(hard_stop_no_progress.into()),
    );
    Ok(())
}

include!("performance_routing_config/memory_skills_config.rs");