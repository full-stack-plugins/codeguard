//! ESLint 脱敏本地报告的严格输入复核，不接受自足质量批准。
use super::{FindingInput, ReportInput, safe_relative_path, safe_run_id};
use codeguard_runtime::read_bounded_regular_file;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
pub(super) fn parse(
    root: &Path,
    workspace_id: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    if !exact(
        report,
        &[
            "schema_version",
            "report_type",
            "workspace_binding",
            "workspace_id",
            "run_id",
            "authority",
            "coverage_proven",
            "delivery_decision",
            "checker_id",
            "source_path",
            "inputs",
            "cwd",
            "native_version",
            "local_coherent",
            "reason",
            "findings",
        ],
    ) || !exact(&report["inputs"], &["source", "config", "node", "eslint"])
    {
        return Err("eslint_report_shape_invalid");
    }
    let run = report["run_id"]
        .as_str()
        .filter(|run| safe_run_id(run))
        .ok_or("report_run_id_invalid")?;
    if report["schema_version"] != "0.1.0"
        || report["workspace_binding"] != "bound"
        || report["workspace_id"] != workspace_id
        || report["authority"] != "local_unverified"
        || report["coverage_proven"] != false
        || report["delivery_decision"] != "not_evaluated"
        || report["checker_id"] != "node.eslint"
        || path.file_stem().and_then(|value| value.to_str()) != Some(run)
    {
        return Err("eslint_report_identity_invalid");
    }
    let version = report["native_version"]
        .as_str()
        .ok_or("eslint_version_invalid")?;
    if !codeguard_adapters::eslint_report_version_matches(version, version) {
        return Err("eslint_version_invalid");
    }
    let cwd = Path::new(report["cwd"].as_str().ok_or("eslint_cwd_invalid")?);
    if !cwd.is_absolute() || !cwd.is_dir() || cwd.canonicalize().ok().as_deref() != Some(cwd) {
        return Err("eslint_cwd_invalid");
    }
    let relative = report["source_path"]
        .as_str()
        .filter(|value| safe_relative_path(value))
        .ok_or("finding_path_invalid")?;
    let mut bytes = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for (name, limit) in [
        ("source", 16 * 1024 * 1024),
        ("config", 1024 * 1024),
        ("node", 128 * 1024 * 1024),
        ("eslint", 16 * 1024 * 1024),
    ] {
        let input = &report["inputs"][name];
        if !exact(input, &["path", "sha256"]) {
            return Err("eslint_input_invalid");
        }
        let file = Path::new(input["path"].as_str().ok_or("eslint_input_invalid")?);
        if !file.is_absolute()
            || file.canonicalize().ok().as_deref() != Some(file)
            || (!seen.insert(file.to_owned())
                && !(name == "config" && input == &report["inputs"]["source"]))
        {
            return Err("eslint_input_path_invalid");
        }
        let value =
            read_bounded_regular_file(file, limit).map_err(|_| "eslint_input_unavailable")?;
        if input["sha256"] != format!("{:x}", Sha256::digest(&value)) {
            return Err("eslint_input_changed");
        }
        bytes.insert(name, value);
    }
    let absolute = report["inputs"]["source"]["path"]
        .as_str()
        .ok_or("source_path_invalid")?;
    if root.join(relative).canonicalize().ok().as_deref() != Some(Path::new(absolute)) {
        return Err("finding_source_outside_workspace");
    }
    let records = report["findings"]
        .as_array()
        .filter(|values| values.len() <= 100_000)
        .ok_or("eslint_findings_invalid")?;
    if !report["local_coherent"].is_boolean()
        || !(report["reason"].is_null()
            || report["reason"].as_str().is_some_and(|value| {
                !value.is_empty() && value.len() <= 128 && !value.chars().any(char::is_control)
            }))
        || (report["local_coherent"] == false && !records.is_empty())
        || (report["local_coherent"] == true && !report["reason"].is_null())
        || (report["local_coherent"] == false && report["reason"].is_null())
    {
        return Err("eslint_completion_invalid");
    }
    if !codeguard_adapters::validate_records(
        relative,
        absolute,
        &bytes["source"],
        records,
    ) {
        return Err("eslint_finding_identity_invalid");
    }
    let findings = records
        .iter()
        .map(|record| FindingInput {
            checker_id: "node.eslint".into(),
            id: record["finding_id"].as_str().unwrap().into(),
            fingerprint: record["finding_fingerprint"].as_str().unwrap().into(),
            path: relative.into(),
            source_sha256: record["source_sha256"].as_str().unwrap().into(),
            rule_id: record["rule_id"].as_str().unwrap().into(),
            line: record["line"].as_u64().unwrap(),
        })
        .collect();
    Ok(ReportInput {
        workspace_id: workspace_id.into(),
        run_id: run.into(),
        digest,
        findings,
        blockers: Vec::new(),
        historical_findings: 0,
    })
}
fn exact(value: &Value, keys: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|map| map.len() == keys.len() && keys.iter().all(|key| map.contains_key(*key)))
}

pub(super) fn parse_recheck(
    root: &Path,
    workspace: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    let run = report["run_id"]
        .as_str()
        .filter(|r| safe_run_id(r) && r.starts_with("eslint-task-"))
        .ok_or("report_run_id_invalid")?;
    if !crate::eslint_task_recheck::valid_shape(report)
        || report["workspace_id"] != workspace
        || path.file_stem().and_then(|s| s.to_str()) != Some(run)
    {
        return Err("eslint_recheck_report_invalid");
    }
    if report["scan"].is_null() {
        return Ok(ReportInput {
            workspace_id: workspace.into(),
            run_id: run.into(),
            digest,
            findings: Vec::new(),
            blockers: Vec::new(),
            historical_findings: 0,
        });
    }
    if report["scan"]["run_id"] != run || report["scan"]["source_path"] != report["target"]["path"]
    {
        return Err("eslint_recheck_scope_invalid");
    }
    parse(root, workspace, path, &report["scan"], digest)
}

pub(super) fn parse_preparation(
    root: &Path,
    workspace: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    if !crate::eslint_preparation::valid_report(report, workspace) {
        return Err("eslint_preparation_report_invalid");
    }
    let run = report["run_id"].as_str().ok_or("report_run_id_invalid")?;
    if path.file_stem().and_then(|s| s.to_str()) != Some(run) {
        return Err("report_run_id_invalid");
    }
    let relative = report["scope"]
        .as_str()
        .ok_or("preparation_scope_invalid")?;
    let source = root.join(relative);
    if source.canonicalize().ok().as_deref() != Some(source.as_path())
        || read_bounded_regular_file(&source, 16 * 1024 * 1024)
            .ok()
            .is_none_or(|bytes| report["source_sha256"] != format!("{:x}", Sha256::digest(bytes)))
    {
        return Err("eslint_preparation_source_changed");
    }
    Ok(ReportInput {
        workspace_id: workspace.into(),
        run_id: run.into(),
        digest,
        findings: Vec::new(),
        historical_findings: 0,
        blockers: vec![super::BlockerInput {
            checker_id: "node.eslint.preparation".into(),
            id: report["blocker_id"].as_str().unwrap().into(),
            fingerprint: report["fingerprint"].as_str().unwrap().into(),
            reason: "eslint_prerequisites_require_review".into(),
            diagnostic_reason: report["diagnostic_reason"].as_str().map(str::to_owned),
            build_root: ".".into(),
            scope: relative.into(),
            affected_paths: vec![relative.into()],
        }],
    })
}
