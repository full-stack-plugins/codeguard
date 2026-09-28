//! npm局部CVE报告只生成检查完整性任务，不能确认漏洞或关闭任务。
use super::{BlockerInput, ReportInput, safe_reason, safe_relative_path, safe_run_id};
use codeguard_runtime::read_bounded_regular_file;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;
pub(super) fn parse(
    root: &Path,
    workspace_id: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    if report["schema_version"] == "0.3.0" {
        return super::npm_preparation_report::parse(root, workspace_id, path, report, digest);
    }
    validate_shape(workspace_id, path, report)?;
    let run = report["run_id"].as_str().expect("结构已经核验");
    let build = report["build_root"].as_str().expect("结构已经核验");
    let reason = report["diagnostic_reason"].as_str().expect("结构已经核验");
    let advisories = report["advisories"].as_array().expect("结构已经核验");
    let missing_lock = report["schema_version"] == "0.2.0";
    let mut affected = Vec::new();
    for (file, key, limit) in [
        ("package.json", "manifest_sha256", 256 * 1024),
        ("package-lock.json", "lock_sha256", 8 * 1024 * 1024),
    ] {
        let relative = if build == "." {
            file.into()
        } else {
            format!("{build}/{file}")
        };
        let input = root.join(&relative);
        if file == "package-lock.json" && missing_lock {
            if !matches!(std::fs::symlink_metadata(&input), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
            {
                return Err("npm_report_input_changed");
            }
            affected.push(relative);
            continue;
        }
        if input.canonicalize().ok().as_ref() != Some(&input) {
            return Err("npm_report_input_invalid");
        }
        let bytes =
            read_bounded_regular_file(&input, limit).map_err(|_| "npm_report_input_unavailable")?;
        if report[key] != format!("{:x}", Sha256::digest(&bytes)) {
            return Err("npm_report_input_changed");
        }
        if file == "package.json"
            && matches!(reason, "npm_manifest_invalid" | "npm_scripts_invalid")
            && codeguard_adapters::inspect_npm_audit_config(&bytes, build, file).reason != reason
        {
            return Err("npm_report_preparation_invalid");
        }
        affected.push(relative);
    }
    if !missing_lock {
        let lock_path = if build == "." {
            root.join("package-lock.json")
        } else {
            root.join(build).join("package-lock.json")
        };
        let lock_bytes = read_bounded_regular_file(&lock_path, 8 * 1024 * 1024)
            .map_err(|_| "npm_report_input_unavailable")?;
        if report["lock_sha256"] != format!("{:x}", Sha256::digest(&lock_bytes)) {
            return Err("npm_report_input_changed");
        }
        let nodes = if advisories.is_empty() {
            Vec::new()
        } else {
            codeguard_adapters::NpmLockedNode::parse(&lock_bytes)
                .map_err(|_| "npm_report_lock_invalid")?
        };
        for advisory in advisories {
            for reference in advisory["nodes"].as_array().expect("结构已经核验") {
                if !nodes.iter().any(|node| {
                    reference["node_ref"]
                        == format!("{:x}", Sha256::digest(node.location.as_bytes()))
                        && reference["resolved_version"] == node.resolved_version
                        && advisory["component_ref"]
                            == format!("{:x}", Sha256::digest(node.native_name.as_bytes()))
                }) {
                    return Err("npm_report_node_identity_invalid");
                }
            }
        }
    }
    Ok(make_report(
        workspace_id,
        run,
        digest,
        build,
        reason,
        affected,
    ))
}

/// 同一npm完整性义务跨输入修复与原生观察共用稳定身份。
/// 参数为工作区、运行、摘要、构建根、原因和路径；返回无源码违规的稳定阻塞记录。
pub(super) fn make_report(
    workspace_id: &str,
    run: &str,
    digest: String,
    build: &str,
    reason: &str,
    affected: Vec<String>,
) -> ReportInput {
    // 同一根的CVE完整性义务保持稳定；具体执行原因作为诊断历史更新。
    let stable_reason = "npm_audit_coverage_unverified";
    let mut hash = Sha256::new();
    for part in [
        "codeguard-blocker-v1",
        "node.npm.audit",
        stable_reason,
        build,
        build,
    ] {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    let fingerprint = format!("{:x}", hash.finalize());
    ReportInput {
        workspace_id: workspace_id.into(),
        run_id: run.into(),
        digest,
        findings: Vec::new(),
        blockers: vec![BlockerInput {
            checker_id: "node.npm.audit".into(),
            id: format!("CG-B-{}", &fingerprint[..32]),
            fingerprint,
            reason: stable_reason.into(),
            diagnostic_reason: Some(reason.into()),
            build_root: build.into(),
            scope: build.into(),
            affected_paths: affected,
        }],
        historical_findings: 0,
    }
}

fn validate_shape(workspace_id: &str, path: &Path, report: &Value) -> Result<(), &'static str> {
    let mut fields = vec![
        "schema_version",
        "report_type",
        "workspace_binding",
        "workspace_id",
        "run_id",
        "authority",
        "coverage_proven",
        "delivery_decision",
        "checker_id",
        "build_root",
        "manifest_sha256",
        "lock_sha256",
        "diagnostic_reason",
        "component_count",
        "advisories",
    ];
    let missing_lock = report["schema_version"] == "0.2.0";
    if missing_lock {
        fields.push("lock_state");
    }
    if !report
        .as_object()
        .is_some_and(|m| m.len() == fields.len() && fields.iter().all(|f| m.contains_key(*f)))
    {
        return Err("npm_report_shape_invalid");
    }
    let run = report["run_id"]
        .as_str()
        .filter(|r| safe_run_id(r))
        .ok_or("report_run_id_invalid")?;
    if !(report["schema_version"] == "0.1.0" || missing_lock)
        || report["report_type"] != "npm_cve_workbench_observation"
        || report["workspace_binding"] != "bound"
        || report["workspace_id"] != workspace_id
        || report["authority"] != "local_unverified"
        || report["coverage_proven"] != false
        || report["delivery_decision"] != "not_evaluated"
        || report["checker_id"] != "node.npm.audit"
        || path.file_stem().and_then(|p| p.to_str()) != Some(run)
    {
        return Err("npm_report_identity_invalid");
    }
    let _build = report["build_root"]
        .as_str()
        .filter(|r| *r == "." || safe_relative_path(r))
        .ok_or("npm_report_root_invalid")?;
    let _reason = report["diagnostic_reason"]
        .as_str()
        .filter(|r| safe_reason(r))
        .ok_or("npm_report_reason_invalid")?;
    if !report["component_count"]
        .as_u64()
        .is_some_and(|n| n <= 10_000)
    {
        return Err("npm_report_count_invalid");
    }
    let advisories = report["advisories"]
        .as_array()
        .filter(|a| report["component_count"].as_u64() == Some(a.len() as u64))
        .ok_or("npm_report_advisories_invalid")?;
    // 准备阻塞未启动工具，不能带有虚构的原生advisory。
    if matches!(
        report["diagnostic_reason"].as_str(),
        Some("npm_execution_context_missing" | "npm_manifest_invalid" | "npm_scripts_invalid")
    ) && !advisories.is_empty()
    {
        return Err("npm_report_preparation_invalid");
    }
    if missing_lock
        && (report["lock_state"] != "missing"
            || !report["lock_sha256"].is_null()
            || report["diagnostic_reason"] != "npm_lock_missing"
            || !advisories.is_empty())
    {
        return Err("npm_report_preparation_invalid");
    }
    if !missing_lock && report["diagnostic_reason"] == "npm_lock_missing" {
        return Err("npm_report_preparation_invalid");
    }
    let mut components = std::collections::BTreeSet::new();
    for advisory in advisories {
        if !exact(
            advisory,
            &["component_ref", "severity", "advisory_sources", "nodes"],
        ) || !advisory["component_ref"].as_str().is_some_and(|s| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        }) || !components.insert(advisory["component_ref"].as_str().unwrap())
            || !matches!(
                advisory["severity"].as_str(),
                Some("info" | "low" | "moderate" | "high" | "critical")
            )
        {
            return Err("npm_report_advisory_invalid");
        }
        let sources = advisory["advisory_sources"]
            .as_array()
            .ok_or("npm_report_advisory_invalid")?;
        let mut seen = std::collections::BTreeSet::new();
        if sources.len() > 10000
            || sources
                .iter()
                .any(|s| !s.as_u64().is_some_and(|n| n > 0 && seen.insert(n)))
        {
            return Err("npm_report_advisory_invalid");
        }
        let refs = advisory["nodes"]
            .as_array()
            .filter(|n| !n.is_empty() && n.len() <= 10000)
            .ok_or("npm_report_node_invalid")?;
        let mut seen_nodes = std::collections::BTreeSet::new();
        for reference in refs {
            if !exact(reference, &["node_ref", "resolved_version"])
                || !seen_nodes.insert(
                    reference["node_ref"]
                        .as_str()
                        .ok_or("npm_report_node_invalid")?,
                )
                || !reference["node_ref"].as_str().is_some_and(valid_sha256)
                || !reference["resolved_version"].as_str().is_some_and(|v| {
                    !v.is_empty() && v.len() <= 256 && !v.chars().any(char::is_control)
                })
            {
                return Err("npm_report_node_identity_invalid");
            }
        }
    }
    if !report["manifest_sha256"].as_str().is_some_and(valid_sha256)
        || (!missing_lock && !report["lock_sha256"].as_str().is_some_and(valid_sha256))
    {
        return Err("npm_report_input_identity_invalid");
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn exact(value: &Value, fields: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|m| m.len() == fields.len() && fields.iter().all(|f| m.contains_key(*f)))
}
