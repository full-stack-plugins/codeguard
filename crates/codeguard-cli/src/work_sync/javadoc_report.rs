//! Javadoc 队列报告的严格归属与当前输入复核；不执行报告内指令。
use super::{
    BlockerInput, FindingInput, ReportInput, safe_relative_path, safe_reason, safe_run_id,
    valid_sha256,
};
use crate::javadoc_workbench::project_finding;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(super) fn parse(
    root: &Path,
    workspace: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    if !exact_keys(
        report,
        &[
            "schema_version",
            "report_type",
            "workspace_binding",
            "workspace_id",
            "run_id",
            "checker_id",
            "authority",
            "coverage_proven",
            "delivery_decision",
            "sources",
        ],
    ) || report["schema_version"] != "0.1.0"
        || report["report_type"] != "javadoc_workbench_observation"
        || report["workspace_binding"] != "bound"
        || report["workspace_id"] != workspace
        || report["checker_id"] != "java.jdk.javadoc"
        || report["authority"] != "local_unverified"
        || report["coverage_proven"] != false
        || report["delivery_decision"] != "not_evaluated"
    {
        return Err("javadoc_report_identity_invalid");
    }
    let run = report["run_id"]
        .as_str()
        .filter(|s| safe_run_id(s) && s.starts_with("javadoc-"))
        .ok_or("javadoc_run_invalid")?;
    if path.file_stem().and_then(|s| s.to_str()) != Some(run) {
        return Err("javadoc_run_invalid");
    }
    let rows = report["sources"]
        .as_array()
        .filter(|r| r.len() <= 100_000)
        .ok_or("javadoc_sources_invalid")?;
    let mut findings = Vec::new();
    let mut blockers = Vec::new();
    let mut seen = BTreeSet::new();
    for row in rows {
        if !exact_keys(
            row,
            &[
                "path",
                "source_sha256",
                "reason",
                "configuration_ref",
                "configuration_sha256",
                "native",
                "findings",
            ],
        ) {
            return Err("javadoc_source_shape_invalid");
        }
        let relative = row["path"]
            .as_str()
            .filter(|p| safe_relative_path(p) && p.ends_with(".java"))
            .ok_or("javadoc_path_invalid")?;
        if !seen.insert(relative) {
            return Err("javadoc_source_duplicate");
        }
        let actual = root
            .join(relative)
            .canonicalize()
            .map_err(|_| "javadoc_source_unavailable")?;
        if !actual.starts_with(root) {
            return Err("javadoc_source_outside_workspace");
        }
        let bytes = read_bounded_regular_file(&actual, 16 * 1024 * 1024)
            .map_err(|_| "javadoc_source_unavailable")?;
        let sha = format!("{:x}", Sha256::digest(&bytes));
        if row["source_sha256"] != sha {
            return Err("javadoc_source_changed");
        }
        if let Some(config) = row["configuration_ref"].as_str() {
            if !safe_relative_path(config) {
                return Err("javadoc_config_invalid");
            }
            let bytes = read_bounded_regular_file(&root.join(config), 4 * 1024 * 1024)
                .map_err(|_| "javadoc_config_unavailable")?;
            if row["configuration_sha256"] != format!("{:x}", Sha256::digest(bytes)) {
                return Err("javadoc_config_changed");
            }
        } else if !row["configuration_ref"].is_null() || !row["configuration_sha256"].is_null() {
            return Err("javadoc_config_invalid");
        }
        let native = &row["native"];
        let complete = matches!(
            native["local_status"].as_str(),
            Some("findings_observed_untrusted" | "clean_scope_unproven")
        );
        let projected = row["findings"]
            .as_array()
            .ok_or("javadoc_findings_invalid")?;
        if complete {
            if !exact_keys(
                native,
                &[
                    "schema_version",
                    "report_type",
                    "operation",
                    "language",
                    "category",
                    "command_status",
                    "exit_code",
                    "local_status",
                    "reason",
                    "path",
                    "source_sha256",
                    "javadoc_tool_sha256",
                    "java_runtime_sha256",
                    "jdk_release_sha256",
                    "checker_identity",
                    "coverage_proven",
                    "findings",
                    "authority",
                    "delivery_decision",
                    "next_actions",
                ],
            ) || native["command_status"] != "incomplete"
                || native["exit_code"] != 3
                || native["checker_identity"] != "unverified"
            {
                return Err("javadoc_native_shape_invalid");
            }
            if native["schema_version"] != "0.1.0"
                || native["report_type"] != "java_javadoc_local_feedback"
                || native["operation"] != "lint"
                || native["language"] != "java"
                || native["category"] != "comments"
                || native["authority"] != "local_unverified"
                || native["coverage_proven"] != false
                || native["delivery_decision"] != "not_evaluated"
                || native["source_sha256"] != sha
                || native["path"].as_str() != actual.to_str()
            {
                return Err("javadoc_native_identity_invalid");
            }
            for key in [
                "javadoc_tool_sha256",
                "java_runtime_sha256",
                "jdk_release_sha256",
            ] {
                if !native[key].as_str().is_some_and(valid_sha256) {
                    return Err("javadoc_tool_identity_invalid");
                }
            }
            let mut occurrences = BTreeMap::new();
            let candidates = native["findings"]
                .as_array()
                .ok_or("javadoc_findings_invalid")?;
            if candidates.len() != projected.len() || candidates.len() > 100_000 {
                return Err("javadoc_findings_invalid");
            }
            if candidates.is_empty() != (native["local_status"] == "clean_scope_unproven") {
                return Err("javadoc_native_status_invalid");
            }
            for (item, candidate) in projected.iter().zip(candidates) {
                if !exact_keys(
                    candidate,
                    &["path", "line", "column", "rule_id", "rule_summary"],
                ) || !candidate["column"].as_u64().is_some_and(|c| c > 0)
                {
                    return Err("javadoc_native_finding_invalid");
                }
                if candidate["path"] != native["path"] {
                    return Err("javadoc_native_path_invalid");
                }
                let expected = project_finding(relative, &bytes, candidate, &mut occurrences)
                    .ok_or("javadoc_finding_invalid")?;
                if item != &expected {
                    return Err("javadoc_finding_identity_invalid");
                }
                findings.push(FindingInput {
                    checker_id: "java.jdk.javadoc".into(),
                    id: item["finding_id"]
                        .as_str()
                        .ok_or("javadoc_finding_invalid")?
                        .into(),
                    fingerprint: item["finding_fingerprint"]
                        .as_str()
                        .ok_or("javadoc_finding_invalid")?
                        .into(),
                    path: relative.into(),
                    source_sha256: sha.clone(),
                    rule_id: item["rule_id"]
                        .as_str()
                        .ok_or("javadoc_finding_invalid")?
                        .into(),
                    line: item["line"].as_u64().ok_or("javadoc_finding_invalid")?,
                });
            }
        } else {
            if !projected.is_empty() {
                return Err("javadoc_incomplete_has_findings");
            }
            let reason = if native.is_null() {
                row["reason"].as_str()
            } else {
                native["reason"].as_str()
            }
            .filter(|r| safe_reason(r))
            .ok_or("javadoc_reason_invalid")?;
            let fingerprint = format!(
                "{:x}",
                Sha256::digest(
                    serde_json::to_vec(&json!(["codeguard-javadoc-blocker-v1", relative, reason]))
                        .map_err(|_| "javadoc_encoding_failed")?
                )
            );
            blockers.push(BlockerInput {
                checker_id: "java.jdk.javadoc".into(),
                id: format!("CG-B-{}", &fingerprint[..32]),
                fingerprint,
                reason: reason.into(),
                diagnostic_reason: None,
                build_root: ".".into(),
                scope: relative.into(),
                affected_paths: vec![relative.into()],
            });
        }
    }
    Ok(ReportInput {
        workspace_id: workspace.into(),
        run_id: run.into(),
        digest,
        findings,
        blockers,
        historical_findings: 0,
    })
}
fn exact_keys(value: &Value, keys: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
}

/// 导入复检容器中的当前观察，失败复检只保留原任务事件，不造源码事实。
pub(super) fn parse_recheck(
    root: &Path,
    workspace: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    if !crate::javadoc_task_recheck::valid_shape(report)
        || report["workspace_id"] != workspace
        || report["run_id"].as_str() != path.file_stem().and_then(|s| s.to_str())
    {
        return Err("javadoc_recheck_identity_invalid");
    }
    if report["scan"].is_null() {
        return Ok(ReportInput {
            workspace_id: workspace.into(),
            run_id: report["run_id"]
                .as_str()
                .ok_or("javadoc_run_invalid")?
                .into(),
            digest,
            findings: Vec::new(),
            blockers: Vec::new(),
            historical_findings: 0,
        });
    }
    if report["task_input_stable"] == true
        && !crate::javadoc_task_recheck::inputs_current(root, report)
    {
        return Err("javadoc_recheck_input_changed");
    }
    parse(root, workspace, path, &report["scan"], digest)
}
