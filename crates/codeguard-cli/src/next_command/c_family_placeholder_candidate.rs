//! 占位文件任务诊断指引绑定首次报告、收据与当前输入；专用修复权限仍待验证闭环。
use super::Candidate;
use crate::work_sync::c_family_placeholder_report::{
    checker, current, fingerprint, positions, valid_shape, RULE,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

fn read(root: &Path, run: &str, digest: Option<&str>) -> Result<Value, &'static str> {
    if !super::safe_run_id(run) {
        return Err("clang_placeholder_origin_invalid");
    }
    let bytes = super::read_bounded(
        &root.join(format!(".codeguard/reports/{run}.json")),
        16 * 1024 * 1024,
    )?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    if digest.is_some_and(|s| s != hash) {
        return Err("clang_placeholder_origin_changed");
    }
    let mut r = codeguard_adapters::parse_unique_json(&bytes)
        .map_err(|_| "clang_placeholder_origin_invalid")?;
    if r["report_type"] == "clang_documentation_placeholder_task_recheck" {
        if !crate::c_family_placeholder_task_recheck::valid_shape(root, &r) {
            return Err("clang_placeholder_recheck_invalid");
        }
        r = crate::c_family_placeholder_task_recheck::normal(&r);
    }
    if !valid_shape(&r) || r["run_id"] != run {
        return Err("clang_placeholder_origin_invalid");
    }
    let expected=serde_json::to_vec_pretty(&json!({"schema_version":"0.1.0","workspace_id":r["workspace_id"],"run_id":run,"report_sha256":hash})).map_err(|_|"clang_placeholder_marker_invalid")?;
    if super::read_bounded(
        &root.join(format!(".codeguard/state/consumed/{run}.json")),
        4096,
    )? != expected
    {
        return Err("clang_placeholder_observation_not_consumed");
    }
    Ok(r)
}
/// 创建收据绑定的诊断指引；专用验证/尝试未接入时不授予允许修改范围。
pub(super) fn candidate(root: &Path, id: &str, fact: &Value) -> Result<Candidate, &'static str> {
    let run = fact["first_run_id"].as_str().ok_or("finding_run_invalid")?;
    let digest = fact["first_report_sha256"]
        .as_str()
        .ok_or("finding_report_invalid")?;
    let first = read(root, run, Some(digest))?;
    let fp = fingerprint(&first);
    if id != format!("CG-{}", &fp[..32])
        || fact["fingerprint"] != fp
        || fact["kind"] != "finding"
        || fact["native_rule_id"] != RULE
        || fact["checker_id"] != checker(&first)
        || fact["path"] != first["path"]
        || fact["workspace_id"] != first["workspace_id"]
        || fact["first_source_sha256"] != first["source_sha256"]
        || positions(&first).is_empty()
    {
        return Err("clang_placeholder_fact_conflict");
    }
    let score = |r: &Value| {
        r["run_id"]
            .as_str()
            .and_then(|s| s.rsplit('-').next())
            .and_then(|n| n.parse::<u128>().ok())
            .unwrap_or(0)
    };
    let mut latest = first.clone();
    let entries = fs::read_dir(root.join(".codeguard/reports"))
        .map_err(|_| "reports_unreadable")?
        .take(1001)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "reports_unreadable")?;
    if entries.len() > 1000 {
        return Err("clang_placeholder_history_limit");
    }
    for entry in entries {
        let path = entry.path();
        let Some(run) = path
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| s.starts_with("clangdocplaceholder-"))
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
    let tool_current = first["selected_tool"]
        .as_str()
        .and_then(|p| Path::new(p).canonicalize().ok())
        .and_then(|p| codeguard_runtime::read_bounded_regular_file(&p, 256 * 1024 * 1024).ok())
        .is_some_and(|b| first["native"]["tool_sha256"] == format!("{:x}", Sha256::digest(b)));
    let context = latest["selected_tool"] == first["selected_tool"]
        && latest["standard"] == first["standard"]
        && latest["native"]["tool_sha256"] == first["native"]["tool_sha256"];
    let status = if !context {
        "original_context_changed"
    } else if !tool_current || !current(root, &latest) {
        "input_changed"
    } else if positions(&latest).is_empty() {
        "candidate_absent_unverified_policy"
    } else {
        "placeholder_descriptions_observed"
    };
    let observed = status == "placeholder_descriptions_observed";
    let brief = json!({"schema_version":"0.34.0","task_id":id,"kind":"finding","checker_id":checker(&first),"scope":first["path"],"source_sha256":latest["source_sha256"],"placeholder_rule_id":RULE,"rule_source":"codeguard_structural_policy","evidence_ref":{"first_run_id":run,"first_report_sha256":digest},"current_run_id":latest["run_id"],"observation_status":status,"placeholder_positions":if observed {positions(&latest)}else{Vec::new()},"rule_basis":"整个用途、参数或适用返回说明仅为明确占位标记，不等于API契约说明；本规则不是Clang警告。","constraints":["不修改API或行为，不关闭检查器或自行批准白名单","专用尝试历史尚未接入；task verify提供局部复检观察，本视图不授予任务修复权限"],"allowed_paths":[],"affected_paths":[first["path"]],"disposition":"needs_decision","reason_code":if observed {"clang_placeholder_task_workflow_not_integrated"}else{status},"step":["依据当前组件与真实API核对占位说明；专用修复闭环未接入前保留任务。","使用绑定原工具、标准与工作区的comments命令诊断复扫；源码或工具失稳时不按历史定位修改。"],"recheck_argv":["codeguard","comments",first["language"],root.join(first["path"].as_str().ok_or("clang_placeholder_scope_invalid")?),"--clang-tool",first["selected_tool"],"--standard",first["standard"],"--workspace",root,"--format=json"],"attempt_history_status":"not_integrated","closure_condition":"原命令局部复扫不关闭任务；原工具task verify仅提供局部观察；专用受控尝试及可信关闭仍须实现并验收。","authority":"local_unverified","delivery_decision":"not_evaluated"});
    Ok(Candidate {
        id: id.into(),
        priority: 0,
        brief,
    })
}
