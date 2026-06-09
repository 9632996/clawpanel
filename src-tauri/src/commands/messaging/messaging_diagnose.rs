use serde_json::Value;
use std::time::Duration;

use super::messaging::read_platform_config;
use super::messaging_common::secret_ref_placeholder;
use super::messaging_common::{platform_list_id, platform_storage_key};
use super::messaging_diagnosis_common::{build_openclaw_channel_diagnosis, channel_diagnosis_credentials_ready};
use super::messaging_plugins::qqbot_plugin_diagnose;
use super::messaging_verify::{verify_bot_token, verify_qqbot};

const QQ_OPENCLAW_FAQ_URL: &str = "https://q.qq.com/qqbot/openclaw/faq.html";

/// 与 `openclaw channels add --channel qqbot` 默认账号 id 一致。
pub(super) const QQBOT_DEFAULT_ACCOUNT_ID: &str = "default";

pub(super) fn qqbot_channel_has_credentials(val: &Value) -> bool {
    val.get("appId").is_some_and(secret_like_value_present)
        || val
            .get("clientSecret")
            .or_else(|| val.get("appSecret"))
            .is_some_and(secret_like_value_present)
        || val.get("token").is_some_and(secret_like_value_present)
}

pub(super) fn secret_like_value_present(value: &Value) -> bool {
    value.as_str().is_some_and(|s| !s.trim().is_empty()) || secret_ref_placeholder(value).is_some()
}

pub(super) fn account_display_value(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(|v| {
        v.as_str()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| secret_ref_placeholder(v))
    })
}

