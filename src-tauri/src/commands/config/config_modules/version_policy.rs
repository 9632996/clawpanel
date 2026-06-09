use crate::models::types::VersionInfo;

struct GuardianPause {
    reason: &'static str,
}

impl GuardianPause {
    fn new(reason: &'static str) -> Self {
        crate::commands::service::guardian_pause(reason);
        Self { reason }
    }
}

impl Drop for GuardianPause {
    fn drop(&mut self) {
        crate::commands::service::guardian_resume(self.reason);
    }
}

/// 预设 npm 源列表
const DEFAULT_REGISTRY: &str = "https://registry.npmmirror.com";
// (target_https_prefix, from_pattern) pairs for Git HTTPS rewriting.
/// Each entry maps a non-HTTPS Git URL pattern to the corresponding HTTPS URL.
const GIT_HTTPS_REWRITES: &[(&str, &str)] = &[
    // github.com
    ("https://github.com/", "ssh://git@github.com/"),
    ("https://github.com/", "ssh://git@github.com"),
    ("https://github.com/", "ssh://git@://github.com/"),
    ("https://github.com/", "git@github.com:"),
    ("https://github.com/", "git://github.com/"),
    ("https://github.com/", "git+ssh://git@github.com/"),
    // gitlab.com
    ("https://gitlab.com/", "ssh://git@gitlab.com/"),
    ("https://gitlab.com/", "git@gitlab.com:"),
    ("https://gitlab.com/", "git://gitlab.com/"),
    ("https://gitlab.com/", "git+ssh://git@gitlab.com/"),
    // bitbucket.org
    ("https://bitbucket.org/", "ssh://git@bitbucket.org/"),
    ("https://bitbucket.org/", "git@bitbucket.org:"),
    ("https://bitbucket.org/", "git://bitbucket.org/"),
    ("https://bitbucket.org/", "git+ssh://git@bitbucket.org/"),
];

const VERSION_POLICY_CACHE_TTL_SECS: u64 = 30 * 60;
static VERSION_POLICY_CACHE: OnceLock<Mutex<Option<(Instant, VersionPolicy)>>> = OnceLock::new();

#[derive(Debug, Clone, Deserialize, Default)]
struct VersionPolicySource {
    recommended: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct VersionPolicyEntry {
    #[serde(default)]
    official: VersionPolicySource,
    #[serde(default)]
    chinese: VersionPolicySource,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Default)]
struct R2Config {
    #[serde(default)]
    #[serde(rename = "baseUrl")]
    base_url: Option<String>,
    #[serde(default)]
    enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct StandaloneConfig {
    #[serde(default)]
    #[serde(rename = "baseUrl")]
    base_url: Option<String>,
    #[serde(default)]
    enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct VersionPolicy {
    #[serde(default)]
    standalone: StandaloneConfig,
    #[serde(default)]
    r2: R2Config,
    #[serde(default)]
    default: VersionPolicyEntry,
    #[serde(default)]
    panels: HashMap<String, VersionPolicyEntry>,
}

fn panel_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn legacy_openclaw_zh_scope() -> String {
    format!("@{}cloud", "qingchen")
}

fn legacy_openclaw_zh_package() -> String {
    format!("openclaw-{}", "zh")
}

fn find_panel_policy_entry<'a>(policy: &'a VersionPolicy, current_version: &str) -> Option<&'a VersionPolicyEntry> {
    if let Some(entry) = policy.panels.get(current_version) {
        return Some(entry);
    }

    let current_parts = parse_version(current_version);
    if current_parts.len() < 2 {
        return None;
    }

    policy
        .panels
        .iter()
        .filter_map(|(version, entry)| {
            let parts = parse_version(version);
            if parts.len() < 2 {
                return None;
            }
            if parts[0] != current_parts[0] || parts[1] != current_parts[1] {
                return None;
            }
            if parts > current_parts {
                return None;
            }
            Some((parts, entry))
        })
        .max_by(|(left, _), (right, _)| left.cmp(right))
        .map(|(_, entry)| entry)
}

fn parse_version(value: &str) -> Vec<u32> {
    value
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|s| s.parse().ok())
        .collect()
}

