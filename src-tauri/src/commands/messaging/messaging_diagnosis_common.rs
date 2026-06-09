use serde_json::{Map, Value};

use super::messaging_common::{
    channel_root_has_messaging_credential, form_string, has_configured_messaging_value, platform_list_id, platform_storage_key,
};
use super::messaging_verify::msteams_credential_missing_labels;
pub(super) fn required_channel_credential_fields(platform: &str, form: &Map<String, Value>) -> Vec<(&'static str, &'static str)> {
    match platform_storage_key(platform) {
        "telegram" => vec![("botToken", "Bot Token")],
        "discord" => vec![("token", "Bot Token")],
        "feishu" => vec![("appId", "App ID"), ("appSecret", "App Secret")],
        "dingtalk-connector" => vec![("clientId", "Client ID"), ("clientSecret", "Client Secret")],
        "mattermost" => vec![("botToken", "Bot Token"), ("baseUrl", "Base URL")],
        "synology-chat" => vec![("token", "Token"), ("incomingUrl", "Incoming URL")],
        "clickclack" => vec![("baseUrl", "Base URL"), ("token", "Token"), ("workspace", "Workspace")],
        "nextcloud-talk" => vec![("baseUrl", "Base URL")],
        "nostr" => vec![("privateKey", "Private Key")],
        "irc" => vec![("host", "Host"), ("nick", "Nick")],
        "tlon" => vec![("ship", "Ship"), ("url", "URL"), ("code", "Code")],
        "twitch" => vec![
            ("username", "Username"),
            ("accessToken", "Access Token"),
            ("clientId", "Client ID"),
            ("channel", "Channel"),
        ],
        "signal" => vec![("account", "Signal 账号")],
        "slack" => {
            let mode = form_string(form, "mode");
            vec![
                ("botToken", "Bot Token"),
                if mode == "http" {
                    ("signingSecret", "Signing Secret")
                } else {
                    ("appToken", "App Token")
                },
            ]
        }
        "matrix" => {
            if has_configured_messaging_value(form.get("accessToken")) {
                vec![("accessToken", "Access Token")]
            } else {
                vec![("homeserver", "Homeserver"), ("userId", "User ID"), ("password", "Password")]
            }
        }
        "msteams" => msteams_credential_missing_labels(form)
            .into_iter()
            .map(|label| {
                if label == "App ID" {
                    ("appId", "App ID")
                } else {
                    ("__msteamsAuth", label)
                }
            })
            .collect(),
        _ => vec![],
    }
}

pub(super) fn channel_any_credential_fields(platform: &str) -> Vec<(&'static str, &'static str)> {
    match platform_storage_key(platform) {
        "zalo" => vec![("botToken", "Bot Token"), ("tokenFile", "Token File")],
        "googlechat" => vec![
            ("serviceAccountFile", "Service Account File"),
            ("serviceAccount", "Service Account JSON"),
            ("serviceAccountRef", "Service Account SecretRef"),
        ],
        _ => vec![],
    }
}

pub(super) fn channel_any_credential_groups(platform: &str) -> Vec<(&'static str, Vec<(&'static str, &'static str)>)> {
    match platform_storage_key(platform) {
        "line" => vec![
            (
                "Channel Access Token 或 Token File",
                vec![("channelAccessToken", "Channel Access Token"), ("tokenFile", "Token File")],
            ),
            (
                "Channel Secret 或 Secret File",
                vec![("channelSecret", "Channel Secret"), ("secretFile", "Secret File")],
            ),
        ],
        "nextcloud-talk" => vec![(
            "Bot Secret 或 Secret File",
            vec![("botSecret", "Bot Secret"), ("botSecretFile", "Secret File")],
        )],
        _ => vec![],
    }
}

