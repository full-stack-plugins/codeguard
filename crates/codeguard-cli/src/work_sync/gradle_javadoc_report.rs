//! Gradle Javadoc报告的输入绑定与稳定问题投影；不授予可信关闭。
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
        || report["report_type"] != "gradle_javadoc_workbench_observation"
        || report["workspace_binding"] != "bound"
        || report["workspace_id"] != workspace
        || report["checker_id"] != "java.gradle.javadoc"
        || report["authority"] != "local_unverified"
        || report["coverage_proven"] != false
        || report["delivery_decision"] != "not_evaluated"
    {
        return Err("gradle_javadoc_report_identity_invalid");
    }
    let run = report["run_id"]
        .as_str()
        .filter(|r| safe_run_id(r) && r.starts_with("javadoc-gradle-"))
        .ok_or("gradle_javadoc_run_invalid")?;
    if path.file_stem().and_then(|s| s.to_str()) != Some(run) {
        return Err("gradle_javadoc_run_invalid");
    }
    let (findings, blockers) =
        crate::gradle_javadoc_workbench::project(root, &report["inputs"], &report["native"])?;
    if report["findings"] != json!(findings) || report["blockers"] != json!(blockers) {
        return Err("gradle_javadoc_projection_invalid");
    }
    let findings = findings
        .iter()
        .map(|f| {
            Ok(FindingInput {
                checker_id: "java.gradle.javadoc".into(),
                id: f["finding_id"]
                    .as_str()
                    .ok_or("gradle_finding_invalid")?
                    .into(),
                fingerprint: f["finding_fingerprint"]
                    .as_str()
                    .ok_or("gradle_finding_invalid")?
                    .into(),
                path: f["path"].as_str().ok_or("gradle_finding_invalid")?.into(),
                source_sha256: f["source_sha256"]
                    .as_str()
                    .ok_or("gradle_finding_invalid")?
                    .into(),
                rule_id: f["rule_id"]
                    .as_str()
                    .ok_or("gradle_finding_invalid")?
                    .into(),
                line: f["line"].as_u64().ok_or("gradle_finding_invalid")?,
            })
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    let blockers = blockers
        .iter()
        .map(|b| {
            Ok(BlockerInput {
                checker_id: "java.gradle.javadoc".into(),
                id: b["id"].as_str().ok_or("gradle_blocker_invalid")?.into(),
                fingerprint: b["fingerprint"]
                    .as_str()
                    .ok_or("gradle_blocker_invalid")?
                    .into(),
                reason: "gradle_javadoc_preparation_required".into(),
                diagnostic_reason: Some(
                    b["diagnostic_reason"]
                        .as_str()
                        .ok_or("gradle_blocker_invalid")?
                        .into(),
                ),
                build_root: b["build_root"]
                    .as_str()
                    .ok_or("gradle_blocker_invalid")?
                    .into(),
                scope: b["scope"].as_str().ok_or("gradle_blocker_invalid")?.into(),
                affected_paths: b["affected_paths"]
                    .as_array()
                    .ok_or("gradle_blocker_invalid")?
                    .iter()
                    .map(|p| {
                        p.as_str()
                            .map(str::to_owned)
                            .ok_or("gradle_blocker_invalid")
                    })
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