// 提取基础版本号（去掉 -zh.x / -nightly.xxx 等后缀，只保留主版本数字部分）
/// "2026.3.13-zh.1" → "2026.3.13", "2026.3.13" → "2026.3.13"
fn base_version(v: &str) -> String {
    // 在第一个 '-' 处截断
    let base = v.split('-').next().unwrap_or(v);
    base.to_string()
}

fn has_version_suffix(v: &str) -> bool {
    v.contains('-')
}

/// 判断 CLI 报告的版本是否与推荐版匹配（考虑汉化版 -zh.x 后缀差异）
fn versions_match(cli_version: &str, recommended: &str) -> bool {
    if cli_version == recommended {
        return true;
    }
    // CLI 报告 "2026.3.13"，推荐版 "2026.3.13-zh.1" → 基础版本相同即视为匹配
    if base_version(cli_version) != base_version(recommended) {
        return false;
    }
    if has_version_suffix(cli_version) {
        return false;
    }
    true
}

/// 判断推荐版是否真的比当前版本更新（忽略 -zh.x 后缀）
fn recommended_is_newer(recommended: &str, current: &str) -> bool {
    let r = parse_version(&base_version(recommended));
    let c = parse_version(&base_version(current));
    if r != c {
        return r > c;
    }
    if has_version_suffix(recommended) && has_version_suffix(current) {
        return parse_version(recommended) > parse_version(current);
    }
    false
}

fn load_embedded_version_policy() -> VersionPolicy {
    serde_json::from_str(include_str!("../../../../../openclaw-version-policy.json")).unwrap_or_else(|_| VersionPolicy::default())
}

fn policy_cache() -> &'static Mutex<Option<(Instant, VersionPolicy)>> {
    VERSION_POLICY_CACHE.get_or_init(|| Mutex::new(None))
}

fn version_policy_urls() -> Vec<String> {
    let mut urls = Vec::new();
    for key in ["ZHIZHUA_OPENCLAW_VERSION_POLICY_URL", "OPENCLAW_VERSION_POLICY_URL"] {
        if let Ok(url) = std::env::var(key) {
            let url = url.trim();
            if !url.is_empty() {
                urls.push(url.to_string());
            }
        }
    }
    urls.push(super::zhizhua_url("/update/openclaw-version-policy.json"));
    urls.push(super::zhizhua_url("/openclaw-version-policy.json"));
    urls
}

async fn fetch_remote_version_policy() -> Option<VersionPolicy> {
    let client = crate::commands::build_http_client(Duration::from_secs(8), Some("ZhizhuaWorkbench")).ok()?;
    for url in version_policy_urls() {
        let Ok(resp) = client.get(&url).header("Accept", "application/json").send().await else {
            continue;
        };
        if !resp.status().is_success() {
            continue;
        }
        if let Ok(policy) = resp.json::<VersionPolicy>().await {
            return Some(policy);
        }
    }
    None
}

fn read_openclaw_version_file(path: &Path) -> Option<(String, Option<String>)> {
    let content = fs::read_to_string(path).ok()?;
    let mut version = None;
    let mut source = None;
    for line in content.lines() {
        if let Some(value) = line.strip_prefix("openclaw_version=") {
            let value = value.trim();
            if !value.is_empty() {
                version = Some(value.to_string());
            }
        } else if let Some(value) = line.strip_prefix("package=") {
            let value = value.trim().to_ascii_lowercase();
            if value.contains(&legacy_openclaw_zh_package()) || value.contains(&legacy_openclaw_zh_scope()) {
                source = Some("chinese".to_string());
            } else if value == "openclaw" {
                source = Some("official".to_string());
            }
        } else if let Some(value) = line.strip_prefix("edition=") {
            let value = value.trim().to_ascii_lowercase();
            if matches!(value.as_str(), "zh" | "zh-cn" | "chinese" | "cn") {
                source = Some("chinese".to_string());
            } else if matches!(value.as_str(), "en" | "official") {
                source = Some("official".to_string());
            }
        }
    }
    version.map(|version| (version, source))
}

fn read_openclaw_package_version(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&content)
        .ok()
        .and_then(|v| v.get("version")?.as_str().map(String::from))
}

fn read_bundled_openclaw_version_from_payload(payload_dir: &Path) -> Option<(String, String)> {
    if let Some((version, source)) = read_openclaw_version_file(&payload_dir.join("VERSION")) {
        return Some((version, source.unwrap_or_else(|| "official".to_string())));
    }
    read_openclaw_package_version(&payload_dir.join("openclaw").join("package.json"))
        .map(|version| (version, "official".to_string()))
}

