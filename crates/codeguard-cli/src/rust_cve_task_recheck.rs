//! Rust CVE 稳定任务的原工具复检解释；未核验数据库始终保持阻塞。

use codeguard_runtime::read_bounded_regular_file;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;

/// 核对复检观察仍属于 Rust CVE 局部扫描，不能从自写报告取得批准。
pub(crate) fn valid_shape(report: &Value) -> bool {
    let scan = &report["scan"];
    report.as_object().is_some_and(|object| {
        object.len() == 14
            && [
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
            ]
            .iter()
            .all(|key| object.contains_key(*key))
    }) && report["schema_version"] == "0.1.0"
        && report["report_type"] == "rust_cve_workbench_observation"
        && report["workspace_binding"] == "bound"
        && report["checker_id"] == "rust.cargo_audit"
        && report["authority"] == "local_unverified"
        && report["coverage_proven"] == false
        && report["delivery_decision"] == "not_evaluated"
        && scan["schema_version"] == "0.1.0"
        && scan["report_type"] == "rust_cve_local_observation"
        && scan["checker_id"] == "rust.cargo_audit"
        && scan["database_freshness"] == "unverified"
        && scan["delivery_decision"] == "not_evaluated"
        && scan["coverage_proven"] == false
}

/// 复检写入前重新读取清单与锁；路径或字节变化使旧结果失效。
pub(crate) fn inputs_current(root: &Path, report: &Value) -> bool {
    valid_shape(report)
        && [
            (
                "Cargo.toml",
                "manifest_state",
                "manifest_sha256",
                2 * 1024 * 1024,
            ),
            ("Cargo.lock", "lock_state", "lock_sha256", 8 * 1024 * 1024),
        ]
        .into_iter()
        .all(|(file, state, digest, limit)| {
            let path = root.join(file);
            let actual = match std::fs::symlink_metadata(&path) {
                Ok(meta) if meta.file_type().is_file() => {
                    match read_bounded_regular_file(&path, limit) {
                        Ok(bytes) => (
                            "present",
                            Value::String(format!("{:x}", Sha256::digest(bytes))),
                        ),
                        Err(_) => ("unavailable", Value::Null),
                    }
                }
                Ok(_) => ("not_regular", Value::Null),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    ("missing", Value::Null)
                }
                Err(_) => ("unavailable", Value::Null),
            };
            report[state] == actual.0 && report[digest] == actual.1
        })
}

/// 原生复检只更新证据；数据库和全项目范围未获信任前不关闭任务。
pub(crate) fn classify(brief: &Value, report: &Value) -> &'static str {
    if brief["kind"] != "blocker"
        || brief["checker_id"] != "rust.cargo_audit"
        || brief["reason_code"] != "rust_cve_coverage_unverified"
        || !valid_shape(report)
    {
        return "incomplete";
    }
    "still_blocked"
}
