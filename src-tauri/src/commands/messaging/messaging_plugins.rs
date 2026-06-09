use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// OpenClaw 配置 schema 中 `plugins.entries` / `plugins.allow` 的合法 QQ 插件键。
/// 插件自身 package 声明 id 为 "qqbot"（openclaw.plugin.json）。
const OPENCLAW_QQBOT_PLUGIN_ID: &str = "qqbot";

/// 腾讯文档推荐的包；CLI 通常安装到 `~/.openclaw/extensions/openclaw-qqbot`（插件运行时 id 仍为 `qqbot`）。
const TENCENT_OPENCLAW_QQBOT_PACKAGE: &str = "@tencent-connect/openclaw-qqbot@latest";
const OPENCLAW_QQBOT_EXTENSION_FOLDER: &str = "openclaw-qqbot";
// ── QQ 插件：扩展目录可能是 ~/.openclaw/extensions/openclaw-qqbot（官方包）或旧版 qqbot 目录 ──

fn qqbot_extension_installed() -> (bool, Option<&'static str>) {
    let d1 = qqbot_plugin_dir();
    if d1.is_dir() && plugin_install_marker_exists(&d1) {
        return (true, Some("qqbot"));
    }
    let d2 = generic_plugin_dir("openclaw-qqbot");
    if d2.is_dir() && plugin_install_marker_exists(&d2) {
        return (true, Some("openclaw-qqbot"));
    }
    (false, None)
}

fn qqbot_plugins_allow_flags(cfg: &Value) -> (bool, bool) {
    let Some(arr) = cfg.get("plugins").and_then(|p| p.get("allow")).and_then(|v| v.as_array()) else {
        return (false, false);
    };
    let aq = arr.iter().any(|v| v.as_str() == Some(OPENCLAW_QQBOT_PLUGIN_ID));
    let ao = arr.iter().any(|v| v.as_str() == Some("openclaw-qqbot"));
    (aq, ao)
}

/// 移除可能导致 OpenClaw 校验失败的旧/误配置。
/// 注意：plugins.entries.qqbot 是合法的（插件 id = "qqbot"），不要删。
fn strip_legacy_qqbot_plugin_config_keys(cfg: &mut Value) {
    let Some(plugins) = cfg.get_mut("plugins").and_then(|p| p.as_object_mut()) else {
        return;
    };
    // 仅删 plugins.allow 里的误识别字符串 "openclaw-qqbot"（插件实际 id 是 qqbot）
    if let Some(allow) = plugins.get_mut("allow").and_then(|a| a.as_array_mut()) {
        allow.retain(|v| v.as_str() != Some("openclaw-qqbot"));
    }
    // plugins.entries.qqbot 本身是合法的，不删除；根级 qqbot 由 strip_ui_fields 处理
}

pub(super) fn ensure_openclaw_qqbot_plugin(cfg: &mut Value) -> Result<(), String> {
    strip_legacy_qqbot_plugin_config_keys(cfg);
    ensure_plugin_allowed(cfg, OPENCLAW_QQBOT_PLUGIN_ID)
}

fn qqbot_entry_enabled_ok(cfg: &Value, plugin_id: &str) -> bool {
    let has_entry = cfg
        .get("plugins")
        .and_then(|p| p.get("entries"))
        .and_then(|e| e.get(plugin_id))
        .is_some();
    if !has_entry {
        return true;
    }
    cfg.get("plugins")
        .and_then(|p| p.get("entries"))
        .and_then(|e| e.get(plugin_id))
        .and_then(|ent| ent.get("enabled"))
        .and_then(|v| v.as_bool())
        != Some(false)
}

/// (plugin_ok, detail_line)
pub(super) fn qqbot_plugin_diagnose(cfg: &Value) -> (bool, String) {
    let (installed, loc) = qqbot_extension_installed();
    let (allow_q, allow_o) = qqbot_plugins_allow_flags(cfg);

    let entry_id_ok = qqbot_entry_enabled_ok(cfg, OPENCLAW_QQBOT_PLUGIN_ID);
    // 与 ensure_plugin_allowed 一致：插件 id 为 qqbot，plugins.entries.qqbot + enabled 为合法配置；
    // 仅当存在该条目且 enabled=false 时判失败（不存在条目视为可接受，由一键修复补齐）。
    let plugin_ok = installed && allow_q && entry_id_ok;
    let mut detail = format!(
        "本地扩展：{}（目录：{}）；plugins.allow：qqbot={}、误识别 openclaw-qqbot={}；plugins.entries.qqbot 未禁用={}。",
        if installed {
            "已检测到插件文件"
        } else {
            "未检测到（~/.openclaw/extensions/openclaw-qqbot 或旧版 …/qqbot）"
        },
        loc.unwrap_or("—"),
        allow_q,
        allow_o,
        entry_id_ok
    );
    if allow_o && !allow_q {
        detail.push_str(" **plugins.allow 仅有 openclaw-qqbot 不够，需包含 qqbot（保存 QQ 渠道或一键修复）。**");
    } else if installed && allow_q && !entry_id_ok {
        detail.push_str(" **plugins.entries.qqbot 已存在但被禁用（enabled=false），请改为启用或删除该条目后一键修复。**");
    }
    (plugin_ok, detail)
}

