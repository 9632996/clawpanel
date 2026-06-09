use crate::utils::openclaw_command;
/// 配置读写命令
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

include!("config_modules/version_policy.rs");
include!("config_modules/openclaw_config_io.rs");
include!("config_modules/config_validation_and_sync.rs");
include!("config_modules/version_status.rs");
include!("config_modules/openclaw_upgrade.rs");
include!("config_modules/install_lifecycle.rs");
include!("config_modules/node_and_backups.rs");
include!("config_modules/gateway_and_updates.rs");
include!("config_modules/path_cache.rs");
