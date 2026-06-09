
/// 从 npm registry 获取最新版本号，超时 5 秒
async fn get_latest_version_for(source: &str) -> Option<String> {
    let client = crate::commands::build_http_client(std::time::Duration::from_secs(2), None).ok()?;
    let pkg = npm_package_name(source).replace('/', "%2F").replace('@', "%40");
    let registry = get_configured_registry();
    let url = format!("{registry}/{pkg}/latest");
    let resp = client.get(&url).send().await.ok()?;
    let json: Value = resp.json().await.ok()?;
    json.get("version").and_then(|v| v.as_str()).map(String::from)
}

/// 从 Windows .cmd shim 文件内容判断实际关联的 npm 包来源
/// npm 生成的 shim 末尾引用实际 JS 入口，据此区分官方版与汉化版
#[cfg(target_os = "windows")]
fn detect_source_from_cmd_shim(cmd_path: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(cmd_path).ok()?;
    let lower = content.to_lowercase();
    // 中文增强版标记：旧包 scope 或 openclaw-zh
    if lower.contains(&legacy_openclaw_zh_package()) || lower.contains(&legacy_openclaw_zh_scope()) {
        return Some("chinese".into());
    }
    // 确认是 npm shim（含 node_modules 引用）→ 官方版
    if lower.contains("node_modules") {
        return Some("official".into());
    }
    // 便携版 shim：payload 根目录下存在 openclaw/package.json
    if cmd_path
        .parent()
        .is_some_and(|dir| dir.join("openclaw").join("package.json").exists())
    {
        return Some("official".into());
    }
    // standalone 的 .cmd 可能不含 node_modules（自定义脚本），由 classify 处理
    None
}

fn detect_standalone_source_from_dir(dir: &std::path::Path) -> Option<String> {
    let version_file = dir.join("VERSION");
    if let Ok(content) = std::fs::read_to_string(&version_file) {
        let mut edition = String::new();
        let mut package = String::new();
        for line in content.lines() {
            if let Some(value) = line.strip_prefix("edition=") {
                edition = value.trim().to_ascii_lowercase();
            } else if let Some(value) = line.strip_prefix("package=") {
                package = value.trim().to_ascii_lowercase();
            }
        }
        if package.contains(&legacy_openclaw_zh_package()) || package.contains(&legacy_openclaw_zh_scope()) {
            return Some("chinese".into());
        }
        if package == "openclaw" {
            return Some("official".into());
        }
        if matches!(edition.as_str(), "zh" | "zh-cn" | "chinese" | "cn") {
            return Some("chinese".into());
        }
        if matches!(edition.as_str(), "en" | "official") {
            return Some("official".into());
        }
    }
    if dir
        .join("node_modules")
        .join(legacy_openclaw_zh_scope())
        .join(legacy_openclaw_zh_package())
        .join("package.json")
        .exists()
    {
        return Some("chinese".into());
    }
    if dir.join("node_modules").join("openclaw").join("package.json").exists() {
        return Some("official".into());
    }
    None
}

fn detect_standalone_source_from_cli_path(cli_path: &std::path::Path) -> Option<String> {
    cli_path.parent().and_then(detect_standalone_source_from_dir)
}