/// 一键修复 QQ 插件：未安装则安装官方包并重启 Gateway；已安装则补齐 plugins.allow / entries 并重载 Gateway。
#[tauri::command]
pub async fn repair_qqbot_channel_setup(app: tauri::AppHandle) -> Result<Value, String> {
    let (installed, _loc) = qqbot_extension_installed();
    if !installed {
        install_qqbot_plugin(app.clone(), None).await?;
        return Ok(crate::jv!({
            "ok": true,
            "action": "installed",
            "message": "已安装腾讯 openclaw-qqbot 插件、写入 plugins 并已触发 Gateway 重启"
        }));
    }

    let mut cfg = super::config::load_openclaw_json()?;
    ensure_openclaw_qqbot_plugin(&mut cfg)?;
    super::config::save_openclaw_json(&cfg)?;
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = super::config::do_reload_gateway(&app2).await;
    });
    Ok(crate::jv!({
        "ok": true,
        "action": "config_repaired",
        "message": "已写入 plugins.allow / entries 并重载 Gateway"
    }))
}

#[tauri::command]
pub async fn get_channel_plugin_status(plugin_id: String) -> Result<Value, String> {
    let plugin_id = plugin_id.trim();
    if plugin_id.is_empty() {
        return Err("plugin_id 不能为空".into());
    }

    let plugin_dir = generic_plugin_dir(plugin_id);
    let (qq_ext_ok, qq_ext_loc) = if plugin_id == OPENCLAW_QQBOT_PLUGIN_ID {
        qqbot_extension_installed()
    } else {
        (false, None)
    };
    // QQ 官方包落在 extensions/openclaw-qqbot，运行时插件 id 仍为 qqbot
    let installed = if plugin_id == OPENCLAW_QQBOT_PLUGIN_ID {
        qq_ext_ok
    } else {
        plugin_dir.is_dir() && plugin_install_marker_exists(&plugin_dir)
    };
    let path_display: PathBuf = if plugin_id == OPENCLAW_QQBOT_PLUGIN_ID {
        match qq_ext_loc {
            Some("openclaw-qqbot") => generic_plugin_dir(OPENCLAW_QQBOT_EXTENSION_FOLDER),
            Some("qqbot") => qqbot_plugin_dir(),
            _ => generic_plugin_dir(OPENCLAW_QQBOT_EXTENSION_FOLDER),
        }
    } else {
        plugin_dir.clone()
    };
    let legacy_backup_detected = legacy_plugin_backup_dir(plugin_id).exists();

    // 检测插件是否为 OpenClaw 内置（新版 OpenClaw 运行时打包了 feishu 等插件）
    let builtin = is_plugin_builtin(plugin_id);

    let cfg = super::config::load_openclaw_json().unwrap_or_else(|_| crate::jv!({}));
    let allowed = cfg
        .get("plugins")
        .and_then(|p| p.get("allow"))
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().any(|v| v.as_str() == Some(plugin_id)))
        .unwrap_or(false);
    let enabled = cfg
        .get("plugins")
        .and_then(|p| p.get("entries"))
        .and_then(|e| e.get(plugin_id))
        .and_then(|entry| entry.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Ok(crate::jv!({
        "installed": installed,
        "builtin": builtin,
        "path": path_display.to_string_lossy(),
        "allowed": allowed,
        "enabled": enabled,
        "legacyBackupDetected": legacy_backup_detected
    }))
}

