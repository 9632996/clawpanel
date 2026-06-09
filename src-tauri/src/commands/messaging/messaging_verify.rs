use serde_json::{Map, Value};

use super::messaging_common::{form_string, has_configured_messaging_value, is_enabled_form_flag};

pub(super) fn msteams_credential_missing_labels(form: &Map<String, Value>) -> Vec<&'static str> {
    if !has_configured_messaging_value(form.get("appId")) {
        return vec!["App ID"];
    }
    if has_configured_messaging_value(form.get("appPassword")) {
        return vec![];
    }
    if is_enabled_form_flag(form.get("useManagedIdentity")) {
        return vec![];
    }

    let auth_type = form_string(form, "authType").to_ascii_lowercase();
    let has_federated_credential = has_configured_messaging_value(form.get("certificatePath"))
        || has_configured_messaging_value(form.get("certificateThumbprint"));
    if auth_type == "federated" && has_federated_credential {
        return vec![];
    }
    if auth_type == "federated" {
        return vec!["Certificate Path / Certificate Thumbprint / Managed Identity / App Password"];
    }
    vec!["App Password"]
}

/// 在线校验 Bot 凭证（调用平台 API 验证 Token 是否有效）
#[tauri::command]
pub async fn verify_bot_token(platform: String, form: Value) -> Result<Value, String> {
    let form_obj = form.as_object().ok_or("表单数据格式错误")?;
    let client = super::build_http_client(std::time::Duration::from_secs(15), None)
        .map_err(|e| format!("HTTP 客户端初始化失败: {}", e))?;

    match platform.as_str() {
        "discord" => verify_discord(&client, form_obj).await,
        "telegram" => verify_telegram(&client, form_obj).await,
        "qqbot" => verify_qqbot(&client, form_obj).await,
        "feishu" => verify_feishu(&client, form_obj).await,
        "dingtalk" | "dingtalk-connector" => verify_dingtalk(&client, form_obj).await,
        "slack" => verify_slack(&client, form_obj).await,
        "zalo" => verify_zalo(&client, form_obj).await,
        "zalouser" => Ok(crate::jv!({
            "valid": true,
            "warnings": ["Zalo Personal 通过二维码登录维护本地会话；请使用 openclaw channels status --probe 检查登录状态"]
        })),
        "matrix" => verify_matrix(&client, form_obj).await,
        "signal" => verify_signal(&client, form_obj).await,
        "msteams" => verify_msteams(&client, form_obj).await,
        "imessage" => Ok(crate::jv!({
            "valid": true,
            "warnings": ["iMessage 使用本机或远端桥接运行，无需在线校验 Bot Token；请通过 Gateway 日志确认桥接进程状态"]
        })),
        "whatsapp" => Ok(crate::jv!({
            "valid": true,
            "warnings": ["WhatsApp 使用扫码登录，无需在线校验凭证；请通过「启动扫码登录」完成配对"]
        })),
        "clickclack" => Ok(crate::jv!({
            "valid": true,
            "warnings": ["ClickClack 面板已完成基础字段校验；实际连通性请通过 Gateway 启动日志或 openclaw channels status --probe 验证"]
        })),
        "nextcloud-talk" => Ok(crate::jv!({
            "valid": true,
            "warnings": ["Nextcloud Talk 面板已完成基础字段校验；实际连通性请通过 Gateway 启动日志或 openclaw channels status --probe 验证"]
        })),
        "twitch" => Ok(crate::jv!({
            "valid": true,
            "warnings": ["Twitch 面板已完成基础字段校验；实际连通性请通过 Gateway 启动日志或 openclaw channels status --probe 验证"]
        })),
        "nostr" => Ok(crate::jv!({
            "valid": true,
            "warnings": ["Nostr 面板已完成基础字段校验；实际连通性请通过 Gateway 启动日志或 openclaw channels status --probe 验证"]
        })),
        "irc" => Ok(crate::jv!({
            "valid": true,
            "warnings": ["IRC 面板已完成基础字段校验；实际连通性请通过 Gateway 启动日志或 openclaw channels status --probe 验证"]
        })),
        "tlon" => Ok(crate::jv!({
            "valid": true,
            "warnings": ["Tlon 面板已完成基础字段校验；实际连通性请通过 Gateway 启动日志或 openclaw channels status --probe 验证"]
        })),
        _ => Ok(crate::jv!({
            "valid": true,
            "warnings": ["该平台暂不支持在线校验"]
        })),
    }
}