/// 检测当前安装的是官方版还是汉化版
/// macOS: 优先检查 symlink 指向的实际路径
/// Windows: 读取 .cmd shim 内容判断实际关联的包
/// Linux: 直接用 npm list
fn detect_installed_source() -> String {
    // macOS: 检查 openclaw bin 的 symlink 指向
    #[cfg(target_os = "macos")]
    {
        if let Some(cli_path) = crate::utils::resolve_openclaw_cli_path() {
            let resolved = std::fs::canonicalize(&cli_path)
                .ok()
                .unwrap_or_else(|| PathBuf::from(&cli_path));
            let source = crate::utils::classify_cli_source(&resolved.to_string_lossy());
            if source == "standalone" {
                return detect_standalone_source_from_cli_path(&resolved).unwrap_or_else(|| "chinese".into());
            }
            if source == "npm-zh" {
                return "chinese".into();
            }
            if source == "npm-official" || source == "npm-global" {
                return "official".into();
            }
        }
        // 兼容 ARM (/opt/homebrew) 和 Intel (/usr/local) 两种 Homebrew 路径
        for brew_prefix in &["/opt/homebrew/bin/openclaw", "/usr/local/bin/openclaw"] {
            if let Ok(target) = std::fs::read_link(brew_prefix) {
                if target.to_string_lossy().contains(&legacy_openclaw_zh_package()) {
                    return "chinese".into();
                }
                return "official".into();
            }
        }
        for sa_dir in all_standalone_dirs() {
            if sa_dir.join("openclaw").exists() || sa_dir.join("VERSION").exists() {
                return detect_standalone_source_from_dir(&sa_dir).unwrap_or_else(|| "chinese".into());
            }
        }
        "unknown".into()
    }
    // Windows: 通过活跃 CLI 的 .cmd shim 内容判断来源
    // npm 生成的 .cmd shim 最后一行包含实际 JS 入口路径，例如:
    //   "%dp0%\node_modules\openclaw\bin\openclaw.js"           → 官方版
    //   旧中文增强包路径                                      → 中文增强版
    // 读取内容即可一锤定音，不依赖文件系统扫描（避免残留目录误判）
    #[cfg(target_os = "windows")]
    {
        if let Some(cli_path) = crate::utils::resolve_openclaw_cli_path() {
            let source = crate::utils::classify_cli_source(&cli_path);
            // 路径本身能确定的情况（standalone 目录、npm-zh 路径含中文增强包名）
            if source == "standalone" {
                return detect_standalone_source_from_cli_path(std::path::Path::new(&cli_path))
                    .unwrap_or_else(|| "chinese".into());
            }
            if source == "npm-zh" {
                return "chinese".into();
            }
            // npm-official / npm-global / unknown: 路径不含包名，读 .cmd 内容判断
            if let Some(shim_source) = detect_source_from_cmd_shim(std::path::Path::new(&cli_path)) {
                return shim_source;
            }
        }
        // 无活跃 CLI 时的兜底：仅检查 npm 全局目录中实际存在的 shim
        if let Some(npm_bin) = npm_global_bin_dir() {
            let shim = npm_bin.join("openclaw.cmd");
            if let Some(s) = detect_source_from_cmd_shim(&shim) {
                return s;
            }
        }
        for sa_dir in all_standalone_dirs() {
            if sa_dir.join("openclaw.cmd").exists() || sa_dir.join("VERSION").exists() {
                return detect_standalone_source_from_dir(&sa_dir).unwrap_or_else(|| "chinese".into());
            }
        }
        // 确实无法判断
        "unknown".into()
    }
    // Linux: 参照 macOS 实现，完整检测链
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        // 1. 活跃 CLI 路径分类（与 macOS 一致）
        if let Some(cli_path) = crate::utils::resolve_openclaw_cli_path() {
            let resolved = std::fs::canonicalize(&cli_path)
                .ok()
                .unwrap_or_else(|| PathBuf::from(&cli_path));
            let source = crate::utils::classify_cli_source(&resolved.to_string_lossy());
            if source == "standalone" {
                return detect_standalone_source_from_cli_path(&resolved).unwrap_or_else(|| "chinese".into());
            }
            if source == "npm-zh" {
                return "chinese".into();
            }
            if source == "npm-official" || source == "npm-global" {
                return "official".into();
            }
        }
        // 2. 检查 symlink 指向（/usr/local/bin/openclaw, ~/bin/openclaw）
        let home = dirs::home_dir().unwrap_or_default();
        for link in &[PathBuf::from("/usr/local/bin/openclaw"), home.join("bin").join("openclaw")] {
            if let Ok(target) = std::fs::read_link(link) {
                if target.to_string_lossy().contains(&legacy_openclaw_zh_package()) {
                    return "chinese".into();
                }
                return "official".into();
            }
        }
        // 3. standalone 目录检测
        for sa_dir in all_standalone_dirs() {
            if sa_dir.join("openclaw").exists() || sa_dir.join("VERSION").exists() {
                return detect_standalone_source_from_dir(&sa_dir).unwrap_or_else(|| "chinese".into());
            }
        }
        // 4. npm list 兜底
        if let Ok(o) = npm_command().args(["list", "-g", "openclaw", "--depth=0"]).output() {
            if String::from_utf8_lossy(&o.stdout).contains(&format!("{}@", legacy_openclaw_zh_package())) {
                return "chinese".into();
            }
        }
        "unknown".into()
    }
}

