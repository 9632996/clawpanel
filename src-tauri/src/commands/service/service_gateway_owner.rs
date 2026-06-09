use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct GatewayOwnerRecord {
    pub(super) pid: Option<u32>,
    port: u16,
    cli_path: Option<String>,
    openclaw_dir: String,
    started_at: String,
    started_by: String,
}

pub(super) fn read_gateway_owner() -> Option<GatewayOwnerRecord> {
    let content = std::fs::read_to_string(gateway_owner_path()).ok()?;
    serde_json::from_str(&content).ok()
}

pub(super) fn write_gateway_owner(pid: Option<u32>) -> Result<(), String> {
    let owner_path = gateway_owner_path();
    if let Some(parent) = owner_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建 Gateway owner 目录失败: {e}"))?;
    }
    let (port, openclaw_dir, cli_path) = current_gateway_owner_signature();
    let record = GatewayOwnerRecord {
        pid,
        port,
        cli_path,
        openclaw_dir,
        started_at: chrono::Local::now().to_rfc3339(),
        started_by: "clawpanel".into(),
    };
    let content = serde_json::to_string_pretty(&record).map_err(|e| format!("序列化 Gateway owner 失败: {e}"))?;
    std::fs::write(owner_path, content).map_err(|e| format!("写入 Gateway owner 失败: {e}"))
}

pub(super) fn clear_gateway_owner() {
    let _ = std::fs::remove_file(gateway_owner_path());
}

pub(super) fn is_current_gateway_owner(owner: &GatewayOwnerRecord, _pid: Option<u32>) -> bool {
    matches_current_gateway_owner_signature(owner)
}

pub(super) fn should_auto_claim_gateway(owner: &Option<GatewayOwnerRecord>) -> bool {
    let (port, openclaw_dir, _cli_path) = current_gateway_owner_signature();
    match owner {
        None => true,
        Some(record) => record.port == port && normalize_owned_path(&record.openclaw_dir) == openclaw_dir,
    }
}

pub(super) fn gateway_owner_pid_needs_refresh(owner: &GatewayOwnerRecord, pid: Option<u32>) -> bool {
    matches_current_gateway_owner_signature(owner) && matches!(pid, Some(current_pid) if owner.pid != Some(current_pid))
}

pub(super) fn ensure_owned_gateway_or_err(pid: Option<u32>) -> Result<(), String> {
    let owner = read_gateway_owner();
    if let Some(ref record) = owner {
        if is_current_gateway_owner(record, pid) {
            if gateway_owner_pid_needs_refresh(record, pid) {
                write_gateway_owner(pid)?;
            }
            return Ok(());
        }
    }
    if should_auto_claim_gateway(&owner) {
        write_gateway_owner(pid)?;
        return Ok(());
    }
    Err(foreign_gateway_error(pid))
}

fn gateway_owner_path() -> std::path::PathBuf {
    crate::commands::openclaw_dir().join("gateway-owner.json")
}

fn current_gateway_owner_signature() -> (u16, String, Option<String>) {
    let openclaw_dir = normalize_owned_path(crate::commands::openclaw_dir());
    let cli_path = crate::utils::resolve_openclaw_cli_path().map(|p| normalize_owned_path(std::path::PathBuf::from(p)));
    (crate::commands::gateway_listen_port(), openclaw_dir, cli_path)
}

fn matches_current_gateway_owner_signature(owner: &GatewayOwnerRecord) -> bool {
    if owner.started_by != "clawpanel" {
        return false;
    }
    let (port, openclaw_dir, cli_path) = current_gateway_owner_signature();
    if owner.port != port {
        return false;
    }
    if normalize_owned_path(&owner.openclaw_dir) != openclaw_dir {
        return false;
    }
    let owner_cli_path = owner.cli_path.as_ref().map(normalize_owned_path);
    match (owner_cli_path.as_deref(), cli_path.as_deref()) {
        (Some(a), Some(b)) => a == b,
        _ => true,
    }
}

fn foreign_gateway_error(pid: Option<u32>) -> String {
    let pid_suffix = pid.map(|value| format!(" (PID: {value})")).unwrap_or_default();
    format!(
        "检测到端口 {} 上已有其他 OpenClaw Gateway 正在运行{}，且不属于当前面板实例。为避免误接管，请先关闭该实例，或将当前 CLI/目录绑定到它对应的安装。",
        crate::commands::gateway_listen_port(),
        pid_suffix
    )
}

fn normalize_owned_path(path: impl AsRef<std::path::Path>) -> String {
    let path_ref = path.as_ref();
    path_ref
        .canonicalize()
        .unwrap_or_else(|_| path_ref.to_path_buf())
        .to_string_lossy()
        .to_string()
}