#[tauri::command]
pub async fn list_all_plugins() -> Result<Value, String> {
    let cfg = super::config::load_openclaw_json().unwrap_or_else(|_| crate::jv!({}));
    let entries = cfg
        .pointer("/plugins/entries")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let allow_arr = cfg
        .pointer("/plugins/allow")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let ext_dir = super::openclaw_dir().join("extensions");
    let mut plugins: Vec<Value> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Scan extensions directory
    if ext_dir.is_dir() {
        if let Ok(rd) = std::fs::read_dir(&ext_dir) {
            for entry in rd.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                let p = entry.path();
                if !p.is_dir() {
                    continue;
                }
                let has_marker =
                    p.join("package.json").is_file() || p.join("plugin.ts").is_file() || p.join("index.js").is_file();
                if !has_marker {
                    continue;
                }

                let plugin_id = name.clone();
                seen.insert(plugin_id.clone());

                let entry_cfg = entries.get(&plugin_id);
                let enabled = entry_cfg
                    .and_then(|e| e.get("enabled"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let allowed = allow_arr.iter().any(|v| v.as_str() == Some(&plugin_id));
                let builtin = is_plugin_builtin(&plugin_id);

                // Try to read version from package.json
                let version = std::fs::read_to_string(p.join("package.json"))
                    .ok()
                    .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                    .and_then(|v| v.get("version").and_then(|v| v.as_str().map(String::from)));

                let description = std::fs::read_to_string(p.join("package.json"))
                    .ok()
                    .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                    .and_then(|v| v.get("description").and_then(|v| v.as_str().map(String::from)));

                plugins.push(crate::jv!({
                    "id": plugin_id,
                    "installed": true,
                    "builtin": builtin,
                    "enabled": enabled,
                    "allowed": allowed,
                    "version": version,
                    "description": description,
                    "config": entry_cfg.and_then(|e| e.get("config")),
                }));
            }
        }
    }

    // Also include entries from config that might not be in extensions dir (built-in)
    for (pid, entry_val) in &entries {
        if seen.contains(pid.as_str()) {
            continue;
        }
        seen.insert(pid.clone());
        let enabled = entry_val.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
        let allowed = allow_arr.iter().any(|v| v.as_str() == Some(pid.as_str()));
        let builtin = is_plugin_builtin(pid);
        plugins.push(crate::jv!({
            "id": pid,
            "installed": builtin,
            "builtin": builtin,
            "enabled": enabled,
            "allowed": allowed,
            "version": crate::jv!(null),
            "description": crate::jv!(null),
            "config": entry_val.get("config"),
        }));
    }

    plugins.sort_by(|a, b| {
        let ae = a.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
        let be = b.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
        be.cmp(&ae).then_with(|| {
            let an = a.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let bn = b.get("id").and_then(|v| v.as_str()).unwrap_or("");
            an.cmp(bn)
        })
    });

    Ok(crate::jv!({ "plugins": plugins }))
}

#[tauri::command]
pub async fn toggle_plugin(plugin_id: String, enabled: bool) -> Result<Value, String> {
    let plugin_id = plugin_id.trim();
    if plugin_id.is_empty() {
        return Err("plugin_id 不能为空".into());
    }

    let mut cfg = super::config::load_openclaw_json().unwrap_or_else(|_| crate::jv!({}));

    if enabled {
        ensure_plugin_allowed(&mut cfg, plugin_id)?;
    } else {
        disable_legacy_plugin(&mut cfg, plugin_id);
    }

    // 使用 save_openclaw_json 写入（含备份和 UI 字段清理），而非直接 fs::write
    super::config::save_openclaw_json(&cfg)?;

    Ok(crate::jv!({ "ok": true, "enabled": enabled, "pluginId": plugin_id }))
}

#[tauri::command]
pub async fn install_plugin(package_name: String) -> Result<Value, String> {
    let package_name = package_name.trim().to_string();
    if package_name.is_empty() {
        return Err("包名不能为空".into());
    }

    let cli = crate::utils::resolve_openclaw_cli_path().ok_or_else(|| "找不到 OpenClaw CLI，请先安装".to_string())?;
    let output = std::process::Command::new(&cli)
        .args(["plugins", "install", &package_name])
        .current_dir(dirs::home_dir().unwrap_or_default())
        .output()
        .map_err(|e| format!("执行 openclaw plugins install 失败: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(format!("安装失败: {}{}", stdout, stderr));
    }

    Ok(crate::jv!({ "ok": true, "output": format!("{}{}", stdout, stderr).trim().to_string() }))
}

pub(super) fn ensure_plugin_allowed(cfg: &mut Value, plugin_id: &str) -> Result<(), String> {
    let root = cfg.as_object_mut().ok_or("配置格式错误")?;
    let plugins = root.entry("plugins").or_insert_with(|| crate::jv!({}));
    let plugins_map = plugins.as_object_mut().ok_or("plugins 节点格式错误")?;

    let allow = plugins_map.entry("allow").or_insert_with(|| crate::jv!([]));
    let allow_arr = allow.as_array_mut().ok_or("plugins.allow 节点格式错误")?;
    if !allow_arr.iter().any(|v| v.as_str() == Some(plugin_id)) {
        allow_arr.push(Value::String(plugin_id.to_string()));
    }

    let entries = plugins_map.entry("entries").or_insert_with(|| crate::jv!({}));
    let entries_map = entries.as_object_mut().ok_or("plugins.entries 节点格式错误")?;
    let entry = entries_map.entry(plugin_id.to_string()).or_insert_with(|| crate::jv!({}));
    let entry_obj = entry.as_object_mut().ok_or("plugins.entries 条目格式错误")?;
    entry_obj.insert("enabled".into(), Value::Bool(true));
    Ok(())
}

/// 禁用旧版插件：在 plugins.entries 中设置 enabled=false，并从 plugins.allow 中移除
pub(super) fn disable_legacy_plugin(cfg: &mut Value, plugin_id: &str) {
    if let Some(root) = cfg.as_object_mut() {
        if let Some(plugins) = root.get_mut("plugins").and_then(|p| p.as_object_mut()) {
            // 从 allow 列表中移除
            if let Some(allow) = plugins.get_mut("allow").and_then(|a| a.as_array_mut()) {
                allow.retain(|v| v.as_str() != Some(plugin_id));
            }
            // 在 entries 中设置 enabled=false
            if let Some(entries) = plugins.get_mut("entries").and_then(|e| e.as_object_mut()) {
                if let Some(entry) = entries.get_mut(plugin_id).and_then(|e| e.as_object_mut()) {
                    entry.insert("enabled".into(), Value::Bool(false));
                }
            }
        }
    }
}

include!("messaging_plugins_modules/install.rs");
