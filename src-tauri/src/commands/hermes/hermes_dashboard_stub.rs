use tauri::Emitter;

use super::hermes_runtime::locate_hermes_cli_package_dir;

// ---------------------------------------------------------------------------
// Hermes Dashboard compat stubs
//
// hermes-agent 0.14.0 (both PyPI wheel and `git+...` source) ships
// `hermes_cli/web_server.py` with hard imports of `hermes_cli.dashboard_auth.*`
// submodules whose source files are NOT included in the distribution. It also
// omits the built dashboard SPA (`hermes_cli/web_dist/`). On Windows in
// particular, the missing dashboard_auth subpackage breaks `hermes dashboard`
// completely, taking down every ClawPanel page that talks to port 9119
// (Profile, Kanban, OAuth, Channels, Sessions detail).
//
// To stay self-sufficient (per project policy: do not patch upstream), we
// inject a minimal pass-through stub into the installed venv:
//   - `hermes_cli/dashboard_auth/{__init__,audit,middleware,prefix,routes,ws_tickets}.py`
//     so all `from hermes_cli.dashboard_auth.* import ...` lines resolve.
//     Auth is a no-op; valid for loopback (127.0.0.1) bindings where the
//     auth gate is intentionally disabled.
//   - `hermes_cli/web_dist/index.html` so `mount_spa()` takes the
//     token-injecting branch instead of the `Frontend not built` 404 branch.
//     Without this, the panel's `dashboard_session_token` scrape returns
//     404 and all `/api/*` calls fail with 401.
//
// The injection is idempotent: if upstream eventually ships either piece,
// the corresponding stub write is skipped so the real implementation wins.
// ---------------------------------------------------------------------------

const HERMES_DASHBOARD_AUTH_INIT_PY: &str = r#""""Workbench-injected stub for hermes_cli.dashboard_auth.

Upstream hermes-agent ships web_server.py with imports referencing this
subpackage, but the actual source files are NOT included in the wheel or
the public git repo. To keep Hermes Dashboard usable in loopback
(127.0.0.1) mode, the workbench injects this minimal pass-through stub at
install/upgrade time.

When upstream eventually ships the real module, delete this directory
and reinstall hermes-agent; the real implementation will be picked up.
"""
from __future__ import annotations

from typing import Iterable, List


class DashboardAuthProvider:
    """Stub base class. Real providers inherit from this."""

    name: str = ""


_REGISTERED: List["DashboardAuthProvider"] = []


def register_provider(provider: "DashboardAuthProvider") -> None:
    """No-op stub. The workbench binds to 127.0.0.1 so the gate is disabled."""
    if isinstance(provider, DashboardAuthProvider):
        _REGISTERED.append(provider)


def list_providers() -> Iterable["DashboardAuthProvider"]:
    """Return registered providers (empty on loopback)."""
    return list(_REGISTERED)


__all__ = ["DashboardAuthProvider", "register_provider", "list_providers"]
"#;

const HERMES_DASHBOARD_AUTH_AUDIT_PY: &str = r#""""Workbench stub: hermes_cli.dashboard_auth.audit"""
from __future__ import annotations

from enum import Enum
from typing import Any


class AuditEvent(str, Enum):
    LOGIN = "login"
    LOGOUT = "logout"
    LOGIN_FAILED = "login_failed"
    WS_TICKET_MINTED = "ws_ticket_minted"
    WS_TICKET_REJECTED = "ws_ticket_rejected"
    PROVIDER_REGISTERED = "provider_registered"


def audit_log(event: Any, **fields: Any) -> None:
    """No-op stub. Real implementation appends to an audit log file."""
    return None


__all__ = ["AuditEvent", "audit_log"]
"#;

const HERMES_DASHBOARD_AUTH_MIDDLEWARE_PY: &str = r#""""Workbench stub: hermes_cli.dashboard_auth.middleware"""
from __future__ import annotations


async def gated_auth_middleware(request, call_next):
    """Pass-through ASGI middleware. Real one enforces JWT on non-loopback."""
    return await call_next(request)


__all__ = ["gated_auth_middleware"]
"#;

const HERMES_DASHBOARD_AUTH_PREFIX_PY: &str = r#""""Workbench stub: hermes_cli.dashboard_auth.prefix"""
from __future__ import annotations


def normalise_prefix(prefix: str) -> str:
    """Normalise X-Forwarded-Prefix style values to a leading-slash form."""
    if not prefix:
        return ""
    return "/" + prefix.strip("/")


__all__ = ["normalise_prefix"]
"#;

const HERMES_DASHBOARD_AUTH_ROUTES_PY: &str = r#""""Workbench stub: hermes_cli.dashboard_auth.routes"""
from __future__ import annotations

