//! Hermes Agent 安装与管理命令
//!
//! 通过 uv 实现零依赖安装：
//!   1. 下载 uv 单文件二进制
//!   2. uv tool install hermes-agent --python 3.11
//!   3. 写入 ~/.hermes/config.yaml + .env

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::OnceLock;
use tauri::Emitter;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

use super::hermes_api_server::ensure_api_server_enabled;
use super::hermes_dashboard::ensure_managed_dashboard_ready;
use super::hermes_dashboard_stub::inject_hermes_dashboard_compat_stub;
use super::hermes_env_config::parse_env_file_lines;
use super::hermes_runtime::{
    apply_hermes_runtime_env_std, apply_hermes_runtime_env_tokio, hermes_enhanced_path, hermes_executable_path, hermes_home,
    hermes_venv_dir, parse_python_version, run_at_path, run_silent, uv_bin_dir, uv_bin_path, uv_download_url,
    HERMES_DASHBOARD_SESSION_TOKEN,
};

include!("hermes_modules/gateway_runtime.rs");
include!("hermes_modules/install_detection.rs");
include!("hermes_modules/config_merge_common.rs");
include!("hermes_modules/channel_config_values.rs");
include!("hermes_modules/performance_routing_config.rs");
include!("hermes_modules/model_and_hooks_config.rs");
include!("hermes_modules/mcp_provider_toolsets.rs");
include!("hermes_modules/display_security_config.rs");
include!("hermes_modules/config_normalizers.rs");
include!("hermes_modules/system_policy_config.rs");
include!("hermes_modules/integration_config.rs");
include!("hermes_modules/runtime_channel_config.rs");
include!("hermes_modules/channel_commands.rs");
include!("hermes_modules/advanced_config_commands.rs");
include!("hermes_modules/model_gateway_commands.rs");
include!("hermes_modules/health_proxy_stream.rs");
include!("hermes_modules/agent_run_dashboard.rs");
include!("hermes_modules/session_analytics.rs");
include!("hermes_modules/tests/auxiliary_tool_loop_streaming_tests.rs");
include!("hermes_modules/tests/web_model_catalog_context_tests.rs");
include!("hermes_modules/tests/checkpoints_cron_logging_tests.rs");
include!("hermes_modules/tests/memory_skills_model_tests.rs");
include!("hermes_modules/tests/hooks_mcp_toolsets_tests.rs");
include!("hermes_modules/tests/runtime_display_kanban_tests.rs");
include!("hermes_modules/tests/security_channel_tests.rs");
