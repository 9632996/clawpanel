use serde_json::Value;
use std::path::{Component, Path, PathBuf};

const WORKSPACE_TEXT_EXTENSIONS: &[&str] = &[
    "md",
    "markdown",
    "mdx",
    "txt",
    "json",
    "jsonc",
    "yaml",
    "yml",
    "toml",
    "ini",
    "cfg",
    "conf",
    "log",
    "csv",
    "env",
    "gitignore",
    "gitattributes",
    "editorconfig",
    "js",
    "mjs",
    "cjs",
    "ts",
    "tsx",
    "jsx",
    "html",
    "htm",
    "css",
    "scss",
    "less",
    "rs",
    "py",
    "sh",
    "bash",
    "zsh",
    "fish",
    "ps1",
    "bat",
    "cmd",
    "sql",
    "xml",
    "java",
    "kt",
    "go",
    "rb",
    "php",
    "c",
    "cc",
    "cpp",
    "h",
    "hpp",
    "vue",
    "svelte",
    "lock",
    "sample",
];

const WORKSPACE_TEXT_BASENAMES: &[&str] = &[
    "dockerfile",
    "makefile",
    "readme",
    "license",
    ".env",
    ".env.local",
    ".env.example",
    ".gitignore",
    ".gitattributes",
    ".editorconfig",
    ".npmrc",
];

const WORKSPACE_PREVIEW_EXTENSIONS: &[&str] = &["md", "markdown", "mdx"];

pub(super) fn resolve_agent_workspace(id: &str, config: &Value) -> String {
    config
        .get("agents")
        .and_then(|a| a.get("list"))
        .and_then(|l| l.as_array())
        .and_then(|list| {
            list.iter()
                .find(|a| a.get("id").and_then(|v| v.as_str()) == Some(id))
                .and_then(|a| a.get("workspace"))
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| {
            if id == "main" {
                super::openclaw_dir().join("workspace").to_string_lossy().to_string()
            } else {
                super::openclaw_dir()
                    .join("agents")
                    .join(id)
                    .join("workspace")
                    .to_string_lossy()
                    .to_string()
            }
        })
}

pub fn expand_user_path_pub(raw: &str) -> PathBuf {
    expand_user_path(raw)
}

pub(super) fn resolve_agent_workspace_path(id: &str, config: &Value) -> PathBuf {
    expand_user_path(&resolve_agent_workspace(id, config))
}

pub(super) fn resolve_workspace_target_path(root: &Path, relative_path: Option<&str>) -> Result<PathBuf, String> {
    let normalized = normalize_workspace_relative_path(relative_path.unwrap_or_default())?;
    Ok(root.join(normalized))
}

pub(super) fn to_workspace_relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .filter_map(|component| match component {
            Component::Normal(seg) => Some(seg.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

pub(super) fn is_workspace_text_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
        if WORKSPACE_TEXT_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()) {
            return true;
        }
    }

    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| WORKSPACE_TEXT_BASENAMES.contains(&name.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

pub(super) fn is_workspace_previewable_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| WORKSPACE_PREVIEW_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

pub(super) fn looks_binary_bytes(bytes: &[u8]) -> bool {
    bytes.iter().take(512).any(|b| *b == 0)
}

fn expand_user_path(raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    let path = if let Some(rest) = trimmed.strip_prefix("~/").or_else(|| trimmed.strip_prefix("~\\")) {
        dirs::home_dir().unwrap_or_default().join(rest)
    } else {
        PathBuf::from(trimmed)
    };

    if path.is_absolute() {
        path
    } else {
        std::env::current_dir().map(|cwd| cwd.join(&path)).unwrap_or(path)
    }
}

pub(super) fn normalize_workspace_relative_path(raw: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(PathBuf::new());
    }

    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        return Err("不允许使用绝对路径".to_string());
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(seg) => normalized.push(seg),
            Component::CurDir => {}
            Component::ParentDir => return Err("不允许访问工作区外部路径".to_string()),
            Component::RootDir | Component::Prefix(_) => return Err("不允许使用绝对路径".to_string()),
        }
    }
    Ok(normalized)
}
