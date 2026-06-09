
async fn status_summary_fallback(reason: Option<String>) -> Value {
    let config = load_openclaw_json().ok();
    let default_model = config
        .as_ref()
        .and_then(|v| v.pointer("/agents/defaults/model/primary"))
        .and_then(|v| v.as_str())
        .map(|v| Value::String(v.to_string()))
        .unwrap_or(Value::Null);
    let default_context_tokens = config
        .as_ref()
        .and_then(|v| v.pointer("/agents/defaults/contextTokens"))
        .and_then(|v| v.as_u64())
        .map(|v| Value::Number(v.into()))
        .unwrap_or(Value::Null);
    let runtime_version = get_local_version().await.map(Value::String).unwrap_or(Value::Null);
    let mut value = crate::jv!({
        "source": "file-read",
        "runtimeVersion": runtime_version,
        "sessions": {
            "count": 0,
            "recent": [],
            "defaults": {
                "model": default_model,
                "contextTokens": default_context_tokens
            }
        }
    });
    if let Some(reason) = reason {
        if let Some(obj) = value.as_object_mut() {
            obj.insert("statusError".into(), Value::String(reason));
        }
    }
    value
}

fn is_portable_runtime_config_dir() -> bool {
    let config_dir = super::openclaw_dir();
    let Some(data_dir) = config_dir.parent() else {
        return false;
    };
    data_dir.join("config") == config_dir
        && data_dir.join("state").is_dir()
        && data_dir.join("cache").is_dir()
        && data_dir.join("logs").is_dir()
}

/// npm 包名映射
fn npm_package_name(source: &str) -> &'static str {
    match source {
        "official" => "openclaw",
        _ => "openclaw",
    }
}

/// 获取指定源的所有可用版本列表（从 npm registry 查询）
#[tauri::command]
pub async fn list_openclaw_versions(source: String) -> Result<Vec<String>, String> {
    let client = crate::commands::build_http_client(std::time::Duration::from_secs(10), None)
        .map_err(|e| format!("HTTP 初始化失败: {e}"))?;
    let pkg = npm_package_name(&source).replace('/', "%2F");
    let registry = get_configured_registry();
    let url = format!("{registry}/{pkg}");
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("查询版本失败: {e}"))?;
    let json: Value = resp.json().await.map_err(|e| format!("解析响应失败: {e}"))?;
    let mut versions = json
        .get("versions")
        .and_then(|v| v.as_object())
        .map(|obj| {
            let mut vers: Vec<String> = obj.keys().cloned().collect();
            vers.sort_by(|a, b| {
                let pa = parse_version(a);
                let pb = parse_version(b);
                pb.cmp(&pa)
            });
            vers
        })
        .unwrap_or_default();
    if let Some(recommended) = recommended_version_for(&source).await {
        if let Some(pos) = versions.iter().position(|v| v == &recommended) {
            let version = versions.remove(pos);
            versions.insert(0, version);
        } else {
            versions.insert(0, recommended);
        }
    }
    Ok(versions)
}

// 执行 npm 全局安装/升级/降级 openclaw（后台执行，通过 event 推送进度）
/// 立即返回，不阻塞前端。完成后 emit "upgrade-done" 或 "upgrade-error"。
#[tauri::command]
pub async fn upgrade_openclaw(
    app: tauri::AppHandle,
    source: String,
    version: Option<String>,
    method: Option<String>,
) -> Result<String, String> {
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        use tauri::Emitter;
        let result = upgrade_openclaw_inner(app2.clone(), source, version, method.unwrap_or_else(|| "auto".into())).await;
        match result {
            Ok(msg) => {
                let _ = app2.emit("upgrade-done", &msg);
            }
            Err(err) => {
                let _ = app2.emit("upgrade-error", &err);
            }
        }
    });
    Ok("任务已启动".into())
}

/// 检测当前平台标识（用于 R2 归档文件名）
#[allow(dead_code)]
fn r2_platform_key() -> &'static str {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "win-x64"
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "darwin-arm64"
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        "darwin-x64"
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

/// npm 全局 node_modules 目录
#[allow(dead_code)]
fn npm_global_modules_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        super::windows_npm_global_prefix()
            .map(|prefix| PathBuf::from(prefix).join("node_modules"))
            .or_else(|| {
                std::env::var("APPDATA")
                    .ok()
                    .map(|a| PathBuf::from(a).join("npm").join("node_modules"))
            })
    }
    #[cfg(target_os = "macos")]
    {
        // homebrew 或系统 node
        let brew = PathBuf::from("/opt/homebrew/lib/node_modules");
        if brew.exists() {
            return Some(brew);
        }
        let sys = PathBuf::from("/usr/local/lib/node_modules");
        if sys.exists() {
            return Some(sys);
        }
        Some(brew) // fallback to homebrew path
    }
    #[cfg(target_os = "linux")]
    {
        // 尝试 npm config get prefix
        if let Ok(output) = Command::new("npm").args(["config", "get", "prefix"]).output() {
            let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !prefix.is_empty() {
                return Some(PathBuf::from(prefix).join("lib").join("node_modules"));
            }
        }
        Some(PathBuf::from("/usr/local/lib/node_modules"))
    }
}

/// npm 全局 bin 目录
#[allow(dead_code)]
fn npm_global_bin_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        super::windows_npm_global_prefix()
            .map(PathBuf::from)
            .or_else(|| std::env::var("APPDATA").ok().map(|a| PathBuf::from(a).join("npm")))
    }
    #[cfg(target_os = "macos")]
    {
        let brew = PathBuf::from("/opt/homebrew/bin");
        if brew.exists() {
            return Some(brew);
        }
        Some(PathBuf::from("/usr/local/bin"))
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = Command::new("npm").args(["config", "get", "prefix"]).output() {
            let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !prefix.is_empty() {
                return Some(PathBuf::from(prefix).join("bin"));
            }
        }
        Some(PathBuf::from("/usr/local/bin"))
    }
}

// 尝试从 standalone 独立安装包安装 OpenClaw（自带 Node.js，零依赖）
// 动态查询 latest.json 获取最新版本，下载对应平台的归档并解压
// 成功返回 Ok(版本号)，失败返回 Err(原因) 供 caller 降级到 R2/npm
include!("openclaw_upgrade/standalone_installers.rs");