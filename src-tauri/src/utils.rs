#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
fn legacy_openclaw_zh_scope() -> String {
    format!("@{}cloud", "qingchen")
}

#[cfg(target_os = "windows")]
fn legacy_openclaw_zh_package() -> String {
    format!("openclaw-{}", "zh")
}

#[cfg(target_os = "windows")]
fn push_unique_candidate(
    candidates: &mut Vec<std::path::PathBuf>,
    seen: &mut std::collections::HashSet<String>,
    path: std::path::PathBuf,
) {
    let key = path.to_string_lossy().replace('/', "\\").to_lowercase();
    if seen.insert(key) {
        candidates.push(path);
    }
}

#[cfg(target_os = "windows")]
fn push_windows_cli_files(
    candidates: &mut Vec<std::path::PathBuf>,
    seen: &mut std::collections::HashSet<String>,
    base: std::path::PathBuf,
) {
    push_unique_candidate(candidates, seen, base.join("openclaw.cmd"));
    push_unique_candidate(candidates, seen, base.join("openclaw.exe"));
    push_unique_candidate(candidates, seen, base.join("openclaw"));
    push_unique_candidate(
        candidates,
        seen,
        base.join("node_modules")
            .join(legacy_openclaw_zh_scope())
            .join(legacy_openclaw_zh_package())
            .join("bin")
            .join("openclaw.js"),
    );
    push_unique_candidate(
        candidates,
        seen,
        base.join("node_modules")
            .join("openclaw")
            .join("bin")
            .join("openclaw.js"),
    );
}

#[cfg(target_os = "windows")]
fn common_windows_cli_candidates() -> Vec<std::path::PathBuf> {
    let mut candidates = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // 先按 enhanced PATH 顺序找，保持与用户命令行优先级一致。
    for dir in crate::commands::enhanced_path().split(';') {
        let dir = dir.trim();
        if dir.is_empty() {
            continue;
        }
        push_windows_cli_files(&mut candidates, &mut seen, std::path::PathBuf::from(dir));
    }

    if let Ok(appdata) = std::env::var("APPDATA") {
        push_windows_cli_files(
            &mut candidates,
            &mut seen,
            std::path::PathBuf::from(appdata).join("npm"),
        );
    }
    if let Some(prefix) = crate::commands::windows_npm_global_prefix() {
        push_windows_cli_files(&mut candidates, &mut seen, std::path::PathBuf::from(prefix));
    }
    for sa_dir in crate::commands::config::all_standalone_dirs() {
        push_windows_cli_files(&mut candidates, &mut seen, sa_dir);
    }
    if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
        let localappdata = std::path::PathBuf::from(localappdata);
        push_windows_cli_files(
            &mut candidates,
            &mut seen,
            localappdata.join("Programs").join("OpenClaw"),
        );
        push_windows_cli_files(&mut candidates, &mut seen, localappdata.join("OpenClaw"));
        push_windows_cli_files(
            &mut candidates,
            &mut seen,
            localappdata.join("Programs").join("nodejs"),
        );
    }
    if let Ok(program_files) = std::env::var("ProgramFiles") {
        let program_files = std::path::PathBuf::from(program_files);
        push_windows_cli_files(&mut candidates, &mut seen, program_files.join("nodejs"));
        push_windows_cli_files(&mut candidates, &mut seen, program_files.join("OpenClaw"));
    }
    if let Ok(program_files_x86) = std::env::var("ProgramFiles(x86)") {
        push_windows_cli_files(
            &mut candidates,
            &mut seen,
            std::path::PathBuf::from(program_files_x86).join("nodejs"),
        );
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        push_windows_cli_files(
            &mut candidates,
            &mut seen,
            std::path::PathBuf::from(profile).join(".openclaw-bin"),
        );
    }
    for drive in ["C", "D", "E", "F", "G"] {
        push_windows_cli_files(
            &mut candidates,
            &mut seen,
            std::path::PathBuf::from(format!(r"{drive}:\OpenClaw")),
        );
        push_windows_cli_files(
            &mut candidates,
            &mut seen,
            std::path::PathBuf::from(format!(r"{drive}:\AI\OpenClaw")),
        );
    }

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let mut where_cmd = std::process::Command::new("where");
    where_cmd.arg("openclaw");
    where_cmd.env("PATH", crate::commands::enhanced_path());
    where_cmd.creation_flags(CREATE_NO_WINDOW);
    if let Ok(output) = where_cmd.output() {
        if output.status.success() {
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    push_unique_candidate(
                        &mut candidates,
                        &mut seen,
                        std::path::PathBuf::from(trimmed),
                    );
                }
            }
        }
    }

    candidates
}