/// QQ 渠道深度诊断：凭证 + 本机 Gateway + HTTP 健康检查 + 配置与插件。
/// 用于解释 QQ 客户端「灵魂不在线」等（多为 Gateway / 长连接侧，而非 AppID 填错）。
#[tauri::command]
pub async fn diagnose_channel(platform: String, account_id: Option<String>) -> Result<Value, String> {
    let platform = platform.trim().to_string();
    if platform.is_empty() {
        return Err("platform 不能为空".into());
    }
    if platform == "qqbot" {
        return diagnose_qqbot_channel(account_id).await;
    }

    let cfg = super::config::load_openclaw_json().unwrap_or_else(|_| crate::jv!({}));
    let storage_key = platform_storage_key(&platform);
    let normalized_account_id = account_id.as_deref().map(str::trim).filter(|id| !id.is_empty());
    let channel_root = cfg.get("channels").and_then(|c| c.get(storage_key));
    let channel_enabled = channel_root
        .and_then(|node| node.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let saved = read_platform_config(platform.clone(), normalized_account_id.map(str::to_string)).await?;
    let config_exists = saved.get("exists").and_then(|v| v.as_bool()).unwrap_or(false);
    let form = saved.get("values").and_then(|v| v.as_object()).cloned().unwrap_or_default();
    let credentials_ready = channel_diagnosis_credentials_ready(&platform, &form);
    let (verify_result, verify_error) = if config_exists && credentials_ready {
        match verify_bot_token(platform.clone(), Value::Object(form.clone())).await {
            Ok(result) => (Some(result), None),
            Err(error) => (None, Some(error)),
        }
    } else {
        (None, None)
    };

    Ok(build_openclaw_channel_diagnosis(
        &platform,
        normalized_account_id,
        config_exists,
        channel_enabled,
        &form,
        verify_result,
        verify_error,
    ))
}

async fn diagnose_qqbot_channel(account_id: Option<String>) -> Result<Value, String> {
    let port = crate::commands::gateway_listen_port();
    let cfg = super::config::load_openclaw_json().unwrap_or_else(|_| crate::jv!({}));

    let mut checks: Vec<Value> = vec![];

    // ── 1) 已保存的凭证 ──
    let saved = read_platform_config("qqbot".to_string(), account_id.clone()).await?;
    let exists = saved.get("exists").and_then(|v| v.as_bool()).unwrap_or(false);
    let values = saved.get("values").and_then(|v| v.as_object()).cloned().unwrap_or_default();

    let cred_ok = if !exists {
        checks.push(crate::jv!({
            "id": "credentials",
            "ok": false,
            "title": "QQ 凭证已写入配置",
            "detail": "未在 openclaw.json 中找到 qqbot 渠道配置，请先在「渠道列表」完成接入并保存。"
        }));
        false
    } else {
        match verify_qqbot(
            &super::build_http_client(Duration::from_secs(15), None).map_err(|e| format!("HTTP 客户端初始化失败: {}", e))?,
            &values,
        )
        .await
        {
            Ok(r) if r.get("valid").and_then(|v| v.as_bool()) == Some(true) => {
                let details: Vec<String> = r
                    .get("details")
                    .and_then(|d| d.as_array())
                    .map(|arr| arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default();
                checks.push(crate::jv!({
                    "id": "credentials",
                    "ok": true,
                    "title": "QQ 开放平台凭证（getAppAccessToken）",
                    "detail": if details.is_empty() {
                        "AppID / ClientSecret 可通过腾讯接口换取 access_token。".to_string()
                    } else {
                        details.join(" · ")
                    }
                }));
                true
            }
            Ok(r) => {
                let errs: Vec<String> = r
                    .get("errors")
                    .and_then(|e| e.as_array())
                    .map(|arr| arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_else(|| vec!["凭证校验失败".into()]);
                checks.push(crate::jv!({
                    "id": "credentials",
                    "ok": false,
                    "title": "QQ 开放平台凭证（getAppAccessToken）",
                    "detail": errs.join("；")
                }));
                false
            }
            Err(e) => {
                checks.push(crate::jv!({
                    "id": "credentials",
                    "ok": false,
                    "title": "QQ 开放平台凭证（getAppAccessToken）",
                    "detail": e
                }));
                false
            }
        }
    };

    // ── 2) channels.qqbot.enabled ──
    let qq_node = cfg.get("channels").and_then(|c| c.get("qqbot"));
    let qq_enabled = qq_node
        .and_then(|n| n.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    checks.push(crate::jv!({
        "id": "qq_channel_enabled",
        "ok": qq_enabled,
        "title": "配置中 QQ 渠道已启用",
        "detail": if qq_enabled {
            "channels.qqbot.enabled 为 true（或未写，默认启用）。"
        } else {
            "channels.qqbot.enabled 为 false，Gateway 不会连接 QQ，请在渠道列表中启用。"
        }
    }));

    // ── 3) chatCompletions（QQ 常见问题里 405 等） ──
    let chat_on = cfg
        .get("gateway")
        .and_then(|g| g.get("http"))
        .and_then(|h| h.get("endpoints"))
        .and_then(|e| e.get("chatCompletions"))
        .and_then(|c| c.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    checks.push(crate::jv!({
        "id": "chat_completions",
        "ok": chat_on,
        "title": "Gateway HTTP · chatCompletions 端点",
        "detail": if chat_on {
            "gateway.http.endpoints.chatCompletions.enabled 已开启。"
        } else {
            "未启用 chatCompletions 时，机器人往往无法正常对话（如 405）。保存 QQ 渠道时面板通常会打开此项；若手动改过配置请检查。"
        }
    }));

    // ── 4) QQ 插件（extensions/qqbot 或 extensions/openclaw-qqbot + plugins.allow） ──
    let (plugin_ok, plugin_detail) = qqbot_plugin_diagnose(&cfg);
    checks.push(crate::jv!({
        "id": "qq_plugin",
        "ok": plugin_ok,
        "title": "QQ 机器人插件（qqbot / openclaw-qqbot）",
        "detail": plugin_detail
    }));

    // ── 5) Gateway TCP ──
    let port_copy = port;
    let tcp_ok = tokio::task::spawn_blocking(move || {
        let addr = format!("127.0.0.1:{}", port_copy);
        match addr.parse::<std::net::SocketAddr>() {
            Ok(a) => std::net::TcpStream::connect_timeout(&a, Duration::from_secs(2)).is_ok(),
            Err(_) => false,
        }
    })
    .await
    .unwrap_or(false);
    checks.push(crate::jv!({
        "id": "gateway_tcp",
        "ok": tcp_ok,
        "title": format!("本机 Gateway 端口 {}（TCP）", port),
        "detail": if tcp_ok {
            format!("可在 {}s 内连接到 127.0.0.1:{}。", 2, port)
        } else {
            format!(
                "无法连接 127.0.0.1:{}。QQ 提示「灵魂不在线」时最常见原因是 OpenClaw Gateway 未在本机运行或未监听该端口。请在面板「Gateway」页或托盘菜单启动 Gateway。",
                port
            )
        }
    }));

    // ── 6) Gateway HTTP /__api/health ──
    let (http_ok, http_detail) = if tcp_ok {
        let url = format!("http://127.0.0.1:{}/__api/health", port);
        match super::build_http_client(Duration::from_secs(3), None) {
            Ok(client) => match client.get(&url).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    let ok = status.is_success() || status.is_redirection();
                    (ok, format!("GET {} → HTTP {}", url, status))
                }
                Err(e) => (false, format!("请求 {} 失败: {}", url, e)),
            },
            Err(e) => (false, format!("HTTP 客户端错误: {}", e)),
        }
    } else {
        (false, "已跳过（TCP 未连通）。".to_string())
    };
    checks.push(crate::jv!({
        "id": "gateway_http",
        "ok": http_ok,
        "title": "Gateway HTTP 探测（/__api/health）",
        "detail": http_detail
    }));

    let overall_ready = cred_ok && qq_enabled && chat_on && plugin_ok && tcp_ok && http_ok;

    let hints: Vec<String> = vec![
        "QQ 客户端提示「灵魂不在线」表示消息到了腾讯侧，但本机 OpenClaw Gateway 未就绪或未建立 QQ 长连接；仅通过「换 token」校验不能发现该问题。".to_string(),
        format!(
            "请确认本机 Gateway 已启动、端口与 openclaw.json 中 gateway.port（当前 {}）一致，并查看日志目录（如 ~/.openclaw/logs/）中 gateway 与 qqbot 相关报错。",
            port
        ),
        format!("官方排查说明见：{}", QQ_OPENCLAW_FAQ_URL),
    ];

    Ok(crate::jv!({
        "platform": "qqbot",
        "gatewayPort": port,
        "faqUrl": QQ_OPENCLAW_FAQ_URL,
        "checks": checks,
        "overallReady": overall_ready,
        "userHints": hints,
    }))
}

/// 列出当前已配置的平台清单
/// 若平台包含 accounts 子对象（多账号模式），返回各账号的安全显示字段
#[tauri::command]
pub async fn list_configured_platforms() -> Result<Value, String> {
    let cfg = super::config::load_openclaw_json()?;
    let mut result: Vec<Value> = vec![];

    if let Some(channels) = cfg.get("channels").and_then(|c| c.as_object()) {
        for (name, val) in channels {
            let enabled = val.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
            let mut accounts: Vec<Value> = vec![];

            // 提取多账号信息（仅安全字段，不含 appSecret 等敏感数据）
            if let Some(accts) = val.get("accounts").and_then(|a| a.as_object()) {
                for (acct_id, acct_val) in accts {
                    let mut entry = crate::jv!({ "accountId": acct_id });
                    if let Some(display_id) = account_display_value(acct_val, "appId")
                        .or_else(|| account_display_value(acct_val, "clientId"))
                        .or_else(|| account_display_value(acct_val, "account"))
                        .or_else(|| account_display_value(acct_val, "nick"))
                        .or_else(|| account_display_value(acct_val, "ship"))
                    {
                        entry["appId"] = Value::String(display_id);
                    }
                    accounts.push(entry);
                }
            }

            result.push(crate::jv!({
                "id": platform_list_id(name),
                "enabled": enabled,
                "accounts": accounts
            }));
        }
    }

    Ok(crate::jv!(result))
}
