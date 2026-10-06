//! Maven Javadoc报告的输入绑定与稳定问题投影；不授予可信关闭。
use super::{BlockerInput, FindingInput, ReportInput, safe_run_id};
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn parse(
    root: &Path,
    workspace: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    let keys = [
        "schema_version",
        "report_type",
        "workspace_binding",
        "workspace_id",
        "run_id",
        "checker_id",
        "authority",
        "coverage_proven",
        "delivery_decision",
        "inputs",
        "native",
        "findings",
        "blockers",
    ];
    if !report
        .as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
        || report["schema_version"] != "0.1.0"
        || report["report_type"] != "maven_javadoc_workbench_observation"
        || report["workspace_binding"] != "bound"
        || report["workspace_id"] != workspace
        || report["checker_id"] != "java.maven.javadoc"
        || report["authority"] != "local_unverified"
        || report["coverage_proven"] != false
        || report["delivery_decision"] != "not_evaluated"
    {
        return Err("maven_javadoc_report_identity_invalid");
    }
    let run = report["run_id"]
        .as_str()
        .filter(|r| safe_run_id(r) && r.starts_with("javadoc-maven-"))
        .ok_or("maven_javadoc_run_invalid")?;
    if path.file_stem().and_then(|s| s.to_str()) != Some(run) {
        return Err("maven_javadoc_run_invalid");
    }
    let (findings, blockers) =
        crate::maven_javadoc_workbench::project(root, &report["inputs"], &report["native"])?;
    if report["findings"] != json!(findings) || report["blockers"] != json!(blockers) {
        return Err("maven_javadoc_projection_invalid");
    }
    let findings = findings
        .iter()
        .map(|f| {
            Ok(FindingInput {
                checker_id: "java.maven.javadoc".into(),
                id: f["finding_id"]
                    .as_str()
                    .ok_or("maven_finding_invalid")?
                    .into(),
                fingerprint: f["finding_fingerprint"]
                    .as_str()
                    .ok_or("maven_finding_invalid")?
                    .into(),
                path: f["path"].as_str().ok_or("maven_finding_invalid")?.into(),
                source_sha256: f["source_sha256"]
                    .as_str()
                    .ok_or("maven_finding_invalid")?
                    .into(),
                rule_id: f["rule_id"].as_str().ok_or("maven_finding_invalid")?.into(),
                line: f["line"].as_u64().ok_or("maven_finding_invalid")?,
            })
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    let blockers = blockers
        .iter()
        .map(|b| {
            Ok(BlockerInput {
                checker_id: "java.maven.javadoc".into(),
                id: b["id"].as_str().ok_or("maven_blocker_invalid")?.into(),
                fingerprint: b["fingerprint"]
                    .as_str()
                    .ok_or("maven_blocker_invalid")?
                    .into(),
                reason: b["reason"].as_str().ok_or("maven_blocker_invalid")?.into(),
                diagnostic_reason: None,
                build_root: b["build_root"]
                    .as_str()
                    .ok_or("maven_blocker_invalid")?
                    .into(),
                scope: b["scope"].as_str().ok_or("maven_blocker_invalid")?.into(),
                affected_paths: b["affected_paths"]
                    .as_array()
                    .ok_or("maven_blocker_invalid")?
                    .iter()
                    .map(|p| p.as_str().map(str::to_owned).ok_or("maven_blocker_invalid"))
                    .collect::<Result<Vec<_>, _>>()?,
            })
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    Ok(ReportInput {
        workspace_id: workspace.into(),
        run_id: run.into(),
        digest,
        findings,
        blockers,
        historical_findings: 0,
    })
}

/// 导入原任务复检的当前内层扫描；无扫描时仅允许保存失败观察事件。
pub(super) fn parse_recheck(
    root: &Path,
    workspace: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    if !crate::maven_javadoc_task_recheck::valid_shape(report)
        || report["workspace_id"] != workspace
        || report["run_id"].as_str() != path.file_stem().and_then(|p| p.to_str())
    {
        return Err("maven_javadoc_recheck_identity_invalid");
    }
    if report["scan"].is_null() {
        return Ok(ReportInput {
            workspace_id: workspace.into(),
            run_id: report["run_id"]
                .as_str()
                .ok_or("maven_javadoc_run_invalid")?
                .into(),
            digest,
            findings: Vec::new(),
            blockers: Vec::new(),
            historical_findings: 0,
        });
    }
    if report["scan"]["run_id"] != report["run_id"]
        || report["scan"]["workspace_id"] != report["workspace_id"]
    {
        return Err("maven_javadoc_recheck_binding_invalid");
    }
    parse(root, workspace, path, &report["scan"], digest)
}
