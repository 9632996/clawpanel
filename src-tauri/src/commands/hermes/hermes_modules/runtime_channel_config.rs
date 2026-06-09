
fn build_hermes_terminal_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let terminal = root.and_then(|map| yaml_get_mapping(map, "terminal"));
    let terminal_string = |key: &str| {
        terminal
            .and_then(|map| yaml_string_field(map, key))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_default()
    };
    let terminal_backend = normalize_hermes_terminal_backend(terminal.and_then(|map| yaml_string_field(map, "backend")), false)
        .unwrap_or_else(|_| "local".to_string());
    let terminal_cwd = terminal
        .and_then(|map| yaml_string_field(map, "cwd"))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| ".".to_string());
    let terminal_timeout = terminal
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "timeout"), 180, 1, 86400))
        .unwrap_or(180);
    let terminal_lifetime_seconds = terminal
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "lifetime_seconds"), 300, 0, 86400))
        .unwrap_or(300);
    let terminal_shell_init_files = terminal
        .map(|map| yaml_string_sequence_field(map, "shell_init_files").join("\n"))
        .unwrap_or_default();
    let terminal_auto_source_bashrc = terminal
        .and_then(|map| yaml_bool_field(map, "auto_source_bashrc"))
        .unwrap_or(true);
    let terminal_persistent_shell = terminal
        .and_then(|map| yaml_bool_field(map, "persistent_shell"))
        .unwrap_or(true);
    let terminal_env_passthrough = terminal
        .map(|map| yaml_string_sequence_field(map, "env_passthrough").join("\n"))
        .unwrap_or_default();
    let terminal_docker_mount_cwd_to_workspace = terminal
        .and_then(|map| yaml_bool_field(map, "docker_mount_cwd_to_workspace"))
        .unwrap_or(false);
    let terminal_docker_run_as_host_user = terminal
        .and_then(|map| yaml_bool_field(map, "docker_run_as_host_user"))
        .unwrap_or(false);
    let terminal_docker_image = terminal_string("docker_image");
    let terminal_singularity_image = terminal_string("singularity_image");
    let terminal_modal_image = terminal_string("modal_image");
    let terminal_modal_mode =
        normalize_hermes_terminal_modal_mode(terminal.and_then(|map| yaml_string_field(map, "modal_mode")), false)
            .unwrap_or_else(|_| "auto".to_string());
    let terminal_vercel_runtime =
        normalize_hermes_terminal_vercel_runtime(terminal.and_then(|map| yaml_string_field(map, "vercel_runtime")), false)
            .unwrap_or_else(|_| "node24".to_string());
    let terminal_daytona_image = terminal_string("daytona_image");
    let terminal_docker_forward_env = terminal
        .map(|map| yaml_string_sequence_field(map, "docker_forward_env").join("\n"))
        .unwrap_or_default();
    let terminal_docker_env_json = yaml_docker_env_json_field(terminal, "docker_env");
    let terminal_docker_volumes = terminal
        .map(|map| yaml_string_sequence_field(map, "docker_volumes").join("\n"))
        .unwrap_or_default();
    let terminal_docker_extra_args = terminal
        .map(|map| yaml_string_sequence_field(map, "docker_extra_args").join("\n"))
        .unwrap_or_default();
    let terminal_ssh_host = terminal_string("ssh_host");
    let terminal_ssh_user = terminal_string("ssh_user");
    let terminal_ssh_port = terminal
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "ssh_port"), 22, 1, 65535))
        .unwrap_or(22);
    let terminal_ssh_key = terminal_string("ssh_key");
    let terminal_container_cpu = terminal
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "container_cpu"), 1, 1, 64))
        .unwrap_or(1);
    let terminal_container_memory = terminal
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "container_memory"), 5120, 128, 1048576))
        .unwrap_or(5120);
    let terminal_container_disk = terminal
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "container_disk"), 51200, 1024, 10485760))
        .unwrap_or(51200);
    let terminal_container_persistent = terminal
        .and_then(|map| yaml_bool_field(map, "container_persistent"))
        .unwrap_or(true);

    crate::jv!({
        "terminalBackend": terminal_backend,
        "terminalCwd": terminal_cwd,
        "terminalTimeout": terminal_timeout,
        "terminalLifetimeSeconds": terminal_lifetime_seconds,
        "terminalShellInitFiles": terminal_shell_init_files,
        "terminalAutoSourceBashrc": terminal_auto_source_bashrc,
        "terminalPersistentShell": terminal_persistent_shell,
        "terminalEnvPassthrough": terminal_env_passthrough,
        "terminalDockerMountCwdToWorkspace": terminal_docker_mount_cwd_to_workspace,
        "terminalDockerRunAsHostUser": terminal_docker_run_as_host_user,
        "terminalDockerImage": terminal_docker_image,
        "terminalSingularityImage": terminal_singularity_image,
        "terminalModalImage": terminal_modal_image,
        "terminalModalMode": terminal_modal_mode,
        "terminalVercelRuntime": terminal_vercel_runtime,
        "terminalDaytonaImage": terminal_daytona_image,
        "terminalDockerForwardEnv": terminal_docker_forward_env,
        "terminalDockerEnvJson": terminal_docker_env_json,
        "terminalDockerVolumes": terminal_docker_volumes,
        "terminalDockerExtraArgs": terminal_docker_extra_args,
        "terminalSshHost": terminal_ssh_host,
        "terminalSshUser": terminal_ssh_user,
        "terminalSshPort": terminal_ssh_port,
        "terminalSshKey": terminal_ssh_key,
        "terminalContainerCpu": terminal_container_cpu,
        "terminalContainerMemory": terminal_container_memory,
        "terminalContainerDisk": terminal_container_disk,
        "terminalContainerPersistent": terminal_container_persistent,
    })
}