pub fn is_rejected_cli_path(cli_path: &str) -> bool {
    let lower = cli_path.replace('\\', "/").to_lowercase();
    lower.contains("/.cherrystudio/") || lower.contains("cherry-studio")
}

/// 读取 clawpanel.json 中用户绑定的 CLI 路径
fn bound_cli_path() -> Option<std::path::PathBuf> {
    let config = crate::commands::read_panel_config_value()?;
    let raw = config.get("openclawCliPath")?.as_str()?;
    if raw.is_empty() {
        return None;
    }
    let p = std::path::PathBuf::from(raw);
    crate::commands::config::resolve_openclaw_cli_input_path(&p)
}

struct OpenClawRuntimeEnv {
    config_dir: std::path::PathBuf,
    home_dir: std::path::PathBuf,
    state_dir: std::path::PathBuf,
    cache_dir: std::path::PathBuf,
    log_dir: std::path::PathBuf,
    workspace_dir: std::path::PathBuf,
    temp_dir: std::path::PathBuf,
}

fn openclaw_runtime_env() -> OpenClawRuntimeEnv {
    let openclaw_dir = crate::commands::openclaw_dir();
    let portable_home_dir = openclaw_dir
        .parent()
        .filter(|parent| parent.join("config").is_dir())
        .map(|parent| parent.to_path_buf())
        .unwrap_or_else(|| openclaw_dir.clone());
    let state_dir = openclaw_dir
        .parent()
        .filter(|parent| parent.join("state").is_dir())
        .map(|parent| parent.join("state"))
        .unwrap_or_else(|| openclaw_dir.clone());
    let cache_dir = openclaw_dir
        .parent()
        .map(|parent| parent.join("cache"))
        .unwrap_or_else(|| openclaw_dir.join("cache"));
    let log_dir = portable_home_dir.join("logs");
    let workspace_dir = openclaw_dir
        .parent()
        .map(|parent| parent.join("workspace").join("main"))
        .unwrap_or_else(|| openclaw_dir.join("workspace").join("main"));
    let temp_dir = cache_dir.join("temp");
    OpenClawRuntimeEnv {
        config_dir: openclaw_dir,
        home_dir: portable_home_dir,
        state_dir,
        cache_dir,
        log_dir,
        workspace_dir,
        temp_dir,
    }
}

fn load_model_credentials_env(config_dir: &std::path::Path) -> Vec<(String, String)> {
    let credentials_path = config_dir.join("model-credentials.env");
    let Ok(content) = std::fs::read_to_string(credentials_path) else {
        return Vec::new();
    };
    let mut values = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, raw_value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty()
            || !key
                .chars()
                .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        {
            continue;
        }
        let value = raw_value
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();
        values.push((key.to_string(), value));
    }
    values
}

fn ensure_runtime_dirs(envs: &OpenClawRuntimeEnv) {
    for dir in [
        &envs.state_dir,
        &envs.cache_dir,
        &envs.log_dir,
        &envs.workspace_dir,
        &envs.temp_dir,
    ] {
        let _ = std::fs::create_dir_all(dir);
    }
}

fn apply_openclaw_dir_env(cmd: &mut std::process::Command) {
    let envs = openclaw_runtime_env();
    ensure_runtime_dirs(&envs);
    cmd.env(
        "OPENCLAW_CONFIG_PATH",
        envs.config_dir.join("openclaw.json"),
    );
    cmd.env("OPENCLAW_HOME", &envs.home_dir);
    cmd.env("OPENCLAW_STATE_DIR", &envs.state_dir);
    cmd.env("OPENCLAW_CACHE_DIR", &envs.cache_dir);
    cmd.env("OPENCLAW_LOG_DIR", &envs.log_dir);
    cmd.env("OPENCLAW_WORKSPACE_DIR", &envs.workspace_dir);
    cmd.env(
        "OPENCLAW_GATEWAY_PORT",
        crate::commands::gateway_listen_port().to_string(),
    );
    cmd.env("OPENCLAW_SHOW_SECRETS", "0");
    cmd.env("OPENCLAW_PORTABLE", "1");
    cmd.env(
        "NODE_COMPILE_CACHE",
        envs.cache_dir.join("node-compile-cache"),
    );
    cmd.env("TEMP", &envs.temp_dir);
    cmd.env("TMP", &envs.temp_dir);
    cmd.env("OPENCLAW_WRAPPER_LOG_REDIRECT", "0");
    for (key, value) in load_model_credentials_env(&envs.config_dir) {
        cmd.env(key, value);
    }
}