#[tauri::command]
pub async fn get_version_info() -> Result<VersionInfo, String> {
    let current = get_local_version().await;
    let mut source = detect_installed_source();
    // 兜底：版本号含 -zh 则一定是汉化版
    if let Some(ref ver) = current {
        if ver.contains("-zh") && source != "chinese" {
            source = "chinese".to_string();
        }
    }
    // unknown 来源不查询 latest/recommended（无法确定对应哪个 npm 包）
    let latest = if source == "unknown" {
        None
    } else {
        get_latest_version_for(&source).await
    };
    let recommended = if source == "unknown" {
        None
    } else {
        recommended_version_for(&source).await
    };
    let update_available = match (&current, &recommended) {
        (Some(c), Some(r)) => recommended_is_newer(r, c),
        (None, Some(_)) => true,
        _ => false,
    };
    let latest_update_available = match (&current, &latest) {
        (Some(c), Some(l)) => recommended_is_newer(l, c),
        (None, Some(_)) => true,
        _ => false,
    };
    let is_recommended = match (&current, &recommended) {
        (Some(c), Some(r)) => versions_match(c, r),
        _ => false,
    };
    let ahead_of_recommended = match (&current, &recommended) {
        (Some(c), Some(r)) => recommended_is_newer(c, r),
        _ => false,
    };

    // 解析当前实际使用的 CLI 路径
    let cli_path = crate::utils::resolve_openclaw_cli_path();
    let cli_source = cli_path.as_ref().map(|p| crate::utils::classify_cli_source(p));

    // 扫描所有可检测到的 OpenClaw 安装
    let all_installations = scan_all_installations(&cli_path);

    Ok(VersionInfo {
        current,
        latest,
        recommended,
        update_available,
        latest_update_available,
        is_recommended,
        ahead_of_recommended,
        panel_version: panel_version().to_string(),
        source,
        cli_path,
        cli_source,
        all_installations: Some(all_installations),
    })
}

fn scan_cli_identity(cli_path: &std::path::Path) -> String {
    #[cfg(target_os = "windows")]
    let identity_path = {
        let mut identity_path = cli_path.to_path_buf();
        let file_name = cli_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if matches!(file_name.as_str(), "openclaw" | "openclaw.exe" | "openclaw.ps1") {
            let cmd_path = cli_path.with_file_name("openclaw.cmd");
            if cmd_path.exists() {
                identity_path = cmd_path;
            }
        }
        identity_path
    };

    #[cfg(not(target_os = "windows"))]
    let identity_path = cli_path.to_path_buf();

    identity_path
        .canonicalize()
        .unwrap_or(identity_path)
        .to_string_lossy()
        .to_lowercase()
}

