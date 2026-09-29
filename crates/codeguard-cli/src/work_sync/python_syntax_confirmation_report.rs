//! Python 语法候选报告只生成原生确认阻塞，不导入源码违规。

use super::{BlockerInput, ReportInput};
use serde_json::Value;
use std::path::Path;

/// 校验固定身份与当前源码后，投影一张稳定的待确认任务。
pub(super) fn parse(
    root: &Path,
    workspace: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    if !crate::python_syntax_confirmation::valid_report(root, workspace, report) {
        return Err("python_syntax_confirmation_report_invalid");
    }
    let run = report["run_id"].as_str().ok_or("report_run_id_invalid")?;
    if path.file_stem().and_then(|stem| stem.to_str()) != Some(run) {
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
            checker_id: "python.ruff".into(),
            id: report["blocker_id"]
                .as_str()
                .ok_or("blocker_id_invalid")?
                .into(),
            fingerprint: report["fingerprint"]
                .as_str()
                .ok_or("fingerprint_invalid")?
                .into(),
            reason: "python_syntax_confirmation_needed".into(),
            diagnostic_reason: Some("python_syntax_confirmation_needed".into()),
            build_root: ".".into(),
            scope: scope.into(),
            affected_paths: vec![scope.into()],
        }],
    })
}
