use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Default)]
struct CodexRuntimeConfig {
    provider: Option<String>,
    model: Option<String>,
    base_url: Option<String>,
    env_key: Option<String>,
}

fn product_root() -> Result<PathBuf, String> {
    super::portable_product_root().ok_or_else(|| "未找到便携产品根目录".to_string())
}

fn codex_home(root: &Path) -> PathBuf {
    root.join("data").join("config").join("codex")
}

fn codex_package_root(root: &Path) -> PathBuf {
    root.join("app").join("engines").join("codex")
}

fn codex_path_env(root: &Path) -> Option<std::ffi::OsString> {
    let package_root = codex_package_root(root);
    let mut paths = vec![package_root.join("bin"), package_root.join("codex-path")];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).ok()
}

fn codex_workspace(root: &Path) -> PathBuf {
    root.join("data").join("workspace").join("main")
}

fn codex_logs_dir(root: &Path) -> PathBuf {
    root.join("data").join("logs")
}

fn resolve_relative(root: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn codex_cli_path(root: &Path, panel: &Value) -> PathBuf {
    let configured = panel
        .get("codex")
        .and_then(|v| v.get("cliPath"))
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(|v| resolve_relative(root, v.trim()));

    if let Some(path) = configured {
        #[cfg(target_os = "windows")]
        {
            if !path.is_file() && path.extension().is_none() {
                let exe_path = path.with_extension("exe");
                if exe_path.is_file() {
                    return exe_path;
                }
            }
        }
        return path;
    }

    root.join("app")
        .join("engines")
        .join("codex")
        .join("bin")
        .join(if cfg!(windows) { "codex.exe" } else { "codex" })
}

fn non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn panel_codex_env_key(panel: &Value) -> Option<String> {
    let codex = panel.get("codex")?;
    let provider = codex.get("provider").and_then(|v| v.as_str()).unwrap_or("deepseek");
    codex
        .get("providers")
        .and_then(|v| v.get(provider))
        .and_then(|v| v.get("envKey"))
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())
}

fn read_codex_toml_config(root: &Path) -> CodexRuntimeConfig {
    let path = codex_home(root).join("config.toml");
    let Ok(content) = std::fs::read_to_string(path) else {
        return CodexRuntimeConfig::default();
    };
    let Ok(value) = content.parse::<toml::Value>() else {
        return CodexRuntimeConfig::default();
    };

    let provider = non_empty(value.get("model_provider").and_then(|v| v.as_str()));
    let model = non_empty(value.get("model").and_then(|v| v.as_str()));
    let provider_table = provider
        .as_deref()
        .and_then(|provider| value.get("model_providers").and_then(|v| v.get(provider)));

    CodexRuntimeConfig {
        provider,
        model,
        base_url: provider_table
            .and_then(|v| v.get("base_url"))
            .and_then(|v| v.as_str())
            .and_then(|v| non_empty(Some(v))),
        env_key: provider_table
            .and_then(|v| v.get("env_key"))
            .and_then(|v| v.as_str())
            .and_then(|v| non_empty(Some(v))),
    }
}