async fn verify_slack(client: &reqwest::Client, form: &Map<String, Value>) -> Result<Value, String> {
    let bot_token = form.get("botToken").and_then(|v| v.as_str()).unwrap_or("").trim();
    if bot_token.is_empty() {
        return Ok(crate::jv!({ "valid": false, "errors": ["Bot Token 不能为空"] }));
    }

    let resp = client
        .post("https://slack.com/api/auth.test")
        .bearer_auth(bot_token)
        .send()
        .await
        .map_err(|e| format!("Slack API 连接失败: {}", e))?;

    let body: Value = resp.json().await.map_err(|e| format!("解析 Slack 响应失败: {}", e))?;

    if body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        let err = body.get("error").and_then(|v| v.as_str()).unwrap_or("unknown_error");
        return Ok(crate::jv!({ "valid": false, "errors": [format!("Slack 鉴权失败: {}", err)] }));
    }

    let team = body.get("team").and_then(|v| v.as_str()).unwrap_or("未知工作区");
    let user = body.get("user").and_then(|v| v.as_str()).unwrap_or("未知用户");

    Ok(crate::jv!({
        "valid": true,
        "details": [format!("工作区: {}", team), format!("Bot 用户: {}", user)]
    }))
}

async fn verify_matrix(client: &reqwest::Client, form: &Map<String, Value>) -> Result<Value, String> {
    let homeserver = form.get("homeserver").and_then(|v| v.as_str()).unwrap_or("").trim();
    let access_token = form.get("accessToken").and_then(|v| v.as_str()).unwrap_or("").trim();

    if homeserver.is_empty() {
        return Ok(crate::jv!({ "valid": false, "errors": ["Homeserver 不能为空"] }));
    }
    if access_token.is_empty() {
        return Ok(crate::jv!({ "valid": false, "errors": ["Access Token 不能为空"] }));
    }

    let base = homeserver.trim_end_matches('/');
    let resp = client
        .get(format!("{}/_matrix/client/v3/account/whoami", base))
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Matrix API 连接失败: {}", e))?;

    if resp.status() == 401 {
        return Ok(crate::jv!({ "valid": false, "errors": ["Access Token 无效或已失效"] }));
    }
    if !resp.status().is_success() {
        return Ok(crate::jv!({
            "valid": false,
            "errors": [format!("Matrix API 返回异常: {}", resp.status())]
        }));
    }

    let body: Value = resp.json().await.map_err(|e| format!("解析 Matrix 响应失败: {}", e))?;
    let user_id = body.get("user_id").and_then(|v| v.as_str()).unwrap_or("未知用户");
    let device_id = body.get("device_id").and_then(|v| v.as_str()).unwrap_or("未返回");

    Ok(crate::jv!({
        "valid": true,
        "details": [format!("用户: {}", user_id), format!("设备: {}", device_id)]
    }))
}

async fn verify_signal(client: &reqwest::Client, form: &Map<String, Value>) -> Result<Value, String> {
    let account = form.get("account").and_then(|v| v.as_str()).unwrap_or("").trim();
    if account.is_empty() {
        return Ok(crate::jv!({ "valid": false, "errors": ["Signal 号码不能为空"] }));
    }

    let http_url = form.get("httpUrl").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let http_host = form
        .get("httpHost")
        .and_then(|v| v.as_str())
        .unwrap_or("127.0.0.1")
        .trim()
        .to_string();
    let http_port = form
        .get("httpPort")
        .and_then(|v| v.as_str())
        .unwrap_or("8080")
        .trim()
        .to_string();

    let base = if !http_url.is_empty() {
        http_url
    } else {
        format!("http://{}:{}", http_host, http_port)
    };

    let url = format!("{}/v1/about", base.trim_end_matches('/'));
    match client.get(&url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let body: Value = resp.json().await.map_err(|e| format!("解析 signal-cli 响应失败: {}", e))?;
                let versions = body
                    .get("versions")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                    .unwrap_or_default();
                let mut details = vec![format!("号码: {}", account), format!("signal-cli 端点: {}", base)];
                if !versions.is_empty() {
                    details.push(format!("API 版本: {}", versions));
                }
                Ok(crate::jv!({ "valid": true, "details": details }))
            } else {
                Ok(crate::jv!({
                    "valid": false,
                    "errors": [format!("signal-cli HTTP 返回异常: {} — 请确认 signal-cli daemon 正在运行", resp.status())]
                }))
            }
        }
        Err(e) => Ok(crate::jv!({
            "valid": false,
            "errors": [format!("无法连接 signal-cli HTTP 端点 {} — {}", url, e)]
        })),
    }
}