/// 扫描系统中所有可检测到的 OpenClaw 安装
fn scan_all_installations(active_path: &Option<String>) -> Vec<crate::models::types::OpenClawInstallation> {
    use crate::models::types::OpenClawInstallation;
    let mut results: Vec<OpenClawInstallation> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let active_identity = active_path.as_ref().map(|path| scan_cli_identity(std::path::Path::new(path)));

    let mut try_add = |path: std::path::PathBuf| {
        if !path.exists() {
            return;
        }
        if crate::utils::is_rejected_cli_path(&path.to_string_lossy()) {
            return;
        }
        let identity = scan_cli_identity(&path);
        if seen.contains(&identity) {
            return;
        }
        seen.insert(identity.clone());
        let path_str = path.to_string_lossy().to_string();
        let source = crate::utils::classify_cli_source(&path_str);
        let version = read_version_from_installation(&path);
        let is_active = active_identity.as_ref().map(|active| active == &identity).unwrap_or(false);
        results.push(OpenClawInstallation {
            path: path_str,
            source,
            version,
            active: is_active,
        });
    };

    // standalone 安装目录
    for sa_dir in all_standalone_dirs() {
        #[cfg(target_os = "windows")]
        {
            try_add(sa_dir.join("openclaw.cmd"));
            try_add(sa_dir.join("openclaw.exe"));
        }
        #[cfg(not(target_os = "windows"))]
        {
            try_add(sa_dir.join("openclaw"));
        }
    }

    for configured in super::openclaw_search_paths() {
        if let Some(resolved) = resolve_openclaw_cli_input_path(&configured) {
            try_add(resolved);
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            try_add(std::path::PathBuf::from(&appdata).join("npm").join("openclaw.cmd"));
            try_add(std::path::PathBuf::from(&appdata).join("npm").join("openclaw"));
        }
        if let Some(prefix) = super::windows_npm_global_prefix() {
            let prefix_path = std::path::PathBuf::from(prefix);
            try_add(prefix_path.join("openclaw.cmd"));
            try_add(prefix_path.join("openclaw.exe"));
            try_add(prefix_path.join("openclaw"));
        }
        if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
            let localappdata_path = std::path::PathBuf::from(&localappdata);
            try_add(localappdata_path.join("Programs").join("OpenClaw").join("openclaw.exe"));
            try_add(localappdata_path.join("OpenClaw").join("openclaw.cmd"));
            try_add(localappdata_path.join("OpenClaw").join("openclaw.exe"));
            try_add(localappdata_path.join("Programs").join("nodejs").join("openclaw.cmd"));
            try_add(localappdata_path.join("Programs").join("nodejs").join("openclaw.exe"));
            try_add(
                localappdata_path
                    .join("Programs")
                    .join("nodejs")
                    .join("node_modules")
                    .join(legacy_openclaw_zh_scope())
                    .join(legacy_openclaw_zh_package())
                    .join("bin")
                    .join("openclaw.js"),
            );
        }
        if let Ok(program_files) = std::env::var("ProgramFiles") {
            try_add(std::path::PathBuf::from(&program_files).join("nodejs").join("openclaw.cmd"));
            try_add(std::path::PathBuf::from(&program_files).join("OpenClaw").join("openclaw.cmd"));
        }
        if let Ok(program_files_x86) = std::env::var("ProgramFiles(x86)") {
            try_add(
                std::path::PathBuf::from(&program_files_x86)
                    .join("nodejs")
                    .join("openclaw.cmd"),
            );
        }
        if let Ok(profile) = std::env::var("USERPROFILE") {
            try_add(std::path::PathBuf::from(&profile).join(".openclaw-bin").join("openclaw.cmd"));
        }
        for drive in ["C", "D", "E", "F", "G"] {
            try_add(std::path::PathBuf::from(format!(r"{}:\OpenClaw\openclaw.cmd", drive)));
            try_add(std::path::PathBuf::from(format!(r"{}:\AI\OpenClaw\openclaw.cmd", drive)));
        }
        let mut where_cmd = Command::new("where");
        where_cmd.arg("openclaw");
        where_cmd.creation_flags(0x08000000);
        if let Ok(output) = where_cmd.output() {
            if output.status.success() {
                for line in String::from_utf8_lossy(&output.stdout).lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    try_add(std::path::PathBuf::from(trimmed));
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(home) = dirs::home_dir() {
            try_add(home.join(".npm-global").join("bin").join("openclaw"));
            try_add(home.join(".local").join("bin").join("openclaw"));
            try_add(home.join(".nvm").join("current").join("bin").join("openclaw"));
            try_add(home.join(".volta").join("bin").join("openclaw"));
            try_add(home.join(".fnm").join("current").join("bin").join("openclaw"));
            try_add(home.join("bin").join("openclaw"));
        }
        try_add(std::path::PathBuf::from("/opt/openclaw/openclaw"));
        try_add(std::path::PathBuf::from("/opt/homebrew/bin/openclaw"));
        try_add(std::path::PathBuf::from("/usr/local/bin/openclaw"));
        try_add(std::path::PathBuf::from("/usr/bin/openclaw"));
        try_add(std::path::PathBuf::from("/snap/bin/openclaw"));
        if let Ok(output) = Command::new("which").args(["-a", "openclaw"]).output() {
            if output.status.success() {
                for line in String::from_utf8_lossy(&output.stdout).lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    try_add(std::path::PathBuf::from(trimmed));
                }
            }
        }
    }

    let enhanced = super::enhanced_path();
    #[cfg(target_os = "windows")]
    let sep = ';';
    #[cfg(not(target_os = "windows"))]
    let sep = ':';
    for dir in enhanced.split(sep) {
        let dir = dir.trim();
        if dir.is_empty() {
            continue;
        }
        let base = std::path::Path::new(dir);
        #[cfg(target_os = "windows")]
        {
            try_add(base.join("openclaw.cmd"));
            try_add(base.join("openclaw.exe"));
            try_add(base.join("openclaw"));
            try_add(
                base.join("node_modules")
                    .join(legacy_openclaw_zh_scope())
                    .join(legacy_openclaw_zh_package())
                    .join("bin")
                    .join("openclaw.js"),
            );
        }
        #[cfg(not(target_os = "windows"))]
        {
            try_add(base.join("openclaw"));
        }
    }

    results.sort_by(|a, b| {
        b.active
            .cmp(&a.active)
            .then_with(|| a.source.cmp(&b.source))
            .then_with(|| a.path.cmp(&b.path))
    });

    results
}

