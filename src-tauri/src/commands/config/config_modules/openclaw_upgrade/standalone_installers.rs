async fn try_standalone_install(
    app: &tauri::AppHandle,
    version: &str,
    override_base_url: Option<&str>,
) -> Result<String, String> {
    let source_label = if override_base_url.is_some() { "GitHub" } else { "CDN" };
    use tauri::Emitter;

    let cfg = standalone_config().await;
    if !cfg.enabled {
        return Err("standalone 安装未启用".into());
    }
    let base_url = cfg.base_url.as_deref().ok_or("standalone baseUrl 未配置")?;
    let platform = standalone_platform_key();
    if platform == "unknown" {
        return Err("当前平台不支持 standalone 安装包".into());
    }
    let install_dir = standalone_install_dir().ok_or("无法确定 standalone 安装目录")?;

    // 1. 动态查询最新版本
    let _ = app.emit(
        "upgrade-log",
        "\u{1F4E6} 尝试 standalone 独立安装包（汉化版专属，自带 Node.js 运行时，无需 npm）",
    );
    let _ = app.emit("upgrade-log", "查询最新版本...");
    let manifest_url = format!("{base_url}/latest.json");
    let client = crate::commands::build_http_client(std::time::Duration::from_secs(10), None)
        .map_err(|e| format!("HTTP 客户端创建失败: {e}"))?;
    let manifest_resp = client
        .get(&manifest_url)
        .send()
        .await
        .map_err(|e| format!("standalone 清单获取失败: {e}"))?;
    if !manifest_resp.status().is_success() {
        return Err(format!("standalone 清单不可用 (HTTP {})", manifest_resp.status()));
    }
    let manifest: Value = manifest_resp
        .json()
        .await
        .map_err(|e| format!("standalone 清单解析失败: {e}"))?;

    // 兼容两种 latest.json 格式：
    // 新格式（CI 生成）: { "editions": { "zh": { "version": "...", "base_url": "..." } } }
    // 旧格式（兼容）:   { "version": "...", "base_url": "..." }
    let edition_obj = manifest.get("editions").and_then(|e| e.get("zh"));
    let (remote_version, manifest_base_url, archive_prefix) = if let Some(ed) = edition_obj {
        let ver = ed
            .get("version")
            .and_then(|v| v.as_str())
            .ok_or("standalone 清单 editions.zh 缺少 version 字段")?;
        let bu = ed.get("base_url").and_then(|v| v.as_str());
        (ver, bu, legacy_openclaw_zh_package())
    } else {
        let ver = manifest
            .get("version")
            .and_then(|v| v.as_str())
            .ok_or("standalone 清单缺少 version 字段")?;
        let bu = manifest.get("base_url").and_then(|v| v.as_str());
        (ver, bu, "openclaw".to_string())
    };

    // 版本匹配检查
    if version != "latest" && !versions_match(remote_version, version) {
        return Err(format!("standalone 版本 {remote_version} 与请求版本 {version} 不匹配"));
    }

    let default_base = format!("{base_url}/{remote_version}");
    let remote_base = if let Some(ovr) = override_base_url {
        ovr
    } else {
        manifest_base_url.unwrap_or(&default_base)
    };

    // 2. 构造下载 URL
    let ext = standalone_archive_ext();
    let filename = format!("{archive_prefix}-{remote_version}-{platform}.{ext}");
    let download_url = format!("{remote_base}/{filename}");

    let _ = app.emit("upgrade-log", format!("从 {source_label} 下载: {filename}"));
    let _ = app.emit("upgrade-progress", 15);

    // 3. 流式下载
    let tmp_dir = std::env::temp_dir();
    let archive_path = tmp_dir.join(&filename);
    let dl_client = crate::commands::build_http_client(std::time::Duration::from_secs(600), None)
        .map_err(|e| format!("下载客户端创建失败: {e}"))?;
    let dl_resp = dl_client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("standalone 下载失败: {e}"))?;
    if !dl_resp.status().is_success() {
        return Err(format!("standalone 下载失败 (HTTP {}): {download_url}", dl_resp.status()));
    }
    let total_bytes = dl_resp.content_length().unwrap_or(0);
    let size_mb = if total_bytes > 0 {
        format!("{:.0}MB", total_bytes as f64 / 1_048_576.0)
    } else {
        "未知大小".into()
    };
    let _ = app.emit("upgrade-log", format!("下载中 ({size_mb})..."));

    {
        use futures_util::StreamExt;
        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::File::create(&archive_path)
            .await
            .map_err(|e| format!("创建临时文件失败: {e}"))?;
        let mut stream = dl_resp.bytes_stream();
        let mut downloaded: u64 = 0;
        let mut last_progress: u32 = 15;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("下载中断: {e}"))?;
            file.write_all(&chunk).await.map_err(|e| format!("写入失败: {e}"))?;
            downloaded += chunk.len() as u64;
            if total_bytes > 0 {
                let pct = 15 + ((downloaded as f64 / total_bytes as f64) * 55.0) as u32;
                if pct > last_progress {
                    // 每 5% 输出一次文字进度
                    if pct / 5 > last_progress / 5 {
                        let dl_mb = downloaded as f64 / 1_048_576.0;
                        let total_mb = total_bytes as f64 / 1_048_576.0;
                        let real_pct = (downloaded as f64 / total_bytes as f64 * 100.0) as u32;
                        let _ = app.emit("upgrade-log", format!("下载中 {real_pct}% ({dl_mb:.0}/{total_mb:.0}MB)"));
                    }
                    last_progress = pct;
                    let _ = app.emit("upgrade-progress", pct.min(70));
                }
            }
        }
        file.flush().await.map_err(|e| format!("刷新文件失败: {e}"))?;
    }

    let _ = app.emit("upgrade-log", "下载完成，解压安装中...");
    let _ = app.emit("upgrade-progress", 72);

    // 4. 清理旧安装 & 创建目录
    if install_dir.exists() {
        let _ = std::fs::remove_dir_all(&install_dir);
    }
    std::fs::create_dir_all(&install_dir).map_err(|e| format!("创建安装目录失败: {e}"))?;

    // 5. 解压
    #[cfg(target_os = "windows")]
    {
        // Windows: zip 解压
        let archive_file = std::fs::File::open(&archive_path).map_err(|e| format!("打开归档失败: {e}"))?;
        let mut zip_archive = zip::ZipArchive::new(archive_file).map_err(|e| format!("ZIP 解析失败: {e}"))?;
        zip_archive.extract(&install_dir).map_err(|e| format!("ZIP 解压失败: {e}"))?;
        // 归档内可能有 openclaw/ 子目录，需要提升一层
        let nested = install_dir.join("openclaw");
        if nested.exists() && nested.join("node.exe").exists() {
            for entry in std::fs::read_dir(&nested)
                .map_err(|e| format!("读取目录失败: {e}"))?
                .flatten()
            {
                let dest = install_dir.join(entry.file_name());
                let _ = std::fs::rename(entry.path(), &dest);
            }
            let _ = std::fs::remove_dir_all(&nested);
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Unix: tar.gz 解压
        let status = Command::new("tar")
            .args([
                "-xzf",
                &archive_path.to_string_lossy(),
                "-C",
                &install_dir.to_string_lossy(),
                "--strip-components=1",
            ])
            .status()
            .map_err(|e| format!("解压失败: {e}"))?;
        if !status.success() {
            return Err("tar 解压失败".into());
        }
    }

    // 清理临时文件
    let _ = std::fs::remove_file(&archive_path);
    let _ = app.emit("upgrade-progress", 85);

    // 6. 验证安装
    #[cfg(target_os = "windows")]
    let openclaw_bin = install_dir.join("openclaw.cmd");
    #[cfg(not(target_os = "windows"))]
    let openclaw_bin = install_dir.join("openclaw");

    if !openclaw_bin.exists() {
        return Err("standalone 解压后未找到 openclaw 可执行文件".into());
    }

    // 7. 添加到 PATH（Windows 用户 PATH，Unix 创建 symlink）
    #[cfg(target_os = "windows")]
    {
        let install_str = install_dir.to_string_lossy().to_string();
        // 检查是否已在 PATH 中
        let current_path = std::env::var("PATH").unwrap_or_default();
        if !current_path.split(';').any(|p| p.eq_ignore_ascii_case(&install_str)) {
            // 写入用户 PATH（注册表）
            let _ = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    &format!(
                        "$p = [Environment]::GetEnvironmentVariable('Path','User'); if ($p -notlike '*{}*') {{ [Environment]::SetEnvironmentVariable('Path', $p + ';{}', 'User') }}",
                        install_str.replace('\'', "''"),
                        install_str.replace('\'', "''")
                    ),
                ])
                .creation_flags(0x08000000)
                .status();
            // 同步更新当前进程的 PATH 环境变量，使后续 resolve_openclaw_cli_path()
            // 和 build_enhanced_path() 能立即发现 standalone 安装的 CLI，
            // 无需重启应用（注册表写入仅对新进程生效）
            // SAFETY: 在 Tauri 命令处理器中单次调用，此时无其他线程并发读写 PATH。
            // enhanced_path 使用独立的 RwLock 缓存，不受影响。
            unsafe {
                std::env::set_var("PATH", format!("{};{}", current_path, install_str));
            }
            let _ = app.emit("upgrade-log", format!("已添加到 PATH: {install_str}"));
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Unix: 创建 /usr/local/bin/openclaw symlink 或 ~/bin/openclaw
        let link_targets = [
            PathBuf::from("/usr/local/bin/openclaw"),
            dirs::home_dir().unwrap_or_default().join("bin").join("openclaw"),
        ];
        for link in &link_targets {
            if let Some(parent) = link.parent() {
                if parent.exists() {
                    let _ = std::fs::remove_file(link);
                    #[cfg(unix)]
                    {
                        if std::os::unix::fs::symlink(&openclaw_bin, link).is_ok() {
                            let _ = Command::new("chmod").args(["+x", &openclaw_bin.to_string_lossy()]).status();
                            let _ = app.emit("upgrade-log", format!("symlink 已创建: {}", link.display()));
                            break;
                        }
                    }
                }
            }
        }
    }

    let _ = app.emit("upgrade-progress", 95);
    let _ = app.emit("upgrade-log", format!("✅ standalone 独立安装包安装完成 ({remote_version})"));
    let _ = app.emit("upgrade-log", format!("安装目录: {}", install_dir.display()));

    // 刷新 CLI 检测缓存
    crate::commands::service::invalidate_cli_detection_cache();

    Ok(remote_version.to_string())
}

