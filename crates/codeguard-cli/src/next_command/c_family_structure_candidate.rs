//! 当前结构文件策略组与原始收据绑定；原工具复扫不等于可信关闭。
use super::Candidate;
use crate::work_sync::c_family_structure_report::{
    checker, current, fingerprint, positions, valid_shape, RULE,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

fn read(root: &Path, run: &str, digest: Option<&str>) -> Result<Value, &'static str> {
    if !super::safe_run_id(run) {
        return Err("clang_structure_origin_invalid");
    }
    let bytes = super::read_bounded(
        &root.join(format!(".codeguard/reports/{run}.json")),
        16 * 1024 * 1024,
    )?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    if digest.is_some_and(|s| s != hash) {
        return Err("clang_structure_origin_changed");
    }
    let r = codeguard_adapters::parse_unique_json(&bytes)
        .map_err(|_| "clang_structure_origin_invalid")?;
    if !valid_shape(&r) || r["run_id"] != run {
        return Err("clang_structure_origin_invalid");
    }
    let expected=serde_json::to_vec_pretty(&json!({"schema_version":"0.1.0","workspace_id":r["workspace_id"],"run_id":run,"report_sha256":hash})).map_err(|_|"clang_structure_marker_invalid")?;
    if super::read_bounded(
        &root.join(format!(".codeguard/state/consumed/{run}.json")),
        4096,
    )? != expected
    {
        return Err("clang_structure_observation_not_consumed");
    }
    Ok(r)
}
/// 创建当前局部指引；同文件全部函数定位属于一个策略组，专用verify和尝试未接线。
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
        return Err("clang_structure_fact_conflict");
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
        return Err("clang_structure_history_limit");
    }
    for entry in entries {
        let path = entry.path();
        let Some(run) = path
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| s.starts_with("clangdocstruct-"))
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
    } else if latest["structure"]["status"] != "observed" {
        "native_structure_incomplete"
    } else if positions(&latest).is_empty() {
        "candidate_absent_unverified_policy"
    } else {
        "structural_deficits_observed"
    };
    let actionable = status == "structural_deficits_observed";
    let argv = json!([
        "codeguard",
        "comments",
        first["language"],
        root.join(first["path"].as_str().unwrap_or("")),
        "--clang-tool",
        first["selected_tool"],
        "--standard",
        first["standard"],
        "--workspace",
        root,
        "--format=json"
    ]);
    let brief = json!({"schema_version":"0.31.0","task_id":id,"kind":"finding","checker_id":checker(&first),"scope":first["path"],"source_sha256":latest["source_sha256"],"structural_rule_id":RULE,"rule_source":"codeguard_structural_policy","evidence_ref":{"first_run_id":run,"first_report_sha256":digest},"current_run_id":latest["run_id"],"observation_status":status,"structural_positions":if actionable {positions(&latest)}else{Vec::new()},"rule_basis":"函数文档结构须包含文档、用途、命名参数及适用返回说明；原AST事实不是Clang警告或语义准确性证明。","constraints":["仅修改当前文件文档注释，保留API及行为","同名/重载定位属于文件策略组，不能据名字选择单一函数","不关闭检查器、不以占位文本或白名单自批代替修复"],"allowed_paths":if actionable {json!([first["path"]])}else{json!([])},"affected_paths":[first["path"]],"disposition":if actionable {"actionable"}else{"needs_decision"},"reason_code":status,"step":["按所有当前函数字节/行列证据，依据真实声明和行为补齐缺失组件。","使用绑定原工具、标准与工作区的comments原命令复扫；未知或失稳时先诊断，不按旧位置改源码。"],"recheck_argv":argv,"history":{"status":"native_structural_observations_only","task_verify":"not_integrated","attempt_journal":"not_integrated"},"closure_condition":"原命令局部复扫不关闭；仍须专用task verify、完整详细准确性/项目覆盖与可信关闭/复发验收。","authority":"local_unverified","delivery_decision":"not_evaluated"});
    Ok(Candidate {
        id: id.into(),
        priority: if actionable { 1 } else { 0 },
        brief,
    })
}
