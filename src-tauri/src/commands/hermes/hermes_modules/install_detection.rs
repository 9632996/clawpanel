
/// 查找可执行文件路径
fn find_executable_path(name: &str, enhanced_path: &str) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = std::process::Command::new("where");
        cmd.arg(name).env("PATH", enhanced_path);
        apply_hermes_runtime_env_std(&mut cmd);
        cmd.creation_flags(CREATE_NO_WINDOW);
        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                return stdout.lines().next().map(|s| s.trim().to_string());
            }
        }
        None
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = std::process::Command::new("which");
        cmd.arg(name).env("PATH", enhanced_path);
        apply_hermes_runtime_env_std(&mut cmd);
        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                return Some(stdout.trim().to_string());
            }
        }
        None
    }
}

fn current_platform_key() -> &'static str {
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

// ---------------------------------------------------------------------------
// check_hermes — 检测 Hermes Agent 安装状态
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn check_hermes() -> Result<Value, String> {
    let enhanced = hermes_enhanced_path();
    let mut result = serde_json::Map::new();
    let home = hermes_home();

    // 1. 检测 hermes CLI
    let hermes_version =
        run_at_path("hermes", &["version"], &enhanced).or_else(|_| run_at_path("hermes", &["--version"], &enhanced));

    match hermes_version {
        Ok(ver_raw) => {
            // 提取版本号（格式可能是 "Hermes Agent v0.8.0" 或 "0.8.0"）
            let version = ver_raw
                .split_whitespace()
                .find(|s| s.starts_with('v') || s.chars().next().is_some_and(|c| c.is_ascii_digit()))
                .unwrap_or(&ver_raw)
                .trim_start_matches('v')
                .to_string();
            result.insert("installed".into(), Value::Bool(true));
            result.insert("version".into(), Value::String(version));

            // 获取 hermes 路径
            let path = find_executable_path("hermes", &enhanced);
            result.insert("path".into(), path.map(Value::String).unwrap_or(Value::Null));
        }
        Err(_) => {
            result.insert("installed".into(), Value::Bool(false));
            result.insert("version".into(), Value::Null);
            result.insert("path".into(), Value::Null);
        }
    }

    // 2. 检测安装方式（managed）
    let managed = if let Ok(raw) = std::env::var("HERMES_MANAGED") {
        let lower = raw.trim().to_lowercase();
        match lower.as_str() {
            "true" | "1" | "yes" | "nix" | "nixos" => Some("NixOS"),
            "brew" | "homebrew" => Some("Homebrew"),
            _ => Some("unknown"),
        }
    } else if home.join(".managed").exists() {
        Some("NixOS")
    } else {
        None
    };
    result.insert("managed".into(), managed.map(|s| Value::String(s.into())).unwrap_or(Value::Null));

    // 3. 配置文件检测
    let config_path = home.join("config.yaml");
    let env_path = home.join(".env");
    result.insert("configExists".into(), Value::Bool(config_path.exists()));
    result.insert("envExists".into(), Value::Bool(env_path.exists()));
    result.insert("hermesHome".into(), Value::String(home.to_string_lossy().to_string()));

    // 4. 读取 model 配置（支持 string 和 dict 两种格式）
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        let mut found = false;
        let mut in_model_block = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("model:") {
                let val = rest.trim().trim_matches('"').trim_matches('\'').to_string();
                if !val.is_empty() {
                    // model: some_string 格式
                    result.insert("model".into(), Value::String(val));
                    found = true;
                    break;
                }
                // model: (空) 后面是 dict 块
                in_model_block = true;
                continue;
            }
            if in_model_block {
                if !line.starts_with(' ') && !line.starts_with('\t') && !trimmed.is_empty() {
                    break; // dict 块结束
                }
                if let Some(rest) = trimmed.strip_prefix("default:") {
                    let val = rest.trim().trim_matches('"').trim_matches('\'').to_string();
                    if !val.is_empty() {
                        result.insert("model".into(), Value::String(val));
                        found = true;
                    }
                }
            }
        }
        let _ = found; // suppress unused warning
    }

    // 5. Gateway 运行检测（非阻塞，快速超时）— 使用动态 URL 支持远程目标
    let gw_url = hermes_gateway_url();
    let gateway_port = hermes_gateway_port();
    // 从 URL 中提取 host:port 用于 TCP 探测
    let probe_addr = {
        let stripped = gw_url
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_end_matches('/');
        if stripped.contains(':') {
            stripped.to_string()
        } else {
            format!("{stripped}:{gateway_port}")
        }
    };
    let gateway_running = probe_addr
        .parse::<std::net::SocketAddr>()
        .map(|addr| std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(800)).is_ok())
        .unwrap_or(false);
    result.insert("gatewayRunning".into(), Value::Bool(gateway_running));
    result.insert("gatewayPort".into(), Value::Number(gateway_port.into()));
    result.insert("gatewayUrl".into(), Value::String(gw_url));

    Ok(Value::Object(result))
}

