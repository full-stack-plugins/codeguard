//! ShellCheck固定局部观察严格导入；按文件/方言/原规则形成位置组，不合并不同规则。
use super::{BlockerInput, FindingInput, ReportInput, safe_reason, safe_run_id, valid_sha256};
use crate::shellcheck_config::ShellCheckConfig;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};
/// 只接受工作区内无跳转普通路径。
pub(crate) fn safe_path(path: &str) -> bool {
    super::safe_relative_path(path)
}
fn exact(v: &Value, keys: &[&str]) -> bool {
    v.as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn fingerprint(path: &str, dialect: &str, rule: &str) -> String {
    let mut h = Sha256::new();
    for s in [
        "codeguard-shell-rule-group-v1",
        "shell.shellcheck",
        path,
        dialect,
        rule,
    ] {
        h.update((s.len() as u64).to_be_bytes());
        h.update(s.as_bytes());
    }
    format!("{:x}", h.finalize())
}
/// 复核原始源码及原配置身份；过期观察不得授权直接修复。
pub(crate) fn current(root: &Path, r: &Value) -> bool {
    let Some(path) = r["path"].as_str().filter(|s| safe_path(s)) else {
        return false;
    };
    let file = root.join(path);
    file.canonicalize().is_ok_and(|p| p == file)
        && read_bounded_regular_file(&file, 1024 * 1024)
            .is_ok_and(|b| r["source_sha256"] == sha(&b))
        && ShellCheckConfig::capture(&file, r["requested_config"].as_str().map(Path::new))
            .is_ok_and(|c| c.report() == r["project_configuration"])
        && r["input_stable"] == true
}
fn complete(r: &Value) -> bool {
    matches!(
        r["native"]["status"].as_str(),
        Some("completed" | "diagnostics_observed")
    )
}
/// 返回本次确实可投影的规则组或稳定环境任务身份；不用于授权关闭。
pub(crate) fn task_ids(root: &Path, r: &Value) -> Vec<String> {
    let Some(path) = r["path"].as_str() else {
        return Vec::new();
    };
    let dialect = r["dialect"].as_str().unwrap_or("");
    if !current(root, r) || !complete(r) {
        return vec![format!(
            "CG-B-{}",
            &fingerprint(path, dialect, "environment")[..32]
        )];
    }
    r["native"]["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|d| d["rule_id"].as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|rule| format!("CG-{}", &fingerprint(path, dialect, rule)[..32]))
        .collect()
}
/// 核验封闭协议与当前位置，按既有事务写入脱敏事实和任务投影。
pub(super) fn parse(
    root: &Path,
    workspace_id: &str,
    path: &Path,
    r: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    if !exact(
        r,
        &[
            "schema_version",
            "report_type",
            "workspace_binding",
            "workspace_id",
            "run_id",
            "path",
            "source_sha256",
            "dialect",
            "authority",
            "coverage_proven",
            "delivery_decision",
            "input_stable",
            "native",
            "project_configuration",
            "requested_config",
        ],
    ) || r["schema_version"] != "0.1.0"
        || r["report_type"] != "shellcheck_workbench_observation"
        || r["workspace_binding"] != "bound"
        || r["workspace_id"] != workspace_id
        || r["authority"] != "local_unverified"
        || r["coverage_proven"] != false
        || r["delivery_decision"] != "not_evaluated"
        || !r["input_stable"].is_boolean()
    {
        return Err("shell_report_shape_invalid");
    }
    let run = r["run_id"]
        .as_str()
        .filter(|s| safe_run_id(s) && s.starts_with("shellcheck-"))
        .ok_or("report_run_id_invalid")?;
    if path.file_stem().and_then(|s| s.to_str()) != Some(run) {
        return Err("report_run_id_invalid");
    }
    let target = r["path"]
        .as_str()
        .filter(|s| safe_path(s))
        .ok_or("report_path_invalid")?;
    if root
        .join(target)
        .canonicalize()
        .is_ok_and(|p| p != root.join(target))
    {
        return Err("shell_target_alias_unverified");
    }
    let dialect = r["dialect"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 16 && s.bytes().all(|b| b.is_ascii_lowercase()))
        .ok_or("shell_dialect_invalid")?;
    let source_sha = r["source_sha256"]
        .as_str()
        .filter(|s| valid_sha256(s))
        .ok_or("source_identity_invalid")?;
    if !r["requested_config"].is_null()
        && !r["requested_config"]
            .as_str()
            .is_some_and(|s| Path::new(s).is_absolute())
    {
        return Err("shell_config_invalid");
    }
    let cfg = &r["project_configuration"];
    if !exact(
        cfg,
        &[
            "status",
            "source_path",
            "sha256",
            "reason",
            "global_configuration",
        ],
    ) || cfg["global_configuration"] != "not_loaded"
        || !matches!(
            cfg["status"].as_str(),
            Some("configured" | "missing" | "invalid" | "unknown")
        )
        || (!cfg["sha256"].is_null() && !cfg["sha256"].as_str().is_some_and(valid_sha256))
        || (!cfg["reason"].is_null() && !cfg["reason"].as_str().is_some_and(safe_reason))
        || (!cfg["source_path"].is_null()
            && !cfg["source_path"]
                .as_str()
                .is_some_and(|s| Path::new(s).is_absolute()))
    {
        return Err("shell_config_invalid");
    }
    let native = &r["native"];
    if !exact(
        native,
        &[
            "configuration_mode",
            "diagnostics",
            "environment_codes",
            "reason",
            "report_valid",
            "status",
            "tool_sha256",
            "version",
        ],
    ) || !native["reason"].as_str().is_some_and(safe_reason)
        || !native["report_valid"].is_boolean()
        || !matches!(
            native["status"].as_str(),
            Some("incomplete" | "completed" | "diagnostics_observed")
        )
        || !matches!(
            native["configuration_mode"].as_str(),
            Some("not_applied" | "frozen_project_rc" | "builtin_without_global_rc")
        )
        || (!native["tool_sha256"].is_null()
            && !native["tool_sha256"].as_str().is_some_and(valid_sha256))
        || (!native["version"].is_null() && native["version"] != "0.11.0")
    {
        return Err("shell_native_invalid");
    }
    let env = native["environment_codes"]
        .as_array()
        .filter(|a| {
            a.len() <= 128
                && a.iter().all(|v| {
                    matches!(
                        v.as_str(),
                        Some(
                            "SC1071"
                                | "SC1090"
                                | "SC1091"
                                | "SC1092"
                                | "SC1134"
                                | "SC1144"
                                | "SC1145"
                        )
                    )
                })
        })
        .ok_or("shell_environment_invalid")?;
    let rows = native["diagnostics"]
        .as_array()
        .filter(|a| a.len() <= 128)
        .ok_or("shell_diagnostics_invalid")?;
    if complete(r)
        && (native["report_valid"] != true
            || native["version"] != "0.11.0"
            || native["tool_sha256"].is_null()
            || !env.is_empty()
            || r["input_stable"] != true
            || (native["status"] == "completed") != rows.is_empty()
            || native["reason"]
                != if rows.is_empty() {
                    "shellcheck_no_diagnostics"
                } else {
                    "shellcheck_diagnostics"
                })
    {
        return Err("shell_native_status_invalid");
    }
    let mut groups = BTreeMap::<String, u64>::new();
    let mut json_rows = Vec::new();
    for d in rows {
        if !exact(
            d,
            &[
                "rule_id",
                "severity",
                "line",
                "column",
                "end_line",
                "end_column",
            ],
        ) || !matches!(
            d["severity"].as_str(),
            Some("error" | "warning" | "info" | "style")
        ) || ["line", "column", "end_line", "end_column"]
            .iter()
            .any(|k| {
                !d[*k]
                    .as_u64()
                    .is_some_and(|n| n > 0 && n <= u32::MAX as u64)
            })
        {
            return Err("shell_diagnostic_invalid");
        }
        let rule = d["rule_id"]
            .as_str()
            .filter(|s| {
                s.len() == 6 && s.starts_with("SC") && s[2..].bytes().all(|b| b.is_ascii_digit())
            })
            .ok_or("shell_rule_invalid")?;
        let code = rule[2..].parse::<u32>().map_err(|_| "shell_rule_invalid")?;
        if !(1000..=9999).contains(&code)
            || env.iter().any(|v| v == rule)
            || matches!(code, 1071 | 1090 | 1091 | 1092 | 1134 | 1144 | 1145)
        {
            return Err("shell_rule_invalid");
        }
        let start = (d["line"].as_u64(), d["column"].as_u64());
        let end = (d["end_line"].as_u64(), d["end_column"].as_u64());
        if end < start {
            return Err("shell_location_invalid");
        }
        groups
            .entry(rule.to_owned())
            .and_modify(|n| *n = (*n).min(d["line"].as_u64().unwrap()))
            .or_insert(d["line"].as_u64().unwrap());
        json_rows.push(json!({"file":"-","line":d["line"],"column":d["column"],"endLine":d["end_line"],"endColumn":d["end_column"],"code":code,"level":d["severity"],"message":"native_private","fix":null}));
    }
    if (!rows.is_empty() || native["report_valid"] == true)
        && (native["version"] != "0.11.0" || native["tool_sha256"].is_null())
    {
        return Err("shell_native_identity_invalid");
    }
    let stable = current(root, r);
    if stable {
        let bytes = read_bounded_regular_file(&root.join(target), 1024 * 1024)
            .map_err(|_| "shell_source_unavailable")?;
        let parsed = codeguard_adapters::parse_shellcheck_json1(
            &serde_json::to_vec(&json!({"comments":json_rows}))
                .map_err(|_| "shell_report_encoding")?,
            &bytes,
            if rows.is_empty() { 0 } else { 1 },
        );
        if !parsed.report_valid || parsed.diagnostics.len() != rows.len() {
            return Err("shell_location_invalid");
        }
    }
    let mut findings = Vec::new();
    let mut blockers = Vec::new();
    if stable && complete(r) {
        for (rule, line) in &groups {
            let fp = fingerprint(target, dialect, rule);
            findings.push(FindingInput {
                checker_id: "shell.shellcheck".into(),
                id: format!("CG-{}", &fp[..32]),
                fingerprint: fp,
                path: target.into(),
                source_sha256: source_sha.into(),
                rule_id: rule.clone(),
                line: *line,
            });
        }
    } else {
        let fp = fingerprint(target, dialect, "environment");
        blockers.push(BlockerInput {
            checker_id: "shell.shellcheck".into(),
            id: format!("CG-B-{}", &fp[..32]),
            fingerprint: fp,
            reason: "shell_check_incomplete".into(),
            diagnostic_reason: Some(
                if stable
                    || (!complete(r)
                        && read_bounded_regular_file(&root.join(target), 1024 * 1024)
                            .is_ok_and(|b| r["source_sha256"] == sha(&b)))
                {
                    native["reason"].as_str().unwrap()
                } else {
                    "shell_inputs_changed_after_scan"
                }
                .into(),
            ),
            build_root: ".".into(),
            scope: target.into(),
            affected_paths: vec![target.into()],
        });
    }
    Ok(ReportInput {
        workspace_id: workspace_id.into(),
        run_id: run.into(),
        digest,
        findings,
        blockers,
        historical_findings: if stable { 0 } else { groups.len() as u64 },
    })
}