fn bundled_openclaw_version() -> Option<(String, String)> {
    let root = super::portable_product_root()?;
    let payloads = root.join("app").join("payloads");
    let platform = standalone_platform_key();
    if platform != "unknown" {
        if let Some(version) = read_bundled_openclaw_version_from_payload(&payloads.join(platform)) {
            return Some(version);
        }
    }
    let entries = fs::read_dir(&payloads).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(version) = read_bundled_openclaw_version_from_payload(&path) {
                return Some(version);
            }
        }
    }
    None
}

fn bundled_recommended_version_for(source: &str) -> Option<String> {
    let (version, bundled_source) = bundled_openclaw_version()?;
    let requested_source = if source == "official" { "official" } else { "chinese" };
    if bundled_source == requested_source {
        Some(version)
    } else {
        None
    }
}

fn recommended_version_from_policy(policy: &VersionPolicy, source: &str) -> Option<String> {
    let panel_entry = find_panel_policy_entry(policy, panel_version());
    match source {
        "official" => panel_entry
            .and_then(|entry| entry.official.recommended.clone())
            .or_else(|| policy.default.official.recommended.clone()),
        _ => panel_entry
            .and_then(|entry| entry.chinese.recommended.clone())
            .or_else(|| policy.default.chinese.recommended.clone()),
    }
}

fn offline_recommended_version_for(source: &str) -> Option<String> {
    bundled_recommended_version_for(source).or_else(|| recommended_version_from_policy(&load_embedded_version_policy(), source))
}

async fn load_version_policy() -> VersionPolicy {
    let ttl = Duration::from_secs(VERSION_POLICY_CACHE_TTL_SECS);
    if let Ok(cache) = policy_cache().lock() {
        if let Some((loaded_at, policy)) = cache.as_ref() {
            if loaded_at.elapsed() < ttl {
                return policy.clone();
            }
        }
    }

    if let Some(policy) = fetch_remote_version_policy().await {
        if let Ok(mut cache) = policy_cache().lock() {
            *cache = Some((Instant::now(), policy.clone()));
        }
        return policy;
    }

    load_embedded_version_policy()
}

#[allow(dead_code)]
async fn r2_config() -> R2Config {
    load_version_policy().await.r2
}

async fn standalone_config() -> StandaloneConfig {
    load_version_policy().await.standalone
}

/// standalone 包的平台 key（与 CI 构建矩阵一致）
fn standalone_platform_key() -> &'static str {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "win-x64"
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "mac-arm64"
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        "mac-x64"
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "linux-x64"
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "linux-arm64"
    }
    #[cfg(not(any(
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
    )))]
    {
        "unknown"
    }
}

/// standalone 包的文件扩展名
fn standalone_archive_ext() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "zip"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "tar.gz"
    }
}

/// standalone 安装目录
pub(crate) fn standalone_install_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // Inno Setup PrivilegesRequired=lowest 默认安装到 %LOCALAPPDATA%\Programs
        std::env::var("LOCALAPPDATA")
            .ok()
            .map(|d| PathBuf::from(d).join("Programs").join("OpenClaw"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        dirs::home_dir().map(|h| h.join(".openclaw-bin"))
    }
}

/// 所有可能的 standalone 安装位置（用于检测和卸载）
pub(crate) fn all_standalone_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    #[cfg(target_os = "windows")]
    {
        if let Ok(la) = std::env::var("LOCALAPPDATA") {
            dirs.push(PathBuf::from(&la).join("Programs").join("OpenClaw"));
            dirs.push(PathBuf::from(&la).join("OpenClaw"));
        }
        if let Ok(pf) = std::env::var("ProgramFiles") {
            dirs.push(PathBuf::from(pf).join("OpenClaw"));
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(h) = dirs::home_dir() {
            dirs.push(h.join(".openclaw-bin"));
        }
        dirs.push(PathBuf::from("/opt/openclaw"));
    }
    dirs
}

async fn recommended_version_for(source: &str) -> Option<String> {
    let policy = load_version_policy().await;
    recommended_version_from_policy(&policy, source).or_else(|| offline_recommended_version_for(source))
}

// 获取用户配置的 git 可执行文件路径，回退到 "git"
include!("version_policy/install_environment.rs");
