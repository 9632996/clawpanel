fn plugin_backup_root() -> PathBuf {
    super::openclaw_dir().join("backups").join("plugin-installs")
}

fn qqbot_plugin_dir() -> PathBuf {
    super::openclaw_dir().join("extensions").join("qqbot")
}

fn legacy_plugin_backup_dir(plugin_id: &str) -> PathBuf {
    super::openclaw_dir()
        .join("extensions")
        .join(format!("{plugin_id}.__clawpanel_backup"))
}

pub(super) fn cleanup_legacy_plugin_backup_dir(plugin_id: &str) -> Result<bool, String> {
    let legacy_backup = legacy_plugin_backup_dir(plugin_id);
    if !legacy_backup.exists() {
        return Ok(false);
    }
    if legacy_backup.is_dir() {
        fs::remove_dir_all(&legacy_backup).map_err(|e| format!("清理旧版插件备份失败: {e}"))?;
    } else {
        fs::remove_file(&legacy_backup).map_err(|e| format!("清理旧版插件备份失败: {e}"))?;
    }
    Ok(true)
}

fn plugin_install_marker_exists(plugin_dir: &Path) -> bool {
    plugin_dir.join("package.json").is_file()
        || plugin_dir.join("plugin.ts").is_file()
        || plugin_dir.join("index.js").is_file()
        || plugin_dir.join("dist").join("index.js").is_file()
}

fn restore_path(backup: &Path, target: &Path) -> Result<(), String> {
    if target.exists() {
        if target.is_dir() {
            fs::remove_dir_all(target).map_err(|e| format!("清理目录失败: {e}"))?;
        } else {
            fs::remove_file(target).map_err(|e| format!("清理文件失败: {e}"))?;
        }
    }
    if backup.exists() {
        fs::rename(backup, target).map_err(|e| format!("恢复备份失败: {e}"))?;
    }
    Ok(())
}

fn cleanup_failed_extension_install(
    plugin_dir: &Path,
    plugin_backup: &Path,
    config_backup: &Path,
    had_plugin_backup: bool,
    had_config_backup: bool,
) -> Result<(), String> {
    let config_path = super::openclaw_dir().join("openclaw.json");

    if plugin_dir.exists() {
        fs::remove_dir_all(plugin_dir).map_err(|e| format!("清理坏插件目录失败: {e}"))?;
    }
    if had_plugin_backup {
        restore_path(plugin_backup, plugin_dir)?;
    } else if plugin_backup.exists() {
        fs::remove_dir_all(plugin_backup).map_err(|e| format!("清理插件备份失败: {e}"))?;
    }

    if had_config_backup {
        restore_path(config_backup, &config_path)?;
    } else if config_backup.exists() {
        fs::remove_file(config_backup).map_err(|e| format!("清理配置备份失败: {e}"))?;
    }

    Ok(())
}

/// 检测插件是否为 OpenClaw 内置（作为 npm 依赖打包在 OpenClaw 运行时中）
fn is_plugin_builtin(plugin_id: &str) -> bool {
    // 插件 ID → npm 包名映射
    let pkg_name = match plugin_id {
        "feishu" => "@openclaw/feishu",
        "openclaw-lark" => "@larksuite/openclaw-lark",
        "dingtalk-connector" => "@dingtalk-real-ai/dingtalk-connector",
        _ => return false,
    };
    // 在全局 npm node_modules 中查找 openclaw 安装目录
    let npm_dirs: Vec<PathBuf> = {
        let mut dirs = Vec::new();
        let zh_scope = format!("@{}cloud", "qingchen");
        let zh_package = format!("openclaw-{}", "zh");
        #[cfg(target_os = "windows")]
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let base = PathBuf::from(appdata).join("npm").join("node_modules");
            dirs.push(base.join(&zh_scope).join(&zh_package));
            dirs.push(base.join("openclaw"));
        }
        #[cfg(target_os = "macos")]
        {
            dirs.push(
                PathBuf::from("/opt/homebrew/lib/node_modules")
                    .join(&zh_scope)
                    .join(&zh_package),
            );
            dirs.push(PathBuf::from("/opt/homebrew/lib/node_modules/openclaw"));
            dirs.push(PathBuf::from("/usr/local/lib/node_modules").join(&zh_scope).join(&zh_package));
            dirs.push(PathBuf::from("/usr/local/lib/node_modules/openclaw"));
        }
        #[cfg(target_os = "linux")]
        {
            dirs.push(PathBuf::from("/usr/local/lib/node_modules").join(&zh_scope).join(&zh_package));
            dirs.push(PathBuf::from("/usr/local/lib/node_modules/openclaw"));
            dirs.push(PathBuf::from("/usr/lib/node_modules").join(&zh_scope).join(&zh_package));
            dirs.push(PathBuf::from("/usr/lib/node_modules/openclaw"));
        }
        dirs
    };
    // 插件包名拆分成路径片段，如 @openclaw/feishu → @openclaw/feishu
    let pkg_path: PathBuf = pkg_name.split('/').collect();
    for base in &npm_dirs {
        let candidate = base.join("node_modules").join(&pkg_path);
        if candidate.join("package.json").is_file() {
            return true;
        }
    }
    false
}

