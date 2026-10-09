//! 独立占位任务的原工具身份核验及有界复检；受控尝试共享事件账本，可信关闭仍待接入。
use crate::work_sync::c_family_placeholder_report as contract;
use codeguard_runtime::{SourceSnapshot, read_bounded_regular_file};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
const EXTRA: &[&str] = &[
    "task_id",
    "task_kind",
    "task_rule",
    "original_ref",
    "input_stable",
    "placeholder_observation_status",
    "placeholder_observation_reason",
];

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

/// 冻结当前源码，执行首次原工具/标准的有界AST档案，返回封闭复检报告。
pub(crate) fn run(root: &Path, brief: &Value, deadline: Instant) -> Result<Value, &'static str> {
    preflight(root, brief, None)?;
    let first = original(root, &reference(brief))?;
    let path = PathBuf::from(
        first["path"]
            .as_str()
            .ok_or("clang_placeholder_target_invalid")?,
    );
    let snapshot = SourceSnapshot::capture(root, [path.clone()], 1, 1024 * 1024, 1024 * 1024)
        .map_err(|_| "clang_placeholder_source_unavailable")?;
    let bytes = snapshot
        .files()
        .get(&path)
        .ok_or("clang_placeholder_source_unavailable")?;
    let (native, mut structure, placeholders) =
        crate::clang_syntax_probe::observe_documentation_with_placeholders(
            Path::new(
                first["selected_tool"]
                    .as_str()
                    .ok_or("clang_placeholder_tool_invalid")?,
            ),
            first["language"]
                .as_str()
                .ok_or("clang_placeholder_language_invalid")?,
            first["standard"]
                .as_str()
                .ok_or("clang_placeholder_standard_invalid")?,
            bytes,
            deadline,
            &AtomicBool::new(false),
            true,
        );
    let complete = matches!(
        native["status"].as_str(),
        Some("completed" | "diagnostics_observed")
    ) && !native["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|d| d["level"] == "error");
    if !complete {
        structure["status"] = json!("incomplete");
        structure["reason"] = json!("clang_execution_incomplete");
        structure["observation"] = Value::Null;
    }
    let mut r = first.clone();
    let observed =
        complete && structure["status"] == "observed" && placeholders["status"] == "observed";
    r["schema_version"] = json!("0.2.0");
    r["placeholder_observation_status"] = json!(if observed { "observed" } else { "incomplete" });
    r["placeholder_observation_reason"] = if observed {
        json!("clang_placeholder_observed")
    } else if !complete {
        json!("clang_execution_incomplete")
    } else {
        placeholders["reason"].clone()
    };
    r["placeholders"] = if observed {
        placeholders["observation"].clone()
    } else {
        Value::Null
    };
    r["report_type"] = json!("clang_documentation_placeholder_task_recheck");
    r["run_id"] = json!(format!(
        "clangdocplaceholder-{}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    r["source_sha256"] = json!(format!("{:x}", Sha256::digest(bytes)));
    r["native"] = native;
    r["structure"] = structure;
    r["local_scan_complete"] = json!(complete);
    r["task_id"] = brief["task_id"].clone();
    r["task_kind"] = json!("finding");
    r["task_rule"] = json!(contract::RULE);
    r["original_ref"] = reference(brief);
    r["input_stable"] = json!(normal_inputs_current(root, &normal(&r)) && tool_current(&first));
    if !valid_shape(root, &r) {
        return Err("clang_placeholder_recheck_invalid");
    }
    Ok(r)
}
/// 仅移除任务附加字段以复用独立占位观察校验，不改输入身份。
pub(crate) fn normal(r: &Value) -> Value {
    let mut n = r.clone();
    if let Some(o) = n.as_object_mut() {
        for k in EXTRA {
            o.remove(*k);
        }
        o.insert("schema_version".into(), json!("0.1.0"));
        o.insert(
            "report_type".into(),
            json!("clang_documentation_placeholder_workbench_observation"),
        );
    }
    n
}
/// 校验规范化的当前观察；失败只能携带空占位数据，不会成为干净候选。
pub(crate) fn valid_normal_shape(r: &Value) -> bool {
    if r["placeholders"].is_null() {
        r.is_object()
            && r["report_type"] == "clang_documentation_placeholder_workbench_observation"
            && r["profile"] == "clang-documentation-placeholder-v1"
            && r["run_id"]
                .as_str()
                .is_some_and(|s| s.starts_with("clangdocplaceholder-"))
            && crate::work_sync::c_family_structure_report::valid_shape(
                &contract::structure_packet(r),
            )
    } else {
        contract::valid_shape(r)
    }
}
/// 检查失败观察的源码与底层结构关联；没有占位数据时不验证任何旧占位位置。
pub(crate) fn normal_inputs_current(root: &Path, r: &Value) -> bool {
    valid_normal_shape(r)
        && if r["placeholders"].is_null() {
            crate::work_sync::c_family_structure_report::current(
                root,
                &contract::structure_packet(r),
            )
        } else {
            contract::current(root, r)
        }
}
/// 绑定首次报告、原策略及开放任务；拒绝额外字段与身份篡改。
pub(crate) fn valid_shape(root: &Path, r: &Value) -> bool {
    if !r.as_object().is_some_and(|o| {
        if r["schema_version"] == "0.1.0" {
            o.len() == 23 && EXTRA[..5].iter().all(|k| o.contains_key(*k))
        } else {
            r["schema_version"] == "0.2.0"
                && o.len() == 25
                && EXTRA.iter().all(|k| o.contains_key(*k))
        }
    }) || r["report_type"] != "clang_documentation_placeholder_task_recheck"
        || r["task_kind"] != "finding"
        || r["task_rule"] != contract::RULE
        || !r["input_stable"].is_boolean()
        || (r["schema_version"] == "0.1.0" && !contract::valid_shape(&normal(r)))
        || !valid_normal_shape(&normal(r))
        || (r["schema_version"] == "0.2.0"
            && !((r["placeholder_observation_status"] == "observed"
                && r["placeholder_observation_reason"] == "clang_placeholder_observed"
                && contract::valid_shape(&normal(r)))
                || (r["placeholder_observation_status"] == "incomplete"
                    && r["placeholders"].is_null()
                    && matches!(
                        r["placeholder_observation_reason"].as_str(),
                        Some(
                            "clang_placeholder_unavailable"
                                | "clang_placeholder_report_invalid"
                                | "clang_source_changed"
                                | "request_cancelled"
                                | "clang_execution_incomplete"
                        )
                    ))))
    {
        return false;
    }
    let Ok(first) = original(root, &r["original_ref"]) else {
        return false;
    };
    if !fact_bound(root, &first, &r["task_id"])
        || [
            "workspace_id",
            "path",
            "language",
            "standard",
            "profile",
            "selected_tool",
        ]
        .iter()
        .any(|k| r[*k] != first[*k])
    {
        return false;
    }
    let Some(id) = r["task_id"].as_str() else {
        return false;
    };
    read_bounded_regular_file(
        &root.join(format!(".codeguard/findings/{id}/finding.json")),
        128 * 1024,
    )
    .ok()
    .and_then(|b| codeguard_adapters::parse_unique_json(&b).ok())
    .is_some_and(|f| f["first_report_sha256"] == r["original_ref"]["sha256"])
}
/// 持久化前重新核对当前源码和原工具，防止竞态变成完整复检。
pub(crate) fn inputs_current(root: &Path, r: &Value) -> bool {
    valid_shape(root, r)
        && normal_inputs_current(root, &normal(r))
        && original(root, &r["original_ref"]).is_ok_and(|first| tool_current(&first))
        && (r["native"]["tool_sha256"].is_null() || tool_current(r))
}
/// 分类占位组件局部结果，未知或失稳不得变成消失或可信关闭。
pub(crate) fn classify(brief: &Value, r: &Value) -> &'static str {
    if r["task_id"] != brief["task_id"]
        || r["input_stable"] != true
        || r["local_scan_complete"] != true
        || r["structure"]["status"] != "observed"
        || r["placeholders"].is_null()
    {
        "incomplete"
    } else if contract::positions(&normal(r)).is_empty() {
        "candidate_absent_unverified_policy"
    } else {
        "still_present"
    }
}

/// 计算受控尝试的当前输入身份；包含源码、首次语言/标准与当前工具状态，不能只靠动作名重置预算。
/// 参数为已初始化工作区与原任务摘要；返回当前输入SHA-256，不可读或越界输入返回具体错误。
pub(crate) fn attempt_input_digest(root: &Path, brief: &Value) -> Result<String, &'static str> {
    let first = original(root, &reference(brief))?;
    let path = first["path"].as_str().ok_or("attempt_scope_invalid")?;
    let snapshot =
        SourceSnapshot::capture(root, [PathBuf::from(path)], 1, 1024 * 1024, 1024 * 1024)
            .map_err(|_| "attempt_source_unavailable")?;
    let bytes = snapshot
        .files()
        .get(&PathBuf::from(path))
        .ok_or("attempt_source_unavailable")?;
    let selected = Path::new(
        first["selected_tool"]
            .as_str()
            .ok_or("attempt_tool_invalid")?,
    );
    let canonical = selected.canonicalize().ok();
    let tool_bytes = canonical
        .as_ref()
        .and_then(|p| read_bounded_regular_file(p, 256 * 1024 * 1024).ok());
    let state = if tool_bytes.is_some() {
        "observed"
    } else if std::fs::symlink_metadata(selected)
        .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
    {
        "missing"
    } else {
        "unavailable"
    };
    let payload = json!({"version":"clang-documentation-placeholder-attempt-input-v1","scope":path,"language":first["language"],"standard":first["standard"],"profile":first["profile"],"policy":contract::RULE,"placeholder_engine_sha256":codeguard_adapters::clang_documentation_placeholder_engine_sha256(),"recheck_engine_sha256":format!("{:x}", Sha256::digest(include_bytes!("c_family_placeholder_task_recheck.rs"))),"source_sha256":format!("{:x}",Sha256::digest(bytes)),"selected_tool":first["selected_tool"],"canonical_tool":canonical,"tool_state":state,"tool_sha256":tool_bytes.as_ref().map(|b|format!("{:x}",Sha256::digest(b)))});
    let bytes = serde_json::to_vec(&payload).map_err(|_| "attempt_input_encoding_failed")?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