fn merge_hermes_terminal_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_terminal_config_values(config);
    let terminal_backend = normalize_hermes_terminal_backend(
        if form.get("terminalBackend").is_some() {
            form_string(form, "terminalBackend")
        } else {
            current["terminalBackend"].as_str().map(ToString::to_string)
        },
        true,
    )?;
    let terminal_cwd = if form.get("terminalCwd").is_some() {
        form_string(form, "terminalCwd").unwrap_or_default().trim().to_string()
    } else {
        current["terminalCwd"].as_str().unwrap_or(".").to_string()
    };
    let terminal_cwd = if terminal_cwd.trim().is_empty() {
        ".".to_string()
    } else {
        terminal_cwd
    };
    let terminal_timeout = validate_hermes_i64(
        if form.get("terminalTimeout").is_some() {
            form_i64(form, "terminalTimeout")
        } else {
            Some(current["terminalTimeout"].as_i64().unwrap_or(180))
        },
        "terminal.timeout",
        180,
        1,
        86400,
    )?;
    let terminal_lifetime_seconds = validate_hermes_i64(
        if form.get("terminalLifetimeSeconds").is_some() {
            form_i64(form, "terminalLifetimeSeconds")
        } else {
            Some(current["terminalLifetimeSeconds"].as_i64().unwrap_or(300))
        },
        "terminal.lifetime_seconds",
        300,
        0,
        86400,
    )?;
    let terminal_shell_init_files = normalize_hermes_shell_init_file_list(
        form_string(form, "terminalShellInitFiles")
            .or_else(|| current["terminalShellInitFiles"].as_str().map(ToString::to_string)),
        "terminal.shell_init_files",
    )?;
    let terminal_auto_source_bashrc = form_bool(form, "terminalAutoSourceBashrc")
        .unwrap_or_else(|| current["terminalAutoSourceBashrc"].as_bool().unwrap_or(true));
    let terminal_persistent_shell = form_bool(form, "terminalPersistentShell")
        .unwrap_or_else(|| current["terminalPersistentShell"].as_bool().unwrap_or(true));
    let terminal_env_passthrough = normalize_hermes_env_name_list(
        form_string(form, "terminalEnvPassthrough")
            .or_else(|| current["terminalEnvPassthrough"].as_str().map(ToString::to_string)),
        "terminal.env_passthrough",
    )?;
    let terminal_docker_mount_cwd_to_workspace = form_bool(form, "terminalDockerMountCwdToWorkspace")
        .unwrap_or_else(|| current["terminalDockerMountCwdToWorkspace"].as_bool().unwrap_or(false));
    let terminal_docker_run_as_host_user = form_bool(form, "terminalDockerRunAsHostUser")
        .unwrap_or_else(|| current["terminalDockerRunAsHostUser"].as_bool().unwrap_or(false));
    let terminal_modal_mode = normalize_hermes_terminal_modal_mode(
        if form.get("terminalModalMode").is_some() {
            form_string(form, "terminalModalMode")
        } else {
            current["terminalModalMode"].as_str().map(ToString::to_string)
        },
        true,
    )?;
    let terminal_vercel_runtime = normalize_hermes_terminal_vercel_runtime(
        if form.get("terminalVercelRuntime").is_some() {
            form_string(form, "terminalVercelRuntime")
        } else {
            current["terminalVercelRuntime"].as_str().map(ToString::to_string)
        },
        true,
    )?;
    let terminal_docker_image = form_string(form, "terminalDockerImage")
        .or_else(|| current["terminalDockerImage"].as_str().map(ToString::to_string))
        .unwrap_or_default()
        .trim()
        .to_string();
    let terminal_singularity_image = form_string(form, "terminalSingularityImage")
        .or_else(|| current["terminalSingularityImage"].as_str().map(ToString::to_string))
        .unwrap_or_default()
        .trim()
        .to_string();
    let terminal_modal_image = form_string(form, "terminalModalImage")
        .or_else(|| current["terminalModalImage"].as_str().map(ToString::to_string))
        .unwrap_or_default()
        .trim()
        .to_string();
    let terminal_daytona_image = form_string(form, "terminalDaytonaImage")
        .or_else(|| current["terminalDaytonaImage"].as_str().map(ToString::to_string))
        .unwrap_or_default()
        .trim()
        .to_string();
    let terminal_docker_forward_env = normalize_hermes_env_name_list(
        form_string(form, "terminalDockerForwardEnv")
            .or_else(|| current["terminalDockerForwardEnv"].as_str().map(ToString::to_string)),
        "terminal.docker_forward_env",
    )?;
    let terminal_docker_env = normalize_hermes_docker_env_json(
        form_string(form, "terminalDockerEnvJson").or_else(|| current["terminalDockerEnvJson"].as_str().map(ToString::to_string)),
        "terminal.docker_env",
    )?;
    let terminal_docker_volumes = normalize_hermes_docker_volume_list(
        form_string(form, "terminalDockerVolumes").or_else(|| current["terminalDockerVolumes"].as_str().map(ToString::to_string)),
        "terminal.docker_volumes",
    )?;
    let terminal_docker_extra_args = normalize_hermes_docker_extra_args_list(
        form_string(form, "terminalDockerExtraArgs")
            .or_else(|| current["terminalDockerExtraArgs"].as_str().map(ToString::to_string)),
        "terminal.docker_extra_args",
    )?;
    let terminal_ssh_host = form_string(form, "terminalSshHost")
        .or_else(|| current["terminalSshHost"].as_str().map(ToString::to_string))
        .unwrap_or_default()
        .trim()
        .to_string();
    let terminal_ssh_user = form_string(form, "terminalSshUser")
        .or_else(|| current["terminalSshUser"].as_str().map(ToString::to_string))
        .unwrap_or_default()
        .trim()
        .to_string();
    let terminal_ssh_port = validate_hermes_i64(
        if form.get("terminalSshPort").is_some() {
            form_i64(form, "terminalSshPort")
        } else {
            Some(current["terminalSshPort"].as_i64().unwrap_or(22))
        },
        "terminal.ssh_port",
        22,
        1,
        65535,
    )?;
    let terminal_ssh_key = form_string(form, "terminalSshKey")
        .or_else(|| current["terminalSshKey"].as_str().map(ToString::to_string))
        .unwrap_or_default()
        .trim()
        .to_string();
    let terminal_container_cpu = validate_hermes_i64(
        if form.get("terminalContainerCpu").is_some() {
            form_i64(form, "terminalContainerCpu")
        } else {
            Some(current["terminalContainerCpu"].as_i64().unwrap_or(1))
        },
        "terminal.container_cpu",
        1,
        1,
        64,
    )?;
    let terminal_container_memory = validate_hermes_i64(
        if form.get("terminalContainerMemory").is_some() {
            form_i64(form, "terminalContainerMemory")
        } else {
            Some(current["terminalContainerMemory"].as_i64().unwrap_or(5120))
        },
        "terminal.container_memory",
        5120,
        128,
        1048576,
    )?;
    let terminal_container_disk = validate_hermes_i64(
        if form.get("terminalContainerDisk").is_some() {
            form_i64(form, "terminalContainerDisk")
        } else {
            Some(current["terminalContainerDisk"].as_i64().unwrap_or(51200))
        },
        "terminal.container_disk",
        51200,
        1024,
        10485760,
    )?;
    let terminal_container_persistent = form_bool(form, "terminalContainerPersistent")
        .unwrap_or_else(|| current["terminalContainerPersistent"].as_bool().unwrap_or(true));

    let root = ensure_yaml_object(config)?;
    let terminal = yaml_child_object(root, "terminal")?;
    terminal.insert(yaml_key("backend"), serde_yaml::Value::String(terminal_backend));
    terminal.insert(yaml_key("cwd"), serde_yaml::Value::String(terminal_cwd));
    terminal.insert(yaml_key("timeout"), serde_yaml::Value::Number(terminal_timeout.into()));
    terminal.insert(yaml_key("lifetime_seconds"), serde_yaml::Value::Number(terminal_lifetime_seconds.into()));
    if terminal_shell_init_files.is_empty() {
        terminal.remove(yaml_key("shell_init_files"));
    } else {
        terminal.insert(
            yaml_key("shell_init_files"),
            serde_yaml::Value::Sequence(terminal_shell_init_files.into_iter().map(serde_yaml::Value::String).collect()),
        );
    }
    terminal.insert(yaml_key("auto_source_bashrc"), serde_yaml::Value::Bool(terminal_auto_source_bashrc));
    terminal.insert(yaml_key("persistent_shell"), serde_yaml::Value::Bool(terminal_persistent_shell));
    if terminal_env_passthrough.is_empty() {
        terminal.remove(yaml_key("env_passthrough"));
    } else {
        terminal.insert(
            yaml_key("env_passthrough"),
            serde_yaml::Value::Sequence(terminal_env_passthrough.into_iter().map(serde_yaml::Value::String).collect()),
        );
    }
    terminal.insert(
        yaml_key("docker_mount_cwd_to_workspace"),
        serde_yaml::Value::Bool(terminal_docker_mount_cwd_to_workspace),
    );
    terminal.insert(
        yaml_key("docker_run_as_host_user"),
        serde_yaml::Value::Bool(terminal_docker_run_as_host_user),
    );
    set_optional_yaml_string(terminal, "docker_image", terminal_docker_image);
    set_optional_yaml_string(terminal, "singularity_image", terminal_singularity_image);
    set_optional_yaml_string(terminal, "modal_image", terminal_modal_image);
    terminal.insert(yaml_key("modal_mode"), serde_yaml::Value::String(terminal_modal_mode));
    terminal.insert(yaml_key("vercel_runtime"), serde_yaml::Value::String(terminal_vercel_runtime));
    set_optional_yaml_string(terminal, "daytona_image", terminal_daytona_image);
    if terminal_docker_forward_env.is_empty() {
        terminal.remove(yaml_key("docker_forward_env"));
    } else {
        terminal.insert(
            yaml_key("docker_forward_env"),
            serde_yaml::Value::Sequence(
                terminal_docker_forward_env
                    .into_iter()
                    .map(serde_yaml::Value::String)
                    .collect(),
            ),
        );
    }
    if terminal_docker_env.is_empty() {
        terminal.remove(yaml_key("docker_env"));
    } else {
        let mut docker_env = serde_yaml::Mapping::new();
        for (name, value) in terminal_docker_env {
            let value = value.as_str().unwrap_or_default().to_string();
            docker_env.insert(yaml_key(&name), serde_yaml::Value::String(value));
        }
        terminal.insert(yaml_key("docker_env"), serde_yaml::Value::Mapping(docker_env));
    }
    if terminal_docker_volumes.is_empty() {
        terminal.remove(yaml_key("docker_volumes"));
    } else {
        terminal.insert(
            yaml_key("docker_volumes"),
            serde_yaml::Value::Sequence(terminal_docker_volumes.into_iter().map(serde_yaml::Value::String).collect()),
        );
    }
    if terminal_docker_extra_args.is_empty() {
        terminal.remove(yaml_key("docker_extra_args"));
    } else {
        terminal.insert(
            yaml_key("docker_extra_args"),
            serde_yaml::Value::Sequence(
                terminal_docker_extra_args
                    .into_iter()
                    .map(serde_yaml::Value::String)
                    .collect(),
            ),
        );
    }
    set_optional_yaml_string(terminal, "ssh_host", terminal_ssh_host);
    set_optional_yaml_string(terminal, "ssh_user", terminal_ssh_user);
    terminal.insert(yaml_key("ssh_port"), serde_yaml::Value::Number(terminal_ssh_port.into()));
    set_optional_yaml_string(terminal, "ssh_key", terminal_ssh_key);
    terminal.insert(yaml_key("container_cpu"), serde_yaml::Value::Number(terminal_container_cpu.into()));
    terminal.insert(yaml_key("container_memory"), serde_yaml::Value::Number(terminal_container_memory.into()));
    terminal.insert(yaml_key("container_disk"), serde_yaml::Value::Number(terminal_container_disk.into()));
    terminal.insert(yaml_key("container_persistent"), serde_yaml::Value::Bool(terminal_container_persistent));
    Ok(())
}