pub(crate) fn resolve_openclaw_cli_input_path(cli_path: &std::path::Path) -> Option<std::path::PathBuf> {
    if cli_path.as_os_str().is_empty() {
        return None;
    }
    let input = cli_path.to_path_buf();
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();

    if input.is_dir() {
        #[cfg(target_os = "windows")]
        {
            candidates.push(input.join("openclaw.cmd"));
            candidates.push(input.join("openclaw.exe"));
            candidates.push(input.join("openclaw"));
        }
        #[cfg(not(target_os = "windows"))]
        {
            candidates.push(input.join("openclaw"));
        }
    } else {
        candidates.push(input);
    }

    candidates
        .into_iter()
        .find(|candidate| candidate.exists() && !crate::utils::is_rejected_cli_path(&candidate.to_string_lossy()))
}

pub(crate) fn resolve_openclaw_cli_input(cli_path: &str) -> Option<std::path::PathBuf> {
    let raw = cli_path.trim();
    if raw.is_empty() {
        return None;
    }
    resolve_openclaw_cli_input_path(std::path::Path::new(raw))
}

#[tauri::command]
pub fn scan_openclaw_paths() -> Result<Vec<crate::models::types::OpenClawInstallation>, String> {
    super::refresh_enhanced_path();
    crate::commands::service::invalidate_cli_detection_cache();
    let active_path = crate::utils::resolve_openclaw_cli_path();
    Ok(scan_all_installations(&active_path))
}

#[tauri::command]
pub fn check_openclaw_at_path(cli_path: String) -> Result<Value, String> {
    let mut result = serde_json::Map::new();
    if let Some(resolved) = resolve_openclaw_cli_input(&cli_path) {
        let path_str = resolved.to_string_lossy().to_string();
        result.insert("installed".into(), Value::Bool(true));
        result.insert("path".into(), Value::String(path_str.clone()));
        result.insert("source".into(), Value::String(crate::utils::classify_cli_source(&path_str)));
        if let Some(version) = read_version_from_installation(&resolved) {
            result.insert("version".into(), Value::String(version));
        } else {
            result.insert("version".into(), Value::Null);
        }
    } else {
        result.insert("installed".into(), Value::Bool(false));
        result.insert("path".into(), Value::Null);
        result.insert("source".into(), Value::Null);
        result.insert("version".into(), Value::Null);
    }
    Ok(Value::Object(result))
}

