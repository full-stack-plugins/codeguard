//! C/C++原生文档任务的固定工具复检；本地观察不能签发关闭。
use crate::work_sync::c_family_comments_report as contract;
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
];

/// 核验首次报告及消费收据；参数为工作区和精确run/摘要引用，返回原始局部观察。
pub(crate) fn original(root: &Path, reference: &Value) -> Result<Value, &'static str> {
    let run = reference["run_id"]
        .as_str()
        .filter(|s| {
            s.starts_with("clangdoc-")
                && s.len() <= 120
                && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
        .ok_or("clang_documentation_origin_invalid")?;
    if !reference
        .as_object()
        .is_some_and(|o| o.len() == 2 && o.contains_key("sha256"))
    {
        return Err("clang_documentation_origin_invalid");
    }
    let bytes = read_bounded_regular_file(
        &root.join(format!(".codeguard/reports/{run}.json")),
        128 * 1024,
    )
    .map_err(|_| "clang_documentation_origin_unavailable")?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if reference["sha256"] != digest {
        return Err("clang_documentation_origin_changed");
    }
    let r = codeguard_adapters::parse_unique_json(&bytes)
        .map_err(|_| "clang_documentation_origin_invalid")?;
    if !contract::valid_shape(&r) || r["run_id"] != run {
        return Err("clang_documentation_origin_invalid");
    }
    let marker = serde_json::to_vec_pretty(&json!({"schema_version":"0.1.0","workspace_id":r["workspace_id"],"run_id":run,"report_sha256":digest})).map_err(|_| "clang_documentation_marker_invalid")?;
    if read_bounded_regular_file(
        &root.join(format!(".codeguard/state/consumed/{run}.json")),
        4096,
    )
    .map_err(|_| "clang_documentation_marker_invalid")?
        != marker
    {
        return Err("clang_documentation_marker_invalid");
    }
    Ok(r)
}

fn reference(brief: &Value) -> Value {
    json!({"run_id":brief["evidence_ref"]["first_run_id"],"sha256":brief["evidence_ref"]["first_report_sha256"]})
}
fn tool_current(r: &Value) -> bool {
    r["selected_tool"]
        .as_str()
        .and_then(|s| Path::new(s).canonicalize().ok())
        .and_then(|p| read_bounded_regular_file(&p, 256 * 1024 * 1024).ok())
        .is_some_and(|b| r["native"]["tool_sha256"] == format!("{:x}", Sha256::digest(b)))
}

/// 租约与进程启动前核对原工具；不允许参数替换路径或首次已核对制品。
pub(crate) fn preflight(
    root: &Path,
    brief: &Value,
    tool: Option<&Path>,
) -> Result<(), &'static str> {
    let first = original(root, &reference(brief))?;
    if tool.is_some_and(|p| Some(p) != first["selected_tool"].as_str().map(Path::new)) {
        return Err("clang_documentation_original_tool_required");
    }
    if !first["native"]["tool_sha256"].is_null() && !tool_current(&first) {
        return Err("clang_documentation_original_tool_changed");
    }
    Ok(())
}