fn build_hermes_session_runtime_config_values(config: &serde_yaml::Value) -> Value {
    let root = config.as_mapping();
    let session_reset = root.and_then(|map| yaml_get_mapping(map, "session_reset"));
    let mode = session_reset
        .and_then(|map| yaml_string_field(map, "mode"))
        .map(|value| value.trim().to_string())
        .filter(|value| matches!(value.as_str(), "both" | "idle" | "daily" | "none"))
        .unwrap_or_else(|| "both".to_string());
    let idle_minutes = session_reset
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "idle_minutes"), 1440, 1, 525600))
        .unwrap_or(1440);
    let at_hour = session_reset
        .map(|map| bounded_hermes_i64(yaml_i64_field(map, "at_hour"), 4, 0, 23))
        .unwrap_or(4);
    let group_sessions_per_user = root
        .and_then(|map| yaml_bool_field(map, "group_sessions_per_user"))
        .unwrap_or(true);
    let thread_sessions_per_user = root
        .and_then(|map| yaml_bool_field(map, "thread_sessions_per_user"))
        .unwrap_or(false);
    let worktree_enabled = root.and_then(|map| yaml_bool_field(map, "worktree")).unwrap_or(false);

    crate::jv!({
        "sessionResetMode": mode,
        "idleMinutes": idle_minutes,
        "atHour": at_hour,
        "groupSessionsPerUser": group_sessions_per_user,
        "threadSessionsPerUser": thread_sessions_per_user,
        "worktreeEnabled": worktree_enabled,
    })
}

