async fn install_via_uv_tool(app: &tauri::AppHandle, uv_path: &str, extras: &[String]) -> Result<(), String> {
    let _ = app.emit("hermes-install-log", "📦 通过 uv tool install 安装 Hermes Agent...");
    let _ = app.emit("hermes-install-progress", 25u32);

    // 构造安装规格
    let pkg = if extras.is_empty() {
        format!("hermes-agent @ {}", HERMES_GIT_URL)
    } else {
        format!("hermes-agent[{}] @ {}", extras.join(","), HERMES_GIT_URL)
    };

    let mut cmd = tokio::process::Command::new(uv_path);
    cmd.args(["tool", "install", "--force", &pkg, "--python", "3.11"]);
    append_hermes_runtime_extras(&mut cmd);
    apply_hermes_runtime_env_tokio(&mut cmd);

    // 配置 PyPI 镜像（extras 的依赖仍从 PyPI 下载）
    if let Some(mirror) = pypi_mirror_url() {
        cmd.args(["--index-url", &mirror]);
    }

    // 代理
    super::apply_proxy_env_tokio(&mut cmd);
    cmd.env("PATH", hermes_enhanced_path());
    // uv 需要 git 来克隆仓库
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    // 用户配置了 Git 镜像（如 ghproxy）→ 进程级注入 insteadOf 重写
    apply_git_mirror_env(&mut cmd);

    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    // 捕获输出
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let _ = app.emit(
        "hermes-install-log",
        format!("uv tool install hermes-agent --python 3.11 {}", hermes_runtime_extras_log_segment()),
    );

    let child = cmd.spawn().map_err(|e| format!("启动安装进程失败: {e}"))?;
    let output = child.wait_with_output().await.map_err(|e| format!("等待安装进程失败: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // 逐行输出日志
    for line in stdout.lines().chain(stderr.lines()) {
        if !line.trim().is_empty() {
            let _ = app.emit("hermes-install-log", sanitize_hermes_install_output(line.trim()));
        }
    }

    if output.status.success() {
        let _ = app.emit("hermes-install-log", "✓ uv tool install 完成");
        // 更新 shell PATH
        let mut update_cmd = tokio::process::Command::new(uv_path);
        update_cmd.args(["tool", "update-shell"]);
        apply_hermes_runtime_env_tokio(&mut update_cmd);
        #[cfg(target_os = "windows")]
        update_cmd.creation_flags(CREATE_NO_WINDOW);
        let _ = update_cmd.output().await;
        Ok(())
    } else {
        let cleaned = sanitize_hermes_install_output(stderr.trim());
        // 命中 git/network 失败 → 在日志流尾部追加诊断 + 给最终错误消息加上提示
        if let Some(hint) = diagnose_install_network_error(&cleaned) {
            let _ = app.emit("hermes-install-log", &hint);
            return Err(format!("安装失败 (exit {}): {}\n\n{}", output.status.code().unwrap_or(-1), cleaned, hint));
        }
        Err(format!("安装失败 (exit {}): {}", output.status.code().unwrap_or(-1), cleaned))
    }
}

/// 通过 uv pip install 安装到 venv（备选方式）
async fn install_via_uv_pip(app: &tauri::AppHandle, uv_path: &str, extras: &[String]) -> Result<(), String> {
    let _ = app.emit("hermes-install-log", "📦 通过 uv venv + pip install 安装...");
    let _ = app.emit("hermes-install-progress", 25u32);

    let venv_dir = hermes_venv_dir();
    let venv_str = venv_dir.to_string_lossy().to_string();

    // 创建 venv
    let _ = app.emit("hermes-install-log", format!("> uv venv {venv_str} --python 3.11"));
    let mut venv_cmd = tokio::process::Command::new(uv_path);
    venv_cmd.args(["venv", &venv_str, "--python", "3.11"]);
    apply_hermes_runtime_env_tokio(&mut venv_cmd);
    super::apply_proxy_env_tokio(&mut venv_cmd);
    #[cfg(target_os = "windows")]
    venv_cmd.creation_flags(CREATE_NO_WINDOW);
    let venv_out = venv_cmd.output().await.map_err(|e| format!("创建 venv 失败: {e}"))?;
    if !venv_out.status.success() {
        let stderr = String::from_utf8_lossy(&venv_out.stderr);
        return Err(format!("创建 venv 失败: {stderr}"));
    }
    let _ = app.emit("hermes-install-log", "✓ Python 虚拟环境创建完成");
    let _ = app.emit("hermes-install-progress", 40u32);

    // pip install
    let pkg = if extras.is_empty() {
        format!("hermes-agent @ {}", HERMES_GIT_URL)
    } else {
        format!("hermes-agent[{}] @ {}", extras.join(","), HERMES_GIT_URL)
    };
    let _ = app.emit("hermes-install-log", "> uv pip install hermes-agent");

    let mut pip_cmd = tokio::process::Command::new(uv_path);
    pip_cmd.args(["pip", "install", &pkg]);
    apply_hermes_runtime_env_tokio(&mut pip_cmd);
    pip_cmd.env("GIT_TERMINAL_PROMPT", "0");
    pip_cmd.env("VIRTUAL_ENV", &venv_str);
    if let Some(mirror) = pypi_mirror_url() {
        pip_cmd.args(["--index-url", &mirror]);
    }
    apply_git_mirror_env(&mut pip_cmd);
    super::apply_proxy_env_tokio(&mut pip_cmd);
    #[cfg(target_os = "windows")]
    pip_cmd.creation_flags(CREATE_NO_WINDOW);

    let pip_out = pip_cmd.output().await.map_err(|e| format!("pip install 失败: {e}"))?;

    let stdout = String::from_utf8_lossy(&pip_out.stdout);
    let stderr = String::from_utf8_lossy(&pip_out.stderr);
    for line in stdout.lines().chain(stderr.lines()) {
        if !line.trim().is_empty() {
            let _ = app.emit("hermes-install-log", sanitize_hermes_install_output(line.trim()));
        }
    }

    if !pip_out.status.success() {
        return Err(format!("pip install 失败: {}", sanitize_hermes_install_output(stderr.trim())));
    }

    let _ = app.emit("hermes-install-log", "✓ pip install 完成");

    // 创建全局命令链接
    #[cfg(not(target_os = "windows"))]
    {
        let hermes_bin = venv_dir.join("bin").join("hermes");
        let link_dir = home.join(".local").join("bin");
        let _ = std::fs::create_dir_all(&link_dir);
        let link_path = link_dir.join("hermes");
        let _ = std::fs::remove_file(&link_path);
        if let Err(e) = std::os::unix::fs::symlink(&hermes_bin, &link_path) {
            let _ = app.emit(
                "hermes-install-log",
                format!("⚠️ 创建全局链接失败: {e}（hermes 仍可通过 {hermes_bin:?} 使用）"),
            );
        } else {
            let _ = app.emit("hermes-install-log", format!("✓ 全局链接: {link_path:?}"));
        }
    }
    #[cfg(target_os = "windows")]
    {
        // Windows: 将 venv\Scripts 加入用户 PATH（通过注册表）
        let scripts_dir = venv_dir.join("Scripts");
        let _ = app.emit("hermes-install-log", format!("提示：请将 {} 加入系统 PATH", scripts_dir.display()));
    }

    Ok(())
}

/// 获取 PyPI 镜像 URL（如果配置了的话）
fn pypi_mirror_url() -> Option<String> {
    super::read_panel_config_value()
        .and_then(|v| v.get("pypiMirror")?.as_str().map(String::from))
        .filter(|s| !s.trim().is_empty())
}

// ---------------------------------------------------------------------------
// configure_hermes — 写入配置
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn configure_hermes(
    provider: String,
    api_key: String,
    model: Option<String>,
    base_url: Option<String>,
) -> Result<String, String> {
    let home = hermes_home();
    std::fs::create_dir_all(&home).map_err(|e| format!("创建配置目录失败: {e}"))?;

    // 创建子目录
    for dir in &[
        "cron",
        "sessions",
        "logs",
        "memories",
        "skills",
        "pairing",
        "hooks",
        "image_cache",
        "audio_cache",
    ] {
        let _ = std::fs::create_dir_all(home.join(dir));
    }

    // ---- Provider-aware key routing ----
    // ClawPanel 根据内置 provider registry 决定 .env key 名和
    // config.yaml 的 model.provider 字段。
    use super::hermes_providers;

    let requested_provider = provider.trim().to_string();
    let provider = normalize_hermes_provider_for_base_url(&requested_provider, base_url.as_deref());
    let credential_provider = if requested_provider.eq_ignore_ascii_case("aizuopin") {
        "aizuopin"
    } else {
        provider.as_str()
    };
    let pcfg = hermes_providers::get_provider(credential_provider).or_else(|| hermes_providers::get_provider(&provider));

    // 模型标识：优先使用调用方传入，否则用 provider 的首个已知模型；
    // aggregator 没有默认模型，要求调用方显式提供。
    let model_str = model.unwrap_or_else(|| pcfg.and_then(|p| p.models.first().map(|s| s.to_string())).unwrap_or_default());
    if model_str.is_empty() {
        return Err(format!("Provider '{provider}' has no default model; please pass an explicit model name"));
    }

    // ---- 写入 config.yaml（合并模式：保留用户自定义的 hooks/skills/cron 等） ----
    let config_path = home.join("config.yaml");
    let base_url_line = match base_url.as_ref() {
        Some(url) if !url.trim().is_empty() => format!("  base_url: {}\n", url.trim()),
        _ => String::new(),
    };
    // Provider 字段用于稳定选择凭证来源。
    // `custom` 也需要显式写入，避免自定义端点被默认路由接管。
    let provider_line = if provider.is_empty() {
        String::new()
    } else {
        format!("  provider: {provider}\n")
    };
    let api_key_line =
        if !api_key.trim().is_empty() && (provider == "custom" || requested_provider.eq_ignore_ascii_case("aizuopin")) {
            format!("  api_key: {}\n", api_key.trim())
        } else {
            String::new()
        };

    let config_content = if config_path.exists() {
        // 读取现有配置，只更新 model 区块，保留其余内容
        let existing = std::fs::read_to_string(&config_path).unwrap_or_default();
        merge_hermes_config_yaml(&existing, &model_str, &base_url_line, &provider_line, &api_key_line)
    } else {
        // 首次创建：生成完整的基线配置
        format!(
            r#"# Hermes Agent configuration (managed by Zhizhua Workbench)
model:
  default: {model_str}
{provider_line}{base_url_line}{api_key_line}platform_toolsets:
  api_server:
    - hermes-api-server
terminal:
  backend: local
platforms:
  api_server:
    enabled: true
"#
        )
    };
    std::fs::write(&config_path, &config_content).map_err(|e| format!("写入 config.yaml 失败: {e}"))?;

    // ---- 写入 .env（合并模式：保留用户自定义的环境变量如 TAVILY_API_KEY 等） ----
    // 根据 provider 选择正确的 env var；OAuth/external_process 类没有 api_key_env_vars，
    // 此时跳过写 key（CLI 登录后 Hermes 会自行管理 auth.json）。
    let key_env = hermes_providers::primary_api_key_env(credential_provider);
    let url_env =
        hermes_providers::primary_base_url_env(credential_provider).or_else(|| hermes_providers::primary_base_url_env(&provider));

    // ClawPanel 管理的 key 列表：包含所有 provider 的 api_key_env_vars + base_url_env_vars
    // + ClawPanel 特定的两个 key。换 provider 时这些会被重写或清除。
    let managed_keys_owned = hermes_providers::all_managed_env_keys();
    let managed_keys: Vec<&str> = managed_keys_owned.to_vec();

    let mut new_pairs: Vec<(String, String)> = vec![
        ("GATEWAY_ALLOW_ALL_USERS".into(), "true".into()),
        ("API_SERVER_KEY".into(), "clawpanel-local".into()),
    ];

    if let Some(env) = key_env {
        if !api_key.trim().is_empty() {
            new_pairs.push((env.into(), api_key.trim().into()));
            if credential_provider == "aizuopin" {
                for alias in ["OPENAI_API_KEY", "CUSTOM_API_KEY"] {
                    if env != alias {
                        new_pairs.push((alias.into(), api_key.trim().into()));
                    }
                }
            } else if provider == "custom" && env != "CUSTOM_API_KEY" {
                new_pairs.push(("CUSTOM_API_KEY".into(), api_key.trim().into()));
            }
        }
    } else if !api_key.trim().is_empty() {
        // OAuth provider 传了 api_key —— 记日志，不落盘
        eprintln!("[configure_hermes] Provider '{provider}' uses OAuth; ignoring provided api_key");
    }

    if let (Some(env), Some(url)) = (url_env, base_url.as_ref()) {
        let u = url.trim();
        if !u.is_empty() {
            new_pairs.push((env.into(), u.into()));
        }
    }

    let env_path = home.join(".env");
    let env_content = if env_path.exists() {
        let existing = std::fs::read_to_string(&env_path).unwrap_or_default();
        merge_env_file(&existing, &managed_keys, &new_pairs)
    } else {
        new_pairs
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    };
    std::fs::write(&env_path, &env_content).map_err(|e| format!("写入 .env 失败: {e}"))?;

    // Unix: 设置 .env 文件权限为 600
    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&env_path, std::fs::Permissions::from_mode(0o600));
    }

    Ok("配置已保存".into())
}

// ---------------------------------------------------------------------------
// 配置合并帮助函数
// ---------------------------------------------------------------------------