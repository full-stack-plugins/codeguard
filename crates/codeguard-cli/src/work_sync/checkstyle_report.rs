//! Checkstyle 局部观察的输入复核与脱敏导入，不授予检查或审批权威。
use super::{FindingInput, ReportInput, safe_relative_path, safe_run_id};
use crate::checkstyle_workbench::project_finding;
use codeguard_adapters::checkstyle_comment_rule_bindings;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

/// 核对当前输入与稳定诊断，返回既有工作台导入对象；不执行文件内指令。
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
            "authority",
            "coverage_proven",
            "delivery_decision",
            "checker_id",
            "source_path",
            "inputs",
            "findings",
        ],
    ) || !exact_keys(&report["inputs"], &["source", "config", "java", "jar"])
    {
        return Err("checkstyle_report_shape_invalid");
    }
    let run = report["run_id"]
        .as_str()
        .filter(|s| safe_run_id(s))
        .ok_or("report_run_id_invalid")?;
    if !matches!(report["schema_version"].as_str(), Some("0.1.0" | "0.2.0"))
        || report["workspace_binding"] != "bound"
        || report["workspace_id"] != workspace_id
        || report["authority"] != "local_unverified"
        || report["coverage_proven"] != false
        || report["delivery_decision"] != "not_evaluated"
        || report["checker_id"] != "java.checkstyle"
        || path.file_stem().and_then(|s| s.to_str()) != Some(run)
    {
        return Err("checkstyle_report_identity_invalid");
    }
    let relative = report["source_path"]
        .as_str()
        .filter(|s| safe_relative_path(s))
        .ok_or("finding_path_invalid")?;
    let mut frozen = BTreeMap::new();
    let mut paths = std::collections::BTreeSet::new();
    for (key, limit) in [
        ("source", 16 * 1024 * 1024),
        ("config", 1024 * 1024),
        ("java", 128 * 1024 * 1024),
        ("jar", 128 * 1024 * 1024),
    ] {
        let input = &report["inputs"][key];
        if !exact_keys(input, &["path", "sha256"]) {
            return Err("checkstyle_input_invalid");
        }
        let file = Path::new(input["path"].as_str().ok_or("checkstyle_input_invalid")?);
        if !file.is_absolute() || file.canonicalize().ok().as_deref() != Some(file) {
            return Err("checkstyle_input_path_changed");
        }
        if !paths.insert(file.to_owned()) {
            return Err("checkstyle_input_path_collision");
        }
        let bytes =
            read_bounded_regular_file(file, limit).map_err(|_| "checkstyle_input_unavailable")?;
        if input["sha256"] != format!("{:x}", Sha256::digest(&bytes)) {
            return Err("checkstyle_input_changed");
        }
        frozen.insert(key, bytes);
    }
    let actual = root
        .join(relative)
        .canonicalize()
        .map_err(|_| "source_unavailable")?;
    if !actual.starts_with(root)
        || report["inputs"]["source"]["path"] != actual.to_str().ok_or("source_path_invalid")?
    {
        return Err("checkstyle_source_scope_mismatch");
    }
    let bindings = checkstyle_comment_rule_bindings(&frozen["config"], "10.21.4")
        .ok_or("checkstyle_configuration_context_unresolved")?;
    if report["schema_version"] == "0.1.0"
        && bindings
            .values()
            .any(|b| codeguard_adapters::checkstyle_detailed_rule_class(&b.checker_class))
    {
        return Err("checkstyle_rule_protocol_invalid");
    }
    let mut occurrences = BTreeMap::new();
    let mut findings = Vec::new();
    for candidate in report["findings"]
        .as_array()
        .ok_or("checkstyle_findings_invalid")?
    {
        let native = &candidate["native"];
        if !exact_keys(
            candidate,
            &[
                "finding_id",
                "finding_fingerprint",
                "source_sha256",
                "path",
                "rule_id",
                "line",
                "native",
            ],
        ) || !exact_keys(
            native,
            &[
                "path",
                "line",
                "column",
                "rule_id",
                "severity",
                "checker_class",
                "rule_summary",
                "rule_reference",
                "repair_steps",
            ],
        ) {
            return Err("checkstyle_finding_shape_invalid");
        }
        let rule = native["rule_id"]
            .as_str()
            .ok_or("checkstyle_rule_invalid")?;
        let binding = bindings.get(rule).ok_or("checkstyle_rule_source_unbound")?;
        if native["checker_class"] != binding.checker_class
            || native["rule_summary"] != binding.summary
            || native["rule_reference"] != binding.rule_reference
            || native["repair_steps"] != serde_json::json!(binding.repair_steps)
            || native["path"] != report["inputs"]["source"]["path"]
            || !matches!(
                native["severity"].as_str(),
                Some("error" | "warning" | "info")
            )
        {
            return Err("checkstyle_native_binding_invalid");
        }
        let expected = project_finding(relative, &frozen["source"], native, &mut occurrences)
            .ok_or("finding_projection_incomplete")?;
        if candidate != &expected {
            return Err("checkstyle_finding_identity_invalid");
        }
        findings.push(FindingInput {
            checker_id: "java.checkstyle".into(),
            id: candidate["finding_id"]
                .as_str()
                .ok_or("finding_id_invalid")?
                .into(),
            fingerprint: candidate["finding_fingerprint"]
                .as_str()
                .ok_or("finding_fingerprint_invalid")?
                .into(),
            path: relative.into(),
            source_sha256: candidate["source_sha256"]
                .as_str()
                .ok_or("finding_source_invalid")?
                .into(),
            rule_id: rule.into(),
            line: native["line"].as_u64().ok_or("finding_line_invalid")?,
        });
    }
    Ok(ReportInput {
        workspace_id: workspace_id.into(),
        run_id: run.into(),
        digest,
        findings,
        blockers: Vec::new(),
        historical_findings: 0,
    })
}

