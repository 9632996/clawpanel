fn build_hermes_memory_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let memory = root.and_then(|map| yaml_get_mapping(map, "memory"));
    let memory_enabled = memory.and_then(|map| yaml_bool_field(map, "memory_enabled")).unwrap_or(true);
    let user_profile_enabled = memory
        .and_then(|map| yaml_bool_field(map, "user_profile_enabled"))
        .unwrap_or(true);
    let memory_char_limit = memory
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "memory_char_limit"), 2200, 100, 200000))
        .unwrap_or(2200);
    let user_char_limit = memory
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "user_char_limit"), 1375, 100, 200000))
        .unwrap_or(1375);
    let nudge_interval = memory
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "nudge_interval"), 10, 0, 1000))
        .unwrap_or(10);
    let flush_min_turns = memory
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "flush_min_turns"), 6, 0, 1000))
        .unwrap_or(6);

    crate::jv!({
        "memoryEnabled": memory_enabled,
        "userProfileEnabled": user_profile_enabled,
        "memoryCharLimit": memory_char_limit,
        "userCharLimit": user_char_limit,
        "nudgeInterval": nudge_interval,
        "flushMinTurns": flush_min_turns,
    })
}

fn merge_hermes_memory_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_memory_config_values(config);
    let memory_enabled = form_bool(form, "memoryEnabled").unwrap_or_else(|| current["memoryEnabled"].as_bool().unwrap_or(true));
    let user_profile_enabled =
        form_bool(form, "userProfileEnabled").unwrap_or_else(|| current["userProfileEnabled"].as_bool().unwrap_or(true));
    let memory_char_limit = validate_hermes_i64(
        if form.get("memoryCharLimit").is_some() {
            form_i64(form, "memoryCharLimit")
        } else {
            Some(current["memoryCharLimit"].as_i64().unwrap_or(2200))
        },
        "memory.memory_char_limit",
        2200,
        100,
        200000,
    )?;
    let user_char_limit = validate_hermes_i64(
        if form.get("userCharLimit").is_some() {
            form_i64(form, "userCharLimit")
        } else {
            Some(current["userCharLimit"].as_i64().unwrap_or(1375))
        },
        "memory.user_char_limit",
        1375,
        100,
        200000,
    )?;
    let nudge_interval = validate_hermes_i64(
        if form.get("nudgeInterval").is_some() {
            form_i64(form, "nudgeInterval")
        } else {
            Some(current["nudgeInterval"].as_i64().unwrap_or(10))
        },
        "memory.nudge_interval",
        10,
        0,
        1000,
    )?;
    let flush_min_turns = validate_hermes_i64(
        if form.get("flushMinTurns").is_some() {
            form_i64(form, "flushMinTurns")
        } else {
            Some(current["flushMinTurns"].as_i64().unwrap_or(6))
        },
        "memory.flush_min_turns",
        6,
        0,
        1000,
    )?;

    let root = ensure_yaml_object(config)?;
    let memory = yaml_child_object(root, "memory")?;
    memory.insert(yaml_key("memory_enabled"), serde_yaml::Value::Bool(memory_enabled));
    memory.insert(yaml_key("user_profile_enabled"), serde_yaml::Value::Bool(user_profile_enabled));
    memory.insert(yaml_key("memory_char_limit"), serde_yaml::Value::Number(memory_char_limit.into()));
    memory.insert(yaml_key("user_char_limit"), serde_yaml::Value::Number(user_char_limit.into()));
    memory.insert(yaml_key("nudge_interval"), serde_yaml::Value::Number(nudge_interval.into()));
    memory.insert(yaml_key("flush_min_turns"), serde_yaml::Value::Number(flush_min_turns.into()));
    Ok(())
}

fn normalize_hermes_multiline_list(raw: Option<String>) -> Vec<String> {
    raw.unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn build_hermes_skills_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let skills = root.and_then(|map| yaml_get_mapping(map, "skills"));
    let creation_nudge_interval = skills
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "creation_nudge_interval"), 15, 0, 10000))
        .unwrap_or(15);
    let external_dirs = skills
        .map(|map| yaml_string_sequence_field(map, "external_dirs").join("\n"))
        .unwrap_or_default();

    crate::jv!({
        "creationNudgeInterval": creation_nudge_interval,
        "externalDirs": external_dirs,
        "templateVars": skills.and_then(|map| yaml_bool_field(map, "template_vars")).unwrap_or(true),
        "inlineShell": skills.and_then(|map| yaml_bool_field(map, "inline_shell")).unwrap_or(false),
        "inlineShellTimeout": skills
            .map(|map| bounded_hermes_i64(yaml_i64_field(map, "inline_shell_timeout"), 10, 1, 86400))
            .unwrap_or(10),
        "guardAgentCreated": skills.and_then(|map| yaml_bool_field(map, "guard_agent_created")).unwrap_or(false),
    })
}

fn merge_hermes_skills_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_skills_config_values(config);
    let creation_nudge_interval = validate_hermes_i64(
        if form.get("creationNudgeInterval").is_some() {
            form_i64(form, "creationNudgeInterval")
        } else {
            Some(current["creationNudgeInterval"].as_i64().unwrap_or(15))
        },
        "skills.creation_nudge_interval",
        15,
        0,
        10000,
    )?;
    let inline_shell_timeout = validate_hermes_i64(
        if form.get("inlineShellTimeout").is_some() {
            form_i64(form, "inlineShellTimeout")
        } else {
            Some(current["inlineShellTimeout"].as_i64().unwrap_or(10))
        },
        "skills.inline_shell_timeout",
        10,
        1,
        86400,
    )?;
    let external_dirs = normalize_hermes_multiline_list(
        form_string(form, "externalDirs").or_else(|| current["externalDirs"].as_str().map(ToString::to_string)),
    );

    let root = ensure_yaml_object(config)?;
    let skills = yaml_child_object(root, "skills")?;
    skills.insert(
        yaml_key("creation_nudge_interval"),
        serde_yaml::Value::Number(creation_nudge_interval.into()),
    );
    skills.insert(
        yaml_key("template_vars"),
        serde_yaml::Value::Bool(
            form_bool(form, "templateVars").unwrap_or_else(|| current["templateVars"].as_bool().unwrap_or(true)),
        ),
    );
    skills.insert(
        yaml_key("inline_shell"),
        serde_yaml::Value::Bool(
            form_bool(form, "inlineShell").unwrap_or_else(|| current["inlineShell"].as_bool().unwrap_or(false)),
        ),
    );
    skills.insert(yaml_key("inline_shell_timeout"), serde_yaml::Value::Number(inline_shell_timeout.into()));
    skills.insert(
        yaml_key("guard_agent_created"),
        serde_yaml::Value::Bool(
            form_bool(form, "guardAgentCreated").unwrap_or_else(|| current["guardAgentCreated"].as_bool().unwrap_or(false)),
        ),
    );
    if external_dirs.is_empty() {
        skills.remove(yaml_key("external_dirs"));
    } else {
        skills.insert(
            yaml_key("external_dirs"),
            serde_yaml::Value::Sequence(external_dirs.into_iter().map(serde_yaml::Value::String).collect()),
        );
    }
    Ok(())
}