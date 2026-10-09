//! ShellCheck原规则的宿主限定修复闭环；局部收据不签发项目许可。
use crate::task_lifecycle_store::digest;
use crate::{
    ShellTaskResolutionRequest, doctor_scratch::DoctorScratch,
    shell_task_resolution_policy_input::ShellTaskResolutionPolicyInput,
    shellcheck_config::ShellCheckConfig,
};
use codeguard_core::{ResolutionCause, ResolutionEvidence};
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

/// 验签原规则、方言与配置，复检原样本及当前源码并维护同一任务生命周期。
/// 参数来自独立宿主；返回限定收据，配置放宽或原生反证不能认定修复。
pub fn verify_shell_task_resolution(
    request: &ShellTaskResolutionRequest<'_>,
) -> Result<Value, &'static str> {
    check_deadline(request.deadline)?;
    let started = Instant::now();
    let approved = crate::verify_signed_approval(
        request.envelope_bytes,
        request.policy_bytes,
        request.trust,
        request.context,
    )?;
    let remaining = approved
        .expires_at()
        .checked_sub(
            request
                .context
                .now_unix
                .ok_or("approval_clock_unavailable")?,
        )
        .ok_or("approval_lifetime_invalid")?;
    let end = request.deadline.min(
        started
            .checked_add(Duration::from_secs(remaining))
            .ok_or("approval_lifetime_invalid")?,
    );
    let raw = codeguard_adapters::parse_unique_json(request.policy_bytes)
        .map_err(|_| "task_resolution_policy_invalid")?;
    if !raw.get("grammar_sha256").is_some_and(Value::is_null) {
        return Err("task_resolution_policy_invalid");
    }
    let p: ShellTaskResolutionPolicyInput =
        serde_json::from_value(raw).map_err(|_| "task_resolution_policy_invalid")?;
    if p.schema_version != "1.10.0"
        || p.report_type != "task_resolution_policy"
        || p.policy_revision != approved.policy_revision()
        || p.identity.workspace_id != request.context.workspace_id
        || p.identity.task_id != request.task_id
        || p.identity.checker_id != "shell.shellcheck"
        || p.native_version != "0.11.0"
        || p.grammar_sha256.is_some()
        || !crate::work_sync::shell_report::safe_path(&p.identity.scope)
        || request.original_source.len() > 1024 * 1024
        || std::str::from_utf8(request.original_source).is_err()
        || digest(request.original_source) != p.original_source_sha256
    {
        return Err("task_resolution_policy_scope_mismatch");
    }
    let root = request
        .root
        .canonicalize()
        .map_err(|_| "workspace_unreadable")?;
    let brief = crate::next_command::read_task_brief(&root, request.task_id)?;
    let reference = json!({"run_id":brief["evidence_ref"]["first_run_id"],"sha256":brief["evidence_ref"]["first_report_sha256"]});
    let first = crate::shell_task_recheck::original(&root, &reference)?;
    if brief["kind"] != "finding"
        || brief["checker_id"] != p.identity.checker_id
        || brief["scope"] != p.identity.scope
        || brief["native_rule_id"] != p.native_rule_id
        || reference["sha256"] != p.original_report_sha256
        || first["workspace_id"] != p.identity.workspace_id
        || first["path"] != p.identity.scope
        || first["source_sha256"] != p.original_source_sha256
        || first["dialect"] != p.dialect
        || first["project_configuration"] != p.project_configuration
        || first["native"]["tool_sha256"] != p.tool_sha256
        || !crate::shell_resolution_evidence::contains_rule(&first["native"], &p.native_rule_id)
    {
        return Err("task_resolution_original_identity_mismatch");
    }
    let source = root.join(&p.identity.scope);
    if source.canonicalize().ok().as_deref() != Some(source.as_path()) {
        return Err("shell_target_alias_unverified");
    }
    let config =
        ShellCheckConfig::capture(&source, first["requested_config"].as_str().map(Path::new))?;
    if config.report() != p.project_configuration {
        return Err("task_resolution_configuration_mismatch");
    }
    let tool = request
        .tool
        .canonicalize()
        .map_err(|_| "task_resolution_tool_unavailable")?;
    if !request.tool.is_absolute() || hash(&tool, 128 * 1024 * 1024)? != p.tool_sha256 {
        return Err("task_resolution_tool_mismatch");
    }
    let adapter = std::env::current_exe().map_err(|_| "task_resolution_adapter_unavailable")?;
    if hash(&adapter, 256 * 1024 * 1024)? != p.adapter_sha256 {
        return Err("task_resolution_adapter_mismatch");
    }
    crate::task_lifecycle_store::load(&root, &p.identity, &p.original_report_sha256)?;
    let lease = crate::task_lease_command::begin_verification(
        &root,
        request.task_id,
        request.borrowed_lease,
    )?;
    let result = (|| {
        let attempt = crate::task_attempt_command::latest_ready_attempt(&root, request.task_id)?;
        let current = crate::plain_syntax_source::read_plain_source(&source)?;
        let scratch =
            DoctorScratch::create("shell-original").ok_or("shellcheck_scratch_unavailable")?;
        let original_path = scratch.path().join("original.sh");
        std::fs::write(&original_path, request.original_source)
            .map_err(|_| "shellcheck_original_snapshot_unavailable")?;
        let original_config = ShellCheckConfig::capture(
            &original_path,
            p.project_configuration["source_path"]
                .as_str()
                .map(Path::new),
        )?;
        if original_config.report() != p.project_configuration {
            return Err("task_resolution_configuration_mismatch");
        }
        let original_native = crate::shellcheck_probe::observe(
            &tool,
            &original_path,
            request.original_source,
            &p.dialect,
            &original_config,
            end,
            &AtomicBool::new(false),
        );
        check_deadline(end)?;
        let scan = crate::shell_task_recheck::run(&root, &brief, Some(&tool), end)?;
        let current_native = scan["native"].clone();
        let _lock = crate::task_lease_command::lock_verification(&root, request.task_id, &lease)?;
        check_deadline(end)?;
        if crate::task_attempt_command::latest_ready_attempt(&root, request.task_id)? != attempt {
            return Err("attempt_changed_during_verification");
        }
        let stable = crate::plain_syntax_source::read_plain_source(&source)
            .is_ok_and(|b| b == current)
            && config.current(&source)
            && hash(&tool, 128 * 1024 * 1024)? == p.tool_sha256
            && hash(&adapter, 256 * 1024 * 1024)? == p.adapter_sha256
            && crate::shell_task_recheck::inputs_current(&root, &scan);
        crate::task_verify_command::persist_observation(
            &root,
            &brief,
            &scan,
            crate::shell_task_recheck::classify(&brief, &scan),
            attempt.as_deref(),
        )?;
        check_deadline(end)?;
        let stable = stable
            && config.current(&source)
            && crate::plain_syntax_source::read_plain_source(&source).is_ok_and(|b| b == current)
            && hash(&tool, 128 * 1024 * 1024)? == p.tool_sha256
            && hash(&adapter, 256 * 1024 * 1024)? == p.adapter_sha256;
        let complete = crate::shell_resolution_evidence::completed(&original_native)
            && crate::shell_resolution_evidence::completed(&current_native)
            && [&original_native, &current_native]
                .iter()
                .all(|n| n["tool_sha256"] == p.tool_sha256);
        let original_issue =
            crate::shell_resolution_evidence::contains_rule(&original_native, &p.native_rule_id);
        let present =
            crate::shell_resolution_evidence::contains_rule(&current_native, &p.native_rule_id);
        let original_disables = disable_directives(request.original_source);
        let current_disables = disable_directives(&current);
        let suppression = original_disables != current_disables
            || original_disables
                .iter()
                .any(|directive| applies_to_rule(directive, &p.native_rule_id));
        let outcome = if !stable {
            "inputs_stale"
        } else if !complete {
            "native_incomplete"
        } else if present {
            "still_present"
        } else if suppression {
            "suppression_requires_review"
        } else if !original_issue {
            "false_positive_review_required"
        } else {
            "code_fixed"
        };
        let pair = json!({"original":original_native,"current":current_native});
        let policy_sha = approved.snapshot_sha256();
        let evidence = ResolutionEvidence {
            identity: p.identity.clone(),
            original_source_sha256: p.original_source_sha256.clone(),
            current_source_sha256: digest(&current),
            native_report_sha256: digest(
                &serde_json::to_vec(&pair).map_err(|_| "task_resolution_encoding_failed")?,
            ),
            tool_sha256: p.tool_sha256.clone(),
            adapter_sha256: p.adapter_sha256.clone(),
            rulepack_sha256: policy_sha.into(),
            policy_sha256: policy_sha.into(),
            policy_revision: p.policy_revision.clone(),
            native_completed: complete,
            original_rule_checked: original_issue,
            target_covered: true,
            inputs_current: stable,
            policy_verified: true,
            issue_still_present: present,
            suppression_changed: suppression,
            target_removed: false,
            cause: ResolutionCause::CodeFixed,
        };
        let raw = json!({"schema_version":"0.11.0","report_type":"task_resolution_evidence","identity":p.identity,"original_report_sha256":p.original_report_sha256,"original_source_sha256":p.original_source_sha256,"current_source_sha256":digest(&current),"grammar_sha256":null,"tool_sha256":p.tool_sha256,"adapter_sha256":p.adapter_sha256,"policy_sha256":policy_sha,"policy_revision":p.policy_revision,"original_native":original_native,"current_native":current_native,"outcome":outcome,"native_rule_id":p.native_rule_id,"dialect":p.dialect,"project_configuration":p.project_configuration});
        crate::task_resolution_service::commit_resolution(
            &root,
            request.task_id,
            &p.original_report_sha256,
            &evidence,
            raw,
            outcome,
        )
    })();
    let release = crate::task_lease_command::finish_verification(&root, request.task_id, &lease);
    match (result, release) {
        (_, Err(reason)) => Err(reason),
        (v, Ok(())) => v,
    }
}
fn hash(path: &Path, limit: u64) -> Result<String, &'static str> {
    read_bounded_regular_file(path, limit)
        .map(|b| digest(&b))
        .map_err(|_| "task_resolution_artifact_unavailable")
}
fn check_deadline(end: Instant) -> Result<(), &'static str> {
    if codeguard_runtime::sigint_cancellation_requested() {
        Err("request_cancelled")
    } else if Instant::now() >= end {
        Err("request_deadline_exceeded")
    } else {
        Ok(())
    }
}