/// Hermes Gateway 默认端口
fn hermes_gateway_port() -> u16 {
    // 尝试从 config.yaml 读取自定义端口
    let config_path = hermes_home().join("config.yaml");
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        // 简单解析 YAML 中的 api_server_port 或 port
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("api_server_port:") {
                if let Ok(port) = rest.trim().parse::<u16>() {
                    if port > 0 {
                        return port;
                    }
                }
            }
        }
    }
    8642 // Hermes 默认端口
}

// ---------------------------------------------------------------------------
// install_hermes — 一键安装（下载 uv → uv tool install hermes-agent）
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn install_hermes(app: tauri::AppHandle, method: String, extras: Vec<String>) -> Result<String, String> {
    let _ = app.emit("hermes-install-log", "🚀 开始安装 Hermes Agent...");
    let _ = app.emit("hermes-install-progress", 0u32);

    // Step 1: 确保 uv 可用
    let uv_path = ensure_uv(&app).await?;
    let _ = app.emit("hermes-install-progress", 20u32);

    // Step 2: 执行安装
    match method.as_str() {
        "uv-tool" | "" => install_via_uv_tool(&app, &uv_path, &extras).await?,
        "uv-pip" => install_via_uv_pip(&app, &uv_path, &extras).await?,
        other => return Err(format!("不支持的安装方式: {other}")),
    };

    let _ = app.emit("hermes-install-progress", 90u32);

    // Step 2b: 注入 dashboard 兼容 stub（弥补上游 wheel 漏装 dashboard_auth + web_dist）
    inject_hermes_dashboard_compat_stub(&app);

    // Step 3: 验证安装
    let _ = app.emit("hermes-install-log", "🔍 验证安装...");
    let enhanced = hermes_enhanced_path();
    match run_at_path("hermes", &["version"], &enhanced) {
        Ok(ver) => {
            let _ = app.emit("hermes-install-log", format!("✅ Hermes Agent 安装成功: {ver}"));
            let _ = app.emit("hermes-install-progress", 100u32);
            let _ = app.emit("hermes-install-done", crate::jv!({ "success": true, "version": ver }));
            Ok(ver)
        }
        Err(e) => {
            let msg = format!("⚠️ 安装完成但验证失败: {e}");
            let _ = app.emit("hermes-install-log", &msg);
            let _ = app.emit("hermes-install-done", crate::jv!({ "success": false, "error": msg }));
            Err(msg)
        }
    }
}

