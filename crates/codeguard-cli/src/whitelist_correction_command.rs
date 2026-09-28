//! 误报白名单纠错提案；本地记录和候选文件均不授予策略权威。

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use codeguard_core::AllowlistTarget;
use codeguard_runtime::{TaskFileLock, read_bounded_regular_file};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::approval_snapshot::same_finding_scope;
use crate::false_positive_decision::{
    FalsePositiveDecisionCandidate, parse_false_positive_decision_candidate,
};
use crate::next_command::read_task_brief;
use crate::task_lease_command::{ensure_directory, real_directory};
use crate::work_sync::latest_current_finding_observation;
use crate::work_sync::write_once;

struct Args {
    finding_id: String,
    root: PathBuf,
    prior: PathBuf,
    reason: String,
    run_id: String,
    replacement: Option<PathBuf>,
    json: bool,
    record: bool,
}

/// 生成撤销/替代提案，可选持久记录；未获可信批准时返回退出码 3。
pub(crate) fn run(args: &[String]) -> ExitCode {
    let parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    };
    let mut report = inspect(&parsed);
    if parsed.record && !report["proposal"].is_null() {
        match record(&parsed) {
            Ok(recorded) => report = recorded,
            Err(reason) => report = unavailable(&parsed, reason),
        }
    }
    if parsed.json {
        println!("{report}");
    } else {
        println!(
            "误报纠错 {}：{}；仅供复核，门禁不变。",
            parsed.finding_id, report["status"]
        );
        if let Some(reference) = report["projection_ref"].as_str() {
            println!("可读纠错附件：{reference}");
        } else if report["projection_status"] == "failed" {
            println!("纠错事件已保留，但任务附件写入失败；检查附件冲突并重试记录。");
        }
    }
    ExitCode::from(3)
}

fn record(args: &Args) -> Result<Value, &'static str> {
    let root = args.root.canonicalize().map_err(|_| "project_unreadable")?;
    let state = root.join("codeguard/state");
    if !real_directory(&state) {
        return Err("workspace_state_unavailable");
    }
    let locks = state.join("task_locks");
    ensure_directory(&locks)?;
    let _lock = TaskFileLock::acquire(&locks.join(format!("{}.lock", args.finding_id)))
        .map_err(|_| "correction_lock_unavailable")?;
    // 持锁后重新核对本轮收据和候选；过期的预览不能落盘。
    let mut current = inspect(args);
    if current["proposal"].is_null() {
        return Err("correction_evidence_changed");
    }
    let events = root
        .join("codeguard/findings")
        .join(&args.finding_id)
        .join("events");
    if !real_directory(&events) {
        return Err("finding_events_unavailable");
    }
    let event = json!({
        "schema_version":"0.1.0", "event_type":"whitelist_correction_proposed",
        "task_id":args.finding_id, "finding_id":args.finding_id,
        "status":current["status"], "proposal":current["proposal"],
        "missing_evidence":current["missing_evidence"],
        "authority":"unverified", "gate_effect":"none"
    });
    let bytes = serde_json::to_vec_pretty(&event).map_err(|_| "event_encode_failed")?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let name = format!("correction-proposed-{digest}.json");
    write_once(&events.join(&name), &bytes, &state).map_err(|_| "correction_record_failed")?;
    let reference = format!("codeguard/findings/{}/events/{name}", args.finding_id);
    current["record_ref"] = json!(reference);
    // 事件为事实源，附件失败保留事件引用；同一提案可重试重建且不覆盖用户编辑。
    let projection_ref = format!(
        "codeguard/tasks/corrections/{}/{}.md",
        args.finding_id, digest
    );
    let result = (|| {
        let brief = read_task_brief(&root, &args.finding_id)?;
        let projection =
            crate::whitelist_correction_projection::render(&current, &reference, &brief);
        let tasks = root.join("codeguard/tasks");
        if !real_directory(&tasks) {
            return Err("task_directory_unavailable");
        }
        let corrections = tasks.join("corrections");
        ensure_directory(&corrections)?;
        ensure_directory(&corrections.join(&args.finding_id))?;
        write_once(&root.join(&projection_ref), projection.as_bytes(), &state)
    })();
    match result {
        Ok(()) => {
            current["projection_ref"] = json!(projection_ref);
            current["projection_status"] = json!("recorded");
        }
        Err(reason) => {
            current["projection_status"] = json!("failed");
            current["projection_reason"] = json!(reason);
            current["next_actions"]
                .as_array_mut()
                .expect("纠错下一步是数组")
                .push(json!("restore_correction_task_projection"));
        }
    }
    Ok(current)
}