fn generic_plugin_dir(plugin_id: &str) -> PathBuf {
    super::openclaw_dir().join("extensions").join(plugin_id)
}

fn generic_plugin_backup_dir(plugin_id: &str) -> PathBuf {
    plugin_backup_root().join(format!("{plugin_id}.__clawpanel_backup"))
}

fn generic_plugin_config_backup_path(plugin_id: &str) -> PathBuf {
    plugin_backup_root().join(format!("openclaw.{plugin_id}-install.bak"))
}

fn cleanup_failed_plugin_install(plugin_id: &str, had_plugin_backup: bool, had_config_backup: bool) -> Result<(), String> {
    let plugin_dir = generic_plugin_dir(plugin_id);
    let plugin_backup = generic_plugin_backup_dir(plugin_id);
    let config_path = super::openclaw_dir().join("openclaw.json");
    let config_backup = generic_plugin_config_backup_path(plugin_id);

    if plugin_dir.exists() {
        fs::remove_dir_all(&plugin_dir).map_err(|e| format!("清理坏插件目录失败: {e}"))?;
    }
    if had_plugin_backup {
        restore_path(&plugin_backup, &plugin_dir)?;
    } else if plugin_backup.exists() {
        fs::remove_dir_all(&plugin_backup).map_err(|e| format!("清理插件备份失败: {e}"))?;
    }

    if had_config_backup {
        restore_path(&config_backup, &config_path)?;
    } else if config_backup.exists() {
        fs::remove_file(&config_backup).map_err(|e| format!("清理配置备份失败: {e}"))?;
    }

    Ok(())
}

// ── QQ Bot 插件安装（带日志流） ──────────────────────────