/// 确保 uv 二进制可用，不存在则下载
async fn ensure_uv(app: &tauri::AppHandle) -> Result<String, String> {
    let uv_path = uv_bin_path();

    // 已有 uv
    if uv_path.exists() {
        let path_str = uv_path.to_string_lossy().to_string();
        if let Ok(ver) = run_silent(&path_str, &["--version"]) {
            let _ = app.emit("hermes-install-log", format!("✓ uv 已就绪: {ver}"));
            return Ok(path_str);
        }
    }

    // 系统 PATH 中有 uv
    let enhanced = hermes_enhanced_path();
    if let Ok(ver) = run_at_path("uv", &["--version"], &enhanced) {
        let _ = app.emit("hermes-install-log", format!("✓ 系统 uv 已就绪: {ver}"));
        if let Some(path) = find_executable_path("uv", &enhanced) {
            return Ok(path);
        }
        return Ok("uv".into());
    }

    // 需要下载 uv
    let _ = app.emit("hermes-install-log", "📦 下载 uv 包管理器...");
    let _ = app.emit("hermes-install-progress", 5u32);

    let version = "0.7.12"; // 稳定版本
    let url = uv_download_url(version);
    let _ = app.emit("hermes-install-log", format!("下载: {url}"));

    let client = super::build_http_client(std::time::Duration::from_secs(300), Some("ZhizhuaWorkbench"))
        .map_err(|e| format!("HTTP 客户端创建失败: {e}"))?;

    let resp = client.get(&url).send().await.map_err(|e| format!("uv 下载失败: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("uv 下载失败 (HTTP {})", resp.status()));
    }

    let bytes = resp.bytes().await.map_err(|e| format!("uv 下载读取失败: {e}"))?;

    let _ = app.emit(
        "hermes-install-log",
        format!("下载完成 ({:.1}MB)，解压中...", bytes.len() as f64 / 1_048_576.0),
    );
    let _ = app.emit("hermes-install-progress", 12u32);

    // 创建目标目录
    let bin_dir = uv_bin_dir();
    std::fs::create_dir_all(&bin_dir).map_err(|e| format!("创建目录失败: {e}"))?;

    // 解压
    #[cfg(target_os = "windows")]
    {
        extract_uv_zip(&bytes, &bin_dir)?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        extract_uv_tar_gz(&bytes, &bin_dir)?;
    }

    // 验证
    let path_str = uv_path.to_string_lossy().to_string();
    if !uv_path.exists() {
        return Err(format!("uv 解压后未找到: {}", path_str));
    }

    // Unix: 确保可执行
    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&uv_path, std::fs::Permissions::from_mode(0o755));
    }

    match run_silent(&path_str, &["--version"]) {
        Ok(ver) => {
            let _ = app.emit("hermes-install-log", format!("✓ uv 安装成功: {ver}"));
            Ok(path_str)
        }
        Err(e) => Err(format!("uv 安装后验证失败: {e}")),
    }
}

/// Windows: 解压 zip 格式的 uv 二进制
#[cfg(target_os = "windows")]
fn extract_uv_zip(data: &[u8], dest: &std::path::Path) -> Result<(), String> {
    let reader = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| format!("ZIP 解析失败: {e}"))?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("ZIP 条目读取失败: {e}"))?;
        let name = file.name().to_string();
        // 只提取 uv.exe（可能在子目录中）
        if name.ends_with("uv.exe") {
            let out_path = dest.join("uv.exe");
            let mut out_file = std::fs::File::create(&out_path).map_err(|e| format!("创建文件失败: {e}"))?;
            std::io::copy(&mut file, &mut out_file).map_err(|e| format!("写入失败: {e}"))?;
            return Ok(());
        }
    }
    Err("ZIP 中未找到 uv.exe".into())
}

/// Unix: 解压 tar.gz 格式的 uv 二进制
#[cfg(not(target_os = "windows"))]
fn extract_uv_tar_gz(data: &[u8], dest: &std::path::Path) -> Result<(), String> {
    let gz = flate2::read::GzDecoder::new(std::io::Cursor::new(data));
    let mut archive = tar::Archive::new(gz);
    for entry in archive.entries().map_err(|e| format!("tar 解析失败: {e}"))? {
        let mut entry = entry.map_err(|e| format!("tar 条目读取失败: {e}"))?;
        let path = entry.path().map_err(|e| format!("路径读取失败: {e}"))?.to_path_buf();
        if let Some(name) = path.file_name() {
            if name == "uv" {
                let out_path = dest.join("uv");
                let mut out_file = std::fs::File::create(&out_path).map_err(|e| format!("创建文件失败: {e}"))?;
                std::io::copy(&mut entry, &mut out_file).map_err(|e| format!("写入失败: {e}"))?;
                return Ok(());
            }
        }
    }
    Err("tar.gz 中未找到 uv".into())
}