fn merge_hermes_session_runtime_config(config: &mut serde_yaml::Value, form: &Value) -> Result<(), String> {
    let current = build_hermes_session_runtime_config_values(config);
    let current_mode = current["sessionResetMode"].as_str().unwrap_or("both");
    let mode = if form.get("sessionResetMode").is_some() {
        form_string(form, "sessionResetMode")
            .map(|value| value.trim().to_string())
            .filter(|value| matches!(value.as_str(), "both" | "idle" | "daily" | "none"))
            .ok_or_else(|| "session_reset.mode 必须是 both、idle、daily 或 none".to_string())?
    } else {
        current_mode.to_string()
    };
    let current_idle_minutes = current["idleMinutes"].as_i64().unwrap_or(1440);
    let idle_minutes = validate_hermes_i64(
        if form.get("idleMinutes").is_some() {
            form_i64(form, "idleMinutes")
        } else {
            Some(current_idle_minutes)
        },
        "idle_minutes",
        1440,
        1,
        525600,
    )?;
    let current_at_hour = current["atHour"].as_i64().unwrap_or(4);
    let at_hour = validate_hermes_i64(
        if form.get("atHour").is_some() {
            form_i64(form, "atHour")
        } else {
            Some(current_at_hour)
        },
        "at_hour",
        4,
        0,
        23,
    )?;
    let group_sessions_per_user =
        form_bool(form, "groupSessionsPerUser").unwrap_or_else(|| current["groupSessionsPerUser"].as_bool().unwrap_or(true));
    let thread_sessions_per_user =
        form_bool(form, "threadSessionsPerUser").unwrap_or_else(|| current["threadSessionsPerUser"].as_bool().unwrap_or(false));
    let worktree_enabled =
        form_bool(form, "worktreeEnabled").unwrap_or_else(|| current["worktreeEnabled"].as_bool().unwrap_or(false));

    let root = ensure_yaml_object(config)?;
    let session_reset = yaml_child_object(root, "session_reset")?;
    session_reset.insert(yaml_key("mode"), serde_yaml::Value::String(mode));
    session_reset.insert(yaml_key("idle_minutes"), serde_yaml::Value::Number(idle_minutes.into()));
    session_reset.insert(yaml_key("at_hour"), serde_yaml::Value::Number(at_hour.into()));
    root.insert(yaml_key("group_sessions_per_user"), serde_yaml::Value::Bool(group_sessions_per_user));
    root.insert(yaml_key("thread_sessions_per_user"), serde_yaml::Value::Bool(thread_sessions_per_user));
    root.insert(yaml_key("worktree"), serde_yaml::Value::Bool(worktree_enabled));
    Ok(())
}

