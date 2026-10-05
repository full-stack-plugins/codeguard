//! Python候选确认的单文件原生复检绑定；不签发任务关闭或项目覆盖。
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Component, Path};

/// 核对首次已消费报告，返回不可由当前源码替代的引用。
/// 参数为工作区和既有任务摘要；返回首次运行、摘要、源码及grammar身份。
pub(crate) fn original_reference(root: &Path, brief: &Value) -> Result<Value, &'static str> {
    let scope = brief["scope"]
        .as_str()
        .filter(|s| safe_path(s))
        .ok_or("python_confirmation_scope_invalid")?;
    let run = brief["evidence_ref"]["first_run_id"]
        .as_str()
        .filter(|run| {
            (run.starts_with("python-syntax-") || run.starts_with("syntax-confirm-"))
                && run.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                && run.len() <= 96
        })
        .ok_or("python_confirmation_original_invalid")?;
    let bytes = read_bounded_regular_file(
        &root.join(".codeguard/reports").join(format!("{run}.json")),
        1024 * 1024,
    )
    .map_err(|_| "python_confirmation_original_unavailable")?;
    let sha = digest(&bytes);
    if brief["evidence_ref"]["first_report_sha256"] != sha {
        return Err("python_confirmation_original_changed");
    }
    let report = codeguard_adapters::parse_unique_json(&bytes)
        .map_err(|_| "python_confirmation_original_invalid")?;
    let workspace = crate::workspace_refresh::read_workspace_baseline(root)
        .ok()
        .flatten()
        .and_then(|b| b.workspace_id().map(str::to_owned))
        .ok_or("workspace_invalid")?;
    let fingerprint = crate::python_syntax_confirmation::fingerprint(&workspace, scope);
    let dedicated = report["report_type"] == "python_syntax_confirmation_observation"
        && matches!(report["schema_version"].as_str(), Some("0.1.0" | "0.2.0"))
        && run.starts_with("python-syntax-");
    let generic = report["report_type"] == "syntax_confirmation_observation"
        && report["language"] == "python"
        && matches!(report["schema_version"].as_str(), Some("0.1.0" | "0.7.0"))
        && run.starts_with("syntax-confirm-");
    let source_sha = if dedicated {
        &report["source_sha256"]
    } else {
        &report["observations"][0]["source_sha256"]
    };
    let grammar_sha = if dedicated {
        &report["grammar_sha256"]
    } else {
        &report["observations"][0]["grammar_sha256"]
    };
    let grammar_matches = codeguard_adapters::bundled_grammar_candidate("python")
        .ok()
        .is_some_and(|(asset, _)| grammar_sha == &asset.sha256);
    if !(dedicated || generic)
        || !grammar_matches
        || !source_sha.as_str().is_some_and(valid_sha)
        || report["workspace_id"] != workspace
        || report["workspace_binding"] != "bound"
        || report["run_id"] != run
        || report["checker_id"] != "python.ruff"
        || report["reason_code"] != "python_syntax_confirmation_needed"
        || report["scope"] != scope
        || report["affected_paths"] != json!([scope])
        || report["fingerprint"] != fingerprint
        || report["blocker_id"] != brief["task_id"]
        || brief["task_id"] != format!("CG-B-{}", &fingerprint[..32])
        || brief["checker_id"] != "python.ruff"
        || brief["kind"] != "blocker"
        || brief["reason_code"] != "python_syntax_confirmation_needed"
        || report["authority"] != "local_unverified"
        || report["coverage_proven"] != false
        || report["delivery_decision"] != "not_evaluated"
    {
        return Err("python_confirmation_original_invalid");
    }
    // 首次导入时已校验原字节的坐标。这里核对原报告与消费收据，不能用修复后的源码重演旧坐标。
    let receipt = read_bounded_regular_file(
        &root
            .join(".codeguard/state/consumed")
            .join(format!("{run}.json")),
        4096,
    )
    .map_err(|_| "python_confirmation_original_not_synced")?;
    let expected = serde_json::to_vec_pretty(&json!({"schema_version":"0.1.0","workspace_id":workspace,"run_id":run,"report_sha256":sha}))
        .map_err(|_| "python_confirmation_original_invalid")?;
    if receipt != expected {
        return Err("python_confirmation_original_not_synced");
    }
    Ok(
        json!({"run_id":run,"report_sha256":sha,"source_sha256":source_sha,"grammar_sha256":grammar_sha}),
    )
}