fn exact_keys(value: &Value, keys: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
}

/// 导入复检容器；未完成观察只消费历史，不生成源码任务。
pub(super) fn parse_recheck(
    root: &Path,
    workspace_id: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    let run = report["run_id"]
        .as_str()
        .filter(|s| safe_run_id(s))
        .ok_or("report_run_id_invalid")?;
    if !crate::checkstyle_task_recheck::valid_shape(report)
        || report["workspace_id"] != workspace_id
        || path.file_stem().and_then(|s| s.to_str()) != Some(run)
    {
        return Err("checkstyle_recheck_shape_invalid");
    }
    if report["scan"].is_null() {
        return Ok(ReportInput {
            workspace_id: workspace_id.into(),
            run_id: run.into(),
            digest,
            findings: Vec::new(),
            blockers: Vec::new(),
            historical_findings: 0,
        });
    }
    parse(root, workspace_id, path, &report["scan"], digest)
}

/// 将受限环境观察导入原有准备任务槽位，源码变化只标历史，不造源码发现。
pub(super) fn parse_preparation(
    root: &Path,
    workspace: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    let run = report["run_id"]
        .as_str()
        .filter(|s| safe_run_id(s))
        .ok_or("report_run_id_invalid")?;
    if !crate::checkstyle_preparation::valid_report(report, workspace)
        || path.file_stem().and_then(|s| s.to_str()) != Some(run)
    {
        return Err("preparation_report_invalid");
    }
    let relative = report["affected_paths"][0]
        .as_str()
        .ok_or("preparation_scope_invalid")?;
    let source = root.join(relative);
    let current = source.canonicalize().is_ok_and(|p| p.starts_with(root))
        && read_bounded_regular_file(&source, 16 * 1024 * 1024)
            .ok()
            .is_some_and(|b| report["source_sha256"] == format!("{:x}", Sha256::digest(b)));
    let blockers = if current {
        vec![super::BlockerInput {
            checker_id: "java.checkstyle.preparation".into(),
            id: report["blocker_id"]
                .as_str()
                .ok_or("blocker_id_invalid")?
                .into(),
            fingerprint: report["fingerprint"]
                .as_str()
                .ok_or("blocker_fingerprint_invalid")?
                .into(),
            reason: "checkstyle_prerequisites_require_review".into(),
            diagnostic_reason: report["diagnostic_reason"].as_str().map(str::to_owned),
            build_root: ".".into(),
            scope: ".".into(),
            affected_paths: vec![relative.into()],
        }]
    } else {
        Vec::new()
    };
    Ok(ReportInput {
        workspace_id: workspace.into(),
        run_id: run.into(),
        digest,
        findings: Vec::new(),
        blockers,
        historical_findings: 0,
    })
}

/// 导入准备复检；正常原生报告继续产生源码任务，失败观察只消费并保留复检历史。
pub(super) fn parse_preparation_recheck(
    root: &Path,
    workspace: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    let run = report["run_id"]
        .as_str()
        .filter(|s| safe_run_id(s))
        .ok_or("report_run_id_invalid")?;
    if !crate::checkstyle_preparation_recheck::valid_shape(report)
        || report["workspace_id"] != workspace
        || path.file_stem().and_then(|s| s.to_str()) != Some(run)
    {
        return Err("preparation_recheck_shape_invalid");
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
    parse(root, workspace, path, &report["scan"], digest)
}
