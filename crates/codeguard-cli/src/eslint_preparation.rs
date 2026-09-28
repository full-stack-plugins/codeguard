//! ESLint 前置失败的脱敏环境任务；普通文件不具批准或关闭权威。
#[cfg(unix)]
use crate::{
    eslint_lint_arguments::EslintLintArguments, workspace_refresh::read_workspace_baseline,
};
#[cfg(unix)]
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
pub(crate) fn prepare(
    root: &Path,
    args: &EslintLintArguments,
    reason: &str,
) -> Result<Value, &'static str> {
    if !valid_reason(reason) {
        return Err("eslint_preparation_reason_invalid");
    }
    let baseline = read_workspace_baseline(root).map_err(|_| "workspace_invalid")?;
    let workspace = baseline
        .as_ref()
        .and_then(|b| b.workspace_id())
        .ok_or("workspace_not_initialized")?;
    let source = args
        .source
        .canonicalize()
        .map_err(|_| "eslint_source_unavailable")?;
    let relative = source
        .strip_prefix(root)
        .ok()
        .and_then(Path::to_str)
        .filter(|p| safe_path(p))
        .ok_or("source_outside_workspace")?;
    if !source
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| {
            matches!(
                e,
                "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts"
            )
        })
    {
        return Err("eslint_source_kind_not_supported");
    }
    let bytes = read_bounded_regular_file(&source, 16 * 1024 * 1024)
        .map_err(|_| "eslint_source_unavailable")?;
    // 按源码目标限定未核验上下文，不能把不同模块的前置失败合并并误称已恢复。
    let fingerprint = fingerprint(workspace, relative);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    Ok(
        json!({"schema_version":"0.1.0","report_type":"eslint_preparation_observation",
        "workspace_binding":"bound","workspace_id":workspace,"run_id":format!("eslint-preparation-{}-{nanos}",std::process::id()),
        "checker_id":"node.eslint.preparation","authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated",
        "execution":"incomplete","blocker_id":format!("CG-B-{}",&fingerprint[..32]),"fingerprint":fingerprint,
        "reason_code":"eslint_prerequisites_require_review","diagnostic_reason":reason,"build_root":".","scope":relative,
        "affected_paths":[relative],"source_sha256":format!("{:x}",Sha256::digest(bytes))}),
    )
}
pub(crate) fn fingerprint(workspace: &str, scope: &str) -> String {
    let mut hash = Sha256::new();
    for part in [
        "codeguard-eslint-preparation-v1",
        workspace,
        "node.eslint.preparation",
        scope,
    ] {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    format!("{:x}", hash.finalize())
}
pub(crate) fn valid_report(report: &Value, workspace: &str) -> bool {
    let keys = [
        "schema_version",
        "report_type",
        "workspace_binding",
        "workspace_id",
        "run_id",
        "checker_id",
        "authority",
        "coverage_proven",
        "delivery_decision",
        "execution",
        "blocker_id",
        "fingerprint",
        "reason_code",
        "diagnostic_reason",
        "build_root",
        "scope",
        "affected_paths",
        "source_sha256",
    ];
    let Some(scope) = report["scope"].as_str().filter(|s| safe_path(s)) else {
        return false;
    };
    let fp = fingerprint(workspace, scope);
    report
        .as_object()
        .is_some_and(|m| m.len() == keys.len() && keys.iter().all(|k| m.contains_key(*k)))
        && report["schema_version"] == "0.1.0"
        && report["report_type"] == "eslint_preparation_observation"
        && report["workspace_binding"] == "bound"
        && report["workspace_id"] == workspace
        && report["run_id"].as_str().is_some_and(|s| {
            s.strip_prefix("eslint-preparation-")
                .and_then(|s| s.split_once('-'))
                .is_some_and(|(pid, n)| {
                    pid.parse::<u32>().is_ok_and(|v| v > 0)
                        && n.parse::<u128>().is_ok_and(|v| v > 0)
                })
        })
        && report["checker_id"] == "node.eslint.preparation"
        && report["authority"] == "local_unverified"
        && report["coverage_proven"] == false
        && report["delivery_decision"] == "not_evaluated"
        && report["execution"] == "incomplete"
        && report["reason_code"] == "eslint_prerequisites_require_review"
        && report["build_root"] == "."
        && report["affected_paths"] == json!([scope])
        && report["fingerprint"] == fp
        && report["blocker_id"] == format!("CG-B-{}", &fp[..32])
        && report["diagnostic_reason"]
            .as_str()
            .is_some_and(valid_reason)
        && report["source_sha256"].as_str().is_some_and(|s| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
}
pub(crate) fn valid_reason(reason: &str) -> bool {
    matches!(
        reason,
        "eslint_execution_context_missing"
            | "eslint_input_unavailable"
            | "eslint_private_workspace_unavailable"
            | "request_deadline_exceeded"
            | "eslint_probe_request_invalid"
            | "eslint_command_invalid"
            | "eslint_input_identity_missing"
            | "eslint_input_identity_mismatch"
            | "eslint_input_changed"
            | "eslint_report_path_invalid"
            | "eslint_report_preflight_failed"
            | "eslint_version_evidence_failed"
            | "eslint_version_execution_incomplete"
            | "eslint_version_mismatch"
            | "eslint_execution_evidence_failed"
            | "eslint_report_read_failed"
            | "eslint_native_execution_incomplete"
            | "eslint_report_invalid"
            | "eslint_suppression_requires_review"
            | "eslint_execution_incomplete"
            | "eslint_exit_report_conflict"
            | "eslint_expected_scope_invalid"
            | "eslint_message_invalid"
            | "eslint_parser_or_configuration_diagnostic"
            | "eslint_report_counts_invalid"
            | "eslint_report_scope_mismatch"
            | "eslint_report_size_invalid"
            | "eslint_rule_location_invalid"
            | "eslint_unattributed_diagnostic"
            | "eslint_version_unverified"
    )
}
fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains(['\\', ':'])
        && path
            .split('/')
            .all(|s| !matches!(s, "" | "." | "..") && !s.chars().any(char::is_control))
}

#[cfg(unix)]
pub(crate) fn guidance(root: &Path, fact: &Value) -> Value {
    let fallback = json!({"disposition":"verification_required","step":"ESLint准备证据或当前范围无法核对；先复扫原工具，不修改无关源码"});
    let Some(scope) = fact["scope"].as_str() else {
        return fallback;
    };
    let workspace = fact["workspace_id"].as_str().unwrap_or("");
    let Ok(entries) = std::fs::read_dir(root.join(".codeguard/reports")) else {
        return fallback;
    };
    let mut latest = None;
    let mut ambiguous = false;
    for (count, entry) in entries.enumerate() {
        if count >= 1000 {
            return fallback;
        }
        let Ok(entry) = entry else {
            return fallback;
        };
        let Ok(bytes) = read_bounded_regular_file(&entry.path(), 16 * 1024 * 1024) else {
            return fallback;
        };
        let Ok(mut report) = serde_json::from_slice::<Value>(&bytes) else {
            return fallback;
        };
        if report["report_type"] == "eslint_task_recheck" {
            if crate::eslint_task_recheck::valid_shape(&report) && report["scan"].is_object() {
                report = report["scan"].clone();
            } else {
                report["source_path"] = report["target"]["path"].clone();
            }
        }
        let preparation = report["report_type"] == "eslint_preparation_observation";
        let scan = matches!(
            report["report_type"].as_str(),
            Some("eslint_workbench_observation" | "eslint_task_recheck")
        );
        if report["workspace_id"] != workspace
            || (!preparation && !scan)
            || (preparation && report["scope"] != scope)
            || (scan && report["source_path"] != scope)
        {
            continue;
        }
        let Some(sequence) = report["run_id"]
            .as_str()
            .and_then(|s| s.rsplit('-').next())
            .and_then(|s| s.parse::<u128>().ok())
        else {
            return fallback;
        };
        if latest.as_ref().is_none_or(|(old, _, _, _)| sequence > *old) {
            latest = Some((sequence, entry.path(), report, bytes));
            ambiguous = false;
        } else if latest
            .as_ref()
            .is_some_and(|(old, _, _, _)| sequence == *old)
        {
            ambiguous = true;
        }
    }
    if ambiguous {
        return fallback;
    }
    let Some((_, path, report, bytes)) = latest else {
        return fallback;
    };
    let Some(run) = report["run_id"].as_str() else {
        return fallback;
    };
    let sha = format!("{:x}", Sha256::digest(bytes));
    let receipt = read_bounded_regular_file(
        &root
            .join(".codeguard/state/consumed")
            .join(format!("{run}.json")),
        4096,
    )
    .ok()
    .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
    if !receipt.is_some_and(|r| {
        r["workspace_id"] == workspace && r["run_id"] == run && r["report_sha256"] == sha
    }) {
        return fallback;
    }
    if matches!(
        report["report_type"].as_str(),
        Some("eslint_workbench_observation" | "eslint_task_recheck")
    ) {
        if !crate::work_sync::validate_eslint_observation(
            root,
            workspace,
            &path,
            &report,
            sha.clone(),
        ) {
            return fallback;
        }
        return json!({"disposition":"verification_required","step":"同范围已有较新的原工具观察；核对当前结果与必需覆盖并正式复检，不再沿用旧环境诊断，不修改无关源码","run_id":run,"report_sha256":sha});
    }
    if !valid_report(&report, workspace) || report["blocker_id"] != fact["id"] {
        return fallback;
    }
    let source = root.join(scope);
    if source.canonicalize().ok().as_deref() != Some(source.as_path())
        || read_bounded_regular_file(&source, 16 * 1024 * 1024)
            .ok()
            .is_none_or(|b| report["source_sha256"] != format!("{:x}", Sha256::digest(b)))
    {
        return fallback;
    }
    json!({"disposition":"actionable","step":"核对最新ESLint前置诊断、原配置、parser/插件和运行条件；解析或抑制先复现根因，恢复原检查后复扫，不修改无关源码","diagnostic_reason":report["diagnostic_reason"],"run_id":run,"report_sha256":sha})
}
