
#[tauri::command]
pub async fn hermes_sessions_summary_list(
    source: Option<String>,
    limit: Option<usize>,
    profile: Option<String>,
) -> Result<Value, String> {
    let lim = limit.unwrap_or(80).clamp(1, 500);
    let mut args: Vec<String> = Vec::new();
    if let Some(p) = profile.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        args.push("--profile".into());
        args.push(p.to_string());
    }
    args.extend(["sessions", "list", "--limit"].iter().map(|s| s.to_string()));
    args.push(lim.to_string());
    if let Some(s) = source.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        args.push("--source".into());
        args.push(s.to_string());
    }
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = match run_silent("hermes", &refs) {
        Ok(s) => s,
        Err(_) => return Ok(crate::jv!([])),
    };
    let sep = regex::Regex::new(r"\s{2,}").map_err(|e| e.to_string())?;
    let mut has_titles = false;
    let mut sessions: Vec<Value> = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "No sessions found." || trimmed.starts_with('─') {
            continue;
        }
        if trimmed.contains("Title") && trimmed.contains("Preview") && trimmed.contains("ID") {
            has_titles = true;
            continue;
        }
        if trimmed.contains("Preview") && trimmed.contains("Last Active") && trimmed.contains("ID") {
            has_titles = false;
            continue;
        }
        let cols: Vec<&str> = sep.split(trimmed).filter(|s| !s.trim().is_empty()).collect();
        if cols.len() < 3 {
            continue;
        }
        let id = cols.last().copied().unwrap_or("").trim();
        if id.is_empty() {
            continue;
        }
        let (title, preview, last_active, parsed_source) = if has_titles {
            let title = cols.first().copied().unwrap_or("").trim();
            let preview = cols.get(1).copied().unwrap_or("").trim();
            let last_active = cols.get(2).copied().unwrap_or("").trim();
            (
                if title == "—" { "" } else { title },
                preview,
                last_active,
                source.as_deref().unwrap_or(""),
            )
        } else {
            let preview = cols.first().copied().unwrap_or("").trim();
            let last_active = cols.get(1).copied().unwrap_or("").trim();
            let parsed_source = cols.get(2).copied().unwrap_or(source.as_deref().unwrap_or("")).trim();
            ("", preview, last_active, parsed_source)
        };
        sessions.push(crate::jv!({
            "id": id,
            "title": title,
            "source": parsed_source,
            "model": "",
            "created_at": "",
            "updated_at": "",
            "last_active_label": last_active,
            "preview": preview,
            "message_count": 0,
            "input_tokens": 0,
            "output_tokens": 0,
        }));
    }
    Ok(Value::Array(sessions))
}

