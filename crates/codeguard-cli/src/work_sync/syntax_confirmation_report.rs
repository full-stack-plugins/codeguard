//! 通用候选报告只导入稳定原生确认任务，不生成原生违规。
use super::{BlockerInput, ReportInput};
use serde_json::Value;
use std::path::Path;

/// 核验当前输入和报告文件名，返回既有工作台可消费的单张确认阻塞。
pub(super) fn parse(
    root: &Path,
    workspace: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    if !crate::syntax_confirmation::valid_report(root, workspace, report) {
        return Err("syntax_confirmation_report_invalid");
    }
    let run = report["run_id"].as_str().ok_or("report_run_id_invalid")?;
    if path.file_stem().and_then(|s| s.to_str()) != Some(run) {
        return Err("report_run_id_invalid");
    }
    let scope = report["scope"].as_str().ok_or("report_scope_invalid")?;
    Ok(ReportInput {
        workspace_id: workspace.into(),
        run_id: run.into(),
        digest,
        findings: Vec::new(),
        historical_findings: 0,
        blockers: vec![BlockerInput {
            checker_id: report["checker_id"].as_str().unwrap().into(),
            id: report["blocker_id"].as_str().unwrap().into(),
            fingerprint: report["fingerprint"].as_str().unwrap().into(),
            reason: report["reason_code"].as_str().unwrap().into(),
            diagnostic_reason: Some(
                if report["schema_version"] == "0.12.0" {
                    "rust_native_first_observation"
                } else if report["schema_version"] == "0.9.0" {
                    "ruby_native_first_observation"
                } else if report["schema_version"] == "0.8.0" {
                    "go_package_structure_candidate"
                } else if report["schema_version"] == "0.5.0" {
                    "swift_native_first_observation"
                } else if report["schema_version"] == "0.4.0" {
                    "kotlin_native_first_observation"
                } else if report["schema_version"] == "0.2.0" {
                    "erlang_native_first_observation"
                } else if report["observations"].as_array().is_some_and(|rows| {
                    rows.iter().any(|row| {
                        row["recovery_count"] == 0 && row["reason"] == "syntax_recovery_incomplete"
                    })
                }) {
                    "syntax_recovery_incomplete"
                } else if report["checker_id"] == "node.eslint.preparation" {
                    "eslint_syntax_confirmation_needed"
                } else {
                    "native_syntax_confirmation_needed"
                }
                .into(),
            ),
            build_root: ".".into(),
            scope: scope.into(),
            affected_paths: vec![scope.into()],
        }],
    })
}