from fastapi import APIRouter

router = APIRouter()


__all__ = ["router"]
"#;

const HERMES_DASHBOARD_AUTH_WS_TICKETS_PY: &str = r#""""Workbench stub: hermes_cli.dashboard_auth.ws_tickets"""
from __future__ import annotations


class TicketInvalid(Exception):
    """Raised when a WS ticket is rejected. Stub never raises."""


def mint_ticket(*args, **kwargs) -> str:
    """Stub. Real one mints short-lived JWTs."""
    return "stub-loopback-ticket"


def consume_ticket(*args, **kwargs) -> None:
    """Stub. Real one validates signature + expiry. Never raises here."""
    return None


__all__ = ["TicketInvalid", "mint_ticket", "consume_ticket"]
"#;

const HERMES_DASHBOARD_WEB_DIST_INDEX_HTML: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <title>Hermes Dashboard (Workbench stub)</title>
    <meta name="generator" content="zhizhua-workbench-dashboard-spa-stub">
  </head>
  <body>
    <main style="font-family:system-ui,-apple-system,sans-serif;padding:32px;color:#333">
      <h1 style="margin:0 0 16px">Hermes Dashboard</h1>
      <p>This SPA placeholder is injected by the workbench so the dashboard backend
         emits a session token. The workbench provides its own UI; the upstream
         SPA is not shipped with the wheel.</p>
    </main>
  </body>
</html>
"#;

/// Inject the dashboard_auth and web_dist stubs into the installed hermes-agent
/// venv if upstream did not ship them. Idempotent: existing files are never
/// overwritten so the real implementation, if/when it lands, wins.
///
/// Stub injection failures are logged and swallowed — install/upgrade succeeds
/// regardless so users aren't blocked by best-effort compatibility patches.
pub(super) fn inject_hermes_dashboard_compat_stub(app: &tauri::AppHandle) {
    let hermes_cli = match locate_hermes_cli_package_dir() {
        Some(p) => p,
        None => {
            let _ = app.emit("hermes-install-log", "⚠ 跳过 dashboard 兼容 stub 注入：未找到 hermes_cli 包目录");
            return;
        }
    };

    let mut wrote_auth = false;
    let auth_dir = hermes_cli.join("dashboard_auth");
    if !auth_dir.join("__init__.py").exists() {
        if let Err(e) = std::fs::create_dir_all(&auth_dir) {
            let _ = app.emit("hermes-install-log", format!("⚠ 无法创建 dashboard_auth 目录: {e}"));
            return;
        }
        let files: [(&str, &str); 6] = [
            ("__init__.py", HERMES_DASHBOARD_AUTH_INIT_PY),
            ("audit.py", HERMES_DASHBOARD_AUTH_AUDIT_PY),
            ("middleware.py", HERMES_DASHBOARD_AUTH_MIDDLEWARE_PY),
            ("prefix.py", HERMES_DASHBOARD_AUTH_PREFIX_PY),
            ("routes.py", HERMES_DASHBOARD_AUTH_ROUTES_PY),
            ("ws_tickets.py", HERMES_DASHBOARD_AUTH_WS_TICKETS_PY),
        ];
        for (name, content) in files {
            let path = auth_dir.join(name);
            if let Err(e) = std::fs::write(&path, content) {
                let _ = app.emit("hermes-install-log", format!("⚠ 写入 dashboard_auth/{name} 失败: {e}"));
                return;
            }
        }
        wrote_auth = true;
    }

    let mut wrote_dist = false;
    let dist_dir = hermes_cli.join("web_dist");
    let index_path = dist_dir.join("index.html");
    if !index_path.exists() {
        if let Err(e) = std::fs::create_dir_all(dist_dir.join("assets")) {
            let _ = app.emit("hermes-install-log", format!("⚠ 无法创建 web_dist 目录: {e}"));
            return;
        }
        if let Err(e) = std::fs::write(&index_path, HERMES_DASHBOARD_WEB_DIST_INDEX_HTML) {
            let _ = app.emit("hermes-install-log", format!("⚠ 写入 web_dist/index.html 失败: {e}"));
            return;
        }
        wrote_dist = true;
    }

    if wrote_auth || wrote_dist {
        let mut parts: Vec<&str> = Vec::new();
        if wrote_auth {
            parts.push("dashboard_auth");
        }
        if wrote_dist {
            parts.push("web_dist");
        }
        let _ = app.emit(
            "hermes-install-log",
            format!("📦 已注入 Hermes Dashboard 兼容 stub: {}", parts.join(", ")),
        );
    } else {
        let _ = app.emit("hermes-install-log", "✓ Hermes Dashboard 兼容 stub 已存在，无需注入");
    }
}
