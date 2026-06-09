use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn product_root() -> Result<PathBuf, String> {
    super::portable_product_root().ok_or_else(|| "未找到便携产品根目录".to_string())
}

fn codewhale_home(root: &Path) -> PathBuf {
    root.join("data").join("config").join("codewhale")
}

fn resolve_relative(root: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn codewhale_cli_path(root: &Path, panel: &Value) -> PathBuf {
    panel
        .get("codewhale")
        .and_then(|v| v.get("cliPath"))
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(|v| resolve_relative(root, v.trim()))
        .unwrap_or_else(|| {
            root.join("app")
                .join("engines")
                .join("codewhale")
                .join("bin")
                .join(if cfg!(windows) { "codewhale.exe" } else { "codewhale" })
        })
}

fn codewhale_tui_path(root: &Path, panel: &Value) -> PathBuf {
    let cli = codewhale_cli_path(root, panel);
    let dir = cli.parent().unwrap_or_else(|| Path::new("."));
    let name = if cfg!(windows) { "codewhale-tui.exe" } else { "codewhale-tui" };
    dir.join(name)
}

fn codewhale_env_key(panel: &Value) -> Option<String> {
    let section = panel.get("codewhale")?;
    let provider = section.get("provider").and_then(|v| v.as_str()).unwrap_or("deepseek");
    section
        .get("providers")
        .and_then(|v| v.get(provider))
        .and_then(|v| v.get("envKey"))
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())
}

fn codewhale_cli_provider(provider: &str) -> &str {
    if provider.eq_ignore_ascii_case("aizuopin") {
        "openai"
    } else {
        provider
    }
}

fn codewhale_cli_env_key(provider: &str) -> Option<&'static str> {
    match codewhale_cli_provider(provider) {
        "openai" => Some("OPENAI_API_KEY"),
        "deepseek" | "deepseek-cn" | "deepseek-china" | "deepseek_china" | "deepseekcn" => Some("DEEPSEEK_API_KEY"),
        "xiaomi-mimo" | "xiaomi_mimo" | "mimo" | "xiaomi" => Some("MIMO_API_KEY"),
        "moonshot" => Some("MOONSHOT_API_KEY"),
        _ => None,
    }
}

fn read_model_credentials(root: &Path, env_key: &str) -> Option<String> {
    let path = root.join("data").join("config").join("model-credentials.env");
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        if key.trim() == env_key {
            return Some(value.trim().to_string());
        }
    }
    None
}

#[tauri::command]
pub fn codewhale_status() -> Result<Value, String> {
    let root = product_root()?;
    let panel = super::read_panel_config_value().unwrap_or_else(|| crate::jv!({}));
    let home = codewhale_home(&root);
    let cli = codewhale_cli_path(&root, &panel);
    let tui = codewhale_tui_path(&root, &panel);
    let config_path = home.join("config.toml");
    let skills = home.join("skills");
    let skill_count = std::fs::read_dir(&skills)
        .ok()
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.path().join("SKILL.md").is_file())
                .count()
        })
        .unwrap_or(0);
    let env_key = codewhale_env_key(&panel);
    let env_present = env_key.as_deref().is_some_and(|key| {
        std::env::var(key).ok().filter(|v| !v.trim().is_empty()).is_some() || read_model_credentials(&root, key).is_some()
    });

    // 获取版本信息
    let version = if cli.is_file() {
        Command::new(&cli)
            .arg("--version")
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                } else {
                    None
                }
            })
    } else {
        None
    };

    Ok(crate::jv!({
        "root": root.display().to_string(),
        "codewhaleHome": home.display().to_string(),
        "configPath": config_path.display().to_string(),
        "configExists": config_path.is_file(),
        "skillsPath": skills.display().to_string(),
        "skillCount": skill_count,
        "cliPath": cli.display().to_string(),
        "cliExists": cli.is_file(),
        "tuiPath": tui.display().to_string(),
        "tuiExists": tui.is_file(),
        "envKey": env_key,
        "envPresent": env_present,
        "version": version,
        "ready": cli.is_file() && tui.is_file(),
    }))
}

#[tauri::command]
pub fn codewhale_run_once(prompt: String) -> Result<Value, String> {
    let prompt = prompt.trim().to_string();
    if prompt.is_empty() {
        return Err("请输入问题".into());
    }
    if prompt.len() > 12000 {
        return Err("问题过长，请缩短后再试".into());
    }

    let root = product_root()?;
    let panel = super::read_panel_config_value().unwrap_or_else(|| crate::jv!({}));
    let cli = codewhale_cli_path(&root, &panel);
    if !cli.is_file() {
        return Err(format!("CodeWhale 可执行文件不存在: {}", cli.display()));
    }

    let home = codewhale_home(&root);
    let provider = panel
        .get("codewhale")
        .and_then(|v| v.get("provider"))
        .and_then(|v| v.as_str())
        .unwrap_or("deepseek");
    let cli_provider = codewhale_cli_provider(provider);
    let model = panel
        .get("codewhale")
        .and_then(|v| v.get("model"))
        .and_then(|v| v.as_str())
        .unwrap_or("deepseek-chat");

    let mut command = Command::new(&cli);
    command
        .arg("exec")
        .arg("--provider")
        .arg(cli_provider)
        .arg("--model")
        .arg(model)
        .arg("--approval-policy")
        .arg("never")
        .arg(prompt)
        .env("CODEWHALE_HOME", &home)
        .env("CODEWHALE_CONFIG_PATH", home.join("config.toml"))
        .current_dir(root.join("data").join("workspace").join("main"));

    // 注入 API Key
    if let Some(env_key) = codewhale_env_key(&panel) {
        let existing_value = std::env::var(&env_key)
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| read_model_credentials(&root, &env_key));
        if let Some(value) = existing_value {
            command.env(&env_key, &value);
            if let Some(cli_env_key) = codewhale_cli_env_key(provider) {
                if cli_env_key != env_key && std::env::var(cli_env_key).ok().filter(|v| !v.trim().is_empty()).is_none() {
                    command.env(cli_env_key, value);
                }
            }
        }
    }

    // 确保 TUI 二进制可被 dispatcher 找到
    let tui = codewhale_tui_path(&root, &panel);
    if let Some(tui_dir) = tui.parent() {
        let current_path = std::env::var("PATH").unwrap_or_default();
        let tui_dir_str = tui_dir.display().to_string();
        if !current_path.contains(&tui_dir_str) {
            let new_path = if cfg!(windows) {
                format!("{};{}", tui_dir_str, current_path)
            } else {
                format!("{}:{}", tui_dir_str, current_path)
            };
            command.env("PATH", new_path);
        }
    }

    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);

    let mut child = command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动 CodeWhale 失败: {e}"))?;

    let deadline = Instant::now() + Duration::from_secs(180);
    loop {
        if let Some(_status) = child.try_wait().map_err(|e| format!("等待 CodeWhale 失败: {e}"))? {
            let output = child
                .wait_with_output()
                .map_err(|e| format!("读取 CodeWhale 输出失败: {e}"))?;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Ok(crate::jv!({
                "success": output.status.success(),
                "exitCode": output.status.code(),
                "stdout": stdout.chars().take(20000).collect::<String>(),
                "stderr": stderr.chars().take(8000).collect::<String>(),
            }));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("CodeWhale 执行超时 (180s)".into());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}