/// 同工具同配置的原规则复发，沿既有父链重开；历史批准不授予新的关闭权限。
pub(crate) fn record_recurrence(
    root: &Path,
    brief: &Value,
    scan: &Value,
) -> Result<(), &'static str> {
    use codeguard_core::{
        TaskIdentity, TaskLifecycleEvent, TaskLifecycleKind, TaskLifecycleRecord,
        reduce_task_lifecycle,
    };
    if scan["input_stable"] != true
        || !crate::shell_task_recheck::valid_shape(root, scan)
        || !crate::shell_task_recheck::inputs_current(root, scan)
        || !crate::shell_resolution_evidence::completed(&scan["native"])
        || !crate::shell_resolution_evidence::contains_rule(
            &scan["native"],
            brief["native_rule_id"].as_str().unwrap_or(""),
        )
    {
        return Ok(());
    }
    let identity = TaskIdentity {
        workspace_id: scan["workspace_id"]
            .as_str()
            .ok_or("workspace_identity_unavailable")?
            .into(),
        task_id: brief["task_id"].as_str().ok_or("task_id_invalid")?.into(),
        checker_id: "shell.shellcheck".into(),
        scope: brief["scope"].as_str().ok_or("task_scope_invalid")?.into(),
    };
    let original_sha = brief["evidence_ref"]["first_report_sha256"]
        .as_str()
        .ok_or("task_original_report_missing")?;
    let records = crate::task_lifecycle_store::load(root, &identity, original_sha)?;
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
    let sha = previous
        .evidence_sha256
        .as_deref()
        .ok_or("task_lifecycle_evidence_missing")?;
    let bytes = read_bounded_regular_file(
        &root.join(format!(".codeguard/state/resolution_evidence/{sha}.json")),
        128 * 1024,
    )
    .map_err(|_| "task_lifecycle_evidence_missing")?;
    if digest(&bytes) != sha {
        return Err("task_lifecycle_evidence_changed");
    }
    let mut evidence = codeguard_adapters::parse_unique_json(&bytes)
        .map_err(|_| "task_lifecycle_evidence_invalid")?;
    if evidence["schema_version"] != "0.11.0"
        || scan["project_configuration"] != evidence["project_configuration"]
        || scan["dialect"] != evidence["dialect"]
        || scan["native"]["tool_sha256"] != evidence["tool_sha256"]
        || scan["native"]["version"] != "0.11.0"
        || scan["task_rule"] != evidence["native_rule_id"]
    {
        return Ok(());
    }
    let native = scan["native"].clone();
    evidence["current_native"] = native;
    evidence["current_source_sha256"] = scan["source_sha256"].clone();
    evidence["outcome"] = json!("still_present");
    let bytes =
        serde_json::to_vec_pretty(&evidence).map_err(|_| "task_lifecycle_encoding_failed")?;
    let evidence_sha = digest(&bytes);
    let mut record = TaskLifecycleRecord {
        schema_version: "0.1.0".into(),
        report_type: "task_lifecycle_record".into(),
        event: TaskLifecycleEvent {
            event_id: String::new(),
            parent_event_id: Some(previous.event.event_id.clone()),
            identity,
            kind: TaskLifecycleKind::Reopened,
        },
        original_report_sha256: original_sha.into(),
        evidence_sha256: Some(evidence_sha.clone()),
        policy_sha256: previous.policy_sha256.clone(),
    };
    if !crate::task_resolution_evidence_shape::valid(&record, &evidence) {
        return Err("task_lifecycle_evidence_binding_invalid");
    }
    let state = root.join(".codeguard/state");
    crate::work_sync::write_once(
        &state.join(format!("resolution_evidence/{evidence_sha}.json")),
        &bytes,
        &state,
    )?;
    crate::task_lifecycle_store::save(root, &mut record)?;
    Ok(())
}