/// 校验单文件任务报告的范围和首次引用；返回值不证明原生或批准权威。
pub(crate) fn valid_binding(root: &Path, report: &Value) -> bool {
    let binding = &report["task_binding"];
    let Some(id) = binding["task_id"].as_str().filter(|id| {
        id.len() == 37
            && id.starts_with("CG-B-")
            && id[5..]
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    }) else {
        return false;
    };
    // 从事实文件读取首次引用，不能递归读取包含当前复检历史的任务投影。
    let Some(fact) = read_bounded_regular_file(
        &root
            .join(".codeguard/findings")
            .join(id)
            .join("finding.json"),
        128 * 1024,
    )
    .ok()
    .and_then(|b| codeguard_adapters::parse_unique_json(&b).ok()) else {
        return false;
    };
    let brief = json!({"task_id":id,"kind":fact["kind"],"checker_id":fact["checker_id"],
        "scope":fact["scope"],"reason_code":fact["reason_code"],
        "evidence_ref":{"first_run_id":fact["first_run_id"],"first_report_sha256":fact["first_report_sha256"]}});
    let Ok(original) = original_reference(root, &brief) else {
        return false;
    };
    let Some(scope) = brief["scope"].as_str() else {
        return false;
    };
    let keys = [
        "schema_version",
        "report_type",
        "scope",
        "operation",
        "language",
        "run_id",
        "workspace_binding",
        "workspace_id",
        "checker_configurations",
        "files",
        "incomplete_reasons",
        "local_scan_complete",
        "command_status",
        "exit_code",
        "tool_approval",
        "native_tool_version",
        "rulepack_approval",
        "adapter_sha256",
        "delivery_decision",
        "task_scope",
        "task_binding",
        "task_input_stable",
    ];
    report
        .as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|key| o.contains_key(*key)))
        && matches!(report["schema_version"].as_str(), Some("0.18.0" | "0.19.0"))
        && report["report_type"] == "python_lint_feedback"
        && report["scope"] == "local_native_scan_only"
        && report["operation"] == "lint"
        && report["language"] == "python"
        && report["workspace_binding"] == "bound"
        && report["command_status"] == "incomplete"
        && report["exit_code"] == 3
        && report["delivery_decision"] == "not_evaluated"
        && report["tool_approval"] == "unverified"
        && report["rulepack_approval"] == "unverified"
        && report["workspace_id"] == fact["workspace_id"]
        && binding.as_object().is_some_and(|o| o.len() == 4)
        && binding["path"] == scope
        && binding["original_report"] == original
        && binding["source_sha256"].as_str().is_some_and(valid_sha)
        && report["task_scope"] == "single_python_confirmation_file"
        && report["task_input_stable"].is_boolean()
        && report["files"].as_array().is_some_and(|files| {
            files.len() == 1
                && files[0]["path"] == scope
                && (files[0]["source_sha256"].is_null()
                    || files[0]["source_sha256"] == binding["source_sha256"])
        })
}

/// 复核当前源码及原生完整文件的配置绑定，防止记录前输入变化。
pub(crate) fn inputs_current(root: &Path, report: &Value) -> bool {
    let binding = &report["task_binding"];
    let Some(path) = binding["path"].as_str().filter(|s| safe_path(s)) else {
        return false;
    };
    let file = root.join(path);
    if file.canonicalize().ok().as_deref() != Some(file.as_path()) {
        return false;
    }
    if !read_bounded_regular_file(&file, 1024 * 1024)
        .ok()
        .is_some_and(|bytes| binding["source_sha256"] == digest(&bytes))
    {
        return false;
    }
    report["files"].as_array().is_some_and(|files| {
        files.len() == 1
            && files[0]["path"] == path
            && (files[0]["run_status"] == "incomplete"
                || files[0]["configuration_ref"]
                    .as_str()
                    .zip(files[0]["config_sha256"].as_str())
                    .is_some_and(|(config, sha)| {
                        crate::ruff_verification_configuration::is_current(root, path, config, sha)
                    }))
    })
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn valid_sha(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}
fn safe_path(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 512
        && !s.contains('\\')
        && !s.chars().any(char::is_control)
        && Path::new(s)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        && Path::new(s)
            .extension()
            .is_some_and(|e| matches!(e.to_str(), Some("py" | "pyw")))
}