async fn verify_msteams(client: &reqwest::Client, form: &Map<String, Value>) -> Result<Value, String> {
    let app_id = form.get("appId").and_then(|v| v.as_str()).unwrap_or("").trim();
    let app_password = form.get("appPassword").and_then(|v| v.as_str()).unwrap_or("").trim();
    let tenant_id = form
        .get("tenantId")
        .and_then(|v| v.as_str())
        .unwrap_or("botframework.com")
        .trim();

    if app_id.is_empty() {
        return Ok(crate::jv!({ "valid": false, "errors": ["App ID 不能为空"] }));
    }
    let missing_credentials = msteams_credential_missing_labels(form);
    if !missing_credentials.is_empty() {
        return Ok(crate::jv!({ "valid": false, "errors": [format!("缺少 {}", missing_credentials.join(" / "))] }));
    }
    if app_password.is_empty() {
        return Ok(crate::jv!({
            "valid": true,
            "warnings": ["当前 Teams 认证模式不使用 Client Secret；面板已完成结构校验，实际连通性请通过 Gateway 启动日志或 openclaw channels status --probe 验证。"],
            "details": [format!("App ID: {}", app_id)]
        }));
    }

    let token_url = format!(
        "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
        if tenant_id.is_empty() { "botframework.com" } else { tenant_id }
    );

    let resp = client
        .post(&token_url)
        .form(&[
            ("grant_type", "client_credentials"),
            ("client_id", app_id),
            ("client_secret", app_password),
            ("scope", "https://api.botframework.com/.default"),
        ])
        .send()
        .await
        .map_err(|e| format!("Azure AD 连接失败: {}", e))?;

    let body: Value = resp.json().await.map_err(|e| format!("解析 Azure AD 响应失败: {}", e))?;

    if body
        .get("access_token")
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .is_some()
    {
        let expires_in = body.get("expires_in").and_then(|v| v.as_u64()).unwrap_or(0);
        Ok(crate::jv!({
            "valid": true,
            "details": [
                format!("App ID: {}", app_id),
                format!("Tenant: {}", tenant_id),
                format!("Token 有效期: {}s", expires_in)
            ]
        }))
    } else {
        let err = body
            .get("error_description")
            .or_else(|| body.get("error"))
            .and_then(|v| v.as_str())
            .unwrap_or("凭证无效，请检查 App ID 和 App Password");
        Ok(crate::jv!({
            "valid": false,
            "errors": [err]
        }))
    }
}

async fn verify_discord(client: &reqwest::Client, form: &Map<String, Value>) -> Result<Value, String> {
    let token = form.get("token").and_then(|v| v.as_str()).unwrap_or("").trim();
    if token.is_empty() {
        return Ok(crate::jv!({ "valid": false, "errors": ["Bot Token 不能为空"] }));
    }

    let me_resp = client
        .get("https://discord.com/api/v10/users/@me")
        .header("Authorization", format!("Bot {}", token))
        .send()
        .await
        .map_err(|e| format!("Discord API 连接失败: {}", e))?;

    if me_resp.status() == 401 {
        return Ok(crate::jv!({ "valid": false, "errors": ["Bot Token 无效，请检查后重试"] }));
    }
    if !me_resp.status().is_success() {
        return Ok(crate::jv!({
            "valid": false,
            "errors": [format!("Discord API 返回异常: {}", me_resp.status())]
        }));
    }

    let me: Value = me_resp.json().await.map_err(|e| format!("解析响应失败: {}", e))?;
    if me.get("bot").and_then(|v| v.as_bool()) != Some(true) {
        return Ok(crate::jv!({
            "valid": false,
            "errors": ["提供的 Token 不属于 Bot 账号，请使用 Bot Token"]
        }));
    }

    let bot_name = me.get("username").and_then(|v| v.as_str()).unwrap_or("未知");
    let mut details = vec![format!("Bot: @{}", bot_name)];

    let guild_id = form.get("guildId").and_then(|v| v.as_str()).unwrap_or("").trim();
    if !guild_id.is_empty() {
        match client
            .get(format!("https://discord.com/api/v10/guilds/{}", guild_id))
            .header("Authorization", format!("Bot {}", token))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
                Ok(guild) => {
                    let name = guild.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                    details.push(format!("服务器: {}", name));
                }
                Err(err) => {
                    details.push(format!("服务器 ID 未能验证（解析失败: {}）", err));
                }
            },
            Ok(resp) if resp.status().as_u16() == 403 || resp.status().as_u16() == 404 => {
                return Ok(crate::jv!({
                    "valid": false,
                    "errors": [format!("无法访问服务器 {}，请确认 Bot 已加入该服务器", guild_id)]
                }));
            }
            _ => {
                details.push("服务器 ID 未能验证（网络问题）".into());
            }
        }
    }

    Ok(crate::jv!({
        "valid": true,
        "errors": [],
        "details": details
    }))
}

