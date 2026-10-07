//! 独立占位任务原工具复检的身份前置检查；执行、受控尝试与可信关闭仍待接入。
use crate::work_sync::c_family_placeholder_report as contract;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;

fn reference(brief: &Value) -> Value {
    json!({"run_id":brief["evidence_ref"]["first_run_id"],"sha256":brief["evidence_ref"]["first_report_sha256"]})
}
/// 核验首次报告字节和精确消费收据；返回原始独立占位观察。
pub(crate) fn original(root: &Path, reference: &Value) -> Result<Value, &'static str> {
    if !reference
        .as_object()
        .is_some_and(|o| o.len() == 2 && o.contains_key("sha256"))
    {
        return Err("clang_placeholder_origin_invalid");
    }
    let run = reference["run_id"]
        .as_str()
        .filter(|r| {
            r.starts_with("clangdocplaceholder-")
                && r.len() <= 120
                && r.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
        .ok_or("clang_placeholder_origin_invalid")?;
    let bytes = read_bounded_regular_file(
        &root.join(format!(".codeguard/reports/{run}.json")),
        16 * 1024 * 1024,
    )
    .map_err(|_| "clang_placeholder_origin_unavailable")?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    if reference["sha256"] != hash {
        return Err("clang_placeholder_origin_changed");
    }
    let r = codeguard_adapters::parse_unique_json(&bytes)
        .map_err(|_| "clang_placeholder_origin_invalid")?;
    if !contract::valid_shape(&r) || r["run_id"] != run {
        return Err("clang_placeholder_origin_invalid");
    }
    let marker=serde_json::to_vec_pretty(&json!({"schema_version":"0.1.0","workspace_id":r["workspace_id"],"run_id":run,"report_sha256":hash})).map_err(|_|"clang_placeholder_marker_invalid")?;
    if read_bounded_regular_file(
        &root.join(format!(".codeguard/state/consumed/{run}.json")),
        4096,
    )
    .map_err(|_| "clang_placeholder_marker_invalid")?
        != marker
    {
        return Err("clang_placeholder_marker_invalid");
    }
    Ok(r)
}
fn tool_current(r: &Value) -> bool {
    r["selected_tool"]
        .as_str()
        .and_then(|p| Path::new(p).canonicalize().ok())
        .and_then(|p| read_bounded_regular_file(&p, 256 * 1024 * 1024).ok())
        .is_some_and(|b| r["native"]["tool_sha256"] == format!("{:x}", Sha256::digest(b)))
}
fn fact_bound(root: &Path, first: &Value, id: &Value) -> bool {
    let fp = contract::fingerprint(first);
    let expected = format!("CG-{}", &fp[..32]);
    if id != &Value::from(expected.clone()) || contract::positions(first).is_empty() {
        return false;
    }
    let Ok(bytes) = read_bounded_regular_file(
        &root.join(format!(".codeguard/findings/{expected}/finding.json")),
        128 * 1024,
    ) else {
        return false;
    };
    let Ok(f) = codeguard_adapters::parse_unique_json(&bytes) else {
        return false;
    };
    f["id"] == expected
        && f["fingerprint"] == fp
        && f["kind"] == "finding"
        && f["state"] == "open"
        && f["authority"] == "local_unverified"
        && f["delivery_decision"] == "not_evaluated"
        && f["checker_id"] == contract::checker(first)
        && f["native_rule_id"] == contract::RULE
        && f["workspace_id"] == first["workspace_id"]
        && f["path"] == first["path"]
        && f["first_run_id"] == first["run_id"]
        && f["first_source_sha256"] == first["source_sha256"]
}
/// 进程及租约前拒绝替换原工具、伪造策略或任务范围。
pub(crate) fn preflight(
    root: &Path,
    brief: &Value,
    tool: Option<&Path>,
) -> Result<(), &'static str> {
    let first = original(root, &reference(brief))?;
    if !fact_bound(root, &first, &brief["task_id"])
        || brief["kind"] != "finding"
        || brief["checker_id"] != contract::checker(&first)
        || brief["placeholder_rule_id"] != contract::RULE
        || brief["rule_source"] != "codeguard_structural_policy"
        || brief["scope"] != first["path"]
    {
        return Err("clang_placeholder_fact_conflict");
    }
    let fact = read_bounded_regular_file(
        &root.join(format!(
            ".codeguard/findings/{}/finding.json",
            brief["task_id"]
                .as_str()
                .ok_or("clang_placeholder_fact_conflict")?
        )),
        128 * 1024,
    )
    .map_err(|_| "clang_placeholder_fact_conflict")?;
    if codeguard_adapters::parse_unique_json(&fact)
        .map_err(|_| "clang_placeholder_fact_conflict")?["first_report_sha256"]
        != reference(brief)["sha256"]
    {
        return Err("clang_placeholder_fact_conflict");
    }
    if tool.is_some_and(|p| Some(p) != first["selected_tool"].as_str().map(Path::new)) {
        return Err("clang_placeholder_original_tool_required");
    }
    if !tool_current(&first) {
        return Err("clang_placeholder_original_tool_changed");
    }
    Ok(())
}