fn find_git_path() -> Option<String> {
    // #Compat-4: 必须把子进程 PATH 替换成 enhanced_path，否则继承的是 Tauri 启动时快照，
    // 用户新装的 git 不在快照里，`where git` / `which git` 就找不到。对齐 find_node_path 的做法。
    let enhanced = super::enhanced_path();
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("where");
        cmd.arg("git");
        cmd.creation_flags(0x08000000);
        cmd.env("PATH", &enhanced);
        if let Ok(output) = cmd.output() {
            if output.status.success() {
                if let Some(first_line) = String::from_utf8_lossy(&output.stdout).lines().next() {
                    let path = first_line.trim().to_string();
                    if !path.is_empty() && std::path::Path::new(&path).exists() {
                        return Some(path);
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = Command::new("which");
        cmd.arg("git");
        cmd.env("PATH", &enhanced);
        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() && std::path::Path::new(&path).exists() {
                    return Some(path);
                }
            }
        }
    }

    None
}

/// 从安装路径附近读取版本信息
fn read_version_from_installation(cli_path: &std::path::Path) -> Option<String> {
    // 尝试从同目录的 VERSION 文件读取
    if let Some(dir) = cli_path.parent() {
        let version_file = dir.join("VERSION");
        if let Ok(content) = std::fs::read_to_string(&version_file) {
            for line in content.lines() {
                if let Some(ver) = line.strip_prefix("openclaw_version=") {
                    let ver = ver.trim();
                    if !ver.is_empty() {
                        return Some(ver.to_string());
                    }
                }
            }
        }
        // 便携版布局：payload 根目录放 openclaw.cmd，真实包位于同级 openclaw/package.json
        let portable_pkg = dir.join("openclaw").join("package.json");
        if let Ok(content) = std::fs::read_to_string(&portable_pkg) {
            if let Some(ver) = serde_json::from_str::<serde_json::Value>(&content)
                .ok()
                .and_then(|v| v.get("version")?.as_str().map(String::from))
            {
                return Some(ver);
            }
        }
        // CLI 本体位于包目录中时（如 npm 全局安装：nvm、Homebrew 等），
        // 直接读取同目录的 package.json（即该包自身的版本文件）
        let own_pkg = dir.join("package.json");
        if let Ok(content) = std::fs::read_to_string(&own_pkg) {
            if let Some(ver) = serde_json::from_str::<serde_json::Value>(&content)
                .ok()
                .and_then(|v| v.get("version")?.as_str().map(String::from))
            {
                return Some(ver);
            }
        }
        // 根据 CLI 路径判断来源，决定 package.json 检查顺序
        // 避免残留的另一来源包被优先读取
        let pkg_names: &[&str] = &["openclaw"];
        // 尝试从 package.json 读取
        for pkg_name in pkg_names {
            let pkg_json = dir.join("node_modules").join(pkg_name).join("package.json");
            if let Ok(content) = std::fs::read_to_string(&pkg_json) {
                if let Some(ver) = serde_json::from_str::<serde_json::Value>(&content)
                    .ok()
                    .and_then(|v| v.get("version")?.as_str().map(String::from))
                {
                    return Some(ver);
                }
            }
        }
        // npm shim 情况：向上查找 node_modules
        if let Some(parent) = dir.parent() {
            for pkg_name in pkg_names {
                let pkg_json = parent.join("node_modules").join(pkg_name).join("package.json");
                if let Ok(content) = std::fs::read_to_string(&pkg_json) {
                    if let Some(ver) = serde_json::from_str::<serde_json::Value>(&content)
                        .ok()
                        .and_then(|v| v.get("version")?.as_str().map(String::from))
                    {
                        return Some(ver);
                    }
                }
            }
        }
    }
    None
}

/// 获取 OpenClaw 运行时状态摘要（openclaw status --json）
/// 包含 runtimeVersion、会话列表（含 token 用量、fastMode 等标签）
#[tauri::command]
pub async fn get_status_summary() -> Result<Value, String> {
    if is_portable_runtime_config_dir() {
        return Ok(status_summary_fallback(None).await);
    }

    let mut status_cmd = crate::utils::openclaw_command_async();
    status_cmd.args(["status", "--json"]).kill_on_drop(true);
    let output = tokio::time::timeout(std::time::Duration::from_secs(6), status_cmd.output()).await;

    match output {
        Ok(Ok(o)) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            // CLI 输出可能含非 JSON 行，复用 skills 模块的 extract_json
            if let Some(mut value) = crate::commands::skills::extract_json_pub(&stdout) {
                if let Some(obj) = value.as_object_mut() {
                    obj.entry("source").or_insert_with(|| Value::String("live".into()));
                }
                Ok(value)
            } else {
                Ok(status_summary_fallback(Some("openclaw status 输出中未找到有效 JSON".into())).await)
            }
        }
        Ok(Ok(o)) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            Ok(status_summary_fallback(Some(format!("openclaw status 失败: {}", stderr.trim()))).await)
        }
        Ok(Err(e)) => Ok(status_summary_fallback(Some(format!("执行 openclaw 失败: {e}"))).await),
        Err(_) => Ok(status_summary_fallback(Some("openclaw status --json 超时，已改用本地文件摘要".into())).await),
    }
}