fn inspect(args: &Args) -> Value {
    let root = match args.root.canonicalize() {
        Ok(root) if root.is_dir() => root,
        _ => return unavailable(args, "project_unreadable"),
    };
    let brief = match read_task_brief(&root, &args.finding_id) {
        Ok(brief) => brief,
        Err(reason) => return unavailable(args, reason),
    };
    if brief["kind"] != "finding" {
        return unavailable(args, "not_a_finding");
    }
    let prior_bytes = match bounded_file(&args.prior, 128 * 1024) {
        Ok(bytes) => bytes,
        Err(_) => return unavailable(args, "prior_decision_unreadable"),
    };
    let prior = match parse_false_positive_decision_candidate(&prior_bytes) {
        Ok(prior) => prior,
        Err(_) => return unavailable(args, "prior_decision_invalid"),
    };
    if !prior_matches_brief(&prior, &brief, &args.finding_id) {
        return unavailable(args, "old_decision_scope_mismatch");
    }
    if !matches!(
        brief["verification_observation"].as_str(),
        Some("still_present" | "candidate_absent_unverified_policy")
    ) || !brief["verification_run_id"].is_string()
        || !brief["verification_report_sha256"].is_string()
    {
        return unavailable(args, "verification_required");
    }
    if brief["verification_run_id"] != args.run_id {
        return unavailable(args, "verification_reference_mismatch");
    }
    if args.reason == "root_cause_fixed"
        && (brief["verification_observation"] != "candidate_absent_unverified_policy"
            || args.replacement.is_some())
    {
        return unavailable(args, "correction_reason_conflict");
    }
    let mut replacement_summary = Value::Null;
    if let Some(path) = &args.replacement {
        if brief["verification_observation"] != "still_present" {
            return unavailable(args, "replacement_requires_current_finding");
        }
        let bytes = match bounded_file(path, 128 * 1024) {
            Ok(bytes) => bytes,
            Err(_) => return unavailable(args, "replacement_unreadable"),
        };
        let replacement = match parse_false_positive_decision_candidate(&bytes) {
            Ok(replacement) => replacement,
            Err(_) => return unavailable(args, "replacement_invalid"),
        };
        if replacement.replaces_decision_id.as_deref() != Some(prior.id.as_str())
            || !same_finding_scope(&prior.identity, &replacement.identity)
        {
            return unavailable(args, "replacement_scope_mismatch");
        }
        let observed = match latest_current_finding_observation(&root, &args.finding_id) {
            Ok(observed) => observed,
            Err(_) => return unavailable(args, "current_finding_evidence_incomplete"),
        };
        if !replacement_matches_observed(&replacement, &observed, &args.run_id) {
            return unavailable(args, "replacement_identity_mismatch");
        }
        replacement_summary = json!({
            "id":replacement.id,
            "replaces_decision_id":prior.id,
            "candidate_sha256":format!("{:x}", Sha256::digest(bytes)),
            "status":"unverified_draft"
        });
    }
    let has_replacement = args.replacement.is_some();
    json!({
        "schema_version":"0.2.0", "report_type":"whitelist_correction_preview",
        "projection_status":"not_requested", "projection_ref":null, "projection_reason":null,
        "operation":"whitelist_propose_correction", "command_status":"incomplete", "exit_code":3,
        "finding_id":args.finding_id,
        "status":if has_replacement { "evidence_incomplete" } else { "review_required" },
        "reason":if has_replacement { "current_approval_identity_unavailable" } else { "independent_approval_unavailable" },
        "proposal":{
            "old_decision_id":prior.id,
            "finding_id":args.finding_id,
            "correction_reason":args.reason,
            "verification_ref":{
                "run_id":args.run_id,
                "report_sha256":brief["verification_report_sha256"],
                "observation":brief["verification_observation"]
            },
            "revoke_decision_ids":[prior.id],
            "replacement_candidate":replacement_summary,
            "affected_finding_ids":[args.finding_id]
        },
        "missing_evidence":if has_replacement {
            json!(["prior_protected_approval", "adapter_sha256", "approved_rulepack_identity", "trusted_policy_revision", "independent_review"])
        } else {
            json!(["prior_protected_approval", "trusted_policy_revision", "independent_review"])
        },
        "next_actions":["review_native_recheck", "verify_prior_policy_authority", "request_independent_approval"],
        "authority":"unverified", "gate_effect":"none", "delivery_decision":"not_evaluated"
    })
}