#[tauri::command]
pub async fn install_channel_plugin(
    app: tauri::AppHandle,
    package_name: String,
    plugin_id: String,
    version: Option<String>,
) -> Result<String, String> {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    use tauri::Emitter;

    let package_name = package_name.trim();
    let plugin_id = plugin_id.trim();
    if package_name.is_empty() || plugin_id.is_empty() {
        return Err("package_name 和 plugin_id 不能为空".into());
    }
    // 拼接版本号：package@version（兼容用户 OpenClaw 版本的插件）
    let install_spec = match &version {
        Some(v) if !v.is_empty() => format!("{}@{}", package_name, v),
        _ => package_name.to_string(),
    };
    let plugin_dir = generic_plugin_dir(plugin_id);
    let plugin_backup = generic_plugin_backup_dir(plugin_id);
    let config_path = super::openclaw_dir().join("openclaw.json");
    let config_backup = generic_plugin_config_backup_path(plugin_id);
    let had_existing_plugin = plugin_dir.exists();
    let had_existing_config = config_path.exists();

    let _ = app.emit("plugin-log", format!("正在安装插件 {} ...", package_name));
    let _ = app.emit("plugin-progress", 10);

    fs::create_dir_all(plugin_backup_root()).map_err(|e| format!("创建插件备份目录失败: {e}"))?;
    if cleanup_legacy_plugin_backup_dir(plugin_id)? {
        let _ = app.emit("plugin-log", "已清理旧版插件备份目录");
    }

    if plugin_backup.exists() {
        let _ = fs::remove_dir_all(&plugin_backup);
    }
    if had_existing_plugin {
        fs::rename(&plugin_dir, &plugin_backup).map_err(|e| format!("备份旧插件失败: {e}"))?;
        let _ = app.emit("plugin-log", format!("检测到旧插件目录，已备份 {}", plugin_dir.display()));
    }

    if config_backup.exists() {
        let _ = fs::remove_file(&config_backup);
    }
    if had_existing_config {
        fs::copy(&config_path, &config_backup).map_err(|e| format!("备份配置失败: {e}"))?;
    }

    let _ = app.emit("plugin-log", format!("安装规格: {}", install_spec));
    let spawn_result = crate::utils::openclaw_command()
        .args(["plugins", "install", &install_spec])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match spawn_result {
        Ok(child) => child,
        Err(e) => {
            let _ = cleanup_failed_plugin_install(plugin_id, had_existing_plugin, had_existing_config);
            return Err(format!("启动 openclaw 失败: {}", e));
        }
    };

    let stderr = child.stderr.take();
    let app2 = app.clone();
    let stderr_lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let stderr_clone = stderr_lines.clone();
    let handle = std::thread::spawn(move || {
        if let Some(pipe) = stderr {
            for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                let _ = app2.emit("plugin-log", &line);
                stderr_clone
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(line);
            }
        }
    });

    let _ = app.emit("plugin-progress", 30);
    let mut progress = 30;
    if let Some(pipe) = child.stdout.take() {
        for line in BufReader::new(pipe).lines().map_while(Result::ok) {
            let _ = app.emit("plugin-log", &line);
            if progress < 90 {
                progress += 10;
                let _ = app.emit("plugin-progress", progress);
            }
        }
    }

    let _ = handle.join();
    let _ = app.emit("plugin-progress", 95);

    let status = child.wait().map_err(|e| format!("等待安装进程失败: {}", e))?;
    if !status.success() {
        let all_stderr = stderr_lines
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .join("\n");
        let is_host_version_issue = all_stderr.contains("minHostVersion")
            || all_stderr.contains("minimum host version")
            || all_stderr.contains("requires OpenClaw")
            || all_stderr.contains("host version");
        if is_host_version_issue {
            let _ = app.emit("plugin-log", "⚠ 插件要求更高版本的 OpenClaw（minHostVersion 不满足）");
            let _ = app.emit("plugin-log", "请先升级 OpenClaw 到最新版，再安装此插件：");
            let _ = app.emit("plugin-log", "  前往「服务管理」页面点击升级，或在终端执行：");
            let _ = app.emit("plugin-log", "  npm i -g openclaw@latest --registry https://registry.npmmirror.com");
        }
        let rollback_err = cleanup_failed_plugin_install(plugin_id, had_existing_plugin, had_existing_config)
            .err()
            .unwrap_or_default();
        let _ = app.emit("plugin-log", format!("插件 {} 安装失败，已回退", package_name));
        if is_host_version_issue {
            return Err("插件安装失败：当前 OpenClaw 版本过低，请先升级后重试".into());
        }
        return if rollback_err.is_empty() {
            Err(format!("插件安装失败：{}", package_name))
        } else {
            Err(format!("插件安装失败：{}；回退失败：{}", package_name, rollback_err))
        };
    }

    let finalize = (|| -> Result<(), String> {
        let mut cfg = super::config::load_openclaw_json()?;
        ensure_plugin_allowed(&mut cfg, plugin_id)?;
        super::config::save_openclaw_json(&cfg)?;
        Ok(())
    })();

    if let Err(err) = finalize {
        let rollback_err = cleanup_failed_plugin_install(plugin_id, had_existing_plugin, had_existing_config)
            .err()
            .unwrap_or_default();
        let _ = app.emit("plugin-log", format!("插件 {} 安装后收尾失败，已回退: {}", package_name, err));
        return if rollback_err.is_empty() {
            Err(format!("插件安装失败：{err}"))
        } else {
            Err(format!("插件安装失败：{err}；回退失败：{rollback_err}"))
        };
    }

    if plugin_backup.exists() {
        let _ = fs::remove_dir_all(&plugin_backup);
    }
    if config_backup.exists() {
        let _ = fs::remove_file(&config_backup);
    }
    let _ = app.emit("plugin-progress", 100);
    let _ = app.emit("plugin-log", format!("插件 {} 安装完成", package_name));
    Ok("安装成功".into())
}

