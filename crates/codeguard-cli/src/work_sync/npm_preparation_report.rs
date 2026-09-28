//! npm输入不可用的严格准备报告消费；零发现不能形成交付许可。
use super::{ReportInput, safe_relative_path, safe_run_id};
use serde_json::Value;
use std::path::Path;
/// 参数为物理工作区、工作区身份、报告路径、严格观察及摘要；返回稳定阻塞输入或具体错误。
pub(super) fn parse(
    root: &Path,
    workspace: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    let fields = [
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
        "manifest_state",
        "lock_state",
        "manifest_sha256",
        "lock_sha256",
        "diagnostic_reason",
        "component_count",
        "advisories",
    ];
    if !report
        .as_object()
        .is_some_and(|m| m.len() == fields.len() && fields.iter().all(|f| m.contains_key(*f)))
    {
        return Err("npm_report_shape_invalid");
    }
    let run = report["run_id"]
        .as_str()
        .filter(|v| safe_run_id(v))
        .ok_or("report_run_id_invalid")?;
    let build = report["build_root"]
        .as_str()
        .filter(|v| *v == "." || safe_relative_path(v))
        .ok_or("npm_report_root_invalid")?;
    if report["schema_version"] != "0.3.0"
        || report["report_type"] != "npm_cve_workbench_observation"
        || report["workspace_binding"] != "bound"
        || report["workspace_id"] != workspace
        || report["authority"] != "local_unverified"
        || report["coverage_proven"] != false
        || report["delivery_decision"] != "not_evaluated"
        || report["checker_id"] != "node.npm.audit"
        || path.file_stem().and_then(|s| s.to_str()) != Some(run)
        || report["component_count"] != 0
        || !report["advisories"].as_array().is_some_and(Vec::is_empty)
    {
        return Err("npm_report_identity_invalid");
    }
    let states = [
        "present",
        "missing",
        "not_regular",
        "path_alias",
        "unavailable",
    ];
    let mut affected = Vec::new();
    // 先验证全部结构，损坏字段不能因为输入变化而被当作过期历史忽略。
    for prefix in ["manifest", "lock"] {
        let state = report[format!("{prefix}_state")]
            .as_str()
            .filter(|s| states.contains(s))
            .ok_or("npm_report_preparation_invalid")?;
        let hash = &report[format!("{prefix}_sha256")];
        if (state == "present"
            && !hash.as_str().is_some_and(|s| {
                s.len() == 64
                    && s.bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            }))
            || (state != "present" && !hash.is_null())
        {
            return Err("npm_report_preparation_invalid");
        }
    }
    let manifest = report["manifest_state"].as_str().unwrap();
    let lock = report["lock_state"].as_str().unwrap();
    if manifest == "present" && lock == "present" {
        return Err("npm_report_preparation_invalid");
    }
    let reason = if manifest != "present" {
        format!("npm_manifest_{manifest}")
    } else {
        format!("npm_lock_{lock}")
    };
    if report["diagnostic_reason"] != reason {
        return Err("npm_report_preparation_invalid");
    }
    let project = root.join(build);
    if project.canonicalize().ok().as_ref() != Some(&project) && build != "." {
        return Err("npm_report_input_invalid");
    }
    for (file, prefix, limit) in [
        ("package.json", "manifest", 256 * 1024),
        ("package-lock.json", "lock", 8 * 1024 * 1024),
    ] {
        let relative = if build == "." {
            file.into()
        } else {
            format!("{build}/{file}")
        };
        let (state, hash) = crate::npm_input_state::observe(&root.join(&relative), limit);
        if report[format!("{prefix}_state")] != state
            || report[format!("{prefix}_sha256")] != serde_json::json!(hash)
        {
            return Err("npm_report_input_changed");
        }
        affected.push(relative);
    }
    Ok(super::npm_report::make_report(
        workspace, run, digest, build, &reason, affected,
    ))
}
