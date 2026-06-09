#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
#[cfg(target_os = "windows")]
use std::process::Command;
use std::time::Duration;

/// 缓存 gateway 端口，避免频繁读文件（5秒有效期）
static GATEWAY_PORT_CACHE: std::sync::LazyLock<std::sync::Mutex<(u16, std::time::Instant)>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new((18789, std::time::Instant::now() - Duration::from_secs(60))));

pub(crate) fn zhizhua_service_url() -> &'static str {
    option_env!("ZHIZHUA_SERVICE_URL").unwrap_or("https://ai.iazp.cn")
}

pub(crate) fn zhizhua_url(path: &str) -> String {
    let base = zhizhua_service_url().trim_end_matches('/');
    if path.is_empty() {
        base.to_string()
    } else if path.starts_with('/') {
        format!("{base}{path}")
    } else {
        format!("{base}/{path}")
    }
}

pub(crate) fn portable_product_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?;
    let mut candidates = vec![exe_dir.to_path_buf()];
    if let Some(root) = exe_dir.parent().and_then(|app| app.parent()) {
        candidates.push(root.to_path_buf());
    }
    candidates
        .into_iter()
        .find(|root| root.join("data").join("config").is_dir() && root.join("app").is_dir())
}

pub(crate) fn portable_config_dir() -> Option<PathBuf> {
    portable_product_root().map(|root| root.join("data").join("config"))
}

pub(crate) fn portable_panel_config_path() -> Option<PathBuf> {
    portable_config_dir().map(|dir| dir.join("zhizhua-workbench.json"))
}

/// 默认 OpenClaw 配置目录
/// Windows 上优先使用 USERPROFILE（与 Node.js os.homedir() 一致），
/// 并自动检测已有 openclaw.json 的目录，避免创建第二个 .openclaw
pub(crate) fn default_openclaw_dir() -> PathBuf {
    if let Some(portable) = portable_config_dir() {
        return portable;
    }
    #[cfg(target_os = "windows")]
    {
        let mut candidates: Vec<PathBuf> = Vec::new();
        // 优先 USERPROFILE（与 Node.js os.homedir() 一致）
        if let Ok(up) = std::env::var("USERPROFILE") {
            let p = PathBuf::from(up.trim());
            if !p.as_os_str().is_empty() {
                candidates.push(p);
            }
        }
        // dirs::home_dir() 作为补充（Windows API SHGetKnownFolderPath）
        if let Some(dh) = dirs::home_dir() {
            if !candidates.iter().any(|c| panel_path_key(c) == panel_path_key(&dh)) {
                candidates.push(dh);
            }
        }
        // HOMEDRIVE+HOMEPATH（域控/企业环境可能指向网络盘）
        if let (Ok(hd), Ok(hp)) = (std::env::var("HOMEDRIVE"), std::env::var("HOMEPATH")) {
            let combined = format!("{}{}", hd.trim(), hp.trim());
            let p = PathBuf::from(&combined);
            if !combined.is_empty() && !candidates.iter().any(|c| panel_path_key(c) == panel_path_key(&p)) {
                candidates.push(p);
            }
        }
        // 优先选已有 openclaw.json 的目录（自动对齐已安装的 OpenClaw）
        for home in &candidates {
            let dir = home.join(".openclaw");
            if dir.join("openclaw.json").exists() {
                return dir;
            }
        }
        // 都没有 → 用第一个候选（USERPROFILE）
        candidates.first().cloned().unwrap_or_default().join(".openclaw")
    }
    #[cfg(not(target_os = "windows"))]
    {
        dirs::home_dir().unwrap_or_default().join(".openclaw")
    }
}

fn panel_path_key(path: &std::path::Path) -> String {
    #[cfg(target_os = "windows")]
    {
        path.to_string_lossy().replace('/', "\\").to_lowercase()
    }
    #[cfg(not(target_os = "windows"))]
    {
        path.to_string_lossy().to_string()
    }
}

fn push_unique_panel_config_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    let key = panel_path_key(&path);
    if paths.iter().any(|existing| panel_path_key(existing) == key) {
        return;
    }
    paths.push(path);
}

pub(crate) fn panel_config_candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(path) = portable_panel_config_path() {
        push_unique_panel_config_path(&mut paths, path);
    }
    if let Some(path) = portable_config_dir().map(|dir| dir.join("clawpanel.json")) {
        push_unique_panel_config_path(&mut paths, path);
    }
    push_unique_panel_config_path(&mut paths, default_openclaw_dir().join("clawpanel.json"));

    #[cfg(target_os = "windows")]
    {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            let trimmed = profile.trim();
            if !trimmed.is_empty() {
                push_unique_panel_config_path(&mut paths, PathBuf::from(trimmed).join(".openclaw").join("clawpanel.json"));
            }
        }

        if let (Ok(home_drive), Ok(home_path)) = (std::env::var("HOMEDRIVE"), std::env::var("HOMEPATH")) {
            let combined = format!("{}{}", home_drive.trim(), home_path.trim());
            let trimmed = combined.trim();
            if !trimmed.is_empty() {
                push_unique_panel_config_path(&mut paths, PathBuf::from(trimmed).join(".openclaw").join("clawpanel.json"));
            }
        }

        if let Ok(appdata) = std::env::var("APPDATA") {
            let appdata_path = PathBuf::from(appdata.trim());
            if let Some(profile_dir) = appdata_path.parent().and_then(|p| p.parent()) {
                push_unique_panel_config_path(&mut paths, profile_dir.join(".openclaw").join("clawpanel.json"));
            }
        }
    }

    paths
}