#[tauri::command]
pub async fn hermes_usage_analytics(days: Option<u64>, profile: Option<String>) -> Result<Value, String> {
    let days = days.unwrap_or(30).clamp(1, 365);
    let cutoff = chrono::Utc::now().timestamp() - (days as i64 * 86_400);
    let sessions = hermes_sessions_list(None, None, profile).await?;
    let mut total_input: u64 = 0;
    let mut total_output: u64 = 0;
    let mut total_cache_read: u64 = 0;
    let mut total_cache_write: u64 = 0;
    let mut total_estimated_cost = 0.0_f64;
    let mut total_actual_cost = 0.0_f64;
    let mut total_sessions: u64 = 0;
    let mut daily: std::collections::BTreeMap<String, serde_json::Map<String, Value>> = std::collections::BTreeMap::new();
    let mut by_model: std::collections::BTreeMap<String, serde_json::Map<String, Value>> = std::collections::BTreeMap::new();
    if let Some(arr) = sessions.as_array() {
        for s in arr {
            let started = s.get("started_at").and_then(|v| v.as_i64()).unwrap_or(0);
            if started > 0 && started < cutoff {
                continue;
            }
            let input = s.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
            let output = s.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
            let cache_read = s.get("cache_read_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
            let cache_write = s.get("cache_write_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
            let estimated = s.get("estimated_cost_usd").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let actual = s.get("actual_cost_usd").and_then(|v| v.as_f64()).unwrap_or(0.0);
            total_input += input;
            total_output += output;
            total_cache_read += cache_read;
            total_cache_write += cache_write;
            total_estimated_cost += estimated;
            total_actual_cost += actual;
            total_sessions += 1;
            let day = if started > 0 {
                chrono::DateTime::from_timestamp(started, 0)
                    .map(|dt| dt.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "unknown".into())
            } else {
                "unknown".into()
            };
            let d = daily.entry(day.clone()).or_insert_with(|| {
                let mut m = serde_json::Map::new();
                m.insert("day".into(), Value::String(day));
                m.insert("input_tokens".into(), Value::from(0_u64));
                m.insert("output_tokens".into(), Value::from(0_u64));
                m.insert("cache_read_tokens".into(), Value::from(0_u64));
                m.insert("estimated_cost".into(), Value::from(0.0));
                m.insert("actual_cost".into(), Value::from(0.0));
                m.insert("sessions".into(), Value::from(0_u64));
                m
            });
            add_u64_field(d, "input_tokens", input);
            add_u64_field(d, "output_tokens", output);
            add_u64_field(d, "cache_read_tokens", cache_read);
            add_f64_field(d, "estimated_cost", estimated);
            add_f64_field(d, "actual_cost", actual);
            add_u64_field(d, "sessions", 1);
            let model = s.get("model").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if !model.is_empty() {
                let model_key = model.clone();
                let m = by_model.entry(model_key.clone()).or_insert_with(|| {
                    let mut row = serde_json::Map::new();
                    row.insert("model".into(), Value::String(model_key));
                    row.insert("input_tokens".into(), Value::from(0_u64));
                    row.insert("output_tokens".into(), Value::from(0_u64));
                    row.insert("estimated_cost".into(), Value::from(0.0));
                    row.insert("sessions".into(), Value::from(0_u64));
                    row
                });
                add_u64_field(m, "input_tokens", input);
                add_u64_field(m, "output_tokens", output);
                add_f64_field(m, "estimated_cost", estimated);
                add_u64_field(m, "sessions", 1);
            }
        }
    }
    let mut models: Vec<Value> = by_model.into_values().map(Value::Object).collect();
    models.sort_by(|a, b| {
        let at = a["input_tokens"].as_u64().unwrap_or(0) + a["output_tokens"].as_u64().unwrap_or(0);
        let bt = b["input_tokens"].as_u64().unwrap_or(0) + b["output_tokens"].as_u64().unwrap_or(0);
        bt.cmp(&at)
    });
    Ok(crate::jv!({
        "daily": daily.into_values().map(Value::Object).collect::<Vec<_>>(),
        "by_model": models,
        "totals": crate::jv!({
            "total_input": total_input,
            "total_output": total_output,
            "total_cache_read": total_cache_read,
            "total_cache_write": total_cache_write,
            "total_estimated_cost": total_estimated_cost,
            "total_actual_cost": total_actual_cost,
            "total_sessions": total_sessions,
            "total_api_calls": 0,
        }),
        "period_days": days,
        "skills": crate::jv!({
            "summary": crate::jv!({
                "total_skill_loads": 0,
                "total_skill_edits": 0,
                "total_skill_actions": 0,
                "distinct_skills_used": 0,
            }),
            "top_skills": Value::Array(Vec::new()),
        }),
    }))
}

fn add_u64_field(row: &mut serde_json::Map<String, Value>, key: &str, delta: u64) {
    let current = row.get(key).and_then(Value::as_u64).unwrap_or(0);
    row.insert(key.to_string(), Value::from(current.saturating_add(delta)));
}

fn add_f64_field(row: &mut serde_json::Map<String, Value>, key: &str, delta: f64) {
    let current = row.get(key).and_then(Value::as_f64).unwrap_or(0.0);
    row.insert(key.to_string(), Value::from(current + delta));
}

#[tauri::command]
pub async fn hermes_session_detail(session_id: String, profile: Option<String>) -> Result<Value, String> {
    let mut args: Vec<String> = Vec::new();
    if let Some(p) = profile.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        args.push("--profile".into());
        args.push(p.to_string());
    }
    args.extend(["sessions", "export", "-", "--session-id"].iter().map(|s| s.to_string()));
    args.push(session_id.clone());
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = run_silent("hermes", &refs).map_err(|e| format!("Failed to read sessions: {e}"))?;
    for line in output.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if let Ok(obj) = serde_json::from_str::<Value>(t) {
            let id = obj.get("session_id").or(obj.get("id")).and_then(|v| v.as_str()).unwrap_or("");
            if id == session_id {
                let messages = obj
                    .get("messages")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|m| {
                                crate::jv!({
                                    "role": m.get("role").and_then(|v| v.as_str()).unwrap_or(""),
                                    "content": m.get("content").map(|c| {
                                        if let Some(s) = c.as_str() { s.to_string() }
                                        else { c.to_string() }
                                    }).unwrap_or_default(),
                                    "timestamp": m.get("timestamp").or(m.get("created_at")).and_then(|v| v.as_str()).unwrap_or(""),
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                return Ok(crate::jv!({
                    "id": id,
                    "title": obj.get("title").or(obj.get("name")).and_then(|v| v.as_str()).unwrap_or(""),
                    "source": obj.get("source").and_then(|v| v.as_str()).unwrap_or(""),
                    "model": obj.get("model").and_then(|v| v.as_str()).unwrap_or(""),
                    "created_at": obj.get("created_at").and_then(|v| v.as_str()).unwrap_or(""),
                    "messages": messages,
                }));
            }
        }
    }
    Err("Session not found".into())
}

#[tauri::command]
pub async fn hermes_session_delete(session_id: String, profile: Option<String>) -> Result<String, String> {
    let mut args: Vec<String> = Vec::new();
    if let Some(p) = profile.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        args.push("--profile".into());
        args.push(p.to_string());
    }
    args.extend(["sessions", "delete"].iter().map(|s| s.to_string()));
    args.push(session_id);
    args.push("--yes".into());
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run_silent("hermes", &refs)?;
    Ok("ok".into())
}

#[tauri::command]
pub async fn hermes_session_rename(session_id: String, title: String, profile: Option<String>) -> Result<String, String> {
    let mut args: Vec<String> = Vec::new();
    if let Some(p) = profile.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        args.push("--profile".into());
        args.push(p.to_string());
    }
    args.extend(["sessions", "rename"].iter().map(|s| s.to_string()));
    args.push(session_id);
    args.push(title);
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run_silent("hermes", &refs)?;
    Ok("ok".into())
}

// ============================================================================
// Batch 3 §L: 文件管理器（基础 fs 命令）
//
// 限制：所有路径必须在 hermes_home() (~/.hermes) 子树内（防 path traversal）。
// 提供：list / read / write 三个基础命令，前端组合成文件管理器 UI。
// ============================================================================

#[cfg(test)]
mod hermes_session_runtime_config_tests {
    use super::{build_hermes_session_runtime_config_values, merge_hermes_session_runtime_config};

    #[test]
    fn session_runtime_values_have_safe_defaults() {
        let config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let values = build_hermes_session_runtime_config_values(&config);

        assert_eq!(values["sessionResetMode"], "both");
        assert_eq!(values["idleMinutes"], 1440);
        assert_eq!(values["atHour"], 4);
        assert_eq!(values["groupSessionsPerUser"], true);
        assert_eq!(values["threadSessionsPerUser"], false);
        assert_eq!(values["worktreeEnabled"], false);
    }

    #[test]
    fn session_runtime_values_read_worktree_flag() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
session_reset:
  mode: daily
  idle_minutes: 720
  at_hour: 3
group_sessions_per_user: false
thread_sessions_per_user: true
worktree: true
"#,
        )
        .unwrap();
        let values = build_hermes_session_runtime_config_values(&config);

        assert_eq!(values["sessionResetMode"], "daily");
        assert_eq!(values["idleMinutes"], 720);
        assert_eq!(values["atHour"], 3);
        assert_eq!(values["groupSessionsPerUser"], false);
        assert_eq!(values["threadSessionsPerUser"], true);
        assert_eq!(values["worktreeEnabled"], true);
    }

    #[test]
    fn merge_session_runtime_config_preserves_unrelated_yaml() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
  default: claude-sonnet-4-6