/// 尝试从 R2 CDN 下载预装归档安装 OpenClaw（跳过 npm 依赖解析）
/// 成功返回 Ok(版本号)，失败返回 Err(原因) 供 caller 降级到 npm install
#[allow(dead_code)]
async fn try_r2_install(app: &tauri::AppHandle, version: &str, source: &str) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    use tauri::Emitter;

    let r2 = r2_config().await;
    if !r2.enabled {
        return Err("R2 加速未启用".into());
    }
    let base_url = r2.base_url.as_deref().ok_or("R2 baseUrl 未配置")?;
    let platform = r2_platform_key();
    if platform == "unknown" {
        return Err("当前平台不支持 R2 预装归档".into());
    }

    // 1. 获取 latest.json
    let _ = app.emit("upgrade-log", "尝试从 CDN 加速下载...");
    let manifest_url = format!("{}/latest.json", base_url);
    let client = crate::commands::build_http_client(std::time::Duration::from_secs(10), None)
        .map_err(|e| format!("HTTP 客户端创建失败: {e}"))?;
    let manifest_resp = client
        .get(&manifest_url)
        .send()
        .await
        .map_err(|e| format!("获取 CDN 清单失败: {e}"))?;
    if !manifest_resp.status().is_success() {
        return Err(format!("CDN 清单不可用 (HTTP {})", manifest_resp.status()));
    }
    let manifest: Value = manifest_resp.json().await.map_err(|e| format!("CDN 清单解析失败: {e}"))?;

    // 2. 查找归档：优先通用 tarball（全平台），其次平台特定 assets
    let source_key = if source == "official" { "official" } else { "chinese" };
    let source_obj = manifest.get(source_key);
    let cdn_version = source_obj
        .and_then(|s| s.get("version"))
        .and_then(|v| v.as_str())
        .unwrap_or(version);

    // 优先通用 tarball（npm pack 产物，~50MB，全平台通用）
    let tarball = source_obj.and_then(|s| s.get("tarball"));
    // 其次平台特定 assets（预装 node_modules，~200MB）
    let asset = source_obj.and_then(|s| s.get("assets")).and_then(|a| a.get(platform));
    let use_tarball = tarball.and_then(|t| t.get("url")).and_then(|v| v.as_str()).is_some();

    let (archive_url, expected_sha, expected_size) = if let Some(a) = asset {
        // 优先平台预装归档（直接解压，零网络依赖，最快）
        (
            a.get("url").and_then(|v| v.as_str()).ok_or("归档 URL 缺失")?,
            a.get("sha256").and_then(|v| v.as_str()).unwrap_or(""),
            a.get("size").and_then(|v| v.as_u64()).unwrap_or(0),
        )
    } else if use_tarball {
        // 其次通用 tarball（需要 npm install，仍有网络依赖）
        let t = tarball.ok_or("tarball 元数据缺失")?;
        (
            t.get("url").and_then(|v| v.as_str()).ok_or("tarball URL 缺失")?,
            t.get("sha256").and_then(|v| v.as_str()).unwrap_or(""),
            t.get("size").and_then(|v| v.as_u64()).unwrap_or(0),
        )
    } else {
        return Err(format!("CDN 无 {source_key} 可用归档"));
    };

    // 版本匹配检查（如果用户指定了版本，CDN 版本必须匹配）
    if version != "latest" && !versions_match(cdn_version, version) {
        return Err(format!("CDN 版本 {cdn_version} 与请求版本 {version} 不匹配"));
    }

    let size_mb = if expected_size > 0 {
        format!("{:.0}MB", expected_size as f64 / 1_048_576.0)
    } else {
        "未知大小".into()
    };
    let _ = app.emit("upgrade-log", format!("CDN 下载: {cdn_version} ({platform}, {size_mb})"));
    let _ = app.emit("upgrade-progress", 15);

    // 3. 流式下载到临时文件
    let tmp_dir = std::env::temp_dir();
    let archive_path = tmp_dir.join(format!("openclaw-{platform}.tgz"));
    let dl_client = crate::commands::build_http_client(std::time::Duration::from_secs(300), None)
        .map_err(|e| format!("下载客户端创建失败: {e}"))?;
    let dl_resp = dl_client
        .get(archive_url)
        .send()
        .await
        .map_err(|e| format!("CDN 下载失败: {e}"))?;
    if !dl_resp.status().is_success() {
        return Err(format!("CDN 下载失败 (HTTP {})", dl_resp.status()));
    }
    let total_bytes = dl_resp.content_length().unwrap_or(expected_size);

    {
        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::File::create(&archive_path)
            .await
            .map_err(|e| format!("创建临时文件失败: {e}"))?;
        let mut stream = dl_resp.bytes_stream();
        let mut downloaded: u64 = 0;
        let mut last_progress: u32 = 15;
        use futures_util::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("下载中断: {e}"))?;
            file.write_all(&chunk).await.map_err(|e| format!("写入失败: {e}"))?;
            downloaded += chunk.len() as u64;
            if total_bytes > 0 {
                let pct = 15 + ((downloaded as f64 / total_bytes as f64) * 50.0) as u32;
                if pct > last_progress {
                    last_progress = pct;
                    let _ = app.emit("upgrade-progress", pct.min(65));
                }
            }
        }
        file.flush().await.map_err(|e| format!("刷新文件失败: {e}"))?;
    }

    let _ = app.emit("upgrade-log", "下载完成，校验中...");
    let _ = app.emit("upgrade-progress", 68);

    // 4. SHA256 校验
    if !expected_sha.is_empty() {
        let file_bytes = std::fs::read(&archive_path).map_err(|e| format!("读取归档失败: {e}"))?;
        let mut hasher = Sha256::new();
        hasher.update(&file_bytes);
        let actual_sha = format!("{:x}", hasher.finalize());
        if actual_sha != expected_sha {
            let _ = std::fs::remove_file(&archive_path);
            return Err(format!("SHA256 校验失败: 期望 {expected_sha}, 实际 {actual_sha}"));
        }
        let _ = app.emit("upgrade-log", "SHA256 校验通过 ✓");
    }

    let _ = app.emit("upgrade-progress", 72);

    // 5. 安装：通用 tarball 用 npm install -g，平台归档用 tar 解压
    if use_tarball {
        // 通用 tarball 模式：npm install -g ./file.tgz（全平台通用，npm 自动处理原生模块）
        let _ = app.emit("upgrade-log", "通用 tarball 模式，执行 npm install...");
        let mut install_cmd = npm_command_elevated();
        install_cmd.args(["install", "-g", &archive_path.to_string_lossy(), "--force"]);
        apply_git_install_env(&mut install_cmd);
        let install_output = install_cmd.output().map_err(|e| format!("npm install 执行失败: {e}"))?;
        if !install_output.status.success() {
            let stderr = String::from_utf8_lossy(&install_output.stderr);
            let _ = std::fs::remove_file(&archive_path);
            return Err(format!("npm install -g tarball 失败: {}", &stderr[stderr.len().saturating_sub(300)..]));
        }
        let _ = app.emit("upgrade-log", "npm install 完成 ✓");
    } else {
        // 平台特定归档模式：直接解压到 npm 全局 node_modules
        let modules_dir = npm_global_modules_dir().ok_or("无法确定 npm 全局 node_modules 目录")?;
        if !modules_dir.exists() {
            std::fs::create_dir_all(&modules_dir).map_err(|e| format!("创建 node_modules 目录失败: {e}"))?;
        }
        let _ = app.emit("upgrade-log", format!("解压到 {}", modules_dir.display()));

        let legacy_scope = legacy_openclaw_zh_scope();
        let legacy_package = legacy_openclaw_zh_package();
        let qc_dir = modules_dir.join(&legacy_scope);
        if qc_dir.exists() {
            let _ = std::fs::remove_dir_all(&qc_dir);
        }

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            let status = Command::new("tar")
                .args(["-xzf", &archive_path.to_string_lossy(), "-C", &modules_dir.to_string_lossy()])
                .creation_flags(0x08000000)
                .status()
                .map_err(|e| format!("解压失败: {e}"))?;
            if !status.success() {
                return Err("tar 解压失败".into());
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let status = Command::new("tar")
                .args(["-xzf", &archive_path.to_string_lossy(), "-C", &modules_dir.to_string_lossy()])
                .status()
                .map_err(|e| format!("解压失败: {e}"))?;
            if !status.success() {
                return Err("tar 解压失败".into());
            }
        }

        // 归档内目录可能缺少 @ 前缀（Windows tar 不支持 @ 前缀），需要重命名
        let no_at_dir = modules_dir.join(legacy_scope.trim_start_matches('@'));
        if no_at_dir.exists() && !qc_dir.exists() {
            std::fs::rename(&no_at_dir, &qc_dir).map_err(|e| format!("修正中文增强运行时目录失败: {e}"))?;
            let _ = app.emit("upgrade-log", "中文增强运行时目录已修正");
        }

        let _ = app.emit("upgrade-log", "解压完成，创建 bin 链接...");

        // 创建 bin 链接
        let bin_dir = npm_global_bin_dir().ok_or("无法确定 npm bin 目录")?;
        let openclaw_js = modules_dir
            .join(&legacy_scope)
            .join(&legacy_package)
            .join("bin")
            .join("openclaw.js");

        if openclaw_js.exists() {
            #[cfg(target_os = "windows")]
            {
                let cmd_path = bin_dir.join("openclaw.cmd");
                let cmd_content = format!(
                    "@ECHO off\r\nGOTO start\r\n:find_dp0\r\nSET dp0=%~dp0\r\nEXIT /b\r\n:start\r\nSETLOCAL\r\nCALL :find_dp0\r\n\r\nIF EXIST \"%dp0%\\node.exe\" (\r\n  SET \"_prog=%dp0%\\node.exe\"\r\n) ELSE (\r\n  SET \"_prog=node\"\r\n  SET PATHEXT=%PATHEXT:;.JS;=;%\r\n)\r\n\r\nendLocal & goto #_undefined_# 2>NUL || title %COMSPEC% & \"%_prog%\"  \"{}\" %*\r\n",
                    openclaw_js.display()
                );
                std::fs::write(&cmd_path, cmd_content).map_err(|e| format!("创建 openclaw.cmd 失败: {e}"))?;
                let ps1_path = bin_dir.join("openclaw.ps1");
                let ps1_content = format!(
                    "#!/usr/bin/env pwsh\r\n$basedir=Split-Path $MyInvocation.MyCommand.Definition -Parent\r\n\r\n$exe=\"\"\r\nif ($PSVersionTable.PSVersion -lt \"6.0\" -or $IsWindows) {{\r\n  $exe=\".exe\"\r\n}}\r\n$ret=0\r\nif (Test-Path \"$basedir/node$exe\") {{\r\n  if ($MyInvocation.ExpectingInput) {{\r\n    $input | & \"$basedir/node$exe\"  \"{}\" $args\r\n  }} else {{\r\n    & \"$basedir/node$exe\"  \"{}\" $args\r\n  }}\r\n  $ret=$LASTEXITCODE\r\n}} else {{\r\n  if ($MyInvocation.ExpectingInput) {{\r\n    $input | & \"node$exe\"  \"{}\" $args\r\n  }} else {{\r\n    & \"node$exe\"  \"{}\" $args\r\n  }}\r\n  $ret=$LASTEXITCODE\r\n}}\r\nexit $ret\r\n",
                    openclaw_js.display(), openclaw_js.display(), openclaw_js.display(), openclaw_js.display()
                );
                let _ = std::fs::write(&ps1_path, ps1_content);
            }
            #[cfg(not(target_os = "windows"))]
            {
                let link_path = bin_dir.join("openclaw");
                let _ = std::fs::remove_file(&link_path);
                #[cfg(unix)]
                {
                    std::os::unix::fs::symlink(&openclaw_js, &link_path).map_err(|e| format!("创建 symlink 失败: {e}"))?;
                    let _ = Command::new("chmod").args(["+x", &openclaw_js.to_string_lossy()]).status();
                    let _ = Command::new("chmod").args(["+x", &link_path.to_string_lossy()]).status();
                }
            }
            let _ = app.emit("upgrade-log", "bin 链接已创建 ✓");
        } else {
            let _ = app.emit("upgrade-log", "⚠️ openclaw.js 未找到，bin 链接跳过");
        }
    }

    // 清理临时文件
    let _ = std::fs::remove_file(&archive_path);

    let _ = app.emit("upgrade-progress", 95);
    Ok(cdn_version.to_string())
}