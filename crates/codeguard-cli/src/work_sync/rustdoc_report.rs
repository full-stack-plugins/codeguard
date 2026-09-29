//! 文档局部报告导入；重建原生范围身份，未完成只生成准备阻塞。
use super::{
    BlockerInput, FindingInput, MAX_REPORT_BYTES, ReportInput, read_bounded_file, safe_reason,
    safe_relative_path, safe_run_id, valid_sha256,
};
use crate::rustdoc_repair_brief::rustdoc_repair_brief;
use codeguard_adapters::RustdocFinding;
use codeguard_adapters::rustdoc_finding_record;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;

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
            "operation",
            "language",
            "command_status",
            "exit_code",
            "delivery_decision",
            "authority",
            "checker_id",
            "scope",
            "rule_selection",
            "local_scan_complete",
            "coverage_proven",
            "reason",
            "tool_sha256",
            "manifest_sha256",
            "lock_sha256",
            "findings",
            "backlog_status",
            "execution_budget",
            "next_actions",
            "recheck_argv",
            "workspace_binding",
            "workspace_id",
            "run_id",
            "backlog_sync",
        ],
    ) || report["report_type"] != "rustdoc_local_observation"
        || !exact_keys(
            &report["execution_budget"],
            &["timeout_ms", "source", "enforcement"],
        )
        || !report["execution_budget"]["timeout_ms"]
            .as_u64()
            .is_some_and(|n| (1..=86_400_000).contains(&n))
        || !matches!(
            report["execution_budget"]["source"].as_str(),
            Some("cli" | "registered_environment" | "project_default" | "builtin_default")
        )
        || report["execution_budget"]["enforcement"] != "native_execution_only"
        || !report["next_actions"].as_array().is_some_and(|rows| {
            !rows.is_empty()
                && rows
                    .iter()
                    .all(|v| v.as_str().is_some_and(|s| !s.is_empty()))
        })
        || (!report["tool_sha256"].is_null()
            && !report["tool_sha256"].as_str().is_some_and(valid_sha256))
    {
        return Err("rustdoc_report_shape_invalid");
    }
    let run_id = report["run_id"]
        .as_str()
        .filter(|id| safe_run_id(id) && id.starts_with("rustdoc-"))
        .ok_or("report_run_id_invalid")?;
    if report["schema_version"] != "0.4.0"
        || report["operation"] != "comments"
        || report["language"] != "rust"
        || report["checker_id"] != "rust.cargo_rustdoc"
        || report["workspace_binding"] != "bound"
        || report["workspace_id"] != workspace_id
        || report["authority"] != "local_unverified"
        || report["coverage_proven"] != false
        || report["delivery_decision"] != "not_evaluated"
        || report["backlog_status"] != "queued"
        || !report["backlog_sync"].is_null()
        || path.file_stem().and_then(|value| value.to_str()) != Some(run_id)
        || report["scope"] != "selected_root_library_default_features"
        || report["rule_selection"] != "explicit_probe_not_approved_project_policy"
        || report["recheck_argv"]
            != json!([
                "cargo",
                "rustdoc",
                "--lib",
                "--locked",
                "--offline",
                "--message-format=json",
                "--",
                "--warn",
                "missing_docs",
                "--warn",
                "rustdoc::broken_intra_doc_links"
            ])
    {
        return Err("report_identity_invalid");
    }
    let complete = report["local_scan_complete"]
        .as_bool()
        .ok_or("report_status_invalid")?;
    let reason = report["reason"]
        .as_str()
        .filter(|r| safe_reason(r))
        .ok_or("report_status_invalid")?;
    let cancelled = reason == "request_cancelled";
    if complete != (reason == "native_observed_unverified")
        || report["command_status"] != if cancelled { "cancelled" } else { "incomplete" }
        || report["exit_code"] != if cancelled { 130 } else { 3 }
        || (complete && !report["tool_sha256"].as_str().is_some_and(valid_sha256))
    {
        return Err("report_status_invalid");
    }
    let mut inputs_current = true;
    for (file, field) in [
        ("Cargo.toml", "manifest_sha256"),
        ("Cargo.lock", "lock_sha256"),
    ] {
        let expected = report[field].as_str();
        if expected.is_some_and(|s| !valid_sha256(s)) || (complete && expected.is_none()) {
            return Err("report_input_identity_invalid");
        }
        inputs_current &= expected.is_some_and(|sha| {
            read_bounded_file(&root.join(file), MAX_REPORT_BYTES)
                .ok()
                .is_some_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == sha)
        });
    }
    let items = report["findings"]
        .as_array()
        .filter(|items| items.len() <= 100_000)
        .ok_or("findings_invalid")?;
    let mut findings = Vec::new();
    let mut historical = 0;
    let mut ids = BTreeSet::new();
    for item in items {
        if !exact_keys(
            item,
            &[
                "finding_id",
                "finding_fingerprint",
                "source_sha256",
                "rule_id",
                "level",
                "path",
                "line",
                "column",
                "classification",
                "byte_start",
                "byte_end",
                "native_range_sha256",
                "identity_status",
                "target_path",
                "repair_brief",
            ],
        ) {
            return Err("finding_shape_invalid");
        }
        let target = item["path"]
            .as_str()
            .filter(|s| safe_relative_path(s))
            .ok_or("report_path_invalid")?;
        let target_path = item["target_path"]
            .as_str()
            .filter(|s| safe_relative_path(s))
            .ok_or("report_path_invalid")?;
        let source_sha = item["source_sha256"]
            .as_str()
            .filter(|s| valid_sha256(s))
            .ok_or("source_identity_invalid")?;
        let rule = item["rule_id"]
            .as_str()
            .filter(|s| matches!(*s, "missing_docs" | "rustdoc::broken_intra_doc_links"))
            .ok_or("finding_rule_invalid")?;
        if item["classification"] != "observed_unverified"
            || !matches!(item["level"].as_str(), Some("warning" | "error"))
            || item["repair_brief"]
                != rustdoc_repair_brief(item, complete, reason, &report["recheck_argv"])
        {
            return Err("finding_shape_invalid");
        }
        if !complete {
            if !matches!(
                item["identity_status"].as_str(),
                Some("ambiguous" | "unique_candidate")
            ) {
                return Err("finding_identity_invalid");
            }
            continue;
        }
        let id = item["finding_id"].as_str().ok_or("finding_id_invalid")?;
        if !ids.insert(id.to_owned()) || item["identity_status"] != "unique_candidate" {
            return Err("finding_identity_invalid");
        }
        let Ok(source) = read_bounded_file(&root.join(target), MAX_REPORT_BYTES) else {
            historical += 1;
            continue;
        };
        if format!("{:x}", Sha256::digest(&source)) != source_sha || !inputs_current {
            historical += 1;
            continue;
        }
        // 不相信报告自行填写的指纹；用当前源字节和原生范围重新计算。
        let native = RustdocFinding {
            rule_id: rule.into(),
            path: target.into(),
            line: item["line"].as_u64().ok_or("finding_location_invalid")?,
            column: item["column"].as_u64().ok_or("finding_location_invalid")?,
            byte_start: item["byte_start"].as_u64().ok_or("finding_range_invalid")?,
            byte_end: item["byte_end"].as_u64().ok_or("finding_range_invalid")?,
            level: item["level"].as_str().expect("已核对级别").into(),
            package_id: String::new(),
            manifest_path: String::new(),
            target_source: String::new(),
            target_kinds: vec!["lib".into()],
        };
        let rebuilt = rustdoc_finding_record(&native, &source, target_path)?;
        for field in [
            "finding_id",
            "finding_fingerprint",
            "source_sha256",
            "native_range_sha256",
            "identity_status",
        ] {
            if item[field] != rebuilt[field] {
                return Err("finding_identity_mismatch");
            }
        }
        findings.push(FindingInput {
            checker_id: "rust.cargo_rustdoc".into(),
            id: id.into(),
            fingerprint: rebuilt["finding_fingerprint"]
                .as_str()
                .expect("重建指纹")
                .into(),
            path: target.into(),
            source_sha256: source_sha.into(),
            rule_id: rule.into(),
            line: native.line,
        });
    }
    let blockers = if !complete || !inputs_current || historical > 0 {
        let reason = if complete {
            "rustdoc_report_inputs_stale"
        } else {
            reason
        };
        let mut hash = Sha256::new();
        for part in [
            "codeguard-blocker-v1",
            "rust.cargo_rustdoc",
            reason,
            ".",
            ".",
        ] {
            hash.update((part.len() as u64).to_be_bytes());
            hash.update(part.as_bytes());
        }
        let fingerprint = format!("{:x}", hash.finalize());
        vec![BlockerInput {
            checker_id: "rust.cargo_rustdoc".into(),
            id: format!("CG-B-{}", &fingerprint[..32]),
            fingerprint,
            reason: reason.into(),
            diagnostic_reason: None,
            build_root: ".".into(),
            scope: ".".into(),
            affected_paths: vec!["Cargo.toml".into(), "Cargo.lock".into()],
        }]
    } else {
        Vec::new()
    };
    Ok(ReportInput {
        workspace_id: workspace_id.into(),
        run_id: run_id.into(),
        digest,
        findings,
        blockers,
        historical_findings: historical,
    })
}