session_reset:
  mode: idle
  idle_minutes: 60
  custom_flag: keep-me
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_session_runtime_config(
            &mut config,
            &crate::jv!({
                "sessionResetMode": "both",
                "idleMinutes": "90",
                "atHour": "6",
                "groupSessionsPerUser": false,
                "threadSessionsPerUser": true,
                "worktreeEnabled": true,
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["session_reset"]["mode"].as_str(), Some("both"));
        assert_eq!(config["session_reset"]["idle_minutes"].as_i64(), Some(90));
        assert_eq!(config["session_reset"]["at_hour"].as_i64(), Some(6));
        assert_eq!(config["session_reset"]["custom_flag"].as_str(), Some("keep-me"));
        assert_eq!(config["group_sessions_per_user"].as_bool(), Some(false));
        assert_eq!(config["thread_sessions_per_user"].as_bool(), Some(true));
        assert_eq!(config["worktree"].as_bool(), Some(true));
    }

    #[test]
    fn merge_session_runtime_config_rejects_invalid_values() {
        let mut config = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        let err = merge_hermes_session_runtime_config(&mut config, &crate::jv!({ "sessionResetMode": "weekly" })).unwrap_err();
        assert!(err.contains("session_reset.mode"));

        let err = merge_hermes_session_runtime_config(&mut config, &crate::jv!({ "idleMinutes": 0 })).unwrap_err();
        assert!(err.contains("idle_minutes"));

        let err = merge_hermes_session_runtime_config(&mut config, &crate::jv!({ "atHour": 24 })).unwrap_err();
        assert!(err.contains("at_hour"));
    }
}