fn apply_openclaw_dir_env_tokio(cmd: &mut tokio::process::Command) {
    let envs = openclaw_runtime_env();
    ensure_runtime_dirs(&envs);
    cmd.env(
        "OPENCLAW_CONFIG_PATH",
        envs.config_dir.join("openclaw.json"),
    );
    cmd.env("OPENCLAW_HOME", &envs.home_dir);
    cmd.env("OPENCLAW_STATE_DIR", &envs.state_dir);
    cmd.env("OPENCLAW_CACHE_DIR", &envs.cache_dir);
    cmd.env("OPENCLAW_LOG_DIR", &envs.log_dir);
    cmd.env("OPENCLAW_WORKSPACE_DIR", &envs.workspace_dir);
    cmd.env(
        "OPENCLAW_GATEWAY_PORT",
        crate::commands::gateway_listen_port().to_string(),
    );
    cmd.env("OPENCLAW_SHOW_SECRETS", "0");
    cmd.env("OPENCLAW_PORTABLE", "1");
    cmd.env(
        "NODE_COMPILE_CACHE",
        envs.cache_dir.join("node-compile-cache"),
    );
    cmd.env("TEMP", &envs.temp_dir);
    cmd.env("TMP", &envs.temp_dir);
    cmd.env("OPENCLAW_WRAPPER_LOG_REDIRECT", "0");
    for (key, value) in load_model_credentials_env(&envs.config_dir) {
        cmd.env(key, value);
    }
}

fn configured_cli_candidates() -> Vec<std::path::PathBuf> {
    crate::commands::openclaw_search_paths()
        .into_iter()
        .filter_map(|p| crate::commands::config::resolve_openclaw_cli_input_path(&p))
        .filter(|p| !is_rejected_cli_path(&p.to_string_lossy()))
        .collect()
}

/// Windows: 在 PATH 和常见安装目录中查找 openclaw CLI 的完整路径
/// 避免通过 `cmd /c openclaw` 调用时 npm .cmd shim 中的引号导致
/// "\"node\"" is not recognized 错误
#[cfg(target_os = "windows")]
fn find_openclaw_cmd() -> Option<std::path::PathBuf> {
    // 优先使用用户绑定的路径
    if let Some(bound) = bound_cli_path() {
        return Some(bound);
    }
    for candidate in configured_cli_candidates() {
        if candidate.exists() {
            return Some(candidate);
        }
    }
    common_windows_cli_candidates()
        .into_iter()
        .find(|candidate| candidate.exists() && !is_rejected_cli_path(&candidate.to_string_lossy()))
}

#[cfg(not(target_os = "windows"))]
fn common_non_windows_cli_candidates() -> Vec<std::path::PathBuf> {
    let mut candidates = Vec::new();
    // standalone 安装目录（集中管理，避免多处硬编码）
    for sa_dir in crate::commands::config::all_standalone_dirs() {
        candidates.push(sa_dir.join("openclaw"));
    }
    // 其他标准路径
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".local").join("bin").join("openclaw"));
    }
    candidates.push(std::path::PathBuf::from("/opt/homebrew/bin/openclaw"));
    candidates.push(std::path::PathBuf::from("/usr/local/bin/openclaw"));
    candidates.push(std::path::PathBuf::from("/usr/bin/openclaw"));
    candidates
}

