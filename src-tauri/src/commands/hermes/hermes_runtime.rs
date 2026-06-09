use std::path::PathBuf;
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub(super) const HERMES_DASHBOARD_SESSION_TOKEN: &str = "zhizhua-platform-local-dashboard";

pub(super) fn hermes_home() -> PathBuf {
    if let Ok(h) = std::env::var("HERMES_HOME") {
        return PathBuf::from(h);
    }
    if let Some(root) = super::portable_product_root() {
        return root.join("data").join("config").join("hermes");
    }
    dirs::home_dir().unwrap_or_default().join(".hermes")
}

pub(super) fn portable_hermes_runtime_dir() -> Option<PathBuf> {
    super::portable_product_root().map(|root| root.join("data").join("runtime").join("hermes"))
}

pub(super) fn hermes_uv_tool_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("UV_TOOL_DIR") {
        let dir = dir.trim();
        if !dir.is_empty() {
            return Some(PathBuf::from(dir));
        }
    }
    portable_hermes_runtime_dir().map(|dir| dir.join("tools"))
}

pub(super) fn hermes_uv_tool_bin_dir() -> Option<PathBuf> {
    hermes_uv_tool_dir().map(|dir| dir.join("bin"))
}

pub(super) fn hermes_uv_tool_venv_bin_dir() -> Option<PathBuf> {
    hermes_uv_tool_dir().map(|dir| {
        if cfg!(target_os = "windows") {
            dir.join("hermes-agent").join("Scripts")
        } else {
            dir.join("hermes-agent").join("bin")
        }
    })
}

pub(super) fn hermes_executable_path() -> Option<PathBuf> {
    let name = if cfg!(target_os = "windows") { "hermes.exe" } else { "hermes" };
    hermes_uv_tool_venv_bin_dir()
        .map(|dir| dir.join(name))
        .filter(|path| path.is_file())
}

pub(super) fn hermes_uv_cache_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("UV_CACHE_DIR") {
        let dir = dir.trim();
        if !dir.is_empty() {
            return Some(PathBuf::from(dir));
        }
    }
    super::portable_product_root().map(|root| root.join("data").join("cache").join("hermes").join("uv-cache"))
}

pub(super) fn hermes_venv_dir() -> PathBuf {
    portable_hermes_runtime_dir()
        .map(|dir| dir.join("venv"))
        .or_else(|| dirs::home_dir().map(|home| home.join(".hermes-venv")))
        .unwrap_or_else(|| PathBuf::from(".hermes-venv"))
}

pub(super) fn apply_hermes_runtime_env_std(cmd: &mut std::process::Command) {
    cmd.env("HERMES_HOME", hermes_home());
    cmd.env("HERMES_DASHBOARD_SESSION_TOKEN", HERMES_DASHBOARD_SESSION_TOKEN);
    if let Some(tool_dir) = hermes_uv_tool_dir() {
        cmd.env("UV_TOOL_DIR", tool_dir);
    }
    if let Some(cache_dir) = hermes_uv_cache_dir() {
        cmd.env("UV_CACHE_DIR", cache_dir);
    }
}

pub(super) fn apply_hermes_runtime_env_tokio(cmd: &mut tokio::process::Command) {
    cmd.env("HERMES_HOME", hermes_home());
    cmd.env("HERMES_DASHBOARD_SESSION_TOKEN", HERMES_DASHBOARD_SESSION_TOKEN);
    if let Some(tool_dir) = hermes_uv_tool_dir() {
        cmd.env("UV_TOOL_DIR", tool_dir);
    }
    if let Some(cache_dir) = hermes_uv_cache_dir() {
        cmd.env("UV_CACHE_DIR", cache_dir);
    }
}

pub(super) fn uv_bin_dir() -> PathBuf {
    if let Some(dir) = portable_hermes_runtime_dir() {
        return dir.join("uv").join("bin");
    }
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        if !appdata.is_empty() {
            return PathBuf::from(appdata).join("clawpanel").join("bin");
        }
        dirs::home_dir().unwrap_or_default().join(".clawpanel").join("bin")
    }
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .unwrap_or_default()
            .join("Library")
            .join("Application Support")
            .join("clawpanel")
            .join("bin")
    }
    #[cfg(target_os = "linux")]
    {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".local")
            .join("share")
            .join("clawpanel")
            .join("bin")
    }
}

pub(super) fn uv_bin_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        uv_bin_dir().join("uv.exe")
    }
    #[cfg(not(target_os = "windows"))]
    {
        uv_bin_dir().join("uv")
    }
}

pub(super) fn uv_download_url(version: &str) -> String {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    let filename = "uv-x86_64-pc-windows-msvc.zip";
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let filename = "uv-aarch64-apple-darwin.tar.gz";
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    let filename = "uv-x86_64-apple-darwin.tar.gz";
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let filename = "uv-x86_64-unknown-linux-gnu.tar.gz";
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    let filename = "uv-aarch64-unknown-linux-gnu.tar.gz";

    format!("https://github.com/astral-sh/uv/releases/download/{version}/{filename}")
}