fn read_json_file_content(path: &std::path::Path) -> Option<String> {
    let raw = std::fs::read(path).ok()?;
    let bytes = if raw.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &raw[3..]
    } else {
        &raw
    };
    Some(String::from_utf8_lossy(bytes).into_owned())
}

fn read_panel_config_from(path: &std::path::Path) -> Option<serde_json::Value> {
    read_json_file_content(path).and_then(|content| serde_json::from_str(&content).ok())
}

fn normalize_custom_openclaw_dir(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let expanded = if let Some(rest) = trimmed.strip_prefix("~/").or_else(|| trimmed.strip_prefix("~\\")) {
        dirs::home_dir().unwrap_or_default().join(rest)
    } else {
        PathBuf::from(trimmed)
    };

    if expanded.is_absolute() {
        Some(expanded)
    } else {
        std::env::current_dir().ok().map(|cwd| cwd.join(expanded))
    }
}

pub(crate) fn openclaw_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let Some(value) = read_panel_config_value() else {
        return paths;
    };
    let Some(entries) = value.get("openclawSearchPaths").and_then(|v| v.as_array()) else {
        return paths;
    };

    for raw in entries.iter().filter_map(|v| v.as_str()) {
        if let Some(path) = normalize_custom_openclaw_dir(raw) {
            if !paths.iter().any(|p| p == &path) {
                paths.push(path);
            }
        }
    }
    paths
}

/// 获取 OpenClaw 配置目录
/// 优先使用 clawpanel.json 中的 openclawDir 自定义路径，不存在则回退默认 ~/.openclaw
pub(crate) fn openclaw_dir() -> PathBuf {
    if let Some(custom) = read_panel_config_value()
        .and_then(|v| v.get("openclawDir")?.as_str().map(String::from))
        .and_then(|v| normalize_custom_openclaw_dir(&v))
    {
        return custom;
    }
    default_openclaw_dir()
}

/// Gateway 监听端口：读取 `openclaw.json` 的 `gateway.port`，缺省 **18789**。
/// 与面板「Gateway 配置」、服务状态检测（netstat / TCP / launchctl 兜底）共用同一来源，
/// 并尊重 `clawpanel.json` 中的 `openclawDir` 自定义配置目录。
pub(crate) fn gateway_listen_port() -> u16 {
    // 5秒内返回缓存值，避免服务状态检测时频繁读文件
    if let Ok(cache) = GATEWAY_PORT_CACHE.lock() {
        if cache.1.elapsed() < Duration::from_secs(5) {
            return cache.0;
        }
    }
    let port = read_gateway_port_from_config();
    if let Ok(mut cache) = GATEWAY_PORT_CACHE.lock() {
        *cache = (port, std::time::Instant::now());
    }
    port
}

fn read_gateway_port_from_config() -> u16 {
    let config_path = openclaw_dir().join("openclaw.json");
    if let Some(content) = read_json_file_content(&config_path) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(port) = val.get("gateway").and_then(|g| g.get("port")).and_then(|p| p.as_u64()) {
                if port > 0 && port < 65536 {
                    return port as u16;
                }
            }
        }
    }
    18789
}

pub(crate) fn panel_config_path() -> PathBuf {
    let candidates = panel_config_candidate_paths();
    for path in &candidates {
        if read_panel_config_from(path).is_some() {
            return path.clone();
        }
    }
    for path in &candidates {
        if path.exists() {
            return path.clone();
        }
    }
    candidates
        .into_iter()
        .next()
        .unwrap_or_else(|| default_openclaw_dir().join("clawpanel.json"))
}

#[cfg(target_os = "windows")]
pub(crate) fn windows_npm_global_prefix() -> Option<String> {
    if let Ok(prefix) = std::env::var("NPM_CONFIG_PREFIX") {
        let trimmed = prefix.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let mut cmd = Command::new("cmd");
    cmd.args(["/d", "/s", "/c", "npm config get prefix"]);
    cmd.creation_flags(CREATE_NO_WINDOW);
    if let Ok(output) = cmd.output() {
        if output.status.success() {
            let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !prefix.is_empty() && prefix.to_lowercase() != "undefined" {
                return Some(prefix);
            }
        }
    }

    None
}

pub(crate) fn read_panel_config_value() -> Option<serde_json::Value> {
    for path in panel_config_candidate_paths() {
        if let Some(value) = read_panel_config_from(&path) {
            return Some(value);
        }
    }
    None
}