fn exact_keys(value: &Value, expected: &[&str]) -> bool {
    value.as_object().is_some_and(|object| {
        object.len() == expected.len() && expected.iter().all(|key| object.contains_key(*key))
    })
}

pub(super) fn parse_recheck(
    root: &Path,
    workspace_id: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    if !crate::rustdoc_task_recheck::valid_shape(report) {
        return Err("rustdoc_verification_shape_invalid");
    }
    let mut normal = report["normal_scan"].clone();
    let normal_argv = normal["recheck_argv"].clone();
    if !report["forced_scan"].is_null() {
        let mut forced = report["forced_scan"].clone();
        let completed = forced["local_scan_complete"]
            .as_bool()
            .ok_or("rustdoc_forced_status_invalid")?;
        let reason = forced["reason"]
            .as_str()
            .ok_or("rustdoc_forced_status_invalid")?
            .to_owned();
        let forced_argv = forced["recheck_argv"].clone();
        for finding in forced["findings"]
            .as_array_mut()
            .ok_or("rustdoc_forced_findings_invalid")?
        {
            if finding["repair_brief"]
                != rustdoc_repair_brief(finding, completed, &reason, &forced_argv)
            {
                return Err("rustdoc_forced_brief_invalid");
            }
            finding["repair_brief"] =
                rustdoc_repair_brief(finding, completed, &reason, &normal_argv);
        }
        // 参数仍在封套中保留原始强制告警；这里只复用严格报告解析来核验诊断身份。
        forced["recheck_argv"] = normal_argv.clone();
        let forced_run = forced["run_id"]
            .as_str()
            .filter(|id| safe_run_id(id))
            .ok_or("rustdoc_forced_run_invalid")?;
        let forced_path = path.with_file_name(format!("{forced_run}.json"));
        parse(root, workspace_id, &forced_path, &forced, digest.clone())?;
    }
    if (report["input_stable"] != true
        || !crate::rustdoc_task_recheck::inputs_current(root, report))
        && normal["local_scan_complete"] == true
    {
        normal["local_scan_complete"] = json!(false);
        normal["reason"] = json!("rustdoc_task_inputs_changed");
        for finding in normal["findings"]
            .as_array_mut()
            .ok_or("findings_invalid")?
        {
            finding["repair_brief"] =
                rustdoc_repair_brief(finding, false, "rustdoc_task_inputs_changed", &normal_argv);
        }
    }
    parse(root, workspace_id, path, &normal, digest)
}
