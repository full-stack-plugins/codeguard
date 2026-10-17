//! Clang文档规则组的只读当前投影；首次工具/标准不由Markdown重建。
use super::Candidate;
use crate::work_sync::c_family_comments_report::{checker, current, fingerprint, valid_shape};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
fn read(root: &Path, run: &str, expected: Option<&str>) -> Result<Value, &'static str> {
    if !super::safe_run_id(run) {
        return Err("clang_documentation_origin_invalid");
    }
    let bytes = super::read_bounded(
        &root.join(format!(".codeguard/reports/{run}.json")),
        128 * 1024,
    )?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if expected.is_some_and(|s| s != digest) {
        return Err("clang_documentation_origin_changed");
    }
    let mut r = codeguard_adapters::parse_unique_json(&bytes)
        .map_err(|_| "clang_documentation_origin_invalid")?;
    let recheck_input_stable = r.get("input_stable").cloned();
    if r["report_type"] == "clang_documentation_task_recheck" {
        if !crate::c_family_comments_task_recheck::valid_shape(root, &r) {
            return Err("clang_documentation_recheck_invalid");
        }
        r = crate::c_family_comments_task_recheck::normal(&r);
    }
    if !valid_shape(&r) || r["run_id"] != run {
        return Err("clang_documentation_origin_invalid");
    }
    let expected_marker = serde_json::to_vec_pretty(&json!({"schema_version":"0.1.0","workspace_id":r["workspace_id"],"run_id":run,"report_sha256":digest})).map_err(|_| "clang_documentation_marker_invalid")?;
    if super::read_bounded(
        &root.join(format!(".codeguard/state/consumed/{run}.json")),
        4096,
    )? != expected_marker
    {
        return Err("clang_documentation_observation_not_consumed");
    }
    if let Some(stable) = recheck_input_stable {
        r["task_recheck_input_stable"] = stable;
    }
    Ok(r)
}
/// 复核首次事实与消费收据，返回当前位置或具体缺口；零诊断不关闭。
pub(super) fn candidate(root: &Path, id: &str, fact: &Value) -> Result<Candidate, &'static str> {
    let run = fact["first_run_id"].as_str().ok_or("finding_run_invalid")?;
    let digest = fact["first_report_sha256"]
        .as_str()
        .ok_or("finding_report_invalid")?;
    let first = read(root, run, Some(digest))?;
    let kind = fact["kind"].as_str().ok_or("finding_kind_invalid")?;
    let rule = if kind == "blocker" {
        "environment"
    } else {
        fact["native_rule_id"]
            .as_str()
            .ok_or("finding_rule_invalid")?
    };
    let fp = fingerprint(&first, rule);
    let expected_id = format!(
        "{}{}",
        if kind == "blocker" { "CG-B-" } else { "CG-" },
        &fp[..32]
    );
    if !matches!(kind, "finding" | "blocker")
        || id != expected_id
        || fact["fingerprint"] != fp
        || fact["checker_id"] != checker(&first)
        || fact["workspace_id"] != first["workspace_id"]
        || fact[if kind == "blocker" { "scope" } else { "path" }] != first["path"]
        || (kind == "finding" && fact["first_source_sha256"] != first["source_sha256"])
        || (kind == "finding"
            && (first["local_scan_complete"] != true
                || !first["native"]["diagnostics"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|d| d["rule_id"] == rule)
                || codeguard_adapters::clang_documentation_guidance(rule).is_none()))
    {
        return Err("clang_documentation_fact_conflict");
    }
    let mut latest = first.clone();
    let score = |r: &Value| {
        r["run_id"]
            .as_str()
            .and_then(|s| s.rsplit('-').next())
            .and_then(|s| s.parse::<u128>().ok())
            .unwrap_or(0)
    };
    let entries = fs::read_dir(root.join(".codeguard/reports"))
        .map_err(|_| "reports_unreadable")?
        .take(1001)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "reports_unreadable")?;
    if entries.len() > 1000 {
        return Err("clang_documentation_history_limit");
    }
    for entry in entries {
        let path = entry.path();
        let Some(run) = path
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| s.starts_with("clangdoc-"))
        else {
            continue;
        };
        if !root
            .join(format!(".codeguard/state/consumed/{run}.json"))
            .exists()
        {
            continue;
        }
        let r = read(root, run, None)?;
        if r["workspace_id"] == first["workspace_id"]
            && r["path"] == first["path"]
            && r["language"] == first["language"]
            && score(&r) > score(&latest)
        {
            latest = r;
        }
    }
    let tool_current = latest["selected_tool"]
        .as_str()
        .and_then(|tool| Path::new(tool).canonicalize().ok())
        .and_then(|tool| {
            codeguard_runtime::read_bounded_regular_file(&tool, 256 * 1024 * 1024).ok()
        })
        .is_some_and(|bytes| {
            latest["native"]["tool_sha256"] == format!("{:x}", Sha256::digest(bytes))
        });
    let context = tool_current
        && latest["standard"] == first["standard"]
        && latest["selected_tool"] == first["selected_tool"]
        && (kind == "blocker" || latest["native"]["tool_sha256"] == first["native"]["tool_sha256"]);
    let present: Vec<_> = latest["native"]["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|d| d["rule_id"] == rule)
        .cloned()
        .collect();
    let status = if latest["task_recheck_input_stable"] == false || !current(root, &latest) {
        "input_changed"
    } else if latest["local_scan_complete"] != true {
        "native_incomplete"
    } else if !context {
        "tool_or_standard_changed"
    } else if kind == "blocker" {
        "coverage_requires_review"
    } else if present.is_empty() {
        "candidate_absent_unverified"
    } else {
        "native_rule_observed"
    };
    let source_actionable = status == "native_rule_observed";
    let environment_actionable = kind == "blocker"
        && status == "native_incomplete"
        && latest["native"]["reason"] == "clang_tool_unavailable"
        && first["native"]["tool_sha256"].is_null();
    let actionable = source_actionable || environment_actionable;
    let guidance = codeguard_adapters::clang_documentation_guidance(rule).unwrap_or(json!({"rule_summary":"原生文档检查未完成或存在未适配规则","repair_steps":["核对原生原因、工具与源码范围，再运行原文档检查。"]}));
    let mut brief = json!({"schema_version":"0.30.0","task_id":id,"kind":kind,"checker_id":checker(&first),"scope":first["path"],"source_sha256":latest["source_sha256"],"native_rule_id":if kind == "finding" {json!(rule)} else {Value::Null},
        "evidence_ref":{"first_run_id":run,"first_report_sha256":digest},"current_run_id":latest["run_id"],"observation_status":status,"native_reason":latest["native"]["reason"],"native_positions":if source_actionable {json!(present)} else {json!([])},
        "unclassified_native_diagnostics":latest["native"]["diagnostics"].as_array().into_iter().flatten().filter(|d| d["rule_id"].as_str().is_some_and(|r| codeguard_adapters::clang_documentation_guidance(r).is_none())).cloned().collect::<Vec<_>>(),
        "rule_basis":guidance["rule_summary"],"constraints":["仅文档注释；保留源码API与行为","本地报告非可信政策，不能关闭任务或自批白名单"],"allowed_paths":if source_actionable {json!([first["path"]])} else {json!([])},
        "disposition":if actionable {"actionable"} else {"needs_decision"},"reason_code":status,"step":guidance["repair_steps"],"recheck_argv":["codeguard","task","verify",id,root,"--clang-tool",first["selected_tool"],"--format=json"],
        "closure_condition":"尝试和原工具局部复检均不关闭；仍需完整详细文档覆盖、可信政策与关闭/复发验证。零诊断、勾选或本地失败记录均不能放行。","authority":"local_unverified","delivery_decision":"not_evaluated"});
    brief["affected_paths"] = json!([first["path"]]);
    brief["action_id"] = json!(super::canonical_action_id(&brief)?);
    if environment_actionable {
        brief["step"] = json!([format!(
            "恢复首次选择的原工具路径 {}，核对Apple Clang21与固定标准，再执行原任务复检；不修改无关源码。",
            first["selected_tool"].as_str().unwrap_or("")
        )]);
    }
    let history = crate::task_attempt_command::attempt_history(root, id, &brief)?;
    let mut priority = if source_actionable { 1 } else { 0 };
    if history["open_attempt_id"].is_string() {
        brief["disposition"] = json!("waiting");
        brief["reason_code"] = json!("attempt_in_progress");
        brief["step"] = json!(["由持有租约的执行者记录当前尝试结果；结束前不重复修复同一任务。"]);
        priority = 0;
    } else if history["awaiting_verification"] == true {
        brief["disposition"] = json!("verification_required");
        brief["reason_code"] = json!("original_verification_required");
        brief["step"] = json!([
            "修复尝试已结束；按首次工具、标准与规则执行task verify，不能用再次勾选任务代替复检。"
        ]);
        priority = 0;
    } else if history["unverified_prior_attempt_count"]
        .as_u64()
        .unwrap_or(0)
        > 0
    {
        brief["disposition"] = json!("needs_decision");
        brief["reason_code"] = json!("historical_verification_evidence_unavailable");
        brief["step"] = json!([
            "先恢复当前输入下早先尝试的原复检报告或调查失效原因；缺失本地证据不能删除失败尝试或恢复修复预算。"
        ]);
        priority = 0;
    } else if history["no_progress_count"].as_u64().unwrap_or(0) >= 2 && actionable {
        brief["disposition"] = json!("needs_decision");
        brief["reason_code"] = json!("no_progress_budget_exhausted");
        brief["step"] = json!([format!(
            "同一源码、原工具上下文与动作已连续无进展；核对规则 {}、原生原因 {} 和最近失败记录，提出具体修复或误报纠错决策；重命名动作、重复扫描不能重置预算。",
            rule,
            latest["native"]["reason"].as_str().unwrap_or("unknown")
        )]);
        priority = 0;
    }
    if brief["disposition"] != "actionable" {
        brief["allowed_paths"] = json!([]);
    }
    brief["history"] = history;

    Ok(Candidate {
        id: id.into(),
        priority,
        brief,
    })
}
