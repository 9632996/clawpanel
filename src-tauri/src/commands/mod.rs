mod command_config_paths;
mod command_http;
mod command_runtime_path;

pub mod agent;
#[path = "agent/agent_workspace.rs"]
pub mod agent_workspace;
pub mod assistant;
pub mod cli_conflict;
pub mod codewhale;
pub mod codex;
pub mod config;
#[path = "config/config_model_common.rs"]
mod config_model_common;
#[path = "config/config_model_response.rs"]
mod config_model_response;
#[path = "config/config_model_runtime.rs"]
pub mod config_model_runtime;
#[path = "config/config_model_scan.rs"]
pub mod config_model_scan;
pub mod device;
pub mod diagnose;
pub mod extensions;
pub mod hermes;
#[path = "hermes/hermes_api_server.rs"]
mod hermes_api_server;
#[path = "hermes/hermes_dashboard.rs"]
pub mod hermes_dashboard;
#[path = "hermes/hermes_dashboard_assets.rs"]
pub mod hermes_dashboard_assets;
#[path = "hermes/hermes_dashboard_stub.rs"]
pub mod hermes_dashboard_stub;
#[path = "hermes/hermes_env_config.rs"]
pub mod hermes_env_config;
#[path = "hermes/hermes_fs.rs"]
pub mod hermes_fs;
#[path = "hermes/hermes_lazy_deps.rs"]
pub mod hermes_lazy_deps;
#[path = "hermes/hermes_multi_gateway.rs"]
pub mod hermes_multi_gateway;
#[path = "hermes/hermes_providers.rs"]
pub mod hermes_providers;
#[path = "hermes/hermes_runtime.rs"]
pub mod hermes_runtime;
#[path = "hermes/hermes_workspace_assets.rs"]
pub mod hermes_workspace_assets;
pub mod logs;
pub mod memory;
pub mod messaging;
#[path = "messaging/messaging_bindings.rs"]
pub mod messaging_bindings;
#[path = "messaging/messaging_channel_actions.rs"]
pub mod messaging_channel_actions;
#[path = "messaging/messaging_common.rs"]
mod messaging_common;
#[path = "messaging/messaging_diagnose.rs"]
pub mod messaging_diagnose;
#[path = "messaging/messaging_diagnosis_common.rs"]
mod messaging_diagnosis_common;
#[path = "messaging/messaging_plugins.rs"]
pub mod messaging_plugins;
#[path = "messaging/messaging_verify.rs"]
pub mod messaging_verify;
pub mod model_tools;
pub mod pairing;
pub mod service;
#[path = "service/service_gateway_owner.rs"]
pub mod service_gateway_owner;
#[path = "service/service_platform.rs"]
pub mod service_platform;
pub mod skillhub;
pub mod skills;
pub mod update;

#[cfg(target_os = "windows")]
pub(crate) use command_config_paths::windows_npm_global_prefix;
pub(crate) use command_config_paths::{
    gateway_listen_port, openclaw_dir, openclaw_search_paths, panel_config_candidate_paths, panel_config_path,
    portable_product_root, read_panel_config_value, zhizhua_url,
};
pub(crate) use command_http::{
    apply_proxy_env, apply_proxy_env_tokio, build_http_client, build_http_client_no_proxy, configured_proxy_url,
};
pub(crate) use command_runtime_path::{enhanced_path, refresh_enhanced_path};