/// 冻结当前任务源码，沿首次标准与工具执行原生文档检查；返回封闭局部复检观察。
pub(crate) fn run(root: &Path, brief: &Value, deadline: Instant) -> Result<Value, &'static str> {
    preflight(root, brief, None)?;
    let first = original(root, &reference(brief))?;
    let path = first["path"]
        .as_str()
        .ok_or("clang_documentation_target_invalid")?;
    let snapshot =
        SourceSnapshot::capture(root, [PathBuf::from(path)], 1, 1024 * 1024, 1024 * 1024)
            .map_err(|_| "clang_documentation_source_unavailable")?;
    let bytes = snapshot
        .files()
        .get(&PathBuf::from(path))
        .ok_or("clang_documentation_source_unavailable")?;
    let native = crate::clang_syntax_probe::observe_documentation(
        Path::new(
            first["selected_tool"]
                .as_str()
                .ok_or("clang_documentation_tool_invalid")?,
        ),
        first["language"]
            .as_str()
            .ok_or("clang_documentation_language_invalid")?,
        first["standard"]
            .as_str()
            .ok_or("clang_documentation_standard_invalid")?,
        bytes,
        deadline,
        &AtomicBool::new(false),
    );
    let mut r = first.clone();
    r["report_type"] = json!("clang_documentation_task_recheck");
    r["run_id"] = json!(format!(
        "clangdoc-{}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    r["source_sha256"] = json!(format!("{:x}", Sha256::digest(bytes)));
    r["local_scan_complete"] = json!(
        matches!(
            native["status"].as_str(),
            Some("completed" | "diagnostics_observed")
        ) && !native["diagnostics"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|d| d["level"] == "error")
    );
    r["native"] = native;
    r["task_id"] = brief["task_id"].clone();
    r["task_kind"] = brief["kind"].clone();
    r["task_rule"] = brief["native_rule_id"].clone();
    r["original_ref"] = reference(brief);
    r["input_stable"] = json!(
        contract::current(root, &normal(&r))
            && (first["native"]["tool_sha256"].is_null() || tool_current(&first))
    );
    if !valid_shape(root, &r) {
        return Err("clang_documentation_recheck_invalid");
    }
    Ok(r)
}

/// 去掉任务字段以复用观察形状；不改变run或输入身份。
pub(crate) fn normal(r: &Value) -> Value {
    let mut v = r.clone();
    if let Some(o) = v.as_object_mut() {
        for k in EXTRA {
            o.remove(*k);
        }
        o.insert(
            "report_type".into(),
            json!("clang_documentation_workbench_observation"),
        );
    }
    v
}
/// 核验任务、首次报告、语言与原规则绑定；不依赖可编辑任务Markdown。
pub(crate) fn valid_shape(root: &Path, r: &Value) -> bool {
    if !r
        .as_object()
        .is_some_and(|o| o.len() == 21 && EXTRA.iter().all(|k| o.contains_key(*k)))
        || r["report_type"] != "clang_documentation_task_recheck"
        || !r["input_stable"].is_boolean()
        || !contract::valid_shape(&normal(r))
    {
        return false;
    }
    let Ok(first) = original(root, &r["original_ref"]) else {
        return false;
    };
    let rule = match r["task_kind"].as_str() {
        Some("blocker") if r["task_rule"].is_null() => "environment",
        Some("finding") => match r["task_rule"].as_str().filter(|rule| {
            codeguard_adapters::clang_documentation_guidance(rule).is_some()
                && first["local_scan_complete"] == true
                && first["native"]["diagnostics"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|d| d["rule_id"] == *rule)
        }) {
            Some(rule) => rule,
            None => return false,
        },
        _ => return false,
    };
    let id = format!(
        "CG-{}{}",
        if r["task_kind"] == "blocker" {
            "B-"
        } else {
            ""
        },
        &contract::fingerprint(&first, rule)[..32]
    );
    if r["task_id"] != id
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
    let Ok(b) = read_bounded_regular_file(
        &root.join(format!(".codeguard/findings/{id}/finding.json")),
        128 * 1024,
    ) else {
        return false;
    };
    let Ok(fact) = codeguard_adapters::parse_unique_json(&b) else {
        return false;
    };
    fact["id"] == id
        && fact["workspace_id"] == first["workspace_id"]
        && fact["state"] == "open"
        && fact["authority"] == "local_unverified"
        && fact["delivery_decision"] == "not_evaluated"
        && fact[if r["task_kind"] == "blocker" {
            "scope"
        } else {
            "path"
        }] == first["path"]
        && (r["task_kind"] == "blocker"
            || (fact["native_rule_id"] == r["task_rule"]
                && fact["first_source_sha256"] == first["source_sha256"]))
        && fact["kind"] == r["task_kind"]
        && fact["checker_id"] == contract::checker(&first)
        && fact["first_run_id"] == first["run_id"]
        && fact["first_report_sha256"] == r["original_ref"]["sha256"]
        && fact["fingerprint"] == contract::fingerprint(&first, rule)
}
/// 持久化前再次核对输入与首次制品；变化不记录完整复检。
pub(crate) fn inputs_current(root: &Path, r: &Value) -> bool {
    valid_shape(root, r)
        && contract::current(root, &normal(r))
        && original(root, &r["original_ref"])
            .is_ok_and(|first| first["native"]["tool_sha256"].is_null() || tool_current(&first))
        && (r["native"]["tool_sha256"].is_null() || tool_current(r))
}
/// 分类原任务复检；任何结果均保持本地非权威、任务未关闭。
pub(crate) fn classify(brief: &Value, r: &Value) -> &'static str {
    if r["task_id"] != brief["task_id"]
        || r["input_stable"] != true
        || r["local_scan_complete"] != true
    {
        return if brief["kind"] == "blocker" {
            "still_blocked"
        } else {
            "incomplete"
        };
    }
    if brief["kind"] == "blocker" {
        return if r["native"]["diagnostics"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|d| {
                d["rule_id"]
                    .as_str()
                    .is_some_and(|s| codeguard_adapters::clang_documentation_guidance(s).is_none())
            }) {
            "still_blocked"
        } else {
            "environment_restored_unverified_policy"
        };
    }
    if r["native"]["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|d| d["rule_id"] == brief["native_rule_id"])
    {
        "still_present"
    } else {
        "candidate_absent_unverified_policy"
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
    let payload = json!({"version":"clang-documentation-attempt-input-v1","scope":path,"language":first["language"],"standard":first["standard"],"profile":first["profile"],"source_sha256":format!("{:x}",Sha256::digest(bytes)),"selected_tool":first["selected_tool"],"canonical_tool":canonical,"tool_state":state,"tool_sha256":tool_bytes.as_ref().map(|b|format!("{:x}",Sha256::digest(b)))});
    let bytes = serde_json::to_vec(&payload).map_err(|_| "attempt_input_encoding_failed")?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
