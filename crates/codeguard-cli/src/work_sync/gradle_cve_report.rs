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

/// 导入原范围复检收据；只记录观察，不导入漏洞finding或授予关闭。
pub(super) fn parse_recheck(
    root: &Path,
    workspace: &str,
    path: &Path,
    r: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    if !crate::gradle_cve_task_recheck::valid_shape(r)
        || r["workspace_id"] != workspace
        || r["run_id"].as_str() != path.file_stem().and_then(|p| p.to_str())
    {
        return Err("gradle_cve_recheck_invalid");
    }
    let id = r["task_id"].as_str().ok_or("gradle_cve_task_invalid")?;
    let bytes = codeguard_runtime::read_bounded_regular_file(
        &root.join(format!(".codeguard/findings/{id}/finding.json")),
        128 * 1024,
    )
    .map_err(|_| "gradle_cve_task_unavailable")?;
    let f = codeguard_adapters::parse_unique_json(&bytes).map_err(|_| "gradle_cve_task_invalid")?;
    if f["id"] != id || f["workspace_id"] != workspace {
        return Err("gradle_cve_task_invalid");
    }
    let brief = serde_json::json!({"task_id":id,"checker_id":f["checker_id"],"kind":f["kind"],"scope":f["scope"],"evidence_ref":{"first_run_id":f["first_run_id"],"first_report_sha256":f["first_report_sha256"]}});
    crate::gradle_cve_task_recheck::validate_binding(root, &brief, r)?;
    if r["task_input_stable"] == true && !crate::gradle_cve_task_recheck::inputs_current(root, r) {
        return Err("gradle_cve_recheck_inputs_changed");
    }
    Ok(ReportInput {
        workspace_id: workspace.into(),
        run_id: r["run_id"].as_str().ok_or("gradle_cve_run_invalid")?.into(),
        digest,
        findings: Vec::new(),
        blockers: Vec::new(),
        historical_findings: 0,
    })
}