fn read_codex_runtime_config(root: &Path, panel: &Value) -> CodexRuntimeConfig {
    let from_toml = read_codex_toml_config(root);
    CodexRuntimeConfig {
        provider: from_toml.provider.or_else(|| {
            panel
                .get("codex")
                .and_then(|v| v.get("provider"))
                .and_then(|v| v.as_str())
                .and_then(|v| non_empty(Some(v)))
        }),
        model: from_toml.model.or_else(|| {
            panel
                .get("codex")
                .and_then(|v| v.get("model"))
                .and_then(|v| v.as_str())
                .and_then(|v| non_empty(Some(v)))
        }),
        base_url: from_toml.base_url.or_else(|| {
            panel
                .get("codex")
                .and_then(|v| v.get("baseUrl"))
                .and_then(|v| v.as_str())
                .and_then(|v| non_empty(Some(v)))
        }),
        env_key: from_toml.env_key.or_else(|| panel_codex_env_key(panel)),
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

fn inject_codex_env(command: &mut Command, root: &Path, runtime: &CodexRuntimeConfig) {
    let home = codex_home(root);
    command
        .env("CODEX_HOME", &home)
        .env("CODEX_MANAGED_PACKAGE_ROOT", codex_package_root(root))
        .env_remove("CODEX_MANAGED_BY_NPM")
        .env_remove("CODEX_MANAGED_BY_BUN");

    if let Some(path) = codex_path_env(root) {
        command.env("PATH", path);
    }

    if let Some(env_key) = runtime.env_key.as_deref() {
        if std::env::var(env_key).ok().filter(|v| !v.trim().is_empty()).is_none() {
            if let Some(value) = read_model_credentials(root, env_key) {
                command.env(env_key, value);
            }
        }
    }
}

#[tauri::command]
pub fn codex_status() -> Result<Value, String> {
    let root = product_root()?;
    let panel = super::read_panel_config_value().unwrap_or_else(|| crate::jv!({}));
    let home = codex_home(&root);
    let package_root = codex_package_root(&root);
    let cli = codex_cli_path(&root, &panel);
    let runtime = read_codex_runtime_config(&root, &panel);
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
    let env_present = runtime.env_key.as_deref().is_some_and(|key| {
        std::env::var(key).ok().filter(|v| !v.trim().is_empty()).is_some() || read_model_credentials(&root, key).is_some()
    });

    Ok(crate::jv!({
        "root": root.display().to_string(),
        "codexHome": home.display().to_string(),
        "configPath": home.join("config.toml").display().to_string(),
        "configExists": home.join("config.toml").is_file(),
        "skillsPath": skills.display().to_string(),
        "skillCount": skill_count,
        "cliPath": cli.display().to_string(),
        "cliExists": cli.is_file(),
        "packageRoot": package_root.display().to_string(),
        "packageRootExists": package_root.is_dir(),
        "bundledPathExists": package_root.join("codex-path").is_dir(),
        "envKey": runtime.env_key,
        "envPresent": env_present,
        "provider": runtime.provider,
        "model": runtime.model,
        "baseUrl": runtime.base_url,
    }))
}

#[tauri::command]
pub async fn codex_launch_app() -> Result<Value, String> {
    let root = product_root()?;
    let panel = super::read_panel_config_value().unwrap_or_else(|| crate::jv!({}));
    let runtime = read_codex_runtime_config(&root, &panel);
    let cli = codex_cli_path(&root, &panel);
    if !cli.is_file() {
        return Err(format!("Codex 可执行文件不存在: {}", cli.display()));
    }

    let status = codex_status()?;
    if !status.get("configExists").and_then(|v| v.as_bool()).unwrap_or(false) {
        return Err(format!("Codex 配置文件不存在: {}", codex_home(&root).join("config.toml").display()));
    }
    if !status.get("envPresent").and_then(|v| v.as_bool()).unwrap_or(false) {
        let key = status.get("envKey").and_then(|v| v.as_str()).unwrap_or("API Key");
        return Err(format!("未找到 Codex 模型密钥: {key}"));
    }

    let workspace = codex_workspace(&root);
    let logs = codex_logs_dir(&root);
    std::fs::create_dir_all(&workspace).map_err(|e| format!("创建 Codex 工作目录失败: {e}"))?;
    std::fs::create_dir_all(&logs).map_err(|e| format!("创建 Codex 日志目录失败: {e}"))?;

    let stdout =
        std::fs::File::create(logs.join("codex-app.out.log")).map_err(|e| format!("创建 Codex App stdout 日志失败: {e}"))?;
    let stderr =
        std::fs::File::create(logs.join("codex-app.err.log")).map_err(|e| format!("创建 Codex App stderr 日志失败: {e}"))?;

    let mut command = Command::new(&cli);
    command
        .arg("app")
        .arg(&workspace)
        .current_dir(&workspace)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .stdin(Stdio::null());
    inject_codex_env(&mut command, &root, &runtime);

    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);

    let child = command.spawn().map_err(|e| format!("启动原生 Codex App 失败: {e}"))?;

    Ok(crate::jv!({
        "ok": true,
        "pid": child.id(),
        "mode": "native-app",
        "codexHome": codex_home(&root).display().to_string(),
        "workspace": workspace.display().to_string(),
        "provider": status.get("provider").cloned().unwrap_or(Value::Null),
        "model": status.get("model").cloned().unwrap_or(Value::Null),
        "baseUrl": status.get("baseUrl").cloned().unwrap_or(Value::Null),
        "logs": logs.display().to_string(),
    }))
}

#[tauri::command]
pub fn codex_run_once(prompt: String) -> Result<Value, String> {
    let prompt = prompt.trim().to_string();
    if prompt.is_empty() {
        return Err("请输入问题".into());
    }
    if prompt.len() > 12000 {
        return Err("问题过长，请缩短后再试".into());
    }

    let root = product_root()?;
    let panel = super::read_panel_config_value().unwrap_or_else(|| crate::jv!({}));
    let runtime = read_codex_runtime_config(&root, &panel);
    let cli = codex_cli_path(&root, &panel);
    if !cli.is_file() {
        return Err(format!("Codex 可执行文件不存在: {}", cli.display()));
    }

    let mut command = Command::new(&cli);
    command
        .arg("exec")
        .arg("--skip-git-repo-check")
        .arg(prompt)
        .current_dir(codex_workspace(&root));
    inject_codex_env(&mut command, &root, &runtime);

    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);

    let mut child = command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动 Codex 失败: {e}"))?;

    let deadline = Instant::now() + Duration::from_secs(180);
    loop {
        if let Some(_status) = child.try_wait().map_err(|e| format!("等待 Codex 失败: {e}"))? {
            let output = child.wait_with_output().map_err(|e| format!("读取 Codex 输出失败: {e}"))?;
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
            return Err("Codex 执行超时".into());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}
