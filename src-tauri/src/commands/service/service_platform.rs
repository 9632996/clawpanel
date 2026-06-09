#[cfg(target_os = "linux")]
#[path = "service_platform_linux.rs"]
mod service_platform_linux;
#[cfg(target_os = "macos")]
#[path = "service_platform_macos.rs"]
mod service_platform_macos;
#[cfg(target_os = "windows")]
#[path = "service_platform_windows.rs"]
mod service_platform_windows;

#[cfg(target_os = "linux")]
pub(super) use service_platform_linux::platform::*;
#[cfg(target_os = "macos")]
pub(super) use service_platform_macos::platform::*;
#[cfg(target_os = "windows")]
pub(super) use service_platform_windows::platform::*;

#[cfg(target_os = "windows")]
pub fn invalidate_cli_detection_cache() {
    invalidate_cli_cache();
}

#[cfg(not(target_os = "windows"))]
pub fn invalidate_cli_detection_cache() {}
