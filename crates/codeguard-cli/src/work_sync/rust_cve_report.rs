//! Rust CVE 局部报告导入；只建立待核验的稳定完整性任务。

use super::{BlockerInput, ReportInput, safe_reason, safe_run_id, valid_sha256};
use codeguard_runtime::read_bounded_regular_file;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;

/// 核对报告包装与当前 Cargo 输入后生成同一构建根的完整性任务。
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
            "manifest_state",
            "manifest_sha256",
            "lock_state",
            "lock_sha256",
            "scan",
        ],
    ) || report["schema_version"] != "0.1.0"
        || report["report_type"] != "rust_cve_workbench_observation"
        || report["workspace_binding"] != "bound"
        || report["workspace_id"] != workspace_id
        || report["checker_id"] != "rust.cargo_audit"
        || report["authority"] != "local_unverified"
        || report["coverage_proven"] != false
        || report["delivery_decision"] != "not_evaluated"
    {
        return Err("rust_cve_workbench_identity_invalid");
    }
    let run_id = report["run_id"]
        .as_str()
        .filter(|id| safe_run_id(id) && id.starts_with("rust-cve-"))
        .ok_or("report_run_id_invalid")?;
    if path.file_stem().and_then(|stem| stem.to_str()) != Some(run_id) {
        return Err("report_run_id_invalid");
    }
    for (file, state_key, digest_key, limit) in [
        (
            "Cargo.toml",
            "manifest_state",
            "manifest_sha256",
            2 * 1024 * 1024,
        ),
        ("Cargo.lock", "lock_state", "lock_sha256", 8 * 1024 * 1024),
    ] {
        let actual = input_identity(&root.join(file), limit);
        if report[state_key] != actual.0 || report[digest_key] != actual.1 {
            return Err("rust_cve_report_inputs_changed");
        }
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
            "exit_code",
            "delivery_decision",
            "authority",
            "coverage_proven",
            "database_freshness",
            "local_scan_complete",
            "reason",
            "tool_sha256",
            "lock_sha256",
            "manifest_sha256",
            "dependency_count",
            "database_advisory_count",
            "database_last_commit",
            "database_last_updated",
            "native_warnings_present",
            "findings",
            "next_actions",
            "execution_budget",
        ],
    ) || scan["schema_version"] != "0.1.0"
        || scan["report_type"] != "rust_cve_local_observation"
        || scan["operation"] != "cve"
        || scan["language"] != "rust"
        || scan["checker_id"] != "rust.cargo_audit"
        || scan["authority"] != "local_unverified"
        || scan["delivery_decision"] != "not_evaluated"
        || scan["coverage_proven"] != false
        || scan["database_freshness"] != "unverified"
        || !scan["local_scan_complete"].is_boolean()
        || !scan["reason"].as_str().is_some_and(safe_reason)
        || !scan["findings"]
            .as_array()
            .is_some_and(|rows| rows.len() <= 100_000)
        || !scan["native_warnings_present"].is_boolean()
        || !scan["next_actions"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty())
        || !scan["execution_budget"]["timeout_ms"]
            .as_u64()
            .is_some_and(|ms| ms > 0)
        || scan["execution_budget"]["enforcement"] != "native_execution_only"
    {
        return Err("rust_cve_scan_invalid");
    }
    let reason = scan["reason"].as_str().expect("已核验原因");
    let complete = scan["local_scan_complete"] == true;
    let partial_findings = !complete
        && !scan["findings"]
            .as_array()
            .expect("已核验发现数组")
            .is_empty();
    if complete != (reason == "database_freshness_unverified")
        || scan["command_status"]
            != if reason == "request_cancelled" {
                "cancelled"
            } else {
                "incomplete"
            }
        || scan["exit_code"]
            != if reason == "request_cancelled" {
                130
            } else {
                3
            }
        || ((complete || partial_findings)
            && (scan["manifest_sha256"] != report["manifest_sha256"]
                || scan["lock_sha256"] != report["lock_sha256"]
                || !scan["tool_sha256"].as_str().is_some_and(valid_sha256)
                || !scan["dependency_count"].is_u64()
                || !scan["database_advisory_count"].is_u64()))
        || (partial_findings
            && !matches!(
                reason,
                "cargo_audit_execution_incomplete"
                    | "cargo_audit_output_limit"
                    | "request_deadline_exceeded"
            ))
    {
        return Err("rust_cve_scan_status_invalid");
    }
    for field in ["manifest_sha256", "lock_sha256", "tool_sha256"] {
        if !scan[field].is_null() && !scan[field].as_str().is_some_and(valid_sha256) {
            return Err("rust_cve_scan_identity_invalid");
        }
    }
    for finding in scan["findings"].as_array().expect("已核验数组") {
        if !exact_keys(
            finding,
            &[
                "advisory_id",
                "package_name",
                "package_version",
                "package_source_sha256",
                "package_checksum",
                "cve_aliases",
                "cvss_vector",
                "severity",
            ],
        ) || finding["severity"] != "unverified"
            || !finding["advisory_id"]
                .as_str()
                .is_some_and(|id| id.starts_with("RUSTSEC-"))
            || !finding["package_name"]
                .as_str()
                .is_some_and(|name| !name.is_empty())
            || !finding["package_version"]
                .as_str()
                .is_some_and(|version| !version.is_empty())
            || !finding["package_source_sha256"]
                .as_str()
                .is_some_and(valid_sha256)
            || !finding["cve_aliases"].is_array()
        {
            return Err("rust_cve_finding_invalid");
        }
    }
    let stable_reason = "rust_cve_coverage_unverified";
    let mut hash = Sha256::new();
    for part in [
        "codeguard-blocker-v1",
        "rust.cargo_audit",
        stable_reason,
        ".",
        ".",
    ] {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    let fingerprint = format!("{:x}", hash.finalize());
    Ok(ReportInput {
        workspace_id: workspace_id.into(),
        run_id: run_id.into(),
        digest,
        findings: Vec::new(),
        blockers: vec![BlockerInput {
            checker_id: "rust.cargo_audit".into(),
            id: format!("CG-B-{}", &fingerprint[..32]),
            fingerprint,
            reason: stable_reason.into(),
            diagnostic_reason: Some(reason.into()),
            build_root: ".".into(),
            scope: ".".into(),
            affected_paths: vec!["Cargo.toml".into(), "Cargo.lock".into()],
        }],
        historical_findings: 0,
    })
}

fn input_identity(path: &Path, limit: u64) -> (&'static str, Value) {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_file() => match read_bounded_regular_file(path, limit) {
            Ok(bytes) => (
                "present",
                Value::String(format!("{:x}", Sha256::digest(bytes))),
            ),
            Err(_) => ("unavailable", Value::Null),
        },
        Ok(_) => ("not_regular", Value::Null),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => ("missing", Value::Null),
        Err(_) => ("unavailable", Value::Null),
    }
}

fn exact_keys(value: &Value, expected: &[&str]) -> bool {
    value.as_object().is_some_and(|object| {
        object.len() == expected.len() && expected.iter().all(|key| object.contains_key(*key))
    })
}