pub(super) fn hermes_enhanced_path() -> String {
    let current = std::env::var("PATH").unwrap_or_default();
    let home = dirs::home_dir().unwrap_or_default();
    let mut extra: Vec<String> = vec![uv_bin_dir().to_string_lossy().to_string()];

    if let Some(bin) = hermes_uv_tool_bin_dir() {
        extra.push(bin.to_string_lossy().to_string());
    }
    if let Some(bin) = hermes_uv_tool_venv_bin_dir() {
        extra.push(bin.to_string_lossy().to_string());
    }
    if let Some(runtime) = portable_hermes_runtime_dir() {
        #[cfg(target_os = "windows")]
        extra.push(runtime.join("venv").join("Scripts").to_string_lossy().to_string());
        #[cfg(not(target_os = "windows"))]
        extra.push(runtime.join("venv").join("bin").to_string_lossy().to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        if !appdata.is_empty() {
            extra.push(format!(r"{}\uv\tools\bin", appdata));
        }
        extra.push(format!(r"{}\.local\bin", home.display()));
        extra.push(format!(r"{}\.local\bin", home.display()));
        extra.push(format!(r"{}\.cargo\bin", home.display()));
    }
    #[cfg(not(target_os = "windows"))]
    {
        extra.push(format!("{}/.local/bin", home.display()));
        extra.push(format!("{}/.cargo/bin", home.display()));
        extra.push("/usr/local/bin".into());
    }

    let sep = if cfg!(target_os = "windows") { ";" } else { ":" };
    let mut parts: Vec<&str> = extra.iter().map(|s| s.as_str()).collect();
    if !current.is_empty() {
        parts.push(&current);
    }
    parts.join(sep)
}

pub(super) fn run_silent(program: &str, args: &[&str]) -> Result<String, String> {
    let enhanced = hermes_enhanced_path();
    let mut cmd = Command::new(program);
    cmd.args(args).env("PATH", &enhanced);
    apply_hermes_runtime_env_std(&mut cmd);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let output = cmd.output().map_err(|e| format!("{program}: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(stderr)
    }
}

pub(super) fn run_hermes_silent(args: &[&str]) -> Result<String, String> {
    let hermes_cmd = hermes_executable_path().unwrap_or_else(|| PathBuf::from("hermes"));
    let enhanced = hermes_enhanced_path();
    let mut cmd = Command::new(&hermes_cmd);
    cmd.args(args).env("PATH", &enhanced);
    apply_hermes_runtime_env_std(&mut cmd);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let output = cmd.output().map_err(|e| format!("{}: {e}", hermes_cmd.display()))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(stderr)
    }
}

pub(super) fn run_at_path(program: &str, args: &[&str], path: &str) -> Result<String, String> {
    let mut cmd = Command::new(program);
    cmd.args(args).env("PATH", path);
    apply_hermes_runtime_env_std(&mut cmd);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let output = cmd.output().map_err(|e| format!("{program}: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub(super) fn parse_python_version(raw: &str) -> Option<(u32, u32, u32)> {
    let version_str = raw.strip_prefix("Python ").unwrap_or(raw);
    let parts: Vec<&str> = version_str.trim().split('.').collect();
    if parts.len() >= 2 {
        let major = parts.first().and_then(|part| part.parse().ok())?;
        let minor = parts.get(1).and_then(|part| part.parse().ok())?;
        let patch = parts.get(2).and_then(|part| part.parse().ok()).unwrap_or(0);
        Some((major, minor, patch))
    } else {
        None
    }
}

/// Resolve `<uv tool dir>/hermes-agent` — the venv root that `uv tool install`
/// creates. Returns `None` if `uv` is unavailable or hermes-agent isn't installed
/// via the uv-tool path (e.g. user is on the legacy `~/.hermes-venv` uv-pip path).
pub(super) fn hermes_uv_tool_root() -> Option<std::path::PathBuf> {
    let uv_path = uv_bin_path();
    let uv_cmd = if uv_path.exists() {
        uv_path.to_string_lossy().to_string()
    } else {
        "uv".into()
    };
    let mut cmd = std::process::Command::new(&uv_cmd);
    cmd.args(["tool", "dir"]);
    cmd.env("PATH", hermes_enhanced_path());
    apply_hermes_runtime_env_std(&mut cmd);
    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return None;
    }
    let root = std::path::PathBuf::from(&stdout).join("hermes-agent");
    if root.exists() {
        Some(root)
    } else {
        None
    }
}

/// Locate the Python interpreter inside the uv-tool hermes-agent venv.
///
/// Layouts vary by platform:
///   - Windows: `<uv tool dir>/hermes-agent/Scripts/python.exe`
///   - macOS / Linux: `<uv tool dir>/hermes-agent/bin/python`
pub(super) fn hermes_uv_tool_python() -> Option<std::path::PathBuf> {
    let root = hermes_uv_tool_root()?;
    #[cfg(target_os = "windows")]
    let py = root.join("Scripts").join("python.exe");
    #[cfg(not(target_os = "windows"))]
    let py = root.join("bin").join("python");
    if py.exists() {
        Some(py)
    } else {
        None
    }
}

/// Locate the installed `hermes_cli` package directory inside the uv tool venv.
///
/// Layouts vary by platform:
///   - Windows: `<uv tool dir>/hermes-agent/Lib/site-packages/hermes_cli`
///   - macOS / Linux: `<uv tool dir>/hermes-agent/lib/python3.X/site-packages/hermes_cli`
///
/// Returns `None` if uv is unavailable or hermes-agent is not installed.
pub(super) fn locate_hermes_cli_package_dir() -> Option<std::path::PathBuf> {
    let hermes_root = hermes_uv_tool_root()?;

    let windows_path = hermes_root.join("Lib").join("site-packages").join("hermes_cli");
    if windows_path.exists() {
        return Some(windows_path);
    }
    let lib_dir = hermes_root.join("lib");
    if let Ok(entries) = std::fs::read_dir(&lib_dir) {
        for entry in entries.flatten() {
            let pkg = entry.path().join("site-packages").join("hermes_cli");
            if pkg.exists() {
                return Some(pkg);
            }
        }
    }
    None
}
