//! Python CVE 局部报告导入；只建立按构建根稳定的待核验任务。

use super::{BlockerInput, ReportInput, safe_reason, safe_run_id, valid_sha256};
use crate::python_cve_command::{bounded_build_root, input_identity, lock_inputs};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;

/// 核对工作区、构建根及原生输入后，生成一张长期存在的覆盖任务。
pub(super) fn parse(
    root: &Path,
    workspace_id: &str,
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
            "build_root",
            "manifest_state",
            "manifest_sha256",
            "lock_inputs",
            "scan",
        ],
    ) || report["schema_version"] != "0.1.0"
        || report["report_type"] != "python_cve_workbench_observation"
        || report["workspace_binding"] != "bound"
        || report["workspace_id"] != workspace_id
        || report["checker_id"] != "python.pip_audit"
        || report["authority"] != "local_unverified"
        || report["coverage_proven"] != false
        || report["delivery_decision"] != "not_evaluated"
    {
        return Err("python_cve_workbench_identity_invalid");
    }
    let run_id = report["run_id"]
        .as_str()
        .filter(|id| safe_run_id(id) && id.starts_with("python-cve-"))
        .ok_or("report_run_id_invalid")?;
    if path.file_stem().and_then(|stem| stem.to_str()) != Some(run_id) {
        return Err("report_run_id_invalid");
    }
    let build_root = report["build_root"]
        .as_str()
        .ok_or("python_build_root_invalid")?;
    let project = bounded_build_root(root, build_root)?;
    let manifest = input_identity(&project.join("pyproject.toml"), 2 * 1024 * 1024);
    if report["manifest_state"] != manifest.0 || report["manifest_sha256"] != manifest.1 {
        return Err("python_cve_report_manifest_changed");
    }
    let locks = lock_inputs(&project)?;
    if report["lock_inputs"] != locks {
        return Err("python_cve_report_locks_changed");
    }
    let scan = &report["scan"];
    if !exact_keys(
        scan,
        &[
            "schema_version",
            "report_type",
            "operation",
            "language",
            "checker_id",
            "command_status",
            "reason",
            "exit_code",
            "delivery_decision",
            "authority",
            "coverage_proven",
            "advisory_coverage",
            "native_report_valid",
            "local_scan_complete",
            "tool_sha256",
            "manifest_sha256",
            "lock_sha256",
            "lock_file",
            "native_version",
            "native_exit_code",
            "dependency_count",
            "findings",
            "next_action",
            "execution_budget",
        ],
    ) || scan["schema_version"] != "0.1.0"
        || scan["report_type"] != "python_cve_local_observation"
        || scan["operation"] != "cve"
        || scan["language"] != "python"
        || scan["checker_id"] != "python.pip_audit"
        || scan["delivery_decision"] != "not_evaluated"
        || scan["authority"] != "local_unverified"
        || scan["coverage_proven"] != false
        || scan["advisory_coverage"] != "not_evaluated"
        || scan["local_scan_complete"] != false
        || !scan["native_report_valid"].is_boolean()
        || !scan["reason"].as_str().is_some_and(safe_reason)
        || !scan["findings"]
            .as_array()
            .is_some_and(|rows| rows.len() <= 100_000)
        || !scan["next_action"]
            .as_str()
            .is_some_and(|action| !action.is_empty())
        || !scan["execution_budget"]["timeout_ms"]
            .as_u64()
            .is_some_and(|ms| ms > 0)
        || scan["execution_budget"]["enforcement"] != "native_execution_only"
    {
        return Err("python_cve_scan_invalid");
    }
    for field in ["manifest_sha256", "lock_sha256", "tool_sha256"] {
        if !scan[field].is_null() && !scan[field].as_str().is_some_and(valid_sha256) {
            return Err("python_cve_scan_identity_invalid");
        }
    }
    let native_valid = scan["native_report_valid"] == true;
    if native_valid {
        let has_findings = scan["findings"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty());
        let exit_matches_report = match (
            scan["reason"].as_str(),
            scan["native_exit_code"].as_i64(),
            has_findings,
        ) {
            (Some("native_advisories_observed_unverified"), Some(1), true)
            | (Some("native_zero_advisories_unverified"), Some(0), false) => true,
            (Some("pip_audit_execution_incomplete"), Some(code), true) => code != 1,
            (Some("pip_audit_execution_incomplete"), None, true) => true,
            (Some("request_deadline_exceeded" | "pip_audit_output_limit"), None, true) => true,
            _ => false,
        };
        if scan["command_status"] != "native_observed"
            || scan["exit_code"] != 3
            || !matches!(
                scan["reason"].as_str(),
                Some(
                    "native_advisories_observed_unverified"
                        | "native_zero_advisories_unverified"
                        | "pip_audit_execution_incomplete"
                        | "request_deadline_exceeded"
                        | "pip_audit_output_limit"
                )
            )
            || scan["manifest_sha256"] != manifest.1
            || !scan["tool_sha256"].as_str().is_some_and(valid_sha256)
            || !scan["dependency_count"].is_u64()
            || !scan["native_version"].is_string()
            || !(scan["native_exit_code"].is_i64()
                || matches!(
                    scan["reason"].as_str(),
                    Some(
                        "pip_audit_execution_incomplete"
                            | "request_deadline_exceeded"
                            | "pip_audit_output_limit"
                    )
                ) && scan["native_exit_code"].is_null())
            || !locks.as_array().is_some_and(|rows| {
                rows.len() == 1
                    && rows[0]["name"] == scan["lock_file"]
                    && rows[0]["state"] == "present"
                    && rows[0]["sha256"] == scan["lock_sha256"]
            })
            || !exit_matches_report
        {
            return Err("python_cve_scan_status_invalid");
        }
    } else if scan["command_status"] == "native_observed"
        || !scan["findings"]
            .as_array()
            .expect("已核验发现数组")
            .is_empty()
        || !scan["dependency_count"].is_null()
        || scan["command_status"]
            != if scan["reason"] == "request_cancelled" {
                "cancelled"
            } else {
                "incomplete"
            }
        || scan["exit_code"]
            != if scan["reason"] == "request_cancelled" {
                130
            } else {
                3
            }
    {
        return Err("python_cve_scan_status_invalid");
    }
    for finding in scan["findings"].as_array().expect("已核验发现数组") {
        if !exact_keys(
            finding,
            &[
                "advisory_id",
                "package_name",
                "package_version",
                "aliases",
                "cve_aliases",
                "fix_versions",
            ],
        ) || !finding["advisory_id"]
            .as_str()
            .is_some_and(|id| !id.is_empty() && id.len() <= 128)
            || !finding["package_name"]
                .as_str()
                .is_some_and(|name| !name.is_empty() && name.len() <= 256)
            || !finding["package_version"]
                .as_str()
                .is_some_and(|version| !version.is_empty() && version.len() <= 128)
            || !finding["aliases"].is_array()
            || !finding["cve_aliases"].is_array()
            || !finding["fix_versions"].is_array()
        {
            return Err("python_cve_finding_invalid");
        }
    }
    let reason = "python_cve_coverage_unverified";
    let mut hash = Sha256::new();
    for part in [
        "codeguard-blocker-v1",
        "python.pip_audit",
        reason,
        build_root,
        build_root,
    ] {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    let fingerprint = format!("{:x}", hash.finalize());
    let prefix = if build_root == "." {
        String::new()
    } else {
        format!("{build_root}/")
    };
    let mut affected_paths = vec![format!("{prefix}pyproject.toml")];
    for lock in locks.as_array().expect("锁数组") {
        affected_paths.push(format!("{prefix}{}", lock["name"].as_str().expect("锁名")));
    }
    Ok(ReportInput {
        workspace_id: workspace_id.into(),
        run_id: run_id.into(),
        digest,
        findings: Vec::new(),
        blockers: vec![BlockerInput {
            checker_id: "python.pip_audit".into(),
            id: format!("CG-B-{}", &fingerprint[..32]),
            fingerprint,
            reason: reason.into(),
            diagnostic_reason: scan["reason"].as_str().map(str::to_owned),
            build_root: build_root.into(),
            scope: build_root.into(),
            affected_paths,
        }],
        historical_findings: 0,
    })
}

fn exact_keys(value: &Value, expected: &[&str]) -> bool {
    value.as_object().is_some_and(|object| {
        object.len() == expected.len() && expected.iter().all(|key| object.contains_key(*key))
    })
}
