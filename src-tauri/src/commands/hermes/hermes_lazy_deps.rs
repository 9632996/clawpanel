use serde_json::Value;
use std::path::PathBuf;

use super::hermes_runtime::{apply_hermes_runtime_env_tokio, hermes_enhanced_path, hermes_uv_tool_python, hermes_venv_dir};

// ---------------------------------------------------------------------------
// P1-3: lazy_deps 预处理命令 — 让用户启用渠道时不再「首启 Gateway 卡 30 秒后崩」
//
// Hermes 内核 tools/lazy_deps.py 维护了一个 allowlist：每个 feature（如
// `platform.telegram` / `tts.elevenlabs`）对应一组 PyPI 包。原本只有 Gateway
// 启动 platform 模块时才会调 ensure() 装包，导致首次启动卡住甚至超时崩。
//
// 这里把 lazy_deps 暴露给 ClawPanel UI：
//   - hermes_lazy_deps_features() — 列所有可装的 feature（小白选）
//   - hermes_lazy_deps_status(features) — 批量查每个 feature 是否已安装
//   - hermes_lazy_deps_ensure(feature) — 主动预装
// ---------------------------------------------------------------------------

/// 找到 Hermes venv 的 Python 解释器路径
///
/// 优先级：
/// 1. 环境变量 `HERMES_PYTHON` — 适配自定义 venv（brew / 容器 / 任何非默认布局）
/// 2. `~/.hermes-venv/{Scripts,bin}/python` — `uv pip install` 备选安装路径
/// 3. `<uv tool dir>/hermes-agent/{Scripts,bin}/python` — `uv tool install` 默认路径
///    （ClawPanel `install_hermes` 默认走此分支，所以这里的 fallback 必不可少；
///    早期实现只查路径 #2 导致「可选依赖管理」等页面对绝大多数用户都误报「未安装」）
fn hermes_venv_python() -> Option<PathBuf> {
    // 1. HERMES_PYTHON 环境变量优先
    if let Ok(custom) = std::env::var("HERMES_PYTHON") {
        let p = PathBuf::from(custom);
        if p.exists() {
            return Some(p);
        }
    }
    // 2. uv pip install 备选安装路径（便携包内 data/runtime/hermes/venv 或旧 ~/.hermes-venv）
    let venv_dir = hermes_venv_dir();
    #[cfg(target_os = "windows")]
    let py = venv_dir.join("Scripts").join("python.exe");
    #[cfg(not(target_os = "windows"))]
    let py = venv_dir.join("bin").join("python");
    if py.exists() {
        return Some(py);
    }
    // 3. uv tool 默认路径（ClawPanel 默认安装方式）
    hermes_uv_tool_python()
}

/// 统一跑 venv python -c "<script>" 拿 JSON 结果。失败给可读错误。
async fn run_venv_python_json(script: &str) -> Result<Value, String> {
    let py = hermes_venv_python().ok_or_else(|| {
        "Hermes Python 解释器未找到（已尝试 HERMES_PYTHON、~/.hermes-venv 与 uv tool 路径）。请先安装 Hermes。".to_string()
    })?;

    let mut cmd = tokio::process::Command::new(&py);
    cmd.arg("-c").arg(script);
    cmd.env("PYTHONIOENCODING", "utf-8");
    cmd.env("PATH", hermes_enhanced_path());
    apply_hermes_runtime_env_tokio(&mut cmd);

    let output = cmd.output().await.map_err(|e| format!("启动 Python 子进程失败: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stderr_trimmed = stderr.trim();
        return Err(if stderr_trimmed.is_empty() {
            format!("Python 进程退出码 {}，无 stderr 输出", output.status)
        } else {
            stderr_trimmed.to_string()
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    // 取最后一行 JSON（避免被 Python 模块的 print 干扰）
    let last_line = stdout.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
    serde_json::from_str(last_line).map_err(|e| format!("Python 输出解析失败: {e}\n原文: {stdout}"))
}

#[tauri::command]
pub async fn hermes_lazy_deps_features() -> Result<Value, String> {
    let script = r#"
import json
try:
    from tools.lazy_deps import LAZY_DEPS
    out = []
    for feat, specs in LAZY_DEPS.items():
        out.append({"feature": feat, "specs": list(specs)})
    print(json.dumps({"ok": True, "features": out}))
except Exception as e:
    print(json.dumps({"ok": False, "error": str(e)}))
"#;
    run_venv_python_json(script).await
}

#[tauri::command]
pub async fn hermes_lazy_deps_status(features: Vec<String>) -> Result<Value, String> {
    // 把 features 列表序列化成 Python 合法的列表字面量
    // serde_json 的输出（如 ["platform.telegram","platform.discord"]）正好是 Python 合法字面量
    let features_literal = serde_json::to_string(&features).map_err(|e| format!("features 序列化失败: {e}"))?;
    let script = format!(
        r#"
import json
try:
    from tools.lazy_deps import feature_missing, LAZY_DEPS
    targets = {features_literal}
    result = {{}}
    for f in targets:
        if f not in LAZY_DEPS:
            result[f] = {{"known": False, "satisfied": False, "missing": []}}
            continue
        miss = list(feature_missing(f))
        result[f] = {{"known": True, "satisfied": len(miss) == 0, "missing": miss}}
    print(json.dumps({{"ok": True, "status": result}}))
except Exception as e:
    print(json.dumps({{"ok": False, "error": str(e)}}))
"#
    );
    run_venv_python_json(&script).await
}

#[tauri::command]
pub async fn hermes_lazy_deps_ensure(feature: String) -> Result<Value, String> {
    // serde_json::to_string 把字符串包成 Python 合法的字符串字面量（已含引号 + escape）
    let feature_literal = serde_json::to_string(&feature).map_err(|e| format!("feature 名序列化失败: {e}"))?;
    let script = format!(
        r#"
import json, sys
try:
    from tools.lazy_deps import ensure, feature_missing, FeatureUnavailable
    feat = {feature_literal}
    before_missing = list(feature_missing(feat))
    if not before_missing:
        print(json.dumps({{"ok": True, "alreadySatisfied": True, "installed": []}}))
        sys.exit(0)
    try:
        ensure(feat, prompt=False)
        print(json.dumps({{"ok": True, "alreadySatisfied": False, "installed": before_missing}}))
    except FeatureUnavailable as fe:
        print(json.dumps({{"ok": False, "error": str(fe), "missing": list(getattr(fe, "missing", []))}}))
except Exception as e:
    print(json.dumps({{"ok": False, "error": str(e)}}))
"#
    );
    run_venv_python_json(&script).await
}
