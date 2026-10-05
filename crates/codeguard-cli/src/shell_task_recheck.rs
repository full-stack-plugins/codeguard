//! 同一Shell任务的原生复检；绑定方言与原rc，记录本轮事实但不授权关闭。
use crate::{shellcheck_config::ShellCheckConfig, shellcheck_probe, work_sync::shell_report};
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
const EXTRA: &[&str] = &[
    "task_id",
    "task_kind",
    "task_rule",
    "original_ref",
    "original_configuration",
    "suppression_status",
];
/// 读取首次绑定报告，不接受报告摘要漂移或跨任务上下文。
fn original(root: &Path, reference: &Value) -> Result<Value, &'static str> {
    let run = reference["run_id"]
        .as_str()
        .filter(|s| {
            s.starts_with("shellcheck-")
                && s.len() < 120
                && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
        .ok_or("shell_original_ref_invalid")?;
    if !reference
        .as_object()
        .is_some_and(|o| o.len() == 2 && o.contains_key("run_id") && o.contains_key("sha256"))
    {
        return Err("shell_original_ref_invalid");
    }
    let b = read_bounded_regular_file(
        &root.join(format!(".codeguard/reports/{run}.json")),
        128 * 1024,
    )
    .map_err(|_| "shell_original_report_unavailable")?;
    if reference["sha256"] != format!("{:x}", Sha256::digest(&b)) {
        return Err("shell_original_report_changed");
    }
    let r =
        codeguard_adapters::parse_unique_json(&b).map_err(|_| "shell_original_report_invalid")?;
    if r["report_type"] != "shellcheck_workbench_observation" || r["run_id"] != run {
        return Err("shell_original_report_invalid");
    }
    Ok(r)
}
/// 执行当前目标的原工具观察；原方言/显式rc来自首次任务报告，不能由参数换成其它规则。
pub(crate) fn run(
    root: &Path,
    brief: &Value,
    tool: Option<&Path>,
    deadline: Instant,
) -> Result<Value, &'static str> {
    let reference = json!({"run_id":brief["evidence_ref"]["first_run_id"],"sha256":brief["evidence_ref"]["first_report_sha256"]});
    let first = original(root, &reference)?;
    let target = first["path"]
        .as_str()
        .filter(|s| shell_report::safe_path(s))
        .ok_or("shell_target_invalid")?;
    if brief["scope"] != target {
        return Err("shell_task_scope_changed");
    }
    let path = root.join(target);
    if path.canonicalize().ok().as_deref() != Some(path.as_path()) {
        return Err("shell_target_alias_unverified");
    }
    let bytes =
        read_bounded_regular_file(&path, 1024 * 1024).map_err(|_| "shell_source_unavailable")?;
    if std::str::from_utf8(&bytes).is_err() {
        return Err("shell_source_invalid_encoding");
    }
    let requested = first["requested_config"].as_str().map(Path::new);
    let cfg = ShellCheckConfig::capture(&path, requested);
    let configuration = match &cfg {
        Ok(c) => c.report(),
        Err(reason) => {
            json!({"status":if *reason == "shellcheck_external_sources_unverified" {"unknown"} else {"invalid"},"source_path":null,"sha256":null,"reason":reason,"global_configuration":"not_loaded"})
        }
    };
    let selected = tool
        .map(Path::to_path_buf)
        .or_else(crate::shell_lint_command::discover_tool);
    let dialect = first["dialect"].as_str().ok_or("shell_dialect_invalid")?;
    let native = match (&cfg, selected.as_deref()) {
        (Err(reason), _) => shellcheck_probe::unavailable(reason),
        (Ok(c), Some(t))
            if matches!(dialect, "sh" | "bash" | "dash" | "ksh" | "busybox")
                && crate::shell_lint_command::supported_shebang(&bytes)
                && !crate::shell_lint_command::unsupported_file(&path) =>
        {
            shellcheck_probe::observe(
                t,
                &path,
                &bytes,
                dialect,
                c,
                deadline,
                &AtomicBool::new(false),
            )
        }
        (_, None) => shellcheck_probe::unavailable("shellcheck_tool_not_found"),
        _ => shellcheck_probe::unavailable("shell_dialect_unsupported"),
    };
    let stable = path.canonicalize().ok().as_deref() == Some(path.as_path())
        && read_bounded_regular_file(&path, 1024 * 1024).is_ok_and(|b| b == bytes)
        && cfg.as_ref().is_ok_and(|c| c.current(&path));
    let run = format!(
        "shellcheck-{}-{}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos()),
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let suppression = if std::str::from_utf8(&bytes).unwrap().lines().any(|line| {
        line.trim_start().starts_with('#')
            && line.contains("shellcheck")
            && line.contains("disable")
    }) {
        "requires_review"
    } else {
        "no_disable_comment_observed"
    };
    Ok(
        json!({"schema_version":"0.1.0","report_type":"shellcheck_task_recheck","workspace_binding":"bound","workspace_id":first["workspace_id"],"run_id":run,"path":target,"source_sha256":format!("{:x}",Sha256::digest(&bytes)),"dialect":dialect,"authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated","input_stable":stable,"native":native,"project_configuration":configuration,"requested_config":first["requested_config"],"task_id":brief["task_id"],"task_kind":brief["kind"],"task_rule":brief["native_rule_id"],"original_ref":reference,"original_configuration":first["project_configuration"],"suppression_status":suppression}),
    )
}
/// 转换为原生观察，复用严格导入契约；不改变本轮run身份。
pub(crate) fn normal(r: &Value) -> Value {
    let mut v = r.clone();
    if let Some(o) = v.as_object_mut() {
        for k in EXTRA {
            o.remove(*k);
        }
        o.insert(
            "report_type".into(),
            json!("shellcheck_workbench_observation"),
        );
    }
    v
}
/// 绑定首次范围及稳定规则组；真正报告字段和坐标由工作台严格解析核验。
pub(crate) fn valid_binding(root: &Path, r: &Value) -> bool {
    let Ok(first) = original(root, &r["original_ref"]) else {
        return false;
    };
    let Some(path) = first["path"].as_str() else {
        return false;
    };
    let Some(dialect) = first["dialect"].as_str() else {
        return false;
    };
    let kind = r["task_kind"].as_str();
    let rule = if kind == Some("blocker") {
        Some("environment")
    } else if kind == Some("finding") {
        r["task_rule"].as_str()
    } else {
        None
    };
    let Some(rule) = rule else {
        return false;
    };
    if kind == Some("finding")
        && !first["native"]["diagnostics"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|d| d["rule_id"] == rule))
    {
        return false;
    }
    if kind == Some("blocker") && !r["task_rule"].is_null() {
        return false;
    }
    let fp = shell_report::fingerprint(path, dialect, rule);
    let id = format!(
        "CG-{}{}",
        if kind == Some("blocker") { "B-" } else { "" },
        &fp[..32]
    );
    r.as_object()
        .is_some_and(|o| o.len() == 21 && EXTRA.iter().all(|k| o.contains_key(*k)))
        && r["report_type"] == "shellcheck_task_recheck"
        && r["task_id"] == id
        && r["path"] == first["path"]
        && r["dialect"] == first["dialect"]
        && r["workspace_id"] == first["workspace_id"]
        && r["requested_config"] == first["requested_config"]
        && r["original_configuration"] == first["project_configuration"]
        && matches!(
            r["suppression_status"].as_str(),
            Some("requires_review" | "no_disable_comment_observed")
        )
}
/// 复检存储前复核当前源码和配置；失配不能推进历史状态。
pub(crate) fn inputs_current(root: &Path, r: &Value) -> bool {
    shell_report::current(root, &normal(r))
}
/// 将当前原规则事实分类；任何局部结果均不能签发项目关闭。
pub(crate) fn classify(brief: &Value, r: &Value) -> &'static str {
    if r["task_id"] != brief["task_id"]
        || r["input_stable"] != true
        || !matches!(
            r["native"]["status"].as_str(),
            Some("completed" | "diagnostics_observed")
        )
    {
        return if brief["kind"] == "blocker" {
            "still_blocked"
        } else {
            "incomplete"
        };
    }
    if brief["kind"] == "blocker" {
        return "environment_restored_unverified_policy";
    }
    if r["native"]["diagnostics"]
        .as_array()
        .is_some_and(|a| a.iter().any(|d| d["rule_id"] == brief["native_rule_id"]))
    {
        return "still_present";
    }
    if r["project_configuration"] != r["original_configuration"] {
        return "rule_coverage_requires_review";
    }
    if r["suppression_status"] == "requires_review" {
        "suppression_requires_review"
    } else {
        "candidate_absent_unverified_policy"
    }
}
/// 核验完整封套和其可导入原生事实；不当作执行认证。
pub(crate) fn valid_shape(root: &Path, r: &Value) -> bool {
    shell_report::valid_recheck(root, r)
}
