//! Python 候选语法观察到既有工作台的稳定原生确认任务。

#[cfg(feature = "wasm-precheck")]
use crate::work_sync::{save_local_report, sync_local_workspace};
#[cfg(feature = "wasm-precheck")]
use crate::workspace_refresh::read_workspace_baseline;
use codeguard_adapters::bundled_grammar_candidates;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Component, Path};
#[cfg(feature = "wasm-precheck")]
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const MAX_SOURCE_BYTES: u64 = 1024 * 1024;
const MAX_OBSERVATIONS: usize = 128;

/// 将一个已检查的 Python 文件关联到稳定的原生语法确认任务。
/// 参数为规范化工作区和本轮候选观察；返回真实落盘任务 ID，写入或同步失败则保留具体原因。
#[cfg(feature = "wasm-precheck")]
pub(crate) fn persist(
    root: &Path,
    precheck: &Value,
    deadline: Instant,
) -> Result<String, &'static str> {
    if Instant::now() >= deadline || codeguard_runtime::sigint_cancellation_requested() {
        return Err("request_deadline_or_cancelled");
    }
    let baseline = read_workspace_baseline(root).map_err(|_| "workspace_invalid")?;
    let workspace = baseline
        .as_ref()
        .and_then(|value| value.workspace_id())
        .ok_or("workspace_not_initialized")?;
    if precheck["selected_files"] != 1 || precheck["checked_files"] != 1 {
        return Err("confirmation_scope_not_single_file");
    }
    let checked = precheck["checked"]
        .as_array()
        .filter(|items| items.len() == 1)
        .and_then(|items| items.first())
        .ok_or("checked_source_missing")?;
    let scope = checked["path"]
        .as_str()
        .filter(|value| safe_python_path(value))
        .ok_or("checked_source_invalid")?;
    let source = root.join(scope);
    if source.canonicalize().ok().as_deref() != Some(source.as_path()) {
        return Err("checked_source_changed");
    }
    let bytes = read_bounded_regular_file(&source, MAX_SOURCE_BYTES)
        .map_err(|_| "checked_source_unavailable")?;
    let source_sha = format!("{:x}", Sha256::digest(&bytes));
    if checked["source_sha256"] != source_sha {
        return Err("checked_source_changed");
    }
    let grammar_sha = bundled_grammar_candidates()
        .map_err(|_| "grammar_manifest_unavailable")?
        .assets
        .into_iter()
        .find(|asset| asset.language == "python")
        .map(|asset| asset.sha256)
        .ok_or("python_grammar_unavailable")?;
    if checked["grammar_sha256"] != grammar_sha || precheck["grammar_sha256"] != grammar_sha {
        return Err("grammar_identity_changed");
    }
    let observations = precheck["observations"]
        .as_array()
        .filter(|items| items.len() <= MAX_OBSERVATIONS)
        .ok_or("syntax_observations_invalid")?;
    if !observations
        .iter()
        .all(|row| row["path"] == scope && row["source_sha256"] == source_sha)
    {
        return Err("syntax_observations_invalid");
    }
    let fingerprint = fingerprint(workspace, scope);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    let report = json!({
        "schema_version":"0.1.0",
        "report_type":"python_syntax_confirmation_observation",
        "workspace_binding":"bound",
        "workspace_id":workspace,
        "run_id":format!("python-syntax-{}-{nanos}",std::process::id()),
        "checker_id":"python.ruff",
        "authority":"local_unverified",
        "coverage_proven":false,
        "delivery_decision":"not_evaluated",
        "execution":"incomplete",
        "reason_code":"python_syntax_confirmation_needed",
        "blocker_id":format!("CG-B-{}",&fingerprint[..32]),
        "fingerprint":fingerprint,
        "build_root":".",
        "scope":scope,
        "affected_paths":[scope],
        "source_sha256":source_sha,
        "grammar_sha256":grammar_sha,
        "observations":observations,
    });
    if !valid_report(root, workspace, &report) {
        return Err("syntax_confirmation_report_invalid");
    }
    if Instant::now() >= deadline || codeguard_runtime::sigint_cancellation_requested() {
        return Err("request_deadline_or_cancelled");
    }
    save_local_report(root, &report)?;
    let summary = sync_local_workspace(root)?;
    if summary.failed_reports != 0 {
        return Err("syntax_confirmation_sync_incomplete");
    }
    let id = report["blocker_id"].as_str().ok_or("blocker_id_invalid")?;
    if !root
        .join(".codeguard/tasks")
        .join(format!("{id}.md"))
        .is_file()
    {
        return Err("syntax_confirmation_task_missing");
    }
    Ok(id.into())
}

