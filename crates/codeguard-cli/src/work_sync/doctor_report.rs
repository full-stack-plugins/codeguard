//! Doctor 局部观察的严格导入，不生成质量发现或关闭任务。
use super::{BlockerInput, ReportInput, safe_reason, safe_run_id};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;

pub(super) fn parse(
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
            "run_id",
            "workspace_binding",
            "workspace_id",
            "backlog_sync",
            "scope",
            "inspection_status",
            "authority",
            "readiness",
            "delivery_decision",
            "gate_effect",
            "quality_checks",
            "profile_summary",
            "budget",
            "ruff_version",
            "next_actions",
            "persistence",
        ],
    ) || report["report_type"] != "doctor_observation"
        || !report["backlog_sync"].is_null()
        || !exact_keys(
            &report["budget"],
            &["timeout_ms", "probe_limit_ms", "enforcement"],
        )
        || !report["budget"]["timeout_ms"]
            .as_u64()
            .is_some_and(|n| (1..=86_400_000).contains(&n))
        || report["budget"]["probe_limit_ms"] != 10000
        || report["budget"]["enforcement"] != "native_execution_only"
        || !report["next_actions"].as_array().is_some_and(|a| {
            !a.is_empty() && a.iter().all(|v| v.as_str().is_some_and(|s| !s.is_empty()))
        })
    {
        return Err("doctor_report_shape_invalid");
    }
    let run_id = report["run_id"]
        .as_str()
        .filter(|id| safe_run_id(id))
        .ok_or("report_run_id_invalid")?;
    if report["schema_version"] != "0.2.0"
        || report["workspace_binding"] != "bound"
        || report["workspace_id"] != workspace_id
        || report["persistence"] != "queued"
        || report["authority"] != "unverified"
        || report["inspection_status"] != "incomplete"
        || report["readiness"] != "unknown"
        || report["delivery_decision"] != "not_evaluated"
        || report["gate_effect"] != "none"
        || report["quality_checks"] != "not_run"
        || report["scope"] != "configuration_and_ruff_version_only"
        || path.file_stem().and_then(|s| s.to_str()) != Some(run_id)
        || !report["profile_summary"].is_object()
    {
        return Err("doctor_report_binding_invalid");
    }
    let row = &report["ruff_version"];
    if !exact_keys(
        row,
        &[
            "status",
            "reason",
            "version",
            "tool_sha256",
            "required_by_policy",
            "next_action",
        ],
    ) || !row["next_action"].as_str().is_some_and(|s| !s.is_empty())
    {
        return Err("doctor_diagnosis_shape_invalid");
    }
    if row["required_by_policy"] != Value::Null || row.get("required_by_policy").is_none() {
        return Err("doctor_prerequisite_authority_invalid");
    }
    let status = row["status"].as_str().ok_or("doctor_status_invalid")?;
    let mut blockers = Vec::new();
    match status {
        "not_selected"
            if row["reason"] == "ruff_tool_not_selected"
                && row["version"].is_null()
                && row["tool_sha256"].is_null() => {}
        "observed_untrusted"
            if row["reason"].is_null()
                && row["version"] == "ruff 0.16.8"
                && row["tool_sha256"].as_str().is_some_and(|v| {
                    v.len() == 64
                        && v.bytes().any(|c| c != b'0')
                        && v.bytes()
                            .all(|c| c.is_ascii_digit() || matches!(c, b'a'..=b'f'))
                }) => {}
        "incomplete" => {
            let reason = row["reason"]
                .as_str()
                .filter(|v| safe_reason(v))
                .ok_or("doctor_reason_invalid")?;
            if !row["version"].is_null()
                || !row["tool_sha256"].is_null()
                || reason == "ruff_tool_not_selected"
            {
                return Err("doctor_diagnosis_inconsistent");
            }
            let mut hash = Sha256::new();
            // 以工作区内前置槽位归并，不把 run/工具路径/诊断变化变成新任务。
            for part in [
                "codeguard-doctor-prerequisite-v1",
                workspace_id,
                "python.ruff.doctor",
                ".",
            ] {
                hash.update((part.len() as u64).to_be_bytes());
                hash.update(part.as_bytes());
            }
            let fingerprint = format!("{:x}", hash.finalize());
            blockers.push(BlockerInput {
                checker_id: "python.ruff.doctor".into(),
                id: format!("CG-B-{}", &fingerprint[..32]),
                fingerprint,
                reason: "ruff_version_diagnosis_requires_review".into(),
                diagnostic_reason: Some(reason.into()),
                build_root: ".".into(),
                scope: ".".into(),
                affected_paths: vec!["codeguard/workspace.json".into()],
            });
        }
        _ => return Err("doctor_diagnosis_inconsistent"),
    }
    Ok(ReportInput {
        workspace_id: workspace_id.into(),
        run_id: run_id.into(),
        digest,
        findings: Vec::new(),
        blockers,
        historical_findings: 0,
    })
}

fn exact_keys(value: &Value, keys: &[&str]) -> bool {
    value.as_object().is_some_and(|object| {
        object.len() == keys.len() && keys.iter().all(|key| object.contains_key(*key))
    })
}