pub(super) async fn verify_qqbot(client: &reqwest::Client, form: &Map<String, Value>) -> Result<Value, String> {
    let app_id = form.get("appId").and_then(|v| v.as_str()).unwrap_or("").trim();
    let app_secret = form
        .get("clientSecret")
        .or_else(|| form.get("appSecret"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();

    if app_id.is_empty() {
        return Ok(crate::jv!({ "valid": false, "errors": ["AppID 不能为空"] }));
    }
    if app_secret.is_empty() {
        return Ok(crate::jv!({ "valid": false, "errors": ["ClientSecret 不能为空"] }));
    }

    let resp = client
        .post("https://bots.qq.com/app/getAppAccessToken")
        .json(&crate::jv!({
            "appId": app_id,
            "clientSecret": app_secret
        }))
        .send()
        .await
        .map_err(|e| format!("QQ Bot API 连接失败: {}", e))?;

    let body: Value = resp.json().await.map_err(|e| format!("解析响应失败: {}", e))?;

    if body.get("access_token").and_then(|v| v.as_str()).is_some() {
        Ok(crate::jv!({
            "valid": true,
            "errors": [],
            "details": [format!("AppID: {}", app_id)]
        }))
    } else {
        let msg = body
            .get("message")
            .or_else(|| body.get("msg"))
            .and_then(|v| v.as_str())
            .unwrap_or("凭证无效，请检查 AppID 和 AppSecret");
        Ok(crate::jv!({
            "valid": false,
            "errors": [msg]
        }))
    }
}

async fn verify_telegram(client: &reqwest::Client, form: &Map<String, Value>) -> Result<Value, String> {
    let bot_token = form.get("botToken").and_then(|v| v.as_str()).unwrap_or("").trim();
    if bot_token.is_empty() {
        return Ok(crate::jv!({ "valid": false, "errors": ["Bot Token 不能为空"] }));
    }

    let allowed = form.get("allowedUsers").and_then(|v| v.as_str()).unwrap_or("").trim();
    if allowed.is_empty() {
        return Ok(crate::jv!({ "valid": false, "errors": ["至少需要填写一个允许的用户 ID"] }));
    }

    let url = format!("https://api.telegram.org/bot{}/getMe", bot_token);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Telegram API 连接失败: {}", e))?;

    let body: Value = resp.json().await.map_err(|e| format!("解析响应失败: {}", e))?;

    if body.get("ok").and_then(|v| v.as_bool()) == Some(true) {
        let username = body
            .get("result")
            .and_then(|r| r.get("username"))
            .and_then(|v| v.as_str())
            .unwrap_or("未知");
        Ok(crate::jv!({
            "valid": true,
            "errors": [],
            "details": [format!("Bot: @{}", username)]
        }))
    } else {
        let desc = body.get("description").and_then(|v| v.as_str()).unwrap_or("Token 无效");
        Ok(crate::jv!({
            "valid": false,
            "errors": [desc]
        }))
    }
}

async fn verify_zalo(client: &reqwest::Client, form: &Map<String, Value>) -> Result<Value, String> {
    let bot_token = form.get("botToken").and_then(|v| v.as_str()).unwrap_or("").trim();
    let token_file = form.get("tokenFile").and_then(|v| v.as_str()).unwrap_or("").trim();

    if bot_token.is_empty() {
        if token_file.is_empty() {
            return Ok(crate::jv!({ "valid": false, "errors": ["请填写 Bot Token 或 Token File"] }));
        }
        return Ok(crate::jv!({
            "valid": true,
            "warnings": ["已配置 Token File；桌面端不会读取外部文件做在线校验"]
        }));
    }

    let resp = client
        .post(format!("https://bot-api.zaloplatforms.com/bot{}/getMe", bot_token))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("Zalo API 连接失败: {}", e))?;

    let body: Value = resp.json().await.map_err(|e| format!("解析响应失败: {}", e))?;

    if body.get("ok").and_then(|v| v.as_bool()) == Some(true) {
        Ok(crate::jv!({
            "valid": true,
            "errors": [],
            "details": ["Zalo Bot Token 已通过 getMe 校验"]
        }))
    } else {
        let msg = body
            .get("description")
            .or_else(|| body.get("message"))
            .and_then(|v| v.as_str())
            .unwrap_or("Zalo Bot Token 无效");
        Ok(crate::jv!({
            "valid": false,
            "errors": [msg]
        }))
    }
}

async fn verify_feishu(client: &reqwest::Client, form: &Map<String, Value>) -> Result<Value, String> {
    let app_id = form.get("appId").and_then(|v| v.as_str()).unwrap_or("").trim();
    let app_secret = form.get("appSecret").and_then(|v| v.as_str()).unwrap_or("").trim();

    if app_id.is_empty() {
        return Ok(crate::jv!({ "valid": false, "errors": ["App ID 不能为空"] }));
    }
    if app_secret.is_empty() {
        return Ok(crate::jv!({ "valid": false, "errors": ["App Secret 不能为空"] }));
    }

    let domain = form.get("domain").and_then(|v| v.as_str()).unwrap_or("").trim();
    let base_url = if domain == "lark" {
        "https://open.larksuite.com"
    } else {
        "https://open.feishu.cn"
    };

    let resp = client
        .post(format!("{}/open-apis/auth/v3/tenant_access_token/internal", base_url))
        .json(&crate::jv!({
            "app_id": app_id,
            "app_secret": app_secret
        }))
        .send()
        .await
        .map_err(|e| format!("飞书 API 连接失败: {}", e))?;

    let body: Value = resp.json().await.map_err(|e| format!("解析响应失败: {}", e))?;

    let code = body.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    if code == 0 {
        Ok(crate::jv!({
            "valid": true,
            "errors": [],
            "details": [format!("App ID: {}", app_id)]
        }))
    } else {
        let msg = body
            .get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("凭证无效，请检查 App ID 和 App Secret");
        Ok(crate::jv!({
            "valid": false,
            "errors": [msg]
        }))
    }
}

async fn verify_dingtalk(client: &reqwest::Client, form: &Map<String, Value>) -> Result<Value, String> {
    let client_id = form.get("clientId").and_then(|v| v.as_str()).unwrap_or("").trim();
    let client_secret = form.get("clientSecret").and_then(|v| v.as_str()).unwrap_or("").trim();

    if client_id.is_empty() {
        return Ok(crate::jv!({ "valid": false, "errors": ["Client ID 不能为空"] }));
    }
    if client_secret.is_empty() {
        return Ok(crate::jv!({ "valid": false, "errors": ["Client Secret 不能为空"] }));
    }

    let resp = client
        .post("https://api.dingtalk.com/v1.0/oauth2/accessToken")
        .json(&crate::jv!({
            "appKey": client_id,
            "appSecret": client_secret
        }))
        .send()
        .await
        .map_err(|e| format!("钉钉 API 连接失败: {}", e))?;

    let body: Value = resp.json().await.map_err(|e| format!("解析响应失败: {}", e))?;

    if body
        .get("accessToken")
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .is_some()
        || body
            .get("access_token")
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty())
            .is_some()
    {
        Ok(crate::jv!({
            "valid": true,
            "errors": [],
            "details": [
                format!("AppKey: {}", client_id),
                "已通过 accessToken 接口校验".to_string()
            ]
        }))
    } else {
        let msg = body
            .get("message")
            .or_else(|| body.get("msg"))
            .or_else(|| body.get("errmsg"))
            .and_then(|v| v.as_str())
            .unwrap_or("凭证无效，请检查 Client ID 和 Client Secret");
        Ok(crate::jv!({
            "valid": false,
            "errors": [msg]
        }))
    }
}