fn prior_matches_brief(prior: &FalsePositiveDecisionCandidate, brief: &Value, id: &str) -> bool {
    prior.identity.finding_id == id
        && prior.identity.checker_id == brief["checker_id"]
        && prior.identity.native_rule_id == brief["native_rule_id"]
        && prior.identity.category == "lint"
        && matches!(&prior.identity.target, AllowlistTarget::Source { path, .. } if path == &brief["scope"])
}

fn replacement_matches_observed(
    replacement: &FalsePositiveDecisionCandidate,
    observed: &Value,
    run_id: &str,
) -> bool {
    let AllowlistTarget::Source { file_sha256, .. } = &replacement.identity.target else {
        return false;
    };
    observed["run_id"] == run_id
        && observed["source_sha256"] == file_sha256.as_str()
        && observed["finding_fingerprint"] == replacement.identity.finding_fingerprint
        && observed["tool_sha256"] == replacement.identity.tool_sha256
        && observed["rulepack_sha256"] == replacement.identity.rulepack_sha256
        && observed["rulepack_status"] == "candidate_unapproved"
}

fn unavailable(args: &Args, reason: &str) -> Value {
    json!({
        "schema_version":"0.2.0", "report_type":"whitelist_correction_preview",
        "projection_status":"not_requested", "projection_ref":null, "projection_reason":null,
        "operation":"whitelist_propose_correction", "command_status":"incomplete", "exit_code":3,
        "finding_id":args.finding_id, "status":reason, "reason":reason, "proposal":null,
        "missing_evidence":[], "next_actions":["inspect_task_and_original_checker_evidence"],
        "authority":"unverified", "gate_effect":"none", "delivery_decision":"not_evaluated"
    })
}

fn bounded_file(path: &Path, limit: u64) -> Result<Vec<u8>, &'static str> {
    read_bounded_regular_file(path, limit).map_err(|_| "file_not_bounded")
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let id = args.first().ok_or("propose 缺少 finding ID")?;
    let suffix = id.strip_prefix("CG-B-").or_else(|| id.strip_prefix("CG-"));
    if !suffix.is_some_and(|part| {
        part.len() == 32
            && part
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    }) {
        return Err("finding ID 无效".into());
    }
    let mut root = None;
    let mut prior = None;
    let mut reason = None;
    let mut run_id = None;
    let mut replacement = None;
    let mut json = false;
    let mut record = false;
    let mut index = 1;
    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "--correct-decision" => {
                index += 1;
                if prior
                    .replace(PathBuf::from(args.get(index).ok_or("缺少旧决策文件")?))
                    .is_some()
                {
                    return Err("重复旧决策参数".into());
                }
            }
            "--correction-reason" => {
                index += 1;
                if reason
                    .replace(args.get(index).ok_or("缺少纠错原因")?.clone())
                    .is_some()
                {
                    return Err("重复纠错原因".into());
                }
            }
            "--verification-run" => {
                index += 1;
                if run_id
                    .replace(args.get(index).ok_or("缺少复检 run ID")?.clone())
                    .is_some()
                {
                    return Err("重复复检 run ID".into());
                }
            }
            "--replacement" => {
                index += 1;
                if replacement
                    .replace(PathBuf::from(args.get(index).ok_or("缺少替代候选文件")?))
                    .is_some()
                {
                    return Err("重复替代候选参数".into());
                }
            }
            "--format=json" => json = true,
            "--record" if !record => record = true,
            "--format=human" => json = false,
            "--format" => {
                index += 1;
                json = match args.get(index).map(String::as_str) {
                    Some("json") => true,
                    Some("human") => false,
                    _ => return Err("--format 仅支持 human/json".into()),
                };
            }
            _ if arg.starts_with('-') || root.is_some() => {
                return Err(format!("不支持的参数：{arg}"));
            }
            _ => root = Some(PathBuf::from(arg)),
        }
        index += 1;
    }
    let reason = reason.ok_or("缺少 --correction-reason")?;
    if !matches!(
        reason.as_str(),
        "erroneous_approval" | "scope_too_broad" | "evidence_invalid" | "root_cause_fixed"
    ) {
        return Err("纠错原因无效".into());
    }
    let run_id = run_id.ok_or("缺少 --verification-run")?;
    if !run_id.starts_with("lint-")
        || run_id.len() > 120
        || !run_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err("复检 run ID 无效".into());
    }
    Ok(Args {
        finding_id: id.clone(),
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        prior: prior.ok_or("缺少 --correct-decision")?,
        reason,
        run_id,
        replacement,
        json,
        record,
    })
}
