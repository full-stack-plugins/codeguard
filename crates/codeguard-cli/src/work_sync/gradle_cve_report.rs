//! Gradle漏洞准备观察的本地导入；不把未确认advisory投影为源码finding。
use super::{BlockerInput, ReportInput, safe_run_id};
use serde_json::Value;
use std::path::Path;
pub(super) fn parse(
    root: &Path,
    workspace: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    crate::gradle_cve_workbench::validate(root, report, true)?;
    let run = report["run_id"]
        .as_str()
        .filter(|r| safe_run_id(r) && r.starts_with("cve-gradle-"))
        .ok_or("gradle_cve_run_invalid")?;
    if report["workspace_id"] != workspace || path.file_stem().and_then(|p| p.to_str()) != Some(run)
    {
        return Err("gradle_cve_workspace_invalid");
    }
    let fingerprint = crate::gradle_cve_workbench::fingerprint(report)?;
    let paths = report["inputs"]
        .as_array()
        .ok_or("gradle_cve_inputs_invalid")?
        .iter()
        .map(|r| {
            r["path"]
                .as_str()
                .map(str::to_owned)
                .ok_or("gradle_cve_input_invalid")
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ReportInput {
        workspace_id: workspace.into(),
        run_id: run.into(),
        digest,
        findings: Vec::new(),
        historical_findings: 0,
        blockers: vec![BlockerInput {
            checker_id: "java.gradle.dependency_check".into(),
            id: format!("CG-B-{}", &fingerprint[..32]),
            fingerprint,
            reason: "gradle_cve_preparation_required".into(),
            diagnostic_reason: report["diagnostic_reason"].as_str().map(str::to_owned),
            build_root: ".".into(),
            scope: ".".into(),
            affected_paths: paths,
        }],
    })
}
