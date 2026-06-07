use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const LOCK_FILE_NAME: &str = "workbench.lock";

pub struct SingleInstanceGuard {
    path: PathBuf,
    pid: u32,
    _file: File,
}

impl SingleInstanceGuard {
    pub fn acquire() -> Option<Self> {
        let path = lock_file_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        match create_lock_file(&path) {
            Ok(guard) => Some(guard),
            Err(_) if stale_lock_can_be_removed(&path) => {
                let _ = fs::remove_file(&path);
                create_lock_file(&path).ok()
            }
            Err(_) => None,
        }
    }
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        if lock_file_pid(&self.path) == Some(self.pid) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn create_lock_file(path: &Path) -> std::io::Result<SingleInstanceGuard> {
    let pid = std::process::id();
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    writeln!(file, "pid={pid}")?;
    writeln!(file, "created={}", unix_timestamp())?;
    if let Ok(exe) = std::env::current_exe() {
        writeln!(file, "exe={}", exe.display())?;
    }
    file.sync_all().ok();
    Ok(SingleInstanceGuard {
        path: path.to_path_buf(),
        pid,
        _file: file,
    })
}

fn stale_lock_can_be_removed(path: &Path) -> bool {
    match lock_file_pid(path) {
        Some(pid) if pid == std::process::id() => true,
        Some(pid) => !process_is_alive(pid),
        None => true,
    }
}

fn lock_file_pid(path: &Path) -> Option<u32> {
    let content = fs::read_to_string(path).ok()?;
    content.lines().find_map(|line| {
        line.strip_prefix("pid=")
            .and_then(|raw| raw.trim().parse::<u32>().ok())
            .filter(|pid| *pid > 0)
    })
}

fn lock_file_path() -> PathBuf {
    if let Some(root) = portable_product_root() {
        return root.join("data").join("runtime").join(LOCK_FILE_NAME);
    }
    crate::commands::openclaw_dir()
        .join("runtime")
        .join(LOCK_FILE_NAME)
}

fn portable_product_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?;
    let mut candidates = vec![exe_dir.to_path_buf()];
    if let Some(root) = exe_dir.parent().and_then(|app| app.parent()) {
        candidates.push(root.to_path_buf());
    }
    for root in candidates {
        if root.join("data").join("config").is_dir() && root.join("app").is_dir() {
            return Some(root);
        }
    }
    None
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(target_os = "windows")]
fn process_is_alive(pid: u32) -> bool {
    let output = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output();
    let Ok(output) = output else {
        return true;
    };
    if !output.status.success() {
        return true;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line.contains(&pid.to_string()))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn process_is_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|status| status.success())
        .unwrap_or(true)
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn process_is_alive(_pid: u32) -> bool {
    true
}