/// 解析当前实际使用的 openclaw CLI 完整路径（跨平台）
pub fn resolve_openclaw_cli_path() -> Option<String> {
    // 优先使用用户绑定的路径
    if let Some(bound) = bound_cli_path() {
        return Some(bound.to_string_lossy().to_string());
    }
    for candidate in configured_cli_candidates() {
        if candidate.exists() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    #[cfg(target_os = "windows")]
    {
        find_openclaw_cmd().map(|p| p.to_string_lossy().to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Fix #219: 优先通过 enhanced_path 搜索：其中 nvm/volta 等版本管理器路径排在 Homebrew 前面，
        // 与 `which openclaw` 的优先级一致，避免残留的 Homebrew 旧版本被优先检测到
        let path = crate::commands::enhanced_path();
        let sep = ':';
        for dir in path.split(sep) {
            let candidate = std::path::Path::new(dir).join("openclaw");
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
        // 兜底：检查 enhanced_path 可能未覆盖到的固定路径（如 GUI 环境 PATH 受限时）
        for candidate in common_non_windows_cli_candidates() {
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
        None
    }
}

/// 根据 CLI 路径判断安装来源
pub fn classify_cli_source(cli_path: &str) -> String {
    let lower = cli_path.replace('\\', "/").to_lowercase();
    // standalone 安装
    if lower.contains("/programs/openclaw/")
        || lower.contains("/openclaw-bin/")
        || lower.contains("/opt/openclaw/")
    {
        return "standalone".into();
    }
    // npm 中文增强版
    if lower.contains(&format!("openclaw-{}", "zh"))
        || lower.contains(&format!("@{}cloud", "qingchen"))
    {
        return "npm-zh".into();
    }
    // npm 全局（大概率官方版）
    if lower.contains("/npm/") || lower.contains("/node_modules/") {
        return "npm-official".into();
    }
    // Homebrew
    if lower.contains("/homebrew/") || lower.contains("/usr/local/bin") {
        return "npm-global".into();
    }
    "unknown".into()
}

/// 跨平台获取 openclaw 命令的方法（同步版本）
#[allow(dead_code)]
pub fn openclaw_command() -> std::process::Command {
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let enhanced = crate::commands::enhanced_path();
        // 优先：找到 openclaw.cmd 完整路径，用 cmd /c "完整路径" 避免引号问题
        if let Some(cmd_path) = find_openclaw_cmd() {
            let mut cmd = std::process::Command::new("cmd");
            if cmd_path
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("js"))
            {
                cmd.arg("/c").arg("node").arg(cmd_path);
            } else {
                cmd.arg("/c").arg(cmd_path);
            }
            cmd.env("PATH", &enhanced);
            apply_openclaw_dir_env(&mut cmd);
            crate::commands::apply_proxy_env(&mut cmd);
            cmd.creation_flags(CREATE_NO_WINDOW);
            return cmd;
        }
        // 兜底：直接用 cmd /c openclaw
        let mut cmd = std::process::Command::new("cmd");
        cmd.arg("/c").arg("openclaw");
        cmd.env("PATH", &enhanced);
        apply_openclaw_dir_env(&mut cmd);
        crate::commands::apply_proxy_env(&mut cmd);
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd
    }
    #[cfg(not(target_os = "windows"))]
    {
        let bin = resolve_openclaw_cli_path().unwrap_or_else(|| "openclaw".into());
        let mut cmd = std::process::Command::new(bin);
        cmd.env("PATH", crate::commands::enhanced_path());
        apply_openclaw_dir_env(&mut cmd);
        crate::commands::apply_proxy_env(&mut cmd);
        cmd
    }
}

/// 异步版本的 openclaw 命令（推荐使用，避免阻塞 UI）
pub fn openclaw_command_async() -> tokio::process::Command {
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let enhanced = crate::commands::enhanced_path();
        // 优先：找到 openclaw.cmd 完整路径
        if let Some(cmd_path) = find_openclaw_cmd() {
            let mut cmd = tokio::process::Command::new("cmd");
            if cmd_path
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("js"))
            {
                cmd.arg("/c").arg("node").arg(cmd_path);
            } else {
                cmd.arg("/c").arg(cmd_path);
            }
            cmd.env("PATH", &enhanced);
            apply_openclaw_dir_env_tokio(&mut cmd);
            crate::commands::apply_proxy_env_tokio(&mut cmd);
            cmd.creation_flags(CREATE_NO_WINDOW);
            return cmd;
        }
        // 兜底
        let mut cmd = tokio::process::Command::new("cmd");
        cmd.arg("/c").arg("openclaw");
        cmd.env("PATH", &enhanced);
        apply_openclaw_dir_env_tokio(&mut cmd);
        crate::commands::apply_proxy_env_tokio(&mut cmd);
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd
    }
    #[cfg(not(target_os = "windows"))]
    {
        let bin = resolve_openclaw_cli_path().unwrap_or_else(|| "openclaw".into());
        let mut cmd = tokio::process::Command::new(bin);
        cmd.env("PATH", crate::commands::enhanced_path());
        apply_openclaw_dir_env_tokio(&mut cmd);
        crate::commands::apply_proxy_env_tokio(&mut cmd);
        cmd
    }
}