// 比较原始与当前禁用指令，既有且未变的无关抑制不冒充本轮规则放宽。
fn disable_directives(bytes: &[u8]) -> Vec<&str> {
    std::str::from_utf8(bytes)
        .unwrap_or("")
        .lines()
        .filter_map(|line| {
            let rest = line
                .trim_start()
                .strip_prefix('#')?
                .trim_start()
                .strip_prefix("shellcheck")?;
            let rest = rest.trim_start().strip_prefix("disable")?.trim_start();
            rest.strip_prefix('=').map(str::trim)
        })
        .collect()
}

// 原规则的既有抑制也需复核：移动同一指令可改变作用域，不能仅比较文本后批准关闭。
fn applies_to_rule(directive: &str, rule: &str) -> bool {
    let code = rule.trim_start_matches("SC").parse::<u32>().ok();
    directive
        .split(|c: char| c == ',' || c.is_whitespace())
        .any(|item| {
            if item == "all" || item.trim_start_matches("SC").parse::<u32>().ok() == code {
                return true;
            }
            item.split_once('-').is_some_and(|(start, end)| {
                match (
                    start.trim_start_matches("SC").parse::<u32>(),
                    end.trim_start_matches("SC").parse::<u32>(),
                    code,
                ) {
                    (Ok(start), Ok(end), Some(code)) => start <= code && code <= end,
                    _ => false,
                }
            })
        })
}
#[cfg(test)]
mod tests {
    #[test]
    fn unchanged_unrelated_disable_does_not_hide_a_moved_target_suppression() {
        assert!(!super::applies_to_rule("SC2154", "SC2086"));
        for directive in ["SC2086", "2086", "all", "SC2080-SC2090"] {
            assert!(super::applies_to_rule(directive, "SC2086"));
        }
        assert_eq!(
            super::disable_directives(b"# shellcheck disable = SC2154\n"),
            vec!["SC2154"]
        );
        assert!(super::disable_directives(b"# shellcheck disables a rule\n").is_empty());
    }
}
