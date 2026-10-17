//! Shell原规则生命周期证据校验；有效形状不提供宿主权威。
use serde_json::Value;
/// 核对完整原生观察，不将环境阻塞或坏报告当成零诊断。
pub(crate) fn completed(v: &Value) -> bool {
    matches!(
        v["status"].as_str(),
        Some("completed" | "diagnostics_observed")
    ) && v["report_valid"] == true
        && v["version"] == "0.11.0"
        && v["environment_codes"].as_array().is_some_and(Vec::is_empty)
        && v["diagnostics"]
            .as_array()
            .is_some_and(|a| (v["status"] == "completed") == a.is_empty())
}
/// 按原生规则精确查找；另一条规则不能作为原任务复发。
pub(crate) fn contains_rule(v: &Value, rule: &str) -> bool {
    v["diagnostics"]
        .as_array()
        .is_some_and(|a| a.iter().any(|d| d["rule_id"] == rule))
}
/// 核验封闭Shell原生观察；定位的源码范围由实际原生解析阶段验证。
pub(crate) fn native(v: &Value) -> bool {
    let keys = [
        "status",
        "reason",
        "version",
        "tool_sha256",
        "configuration_mode",
        "report_valid",
        "diagnostics",
        "environment_codes",
    ];
    if !v
        .as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
        || !v["report_valid"].is_boolean()
        || !v["reason"].as_str().is_some_and(|s| {
            !s.is_empty()
                && s.len() <= 128
                && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        })
        || !(v["tool_sha256"].is_null() || v["tool_sha256"].as_str().is_some_and(sha))
        || !(v["version"].is_null() || v["version"] == "0.11.0")
        || !matches!(
            v["configuration_mode"].as_str(),
            Some("not_applied" | "frozen_project_rc" | "builtin_without_global_rc")
        )
    {
        return false;
    }
    let Some(rows) = v["diagnostics"].as_array().filter(|a| a.len() <= 128) else {
        return false;
    };
    if !v["environment_codes"]
        .as_array()
        .is_some_and(|a| a.len() <= 128 && a.iter().all(|v| v.as_str().is_some_and(rule)))
    {
        return false;
    }
    if !rows.iter().all(|d| {
        d.as_object().is_some_and(|o| {
            o.len() == 6
                && [
                    "rule_id",
                    "severity",
                    "line",
                    "column",
                    "end_line",
                    "end_column",
                ]
                .iter()
                .all(|k| o.contains_key(*k))
        }) && d["rule_id"].as_str().is_some_and(rule)
            && matches!(
                d["severity"].as_str(),
                Some("error" | "warning" | "info" | "style")
            )
            && ["line", "column", "end_line", "end_column"]
                .iter()
                .all(|k| {
                    d[*k]
                        .as_u64()
                        .is_some_and(|n| n > 0 && n <= u32::MAX as u64)
                })
            && (d["end_line"].as_u64(), d["end_column"].as_u64())
                >= (d["line"].as_u64(), d["column"].as_u64())
    }) {
        return false;
    }
    if v["status"] == "incomplete" {
        return true;
    }
    completed(v)
        && v["tool_sha256"].as_str().is_some_and(sha)
        && v["configuration_mode"] != "not_applied"
        && v["reason"]
            == if rows.is_empty() {
                "shellcheck_no_diagnostics"
            } else {
                "shellcheck_diagnostics"
            }
}
/// 核对原规则、配置和方言，保留另一个规则的发现而不阻止原任务关闭。
pub(crate) fn binding(v: &Value) -> bool {
    if matches!(v["outcome"].as_str(), Some("code_fixed" | "still_present")) {
        let mode = if v["project_configuration"]["status"] == "configured" {
            "frozen_project_rc"
        } else {
            "builtin_without_global_rc"
        };
        if !["original_native", "current_native"]
            .iter()
            .all(|key| v[*key]["configuration_mode"] == mode)
        {
            return false;
        }
    }

    v["identity"]["checker_id"] == "shell.shellcheck"
        && v["grammar_sha256"].is_null()
        && v["native_rule_id"].as_str().is_some_and(rule)
        && matches!(
            v["dialect"].as_str(),
            Some("sh" | "bash" | "dash" | "ksh" | "busybox")
        )
        && ["original_native", "current_native"]
            .iter()
            .all(|k| native(&v[*k]))
        && v["project_configuration"].as_object().is_some_and(|o| {
            o.len() == 5
                && [
                    "status",
                    "source_path",
                    "sha256",
                    "reason",
                    "global_configuration",
                ]
                .iter()
                .all(|k| o.contains_key(*k))
        })
        && v["project_configuration"]["global_configuration"] == "not_loaded"
        && v["project_configuration"]["reason"].is_null()
        && ((v["project_configuration"]["status"] == "missing"
            && v["project_configuration"]["source_path"].is_null()
            && v["project_configuration"]["sha256"].is_null())
            || (v["project_configuration"]["status"] == "configured"
                && v["project_configuration"]["source_path"]
                    .as_str()
                    .is_some_and(|s| std::path::Path::new(s).is_absolute())
                && v["project_configuration"]["sha256"]
                    .as_str()
                    .is_some_and(sha)))
}
fn sha(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}
fn rule(s: &str) -> bool {
    s.len() == 6
        && s.starts_with("SC")
        && s[2..]
            .parse::<u32>()
            .is_ok_and(|n| (1000..=9999).contains(&n))
}