fn merge_hermes_channel_config(config: &mut serde_yaml::Value, platform: &str, form: &Value) -> Result<(), String> {
    let platform = normalize_hermes_channel_platform(platform).ok_or_else(|| format!("不支持的 Hermes 渠道: {platform}"))?;
    let root = ensure_yaml_object(config)?;
    merge_hermes_channel_display_config(root, platform, form)?;
    let platforms = yaml_child_object(root, "platforms")?;
    let entry = yaml_child_object(platforms, platform)?;

    entry.insert(yaml_key("enabled"), serde_yaml::Value::Bool(form_bool(form, "enabled").unwrap_or(false)));

    match platform {
        "telegram" => {
            delete_yaml_key(entry, "token");
            set_extra_string_if_present(
                entry,
                "reply_to_mode",
                Some(normalize_hermes_telegram_reply_to_mode(form_string(form, "replyToMode"), true)?),
            );
            if let Some(value) = form_bool(form, "guestMode") {
                set_extra_bool(entry, "guest_mode", value);
            }
            if let Some(value) = form_bool(form, "disableLinkPreviews") {
                set_extra_bool(entry, "disable_link_previews", value);
            }
        }
        "discord" => {
            delete_yaml_key(entry, "token");
            for (form_key_name, extra_key_name) in [
                ("freeResponseChannels", "free_response_channels"),
                ("allowedChannels", "allowed_channels"),
                ("ignoredChannels", "ignored_channels"),
                ("noThreadChannels", "no_thread_channels"),
            ] {
                if let Some(values) = form_string_array(form, form_key_name) {
                    set_extra_string_array(entry, extra_key_name, values);
                }
            }
            for (form_key_name, extra_key_name) in [
                ("autoThread", "auto_thread"),
                ("reactions", "reactions"),
                ("threadRequireMention", "thread_require_mention"),
                ("historyBackfill", "history_backfill"),
            ] {
                if let Some(value) = form_bool(form, form_key_name) {
                    set_extra_bool(entry, extra_key_name, value);
                }
            }
            set_extra_string_if_present(entry, "history_backfill_limit", form_string(form, "historyBackfillLimit"));
            set_extra_string_if_present(entry, "reply_to_mode", form_string(form, "replyToMode"));
        }
        "slack" => {
            delete_yaml_key(entry, "token");
            delete_extra_key(entry, "app_token");
            delete_extra_key(entry, "signing_secret");
            set_extra_string_if_present(
                entry,
                "webhook_path",
                Some(form_string_or_default(form, "webhookPath", "/slack/events")),
            );
        }
        "feishu" => {
            delete_extra_key(entry, "app_id");
            delete_extra_key(entry, "app_secret");
            set_extra_string_if_present(entry, "domain", Some(form_string_or_default(form, "domain", "feishu")));
            set_extra_string_if_present(
                entry,
                "connection_mode",
                Some(form_string_or_default(form, "connectionMode", "websocket")),
            );
            set_extra_string_if_present(
                entry,
                "webhook_path",
                Some(form_string_or_default(form, "webhookPath", "/feishu/webhook")),
            );
            set_extra_string_if_present(
                entry,
                "reaction_notifications",
                Some(form_string_or_default(form, "reactionNotifications", "off")),
            );
            set_extra_bool(entry, "typing_indicator", form_bool(form, "typingIndicator").unwrap_or(true));
            set_extra_bool(entry, "resolve_sender_names", form_bool(form, "resolveSenderNames").unwrap_or(true));
        }
        "dingtalk" => {
            delete_extra_key(entry, "client_id");
            delete_extra_key(entry, "client_secret");
            delete_extra_key(entry, "allow_from");
            delete_extra_key(entry, "group_allow_from");
        }
        "teams" => {
            delete_extra_key(entry, "client_id");
            delete_extra_key(entry, "client_secret");
            delete_extra_key(entry, "tenant_id");
            set_extra_integer_if_present(entry, "port", form_i64(form, "port"));
            set_extra_string_if_present(entry, "service_url", form_string(form, "serviceUrl"));
            set_hermes_home_channel(entry, form);
        }
        "google_chat" => {
            set_extra_string_if_present(entry, "project_id", form_string(form, "projectId"));
            set_extra_string_if_present(entry, "subscription_name", form_string(form, "subscriptionName"));
            delete_extra_key(entry, "service_account_json");
            set_hermes_home_channel(entry, form);
        }
        "irc" => {
            set_extra_string_if_present(entry, "server", form_string(form, "server"));
            set_extra_integer_if_present(entry, "port", form_i64(form, "port"));
            set_extra_string_if_present(entry, "nickname", form_string(form, "nickname"));
            set_extra_string_if_present(entry, "channel", form_string(form, "channel"));
            if let Some(value) = form_bool(form, "useTls") {
                set_extra_bool(entry, "use_tls", value);
            }
            delete_extra_key(entry, "server_password");
            delete_extra_key(entry, "nickserv_password");
            set_hermes_home_channel(entry, form);
        }
        "line" => {
            delete_extra_key(entry, "channel_access_token");
            delete_extra_key(entry, "channel_secret");
            set_extra_integer_if_present(entry, "port", form_i64(form, "port"));
            set_extra_string_if_present(entry, "host", form_string(form, "host"));
            set_extra_string_if_present(entry, "public_url", form_string(form, "publicUrl"));
            if let Some(values) = form_string_array(form, "allowedGroups") {
                set_extra_string_array(entry, "allowed_groups", values);
            }
            if let Some(values) = form_string_array(form, "allowedRooms") {
                set_extra_string_array(entry, "allowed_rooms", values);
            }
            set_extra_string_if_present(entry, "slow_response_threshold", form_string(form, "slowResponseThreshold"));
            set_hermes_home_channel(entry, form);
        }
        "simplex" => {
            set_extra_string_if_present(entry, "ws_url", form_string(form, "wsUrl"));
            set_hermes_home_channel(entry, form);
        }
        _ => {}
    }

    if form.get("dmPolicy").is_some() {
        set_extra_string_if_present(entry, "dm_policy", Some(normalize_hermes_dm_policy(form_string(form, "dmPolicy"))));
    }
    if form.get("groupPolicy").is_some() {
        let group_policy = normalize_hermes_group_policy(form_string(form, "groupPolicy"));
        set_extra_string_if_present(entry, "group_policy", Some(group_policy.clone()));
        if platform == "feishu" {
            set_extra_string_if_present(entry, "default_group_policy", Some(group_policy));
        }
    }
    if let Some(value) = form_bool(form, "requireMention") {
        set_extra_bool(entry, "require_mention", value);
    }
    if let Some(values) = form_string_array(form, "allowFrom") {
        let key = if ["dingtalk", "irc", "line", "simplex"].contains(&platform) {
            "allowed_users"
        } else {
            "allow_from"
        };
        set_extra_string_array(entry, key, values);
    }
    if let Some(values) = form_string_array(form, "groupAllowFrom") {
        let key = if platform == "dingtalk" {
            "allowed_chats"
        } else {
            "group_allow_from"
        };
        set_extra_string_array(entry, key, values);
    }

    Ok(())
}