#[tauri::command]
pub async fn install_qqbot_plugin(app: tauri::AppHandle, version: Option<String>) -> Result<String, String> {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    use tauri::Emitter;

    let install_spec = match &version {
        Some(v) if !v.is_empty() => format!("{}@{}", TENCENT_OPENCLAW_QQBOT_PACKAGE, v),
        _ => TENCENT_OPENCLAW_QQBOT_PACKAGE.to_string(),
    };

    let plugin_dir = generic_plugin_dir(OPENCLAW_QQBOT_EXTENSION_FOLDER);
    let plugin_backup = generic_plugin_backup_dir(OPENCLAW_QQBOT_EXTENSION_FOLDER);
    let config_path = super::openclaw_dir().join("openclaw.json");
    let config_backup = generic_plugin_config_backup_path(OPENCLAW_QQBOT_EXTENSION_FOLDER);
    let had_existing_plugin = plugin_dir.exists();
    let had_existing_config = config_path.exists();

    let _ = app.emit(
        "plugin-log",
        format!("正在安装腾讯 OpenClaw QQ 插件 {} ...", TENCENT_OPENCLAW_QQBOT_PACKAGE),
    );
    let _ = app.emit("plugin-progress", 10);

    fs::create_dir_all(plugin_backup_root()).map_err(|e| format!("创建插件备份目录失败: {e}"))?;
    if cleanup_legacy_plugin_backup_dir(OPENCLAW_QQBOT_EXTENSION_FOLDER)? {
        let _ = app.emit("plugin-log", "已清理旧版 QQ 插件备份目录");
    }

    if plugin_backup.exists() {
        let _ = fs::remove_dir_all(&plugin_backup);
    }
    if had_existing_plugin {
        fs::rename(&plugin_dir, &plugin_backup).map_err(|e| format!("备份旧 QQBot 插件失败: {e}"))?;
    }

    if config_backup.exists() {
        let _ = fs::remove_file(&config_backup);
    }
    if had_existing_config {
        fs::copy(&config_path, &config_backup).map_err(|e| format!("备份配置失败: {e}"))?;
    }

    let _ = app.emit("plugin-log", format!("安装规格: {}", install_spec));
    let spawn_result = crate::utils::openclaw_command()
        .args(["plugins", "install", &install_spec])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match spawn_result {
        Ok(child) => child,
        Err(e) => {
            let _ = cleanup_failed_extension_install(
                &plugin_dir,
                &plugin_backup,
                &config_backup,
                had_existing_plugin,
                had_existing_config,
            );
            return Err(format!("启动 openclaw 失败: {}", e));
        }
    };

    let stderr = child.stderr.take();
    let app2 = app.clone();
    let qqbot_stderr_lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let qqbot_stderr_clone = qqbot_stderr_lines.clone();
    let handle = std::thread::spawn(move || {
        if let Some(pipe) = stderr {
            for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                let _ = app2.emit("plugin-log", &line);
                qqbot_stderr_clone
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(line);
            }
        }
    });

    let _ = app.emit("plugin-progress", 30);

    let mut progress = 30;
    let mut qqbot_stdout_lines = Vec::new();
    if let Some(pipe) = child.stdout.take() {
        for line in BufReader::new(pipe).lines().map_while(Result::ok) {
            let _ = app.emit("plugin-log", &line);
            qqbot_stdout_lines.push(line);
            if progress < 90 {
                progress += 10;
                let _ = app.emit("plugin-progress", progress);
            }
        }
    }

    let _ = handle.join();
    let _ = app.emit("plugin-progress", 95);

    let status = child.wait().map_err(|e| format!("等待安装进程失败: {}", e))?;

    // 检测 native binding 缺失（macOS/Linux 上 OpenClaw CLI 自身启动失败）
    let all_output = {
        let stderr_guard = qqbot_stderr_lines.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut combined = qqbot_stdout_lines.join("\n");
        combined.push('\n');
        combined.push_str(&stderr_guard.join("\n"));
        combined
    };
    if all_output.contains("native binding") || all_output.contains("Failed to start CLI") {
        let _ = app.emit("plugin-log", "");
        let _ = app.emit("plugin-log", "⚠️ 检测到 OpenClaw CLI 原生依赖问题（native binding 缺失）");
        let _ = app.emit("plugin-log", "这是 OpenClaw 的上游依赖问题，非 QQBot 插件本身的问题。");
        let _ = app.emit("plugin-log", "请在终端手动执行以下命令重装 OpenClaw：");
        let _ = app.emit("plugin-log", "  npm i -g openclaw@latest --registry https://registry.npmmirror.com");
        let _ = app.emit("plugin-log", "重装完成后再回来安装 QQBot 插件。");
        let _ = cleanup_failed_extension_install(
            &plugin_dir,
            &plugin_backup,
            &config_backup,
            had_existing_plugin,
            had_existing_config,
        );
        let _ = app.emit("plugin-progress", 100);
        return Err("OpenClaw CLI 原生依赖缺失，请先在终端重装 OpenClaw（详见上方日志）".into());
    }

    if !status.success() {
        let all_stderr = qqbot_stderr_lines
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .join("\n");
        let is_host_version_issue = all_stderr.contains("minHostVersion")
            || all_stderr.contains("minimum host version")
            || all_stderr.contains("requires OpenClaw")
            || all_stderr.contains("host version");
        if is_host_version_issue {
            let _ = app.emit("plugin-log", "⚠ 插件要求更高版本的 OpenClaw（minHostVersion 不满足）");
            let _ = app.emit("plugin-log", "请先升级 OpenClaw 到最新版，再安装此插件：");
            let _ = app.emit("plugin-log", "  前往「服务管理」页面点击升级，或在终端执行：");
            let _ = app.emit("plugin-log", "  npm i -g openclaw@latest --registry https://registry.npmmirror.com");
        } else {
            let _ = app.emit("plugin-log", "openclaw plugins install 未成功结束，正在回退");
        }
        let _ = cleanup_failed_extension_install(
            &plugin_dir,
            &plugin_backup,
            &config_backup,
            had_existing_plugin,
            had_existing_config,
        );
        let _ = app.emit("plugin-progress", 100);
        if is_host_version_issue {
            return Err("插件安装失败：当前 OpenClaw 版本过低，请先升级后重试".into());
        }
        return Err("QQ 插件安装失败：openclaw plugins install 进程退出码非零".into());
    }

    if !plugin_install_marker_exists(&plugin_dir) {
        let _ = app.emit("plugin-log", format!("未在 {} 检测到插件文件，正在回退", plugin_dir.display()));
        let _ = cleanup_failed_extension_install(
            &plugin_dir,
            &plugin_backup,
            &config_backup,
            had_existing_plugin,
            had_existing_config,
        );
        let _ = app.emit("plugin-progress", 100);
        return Err(format!(
            "安装后未在 extensions/{} 检测到插件，请检查 OpenClaw 版本与网络",
            OPENCLAW_QQBOT_EXTENSION_FOLDER
        ));
    }

    let finalize = (|| -> Result<(), String> {
        let mut cfg = super::config::load_openclaw_json()?;
        ensure_openclaw_qqbot_plugin(&mut cfg)?;
        super::config::save_openclaw_json(&cfg)?;
        let _ = app.emit("plugin-log", "已补齐 plugins.allow 与 entries.qqbot.enabled");
        Ok(())
    })();

    match finalize {
        Ok(()) => {
            let _ = app.emit("plugin-progress", 100);
            if plugin_backup.exists() {
                let _ = fs::remove_dir_all(&plugin_backup);
            }
            if config_backup.exists() {
                let _ = fs::remove_file(&config_backup);
            }
            if qqbot_plugin_dir().is_dir() {
                let _ = app.emit(
                    "plugin-log",
                    "提示：检测到旧的 extensions/qqbot 目录，可能与官方包并存并触发「无 provenance」日志；不需要时可手动删除或改名备份。",
                );
            }
            let _ = app.emit("plugin-log", "QQ 插件安装完成；正在重启 Gateway 以加载插件（与官方文档一致）");
            let app2 = app.clone();
            tauri::async_runtime::spawn(async move {
                let _ = crate::commands::service::restart_service(app2, "ai.openclaw.gateway".into()).await;
            });
            Ok("安装成功".into())
        }
        Err(err) => {
            let _ = app.emit("plugin-log", format!("写入 plugins 配置失败，正在回退: {err}"));
            let rollback_err = cleanup_failed_extension_install(
                &plugin_dir,
                &plugin_backup,
                &config_backup,
                had_existing_plugin,
                had_existing_config,
            )
            .err()
            .unwrap_or_default();
            let _ = app.emit("plugin-progress", 100);
            let _ = app.emit("plugin-log", "QQBot 插件安装失败，已自动回退到安装前状态");
            if rollback_err.is_empty() {
                Err(format!("插件安装失败：{err}"))
            } else {
                Err(format!("插件安装失败：{err}；回退失败：{rollback_err}"))
            }
        }
    }
}