pub(super) fn channel_diagnosis_credentials_ready(platform: &str, form: &Map<String, Value>) -> bool {
    if matches!(platform_storage_key(platform), "zalouser" | "imessage" | "whatsapp") {
        return true;
    }
    if platform_storage_key(platform) == "msteams" {
        return msteams_credential_missing_labels(form).is_empty();
    }
    let required_fields = required_channel_credential_fields(platform, form);
    let any_groups = channel_any_credential_groups(platform);
    if !required_fields.is_empty() {
        return required_fields
            .iter()
            .all(|(key, _)| has_configured_messaging_value(form.get(*key)))
            && any_groups
                .iter()
                .all(|(_, fields)| fields.iter().any(|(key, _)| has_configured_messaging_value(form.get(*key))));
    }
    if !any_groups.is_empty() {
        return any_groups
            .iter()
            .all(|(_, fields)| fields.iter().any(|(key, _)| has_configured_messaging_value(form.get(*key))));
    }
    let any_fields = channel_any_credential_fields(platform);
    if !any_fields.is_empty() {
        return any_fields
            .iter()
            .any(|(key, _)| has_configured_messaging_value(form.get(*key)));
    }
    channel_root_has_messaging_credential(form)
}

pub(super) fn credential_labels(fields: &[(&'static str, &'static str)]) -> String {
    fields.iter().map(|(_, label)| *label).collect::<Vec<_>>().join(" / ")
}

pub(super) fn json_string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str())
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn compact_diagnostic_details(values: &[String]) -> String {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("；")
}

