use std::net::IpAddr;
use std::time::Duration;

use super::command_config_paths::read_panel_config_value;

pub(crate) fn configured_proxy_url() -> Option<String> {
    let value = read_panel_config_value()?;
    let raw = value
        .get("networkProxy")
        .and_then(|entry| {
            if let Some(obj) = entry.as_object() {
                obj.get("url").and_then(|v| v.as_str())
            } else {
                entry.as_str()
            }
        })?
        .trim()
        .to_string();
    if raw.is_empty() {
        None
    } else {
        Some(raw)
    }
}

fn should_bypass_proxy_host(host: &str) -> bool {
    let lower = host.trim().to_ascii_lowercase();
    if lower.is_empty() || lower == "localhost" || lower.ends_with(".local") {
        return true;
    }
    if let Ok(ip) = lower.parse::<IpAddr>() {
        return match ip {
            IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local(),
            IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local() || v6.is_unicast_link_local(),
        };
    }
    false
}

/// 构建 HTTP 客户端，use_proxy=true 时走用户配置的代理
pub(crate) fn build_http_client(timeout: Duration, user_agent: Option<&str>) -> Result<reqwest::Client, String> {
    build_http_client_opt(timeout, user_agent, true)
}

/// 构建模型请求用的 HTTP 客户端
/// 默认不走代理；用户在面板设置中开启 proxyModelRequests 后才走代理
pub(crate) fn build_http_client_no_proxy(timeout: Duration, user_agent: Option<&str>) -> Result<reqwest::Client, String> {
    let use_proxy = read_panel_config_value()
        .and_then(|v| v.get("networkProxy")?.get("proxyModelRequests")?.as_bool())
        .unwrap_or(false);
    build_http_client_opt(timeout, user_agent, use_proxy)
}

fn build_http_client_opt(timeout: Duration, user_agent: Option<&str>, use_proxy: bool) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .timeout(timeout)
        .gzip(true)
        .brotli(true)
        .deflate(true);
    if let Some(ua) = user_agent {
        builder = builder.user_agent(ua);
    }
    if use_proxy {
        if let Some(proxy_url) = configured_proxy_url() {
            let proxy_value = proxy_url.clone();
            builder = builder.proxy(reqwest::Proxy::custom(move |url| {
                let host = url.host_str().unwrap_or("");
                if should_bypass_proxy_host(host) {
                    None
                } else {
                    Some(proxy_value.clone())
                }
            }));
        }
    }
    builder.build().map_err(|e| e.to_string())
}

pub(crate) fn apply_proxy_env(cmd: &mut std::process::Command) {
    if let Some(proxy_url) = configured_proxy_url() {
        cmd.env("HTTP_PROXY", &proxy_url)
            .env("HTTPS_PROXY", &proxy_url)
            .env("http_proxy", &proxy_url)
            .env("https_proxy", &proxy_url)
            .env("NO_PROXY", "localhost,127.0.0.1,::1")
            .env("no_proxy", "localhost,127.0.0.1,::1");
    }
}

pub(crate) fn apply_proxy_env_tokio(cmd: &mut tokio::process::Command) {
    if let Some(proxy_url) = configured_proxy_url() {
        cmd.env("HTTP_PROXY", &proxy_url)
            .env("HTTPS_PROXY", &proxy_url)
            .env("http_proxy", &proxy_url)
            .env("https_proxy", &proxy_url)
            .env("NO_PROXY", "localhost,127.0.0.1,::1")
            .env("no_proxy", "localhost,127.0.0.1,::1");
    }
}