fn read_hermes_channel_yaml_config() -> Result<(PathBuf, bool, serde_yaml::Value), String> {
    let config_path = hermes_home().join("config.yaml");
    if !config_path.exists() {
        return Ok((config_path, false, serde_yaml::Value::Mapping(serde_yaml::Mapping::new())));
    }
    let raw = std::fs::read_to_string(&config_path).map_err(|e| format!("读取 config.yaml 失败: {e}"))?;
    let config = if raw.trim().is_empty() {
        serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
    } else {
        serde_yaml::from_str(&raw).map_err(|e| format!("解析 config.yaml 失败: {e}"))?
    };
    Ok((config_path, true, config))
}

fn write_hermes_yaml_config(path: &PathBuf, config: &serde_yaml::Value) -> Result<String, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建 Hermes 配置目录失败: {e}"))?;
    }
    let mut backup_path = String::new();
    if path.exists() {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let backup = path.with_extension(format!("yaml.bak-{ts}"));
        if std::fs::copy(path, &backup).is_ok() {
            backup_path = backup.to_string_lossy().to_string();
        }
    }
    let yaml = serde_yaml::to_string(config).map_err(|e| format!("序列化 config.yaml 失败: {e}"))?;
    std::fs::write(path, yaml).map_err(|e| format!("写入 config.yaml 失败: {e}"))?;
    Ok(backup_path)
}
