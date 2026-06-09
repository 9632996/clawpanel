/// 消息渠道管理
/// 负责 Telegram / Discord / QQ Bot 等消息渠道的配置持久化与凭证校验
/// 配置写入 openclaw.json 的 channels / plugins 节点
use serde_json::{Map, Value};

use super::messaging_bindings::create_agent_binding;
use super::messaging_common::{
    channel_root_has_messaging_credential, ensure_chat_completions_enabled, form_string, has_configured_messaging_value,
    insert_secret_aware_form_alias, insert_secret_aware_form_value, insert_string_if_present, platform_list_id,
    platform_storage_key, preserve_messaging_credential_refs, resolve_messaging_credential_value_for_save,
    resolve_messaging_credential_value_for_save_alias, secret_ref_placeholder,
};
use super::messaging_diagnose::{qqbot_channel_has_credentials, QQBOT_DEFAULT_ACCOUNT_ID};
use super::messaging_plugins::{
    cleanup_legacy_plugin_backup_dir, disable_legacy_plugin, ensure_openclaw_qqbot_plugin, ensure_plugin_allowed,
};
use super::messaging_verify::msteams_credential_missing_labels;

include!("messaging_modules/form_value_helpers.rs");
include!("messaging_modules/read_platform_config.rs");
include!("messaging_modules/save_platform.rs");
include!("messaging_modules/platform_state.rs");