const HERMES_GIT_URL: &str = "git+https://github.com/NousResearch/hermes-agent.git";

// Runtime Python deps that `hermes-agent` needs at runtime but are NOT declared as
// install-time dependencies in its `[project].dependencies` (e.g. lazy-loaded
// platform adapters). Without these, `hermes gateway run` starts but cannot bring
/// up the API server. Keep in sync between fresh install and upgrade paths.
const HERMES_RUNTIME_EXTRA_DEPS: &[&str] = &["croniter", "httpx", "openai", "aiohttp", "websockets"];

/// Append `--with <dep>` for every required runtime extra to the given command.
fn append_hermes_runtime_extras(cmd: &mut tokio::process::Command) {
    for dep in HERMES_RUNTIME_EXTRA_DEPS {
        cmd.args(["--with", dep]);
    }
}

// Human-readable `--with X --with Y ...` segment for log lines so users see the
/// exact command we ran.
fn hermes_runtime_extras_log_segment() -> String {
    HERMES_RUNTIME_EXTRA_DEPS
        .iter()
        .map(|d| format!("--with {d}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn sanitize_hermes_install_output(text: &str) -> String {
    let mut out = text.replace(HERMES_GIT_URL, "hermes-agent");
    out = out.replace("https://github.com/NousResearch/hermes-agent.git", "hermes-agent");
    out = out.replace("https://github.com/NousResearch/hermes-agent", "hermes-agent");
    out = out.replace("github.com/NousResearch/hermes-agent.git", "hermes-agent");
    out = out.replace("github.com/NousResearch/hermes-agent", "hermes-agent");
    out.replace("NousResearch/hermes-agent", "hermes-agent")
}

// 从 panelConfig.gitMirror 读取镜像前缀（如 "https://ghproxy.com/"）。
/// 为空/未设置 → 不启用镜像。
fn git_mirror_prefix() -> Option<String> {
    super::read_panel_config_value()
        .and_then(|v| v.get("gitMirror")?.as_str().map(String::from))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

// 给 tokio::process::Command 注入 git insteadOf 重写 env，
/// 进程级别（不污染用户全局 ~/.gitconfig）。仅当配置了镜像时会动作。
fn apply_git_mirror_env(cmd: &mut tokio::process::Command) {
    let Some(mirror) = git_mirror_prefix() else {
        return;
    };
    let mirror = if mirror.ends_with('/') { mirror } else { format!("{mirror}/") };
    // git 读取 GIT_CONFIG_COUNT 个临时配置项，仅影响当前进程
    cmd.env("GIT_CONFIG_COUNT", "1");
    cmd.env("GIT_CONFIG_KEY_0", format!("url.{mirror}https://github.com/.insteadOf"));
    cmd.env("GIT_CONFIG_VALUE_0", "https://github.com/");
}

// 诊断 Hermes 安装/升级输出是否命中「网络无法访问」类失败，
/// 命中返回建议文案（含「可在设置页启用 Git 镜像」提示）。
fn diagnose_install_network_error(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let hits = [
        "failed to connect to github.com",
        "could not connect to server",
        "failed to clone",
        "unable to access",
        "git operation failed",
        "connection timed out",
        "connection refused",
        "network is unreachable",
        "could not resolve host",
    ];
    if !hits.iter().any(|h| lower.contains(h)) {
        return None;
    }
    Some(
        "⚠ 检测到安装过程中无法访问外部 Git 服务。请任选一项重试：\
\n  1) 在「设置 → 网络代理」配置可用代理后重试；\
\n  2) 在「设置 → Hermes 安装镜像」填入可用的 Git 镜像前缀。"
            .to_string(),
    )
}

// 通过 uv tool install 安装 Hermes Agent
include!("install_detection/configuration_command.rs");