pub(super) fn build_openclaw_channel_diagnosis(
    platform: &str,
    account_id: Option<&str>,
    config_exists: bool,
    channel_enabled: bool,
    form: &Map<String, Value>,
    verify_result: Option<Value>,
    verify_error: Option<String>,
) -> Value {
    let storage_key = platform_storage_key(platform);
    let display_platform = platform_list_id(storage_key);
    let account_id = account_id.map(str::trim).filter(|id| !id.is_empty());
    let mut checks = Vec::new();

    checks.push(crate::jv!({
        "id": "config_exists",
        "ok": config_exists,
        "title": "渠道配置已保存",
        "detail": if config_exists {
            format!(
                "已读取 channels.{}{} 的配置。",
                storage_key,
                account_id.map(|id| format!(".accounts.{}", id)).unwrap_or_default()
            )
        } else {
            format!(
                "未在 openclaw.json 中找到 {} 渠道配置，请先在「渠道列表」接入并保存。",
                display_platform
            )
        }
    }));

    checks.push(crate::jv!({
        "id": "channel_enabled",
        "ok": channel_enabled,
        "title": "渠道已启用",
        "detail": if channel_enabled {
            "渠道未被显式禁用，Gateway 重启/重载后会尝试加载。".to_string()
        } else {
            format!("channels.{}.enabled 为 false，请先在渠道列表中启用该渠道。", storage_key)
        }
    }));

    let required_fields = required_channel_credential_fields(storage_key, form);
    let any_fields = channel_any_credential_fields(storage_key);
    let any_groups = channel_any_credential_groups(storage_key);
    let missing: Vec<&str> = if storage_key == "msteams" {
        msteams_credential_missing_labels(form)
    } else {
        required_fields
            .iter()
            .filter(|(key, _)| !has_configured_messaging_value(form.get(*key)))
            .map(|(_, label)| *label)
            .collect()
    };
    let missing_groups: Vec<&str> = any_groups
        .iter()
        .filter(|(_, fields)| !fields.iter().any(|(key, _)| has_configured_messaging_value(form.get(*key))))
        .map(|(label, _)| *label)
        .collect();
    let any_credential_ok = if any_fields.is_empty() {
        false
    } else {
        any_fields
            .iter()
            .any(|(key, _)| has_configured_messaging_value(form.get(*key)))
    };
    let credential_ok = if matches!(storage_key, "zalouser" | "imessage" | "whatsapp") {
        config_exists
    } else if !required_fields.is_empty() {
        missing.is_empty() && missing_groups.is_empty()
    } else if !any_groups.is_empty() {
        missing_groups.is_empty()
    } else if !any_fields.is_empty() {
        any_credential_ok
    } else {
        channel_root_has_messaging_credential(form)
    };
    let required_labels = credential_labels(&required_fields);
    let any_labels = credential_labels(&any_fields);
    checks.push(crate::jv!({
        "id": "credentials",
        "ok": credential_ok,
        "title": if storage_key == "zalouser" {
            "登录/会话配置"
        } else if storage_key == "imessage" {
            "桥接运行配置"
        } else if storage_key == "whatsapp" {
            "扫码/会话配置"
        } else {
            "必要凭证字段"
        },
        "detail": if storage_key == "zalouser" {
            "Zalo Personal 通过二维码登录保存本地会话；配置已保存后，请按手动命令完成或刷新登录。".to_string()
        } else if storage_key == "imessage" {
            if config_exists {
                "iMessage 使用本机或远端桥接运行，不需要 Bot Token；已保存基础运行配置。".to_string()
            } else {
                "尚未保存 iMessage 渠道配置，请先填写并保存。".to_string()
            }
        } else if storage_key == "whatsapp" {
            if config_exists {
                "WhatsApp 使用扫码登录保存本地会话，不需要 Bot Token；已保存扫码运行配置。".to_string()
            } else {
                "尚未保存 WhatsApp 渠道配置，请先填写并保存，再启动扫码登录。".to_string()
            }
        } else if credential_ok {
            if !required_fields.is_empty() {
                if !any_groups.is_empty() {
                    format!(
                        "已填写 {}；{}。",
                        required_labels,
                        any_groups
                            .iter()
                            .map(|(label, _)| *label)
                            .collect::<Vec<_>>()
                            .join("；")
                    )
                } else {
                    format!("已填写 {}。", required_labels)
                }
            } else if !any_groups.is_empty() {
                format!(
                    "已填写 {}。",
                    any_groups
                        .iter()
                        .map(|(label, _)| *label)
                        .collect::<Vec<_>>()
                        .join("；")
                )
            } else if !any_fields.is_empty() {
                format!("已填写 {} 其中一项。", any_labels)
            } else {
                "已检测到可用凭证字段。".to_string()
            }
        } else if !missing.is_empty() {
            format!("缺少 {}，请补齐后保存。", missing.join(" / "))
        } else if !missing_groups.is_empty() {
            format!("缺少 {}，请补齐后保存。", missing_groups.join("；"))
        } else if !any_fields.is_empty() {
            format!("缺少 {}，至少填写一项后保存。", any_labels)
        } else {
            "未检测到可用凭证字段，请检查渠道配置。".to_string()
        }
    }));

    if let Some(error) = verify_error.filter(|error| !error.trim().is_empty()) {
        checks.push(crate::jv!({
            "id": "online_verify",
            "ok": false,
            "title": "平台在线校验",
            "detail": error
        }));
    } else if let Some(result) = verify_result {
        let valid = result.get("valid").and_then(|v| v.as_bool()) == Some(true);
        let errors = json_string_list(result.get("errors"));
        let warnings = json_string_list(result.get("warnings"));
        let details = json_string_list(result.get("details"));
        let verify_ok = valid || (!warnings.is_empty() && errors.is_empty());
        checks.push(crate::jv!({
            "id": "online_verify",
            "ok": verify_ok,
            "title": "平台在线校验",
            "detail": if valid {
                let detail = compact_diagnostic_details(&details);
                if detail.is_empty() {
                    "平台 API 已接受当前凭证。".to_string()
                } else {
                    detail
                }
            } else {
                let detail = compact_diagnostic_details(&errors);
                if detail.is_empty() {
                    let warning_detail = compact_diagnostic_details(&warnings);
                    if warning_detail.is_empty() {
                        "该平台暂不支持在线校验。".to_string()
                    } else {
                        warning_detail
                    }
                } else {
                    detail
                }
            }
        }));
    } else {
        checks.push(crate::jv!({
            "id": "online_verify",
            "ok": true,
            "title": "平台在线校验",
            "detail": "未执行在线校验，仅完成本地配置检查。"
        }));
    }

    let failed_count = checks
        .iter()
        .filter(|check| check.get("ok").and_then(|v| v.as_bool()) != Some(true))
        .count();
    crate::jv!({
        "ok": failed_count == 0,
        "overallReady": failed_count == 0,
        "platform": display_platform,
        "accountId": account_id,
        "checks": checks,
        "userHints": if failed_count == 0 {
            vec!["配置侧检查已通过。若仍收不到消息，请确认 Gateway 已重启、机器人已加入目标会话，并检查 Gateway 日志。"]
        } else {
            vec![
                "先修复未通过的检查项，保存渠道后重启或重载 Gateway。",
                "在线校验只能证明平台凭证可用；群聊白名单、机器人邀请和平台回调仍需在对应平台控制台确认。",
            ]
        }
    })
}