#[cfg(test)]
mod hermes_compression_config_tests {
    use super::{build_hermes_compression_config_values, merge_hermes_compression_config};

    #[test]
    fn compression_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_compression_config_values(&config);
        assert_eq!(values["enabled"], true);
        assert_eq!(values["threshold"], 0.5);
        assert_eq!(values["targetRatio"], 0.2);
        assert_eq!(values["protectLastN"], 20);
        assert_eq!(values["protectFirstN"], 3);
        assert_eq!(values["abortOnSummaryFailure"], false);
    }

    #[test]
    fn merge_compression_config_preserves_unrelated_yaml() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
compression:
  enabled: true
  threshold: 0.5
  custom_flag: keep-me
streaming:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_compression_config(
            &mut config,
            &crate::jv!({
                "enabled": false,
                "threshold": "0.7",
                "targetRatio": "0.4",
                "protectLastN": "28",
                "protectFirstN": "0",
                "abortOnSummaryFailure": true,
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["streaming"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["compression"]["enabled"].as_bool(), Some(false));
        assert_eq!(config["compression"]["threshold"].as_f64(), Some(0.7));
        assert_eq!(config["compression"]["target_ratio"].as_f64(), Some(0.4));
        assert_eq!(config["compression"]["protect_last_n"].as_i64(), Some(28));
        assert_eq!(config["compression"]["protect_first_n"].as_i64(), Some(0));
        assert_eq!(config["compression"]["abort_on_summary_failure"].as_bool(), Some(true));
        assert_eq!(config["compression"]["custom_flag"].as_str(), Some("keep-me"));
    }

    #[test]
    fn merge_compression_config_rejects_invalid_values() {
        let mut config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let err = merge_hermes_compression_config(&mut config, &crate::jv!({ "threshold": 0 })).unwrap_err();
        assert!(err.contains("compression.threshold"));
        let err = merge_hermes_compression_config(&mut config, &crate::jv!({ "targetRatio": 0.05 })).unwrap_err();
        assert!(err.contains("compression.target_ratio"));
        let err = merge_hermes_compression_config(&mut config, &crate::jv!({ "protectLastN": 0 })).unwrap_err();
        assert!(err.contains("compression.protect_last_n"));
        let err = merge_hermes_compression_config(&mut config, &crate::jv!({ "protectFirstN": -1 })).unwrap_err();
        assert!(err.contains("compression.protect_first_n"));
    }
}

#[cfg(test)]
mod hermes_prompt_caching_config_tests {
    use super::{build_hermes_prompt_caching_config_values, merge_hermes_prompt_caching_config};

    #[test]
    fn prompt_caching_values_have_upstream_defaults() {
        let config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let values = build_hermes_prompt_caching_config_values(&config);
        assert_eq!(values["promptCacheTtl"], "5m");
    }

    #[test]
    fn prompt_caching_values_normalize_existing_ttl() {
        let config: serde_yaml::Value = serde_yaml::from_str(
            r#"
prompt_caching:
  cache_ttl: "1H"
"#,
        )
        .unwrap();

        let values = build_hermes_prompt_caching_config_values(&config);
        assert_eq!(values["promptCacheTtl"], "1h");
    }

    #[test]
    fn merge_prompt_caching_config_preserves_unrelated_yaml() {
        let mut config: serde_yaml::Value = serde_yaml::from_str(
            r#"
model:
  provider: anthropic
prompt_caching:
  cache_ttl: 5m
  custom_flag: keep-prompt-cache
compression:
  enabled: true
"#,
        )
        .unwrap();

        merge_hermes_prompt_caching_config(
            &mut config,
            &crate::jv!({
                "promptCacheTtl": "1h",
            }),
        )
        .unwrap();

        assert_eq!(config["model"]["provider"].as_str(), Some("anthropic"));
        assert_eq!(config["compression"]["enabled"].as_bool(), Some(true));
        assert_eq!(config["prompt_caching"]["cache_ttl"].as_str(), Some("1h"));
        assert_eq!(config["prompt_caching"]["custom_flag"].as_str(), Some("keep-prompt-cache"));
    }

    #[test]
    fn merge_prompt_caching_config_rejects_invalid_ttl() {
        let mut config: serde_yaml::Value = serde_yaml::from_str("{}").unwrap();
        let err = merge_hermes_prompt_caching_config(&mut config, &crate::jv!({ "promptCacheTtl": "30m" })).unwrap_err();
        assert!(err.contains("prompt_caching.cache_ttl"));
    }
}

include!("session_analytics/provider_routing_tests.rs");