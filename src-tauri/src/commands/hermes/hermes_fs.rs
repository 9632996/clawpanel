use serde_json::Value;
use std::path::PathBuf;

use super::hermes_runtime::hermes_home;

const FS_MAX_READ_BYTES: u64 = 5 * 1024 * 1024; // 5 MB
const FS_MAX_LIST_ENTRIES: usize = 2000; // 单次最多返回 2000 条

/// 验证路径在 hermes_home 子树内（防 path traversal）。
/// 返回安全的绝对路径，或 Err。
fn validate_hermes_fs_path(rel_path: &str) -> Result<PathBuf, String> {
    let root = hermes_home();
    // 空 = 根目录
    let target = if rel_path.is_empty() {
        root.clone()
    } else {
        // 拒绝绝对路径输入（必须相对于 hermes_home）
        let p = std::path::Path::new(rel_path);
        if p.is_absolute() {
            // 允许绝对路径，但必须以 root 开头（用 starts_with 检查）
            let canonical_root = root.canonicalize().unwrap_or(root.clone());
            let canonical_target = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
            if !canonical_target.starts_with(&canonical_root) {
                return Err(format!("路径必须在 {} 子树内", root.to_string_lossy()));
            }
            canonical_target
        } else {
            // 相对路径：拼到 root 下，再 canonicalize 防 ..
            let joined = root.join(p);
            // 父目录必须存在才能 canonicalize；对不存在的新文件 fallback 到 joined
            let canon = joined.canonicalize().unwrap_or(joined.clone());
            let canonical_root = root.canonicalize().unwrap_or(root.clone());
            if !canon.starts_with(&canonical_root) {
                return Err(format!("路径不能跳出 {} 目录", root.to_string_lossy()));
            }
            canon
        }
    };
    Ok(target)
}

#[tauri::command]
pub async fn hermes_fs_list(path: String) -> Result<Value, String> {
    let target = validate_hermes_fs_path(&path)?;
    if !target.exists() {
        return Err(format!("目录不存在: {}", target.to_string_lossy()));
    }
    if !target.is_dir() {
        return Err(format!("不是目录: {}", target.to_string_lossy()));
    }
    let mut entries = Vec::new();
    let read_dir = std::fs::read_dir(&target).map_err(|e| format!("读取目录失败: {e}"))?;
    for entry in read_dir.flatten().take(FS_MAX_LIST_ENTRIES) {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') && name != ".env" && name != ".hermes" {
            continue; // 隐藏文件默认不显示（.env 除外因为 Hermes 用它）
        }
        let ft = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let meta = entry.metadata().ok();
        let size = meta.as_ref().and_then(|m| if m.is_file() { Some(m.len()) } else { None });
        let modified = meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs()));
        entries.push(crate::jv!({
            "name": name,
            "kind": if ft.is_dir() { "dir" } else if ft.is_symlink() { "symlink" } else { "file" },
            "size": size,
            "modified": modified,
        }));
    }
    // 目录在前，文件在后，每组按名字排序
    entries.sort_by(|a, b| {
        let ak = a.get("kind").and_then(|v| v.as_str()).unwrap_or("");
        let bk = b.get("kind").and_then(|v| v.as_str()).unwrap_or("");
        if ak != bk {
            return if ak == "dir" {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        let an = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let bn = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
        an.to_lowercase().cmp(&bn.to_lowercase())
    });
    Ok(crate::jv!({
        "path": target.to_string_lossy(),
        "entries": entries,
    }))
}

#[tauri::command]
pub async fn hermes_fs_read(path: String) -> Result<Value, String> {
    let target = validate_hermes_fs_path(&path)?;
    if !target.exists() {
        return Err(format!("文件不存在: {}", target.to_string_lossy()));
    }
    if !target.is_file() {
        return Err(format!("不是文件: {}", target.to_string_lossy()));
    }
    let meta = target.metadata().map_err(|e| format!("读元数据失败: {e}"))?;
    if meta.len() > FS_MAX_READ_BYTES {
        return Err(format!("文件过大（{} bytes），最大 {} bytes", meta.len(), FS_MAX_READ_BYTES));
    }
    let content = std::fs::read(&target).map_err(|e| format!("读取失败: {e}"))?;
    // 尝试当作 UTF-8 文本；失败 → 二进制（用 base64）
    let (text_content, binary_b64) = match std::str::from_utf8(&content) {
        Ok(s) => (Some(s.to_string()), None),
        Err(_) => {
            // 简单的非文本判定（包含 null byte 即认为是二进制）
            (None, Some(base64_encode(&content)))
        }
    };
    Ok(crate::jv!({
        "path": target.to_string_lossy(),
        "size": meta.len(),
        "text": text_content,
        "binary_b64": binary_b64,
    }))
}

/// 简单的 base64 编码（不引新依赖）
fn base64_encode(bytes: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let n = (u32::from(bytes[i]) << 16) | (u32::from(bytes[i + 1]) << 8) | u32::from(bytes[i + 2]);
        out.push(CHARS[((n >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((n >> 12) & 0x3F) as usize] as char);
        out.push(CHARS[((n >> 6) & 0x3F) as usize] as char);
        out.push(CHARS[(n & 0x3F) as usize] as char);
        i += 3;
    }
    let rem = bytes.len() - i;
    if rem == 1 {
        let n = u32::from(bytes[i]) << 16;
        out.push(CHARS[((n >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((n >> 12) & 0x3F) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = (u32::from(bytes[i]) << 16) | (u32::from(bytes[i + 1]) << 8);
        out.push(CHARS[((n >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((n >> 12) & 0x3F) as usize] as char);
        out.push(CHARS[((n >> 6) & 0x3F) as usize] as char);
        out.push('=');
    }
    out
}

#[tauri::command]
pub async fn hermes_fs_write(path: String, content: String) -> Result<Value, String> {
    let target = validate_hermes_fs_path(&path)?;
    // 父目录必须存在
    if let Some(parent) = target.parent() {
        if !parent.exists() {
            return Err(format!("父目录不存在: {}", parent.to_string_lossy()));
        }
    }
    // 写入大小限制（防止巨型文件意外写入）
    if content.len() as u64 > FS_MAX_READ_BYTES {
        return Err(format!("内容过大（{} bytes），最大 {} bytes", content.len(), FS_MAX_READ_BYTES));
    }
    std::fs::write(&target, content.as_bytes()).map_err(|e| format!("写入失败: {e}"))?;
    let meta = target.metadata().ok();
    Ok(crate::jv!({
        "path": target.to_string_lossy(),
        "size": meta.map(|m| m.len()).unwrap_or(0),
    }))
}