/// 校验本地候选报告身份、源码快照和有界位置；仅允许生成待确认任务。
pub(crate) fn valid_report(root: &Path, workspace: &str, report: &Value) -> bool {
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
        "reason_code",
        "blocker_id",
        "fingerprint",
        "build_root",
        "scope",
        "affected_paths",
        "source_sha256",
        "grammar_sha256",
        "observations",
    ];
    let Some(scope) = report["scope"]
        .as_str()
        .filter(|scope| safe_python_path(scope))
    else {
        return false;
    };
    let fingerprint = fingerprint(workspace, scope);
    let grammar = bundled_grammar_candidates().ok().and_then(|manifest| {
        manifest
            .assets
            .into_iter()
            .find(|asset| asset.language == "python")
    });
    let source = root.join(scope);
    let bytes = if source.canonicalize().ok().as_deref() == Some(source.as_path()) {
        read_bounded_regular_file(&source, MAX_SOURCE_BYTES).ok()
    } else {
        None
    };
    let source_sha = bytes
        .as_ref()
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)));
    let observations = report["observations"].as_array();
    report
        .as_object()
        .is_some_and(|map| map.len() == keys.len() && keys.iter().all(|key| map.contains_key(*key)))
        && report["schema_version"] == "0.1.0"
        && report["report_type"] == "python_syntax_confirmation_observation"
        && report["workspace_binding"] == "bound"
        && report["workspace_id"] == workspace
        && report["run_id"].as_str().is_some_and(|run| {
            run.strip_prefix("python-syntax-")
                .and_then(|suffix| suffix.split_once('-'))
                .is_some_and(|(pid, nanos)| {
                    pid.parse::<u32>().is_ok_and(|value| value > 0)
                        && nanos.parse::<u128>().is_ok_and(|value| value > 0)
                })
        })
        && report["checker_id"] == "python.ruff"
        && report["authority"] == "local_unverified"
        && report["coverage_proven"] == false
        && report["delivery_decision"] == "not_evaluated"
        && report["execution"] == "incomplete"
        && report["reason_code"] == "python_syntax_confirmation_needed"
        && report["blocker_id"] == format!("CG-B-{}", &fingerprint[..32])
        && report["fingerprint"] == fingerprint
        && report["build_root"] == "."
        && report["affected_paths"] == json!([scope])
        && source_sha
            .as_deref()
            .is_some_and(|sha| report["source_sha256"] == sha)
        && grammar
            .as_ref()
            .is_some_and(|asset| report["grammar_sha256"] == asset.sha256)
        && observations.is_some_and(|rows| {
            rows.len() <= MAX_OBSERVATIONS
                && rows
                    .iter()
                    .all(|row| valid_observation(row, scope, bytes.as_deref().unwrap_or_default()))
        })
}

pub(crate) fn fingerprint(workspace: &str, scope: &str) -> String {
    let mut hasher = Sha256::new();
    for part in [
        "codeguard-python-syntax-confirmation-v1",
        workspace,
        "python.ruff",
        scope,
    ] {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn safe_python_path(value: &str) -> bool {
    let path = Path::new(value);
    !value.is_empty()
        && value.len() <= 512
        && !value.contains('\\')
        && !value.chars().any(char::is_control)
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
        && path
            .extension()
            .is_some_and(|ext| matches!(ext.to_str(), Some("py" | "pyw")))
}

fn valid_observation(row: &Value, scope: &str, source: &[u8]) -> bool {
    let Some(text) = std::str::from_utf8(source).ok() else {
        return false;
    };
    let lines: Vec<&str> = text.split('\n').collect();
    let position = |line_key: &str, column_key: &str| -> Option<(usize, usize)> {
        let line = usize::try_from(row[line_key].as_u64()?).ok()?;
        let column = usize::try_from(row[column_key].as_u64()?).ok()?;
        let content = lines.get(line.checked_sub(1)?)?.trim_end_matches('\r');
        (column >= 1 && column <= content.chars().count() + 1).then_some((line, column))
    };
    let (Some(start), Some(end)) = (
        position("start_line", "start_column"),
        position("end_line", "end_column"),
    ) else {
        return false;
    };
    row.as_object().is_some_and(|map| map.len() == 10)
        && row["path"] == scope
        && row["source_sha256"] == format!("{:x}", Sha256::digest(source))
        && row["classification"] == "suspected"
        && matches!(row["kind"].as_str(), Some("ERROR" | "MISSING"))
        && row["syntax_kind"].as_str().is_some_and(|kind| {
            !kind.is_empty() && kind.len() <= 120 && !kind.chars().any(char::is_control)
        })
        && row["group_id"].as_u64().is_some()
        && start <= end
}
