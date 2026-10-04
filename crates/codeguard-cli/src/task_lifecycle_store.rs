//! 生命周期追加存储与严格父链读取；独立于旧观察事件，不改写首次事实。
use codeguard_core::TaskLifecycleRecord;
use codeguard_core::{
    TaskIdentity, TaskLifecycleEvent, TaskLifecycleKind, TaskLifecycleState, reduce_task_lifecycle,
};
use codeguard_runtime::read_bounded_regular_file;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(crate) fn event_id(record: &TaskLifecycleRecord) -> Result<String, &'static str> {
    let mut payload = record.clone();
    payload.event.event_id.clear();
    Ok(format!(
        "event-{}",
        digest(&serde_json::to_vec(&payload).map_err(|_| "task_lifecycle_encoding_failed")?)
    ))
}
pub(crate) fn load(
    root: &Path,
    identity: &TaskIdentity,
    original_sha: &str,
) -> Result<Vec<TaskLifecycleRecord>, &'static str> {
    let directory = root.join(format!(".codeguard/findings/{}/events", identity.task_id));
    safe_directory(&directory)?;
    let mut records = Vec::new();
    let mut origin = None;
    let mut count = 0;
    for entry in fs::read_dir(&directory).map_err(|_| "task_lifecycle_history_unreadable")? {
        count += 1;
        if count > 10000 {
            return Err("task_lifecycle_directory_limit_exceeded");
        }
        let path = entry
            .map_err(|_| "task_lifecycle_history_unreadable")?
            .path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("task_lifecycle_filename_invalid")?;
        if !name.starts_with("lifecycle-") {
            continue;
        }
        if records.len() >= 1000 {
            return Err("task_lifecycle_event_limit_exceeded");
        }
        let raw = read_bounded_regular_file(&path, 8192)
            .map_err(|_| "task_lifecycle_record_unreadable")?;
        let value = codeguard_adapters::parse_unique_json(&raw)
            .map_err(|_| "task_lifecycle_record_invalid")?;
        let record: TaskLifecycleRecord =
            serde_json::from_value(value).map_err(|_| "task_lifecycle_record_invalid")?;
        if record.schema_version != "0.1.0"
            || record.report_type != "task_lifecycle_record"
            || record.event.identity != *identity
            || record.original_report_sha256 != original_sha
            || record.event.event_id != event_id(&record)?
            || name != format!("lifecycle-{}.json", record.event.event_id)
        {
            return Err("task_lifecycle_record_binding_invalid");
        }
        match (&record.evidence_sha256, &record.policy_sha256) {
            (None, None)
                if matches!(record.event.kind, TaskLifecycleKind::Observed)
                    && record.event.parent_event_id.is_none() => {}
            (Some(sha), Some(policy)) if valid_sha(sha) && valid_sha(policy) => {
                let dir = root.join(".codeguard/state/resolution_evidence");
                safe_directory(&dir)?;
                let bytes = read_bounded_regular_file(&dir.join(format!("{sha}.json")), 128 * 1024)
                    .map_err(|_| "task_lifecycle_evidence_missing")?;
                if digest(&bytes) != *sha {
                    return Err("task_lifecycle_evidence_changed");
                }
                let value = codeguard_adapters::parse_unique_json(&bytes)
                    .map_err(|_| "task_lifecycle_evidence_invalid")?;
                let identity_value =
                    serde_json::to_value(identity).map_err(|_| "task_lifecycle_encoding_failed")?;
                if !matches!(
                    value["schema_version"].as_str(),
                    Some("0.1.0" | "0.2.0" | "0.3.0" | "0.4.0")
                ) || value["report_type"] != "task_resolution_evidence"
                    || value["identity"] != identity_value
                    || value["original_report_sha256"] != original_sha
                    || value["policy_sha256"] != *policy
                {
                    return Err("task_lifecycle_evidence_binding_invalid");
                }
                // 证据版本不能自行决定语言；核对首次事实与已同步的原报告，再重放历史。
                if origin.is_none() {
                    origin = Some(original_task_report(root, identity, original_sha)?);
                }
                if !origin
                    .as_ref()
                    .is_some_and(|original| origin_matches_evidence(original, &value))
                {
                    return Err("task_lifecycle_evidence_binding_invalid");
                }
                if !crate::task_resolution_evidence_shape::valid(&record, &value) {
                    return Err("task_lifecycle_evidence_binding_invalid");
                }
                if matches!(&record.event.kind,TaskLifecycleKind::Resolved{evidence_sha256,..}|TaskLifecycleKind::VerificationRequired{evidence_sha256,..} if evidence_sha256!=sha)
                {
                    return Err("task_lifecycle_evidence_binding_invalid");
                }
            }
            _ => return Err("task_lifecycle_record_binding_invalid"),
        }
        records.push(record);
    }
    if !records.is_empty() {
        let events: Vec<TaskLifecycleEvent> = records.iter().map(|r| r.event.clone()).collect();
        if reduce_task_lifecycle(identity, &events, &[]).state
            == TaskLifecycleState::ReconciliationRequired
        {
            return Err("task_lifecycle_reconciliation_required");
        }
    }
    Ok(records)
}
fn original_task_report(
    root: &Path,
    identity: &TaskIdentity,
    sha: &str,
) -> Result<serde_json::Value, &'static str> {
    let bytes = read_bounded_regular_file(
        &root.join(format!(
            ".codeguard/findings/{}/finding.json",
            identity.task_id
        )),
        1024 * 1024,
    )
    .map_err(|_| "task_lifecycle_evidence_binding_invalid")?;
    let fact = codeguard_adapters::parse_unique_json(&bytes)
        .map_err(|_| "task_lifecycle_evidence_binding_invalid")?;
    if fact["id"] != identity.task_id
        || fact["workspace_id"] != identity.workspace_id
        || fact["checker_id"] != identity.checker_id
        || fact["scope"] != identity.scope
        || fact["first_report_sha256"] != sha
    {
        return Err("task_lifecycle_evidence_binding_invalid");
    }
    let brief = serde_json::json!({"task_id":identity.task_id,"scope":identity.scope,"evidence_ref":{"first_run_id":fact["first_run_id"],"first_report_sha256":sha}});
    crate::syntax_task_recheck::original(root, &brief)
        .map_err(|_| "task_lifecycle_evidence_binding_invalid")
}
fn origin_matches_evidence(original: &serde_json::Value, evidence: &serde_json::Value) -> bool {
    if !matches!(
        (
            original["language"].as_str(),
            evidence["schema_version"].as_str()
        ),
        (Some("zig"), Some("0.1.0"))
            | (Some("erlang"), Some("0.2.0"))
            | (Some("swift"), Some("0.3.0"))
            | (Some("kotlin"), Some("0.4.0"))
    ) {
        return false;
    }
    if matches!(
        original["schema_version"].as_str(),
        Some("0.2.0" | "0.4.0" | "0.5.0")
    ) {
        evidence["grammar_sha256"].is_null()
            && evidence["original_source_sha256"]
                == original["native_evidence"]["target"]["source_sha256"]
            && evidence["tool_sha256"] == original["native_evidence"]["native"]["tool_sha256"]
    } else {
        evidence["grammar_sha256"] == original["observations"][0]["grammar_sha256"]
            && evidence["original_source_sha256"] == original["observations"][0]["source_sha256"]
    }
}
pub(crate) fn save(root: &Path, record: &mut TaskLifecycleRecord) -> Result<String, &'static str> {
    record.event.event_id = event_id(record)?;
    let reference = format!(
        ".codeguard/findings/{}/events/lifecycle-{}.json",
        record.event.identity.task_id, record.event.event_id
    );
    let path = root.join(&reference);
    safe_directory(path.parent().ok_or("task_lifecycle_path_invalid")?)?;
    crate::work_sync::write_once(
        &path,
        &serde_json::to_vec_pretty(record).map_err(|_| "task_lifecycle_encoding_failed")?,
        &root.join(".codeguard/state"),
    )?;
    Ok(reference)
}
pub(crate) fn safe_directory(path: &Path) -> Result<(), &'static str> {
    if !fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_dir())
        || path.canonicalize().ok().as_deref() != Some(path)
    {
        return Err("task_lifecycle_directory_invalid");
    }
    Ok(())
}
fn valid_sha(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// 为不持有宿主策略的本地读者提供历史提示；不把记录中的关闭声明升级为可信状态。
pub(crate) fn guidance(
    root: &Path,
    brief: &serde_json::Value,
    workspace_id: &str,
) -> Option<serde_json::Value> {
    let identity = TaskIdentity {
        workspace_id: workspace_id.into(),
        task_id: brief["task_id"].as_str()?.into(),
        checker_id: brief["checker_id"].as_str()?.into(),
        scope: brief["scope"].as_str()?.into(),
    };
    let original_sha = brief["evidence_ref"]["first_report_sha256"].as_str()?;
    let records = match load(root, &identity, original_sha) {
        Ok(records) if records.is_empty() => return None,
        Ok(records) => records,
        Err(reason) => {
            return Some(
                serde_json::json!({"disposition":"needs_decision","step":format!("任务生命周期历史需要核对（{reason}）；保留原问题与历史，用原生检查器重新获取证据，不选择时间最新的关闭文件") }),
            );
        }
    };
    let events = records.iter().map(|r| r.event.clone()).collect::<Vec<_>>();
    let view = reduce_task_lifecycle(&identity, &events, &[]);
    let tip = view
        .tip_event_id
        .as_ref()
        .and_then(|id| records.iter().find(|r| r.event.event_id == *id));
    if tip.is_some_and(|r| {
        matches!(
            r.event.kind,
            TaskLifecycleKind::Reopened | TaskLifecycleKind::Observed
        )
    }) {
        return None;
    }
    let step = match tip.map(|r| &r.event.kind) {
        Some(TaskLifecycleKind::Resolved { .. }) => {
            "历史记录了限定任务关闭；由受保护宿主核验当前策略并重跑原生检查，不继续重复旧源码修复；本地历史不能授权关闭或交付"
        }
        Some(TaskLifecycleKind::Reopened) => {
            "原生复发历史已重开同一任务；核对当前输入并运行原生复检，保留此前解决证据，不新建重复任务"
        }
        Some(TaskLifecycleKind::VerificationRequired { reason_code, .. })
            if reason_code == "false_positive_review_required" =>
        {
            let original = match original_task_report(root, &identity, original_sha) {
                Ok(original) => original,
                Err(reason) => {
                    return Some(serde_json::json!({"disposition":"needs_decision",
                        "step":format!("首次观察来源需要核对（{reason}）；保留原反例及历史，不推断误报原因或自行关闭任务")}));
                }
            };
            // 只从严格绑定的首次报告区分来源；原生任务没有可归咎的 grammar。
            if matches!(
                original["schema_version"].as_str(),
                Some("0.2.0" | "0.4.0" | "0.5.0")
            ) {
                "原样本的原生反证未检出语法诊断；核对首次原生诊断与反证运行的输入、工具及环境差异，保留误报调查证据并提交限定范围纠错请求，不继续修改已合法源码或自行白名单放行"
            } else {
                "原样本的原生反证未检出语法诊断；核对首次 WASM 观察、grammar 语言版本与原生对照差异，保留误报调查证据并提交限定范围纠错请求，不直接认定 grammar 缺陷或自行白名单放行"
            }
        }
        _ => {
            "生命周期已有原生观察；先核对当前输入与原工具证据，不沿用旧 WASM 疑似位置重复修复或重新安装工具"
        }
    };
    Some(serde_json::json!({"disposition":"verification_required","step":step}))
}

/// 普通原生复检发现复发时追加保守重开；旧批准摘要仅作历史引用，不能批准新关闭。
/// 调用方必须持有既有任务租约锁，扫描输入必须当前且归属于同一语法任务。
pub(crate) fn record_native_recurrence(
    root: &Path,
    brief: &serde_json::Value,
    scan: &serde_json::Value,
) -> Result<(), &'static str> {
    if brief["checker_id"] != "syntax.native_confirmation"
        || scan["input_stable"] != true
        || !(scan["native"]["status"] == "diagnostics_observed"
            || (scan["target"]["language"] == "kotlin"
                && crate::task_resolution_evidence_shape::kotlin_syntax_present(&scan["native"])))
    {
        return Ok(());
    }
    if !crate::syntax_task_recheck::inputs_current(root, scan) {
        return Err("task_lifecycle_recurrence_inputs_stale");
    }
    let identity = TaskIdentity {
        workspace_id: scan["workspace_id"]
            .as_str()
            .ok_or("workspace_identity_unavailable")?
            .into(),
        task_id: brief["task_id"].as_str().ok_or("task_id_invalid")?.into(),
        checker_id: "syntax.native_confirmation".into(),
        scope: brief["scope"].as_str().ok_or("task_scope_invalid")?.into(),
    };
    let original_sha = brief["evidence_ref"]["first_report_sha256"]
        .as_str()
        .ok_or("task_original_report_missing")?;
    let records = load(root, &identity, original_sha)?;
    let events = records.iter().map(|r| r.event.clone()).collect::<Vec<_>>();
    let tip = reduce_task_lifecycle(&identity, &events, &[]).tip_event_id;
    let Some(previous) = tip
        .as_ref()
        .and_then(|id| records.iter().find(|r| r.event.event_id == *id))
    else {
        return Ok(());
    };
    if !matches!(previous.event.kind, TaskLifecycleKind::Resolved { .. }) {
        return Ok(());
    }
    let previous_sha = previous
        .evidence_sha256
        .as_deref()
        .ok_or("task_lifecycle_evidence_missing")?;
    let path = root.join(format!(
        ".codeguard/state/resolution_evidence/{previous_sha}.json"
    ));
    let raw = read_bounded_regular_file(&path, 128 * 1024)
        .map_err(|_| "task_lifecycle_evidence_missing")?;
    let mut evidence = codeguard_adapters::parse_unique_json(&raw)
        .map_err(|_| "task_lifecycle_evidence_invalid")?;
    // 更换原生制品或原语法资产只要求重新核验；不把另一个工具的输出归为原任务复发。
    if scan["native"]["tool_sha256"] != evidence["tool_sha256"]
        || scan["native"]["version"] != evidence["original_native"]["version"]
        || !matches!(
            (
                evidence["schema_version"].as_str(),
                scan["target"]["language"].as_str()
            ),
            (Some("0.1.0"), Some("zig"))
                | (Some("0.2.0"), Some("erlang"))
                | (Some("0.3.0"), Some("swift"))
                | (Some("0.4.0"), Some("kotlin"))
        )
        || scan["original_report"]["sha256"] != original_sha
        || scan["original_report"]["grammar_sha256"] != evidence["grammar_sha256"]
    {
        return Ok(());
    }
    evidence["current_source_sha256"] = scan["target"]["source_sha256"].clone();
    evidence["current_native"] = scan["native"].clone();
    evidence["outcome"] = serde_json::json!("still_present");
    let bytes =
        serde_json::to_vec_pretty(&evidence).map_err(|_| "task_lifecycle_encoding_failed")?;
    let evidence_sha = digest(&bytes);
    let state = root.join(".codeguard/state");
    crate::work_sync::write_once(
        &state.join(format!("resolution_evidence/{evidence_sha}.json")),
        &bytes,
        &state,
    )?;
    let mut reopened = TaskLifecycleRecord {
        schema_version: "0.1.0".into(),
        report_type: "task_lifecycle_record".into(),
        event: TaskLifecycleEvent {
            event_id: String::new(),
            parent_event_id: Some(previous.event.event_id.clone()),
            identity,
            kind: TaskLifecycleKind::Reopened,
        },
        original_report_sha256: original_sha.into(),
        evidence_sha256: Some(evidence_sha),
        policy_sha256: previous.policy_sha256.clone(),
    };
    save(root, &mut reopened)?;
    Ok(())
}
