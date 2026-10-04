//! 受保护宿主的真实原生关闭/重开处理器；项目历史不提供策略权威。
use crate::{
    ErlangTaskResolutionRequest, ZigTaskResolutionRequest,
    syntax_task_resolution_request::SyntaxTaskResolutionRequest,
    task_lifecycle_store::{digest, load, safe_directory, save},
    task_resolution_checker::TaskResolutionChecker,
    task_resolution_policy_input::TaskResolutionPolicyInput,
};
use codeguard_core::TaskLifecycleRecord;
use codeguard_core::{
    ResolutionCause, ResolutionEvidence, ResolutionOutcome, TaskLifecycleEvent, TaskLifecycleKind,
    TaskLifecycleState, evaluate_resolution, reduce_task_lifecycle,
};
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use std::{path::Path, time::Instant};

/// 验签并限定原任务后，用同一原生工具对原样本与当前字节复检，追加证据与父链事件。
/// 返回宿主当前上下文中的限定任务状态；任何结果都不签发项目 allow。
/// 宿主必须独立保证密钥/上下文来源；本 API 不接受项目自选的公钥作为信任依据。
pub fn verify_zig_task_resolution(
    request: &ZigTaskResolutionRequest<'_>,
) -> Result<Value, &'static str> {
    verify_task_resolution(request, TaskResolutionChecker::Zig)
}

/// 验签 OTP 28 的限定语法任务，复用原样本/当前样本复检和关闭/复发父链。
/// 参数须由受保护宿主提供可信上下文；返回限定任务收据，不签发项目 allow。
pub fn verify_erlang_task_resolution(
    request: &ErlangTaskResolutionRequest<'_>,
) -> Result<Value, &'static str> {
    verify_task_resolution(request, TaskResolutionChecker::Erlang)
}

/// 验证宿主批准的 Swift 语法修复；返回限定任务收据，不签发项目许可。
/// 参数包括固定工具、原反例和可信上下文，返回值可能要求继续复检。
pub fn verify_swift_task_resolution(
    request: &crate::SwiftTaskResolutionRequest<'_>,
) -> Result<Value, &'static str> {
    verify_task_resolution(request, TaskResolutionChecker::Swift)
}

fn verify_task_resolution(
    request: &SyntaxTaskResolutionRequest<'_>,
    checker: TaskResolutionChecker,
) -> Result<Value, &'static str> {
    let started = Instant::now();
    before_deadline(request.deadline)?;
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
    let expiration = started
        .checked_add(std::time::Duration::from_secs(remaining))
        .ok_or("approval_lifetime_invalid")?;
    let bounded = SyntaxTaskResolutionRequest {
        deadline: request.deadline.min(expiration),
        ..*request
    };
    let request = &bounded;
    let raw = codeguard_adapters::parse_unique_json(request.policy_bytes)
        .map_err(|_| "task_resolution_policy_invalid")?;
    if raw.get("grammar_sha256").is_none() {
        return Err("task_resolution_policy_invalid");
    }
    let policy: TaskResolutionPolicyInput =
        serde_json::from_value(raw).map_err(|_| "task_resolution_policy_invalid")?;
    if policy.schema_version != checker.policy_version()
        || policy.report_type != "task_resolution_policy"
        || policy.policy_revision != approved.policy_revision()
        || policy.identity.workspace_id != request.context.workspace_id
        || policy.identity.task_id != request.task_id
        || policy.identity.checker_id != "syntax.native_confirmation"
        || policy.native_rule_id != checker.rule()
        || policy.native_version != checker.version()
        || request.original_source.len() > 1024 * 1024
        || std::str::from_utf8(request.original_source).is_err()
        || digest(request.original_source) != policy.original_source_sha256
    {
        return Err("task_resolution_policy_scope_mismatch");
    }
    let root = request
        .root
        .canonicalize()
        .map_err(|_| "workspace_unreadable")?;
    safe_directory(&root.join(".codeguard/state"))?;
    let brief = crate::next_command::read_task_brief(&root, request.task_id)?;
    if brief["checker_id"] != policy.identity.checker_id
        || brief["scope"] != policy.identity.scope
        || brief["evidence_ref"]["first_report_sha256"] != policy.original_report_sha256
    {
        return Err("task_resolution_original_identity_mismatch");
    }
    let original = crate::syntax_task_recheck::original(&root, &brief)?;
    if original["workspace_id"] != policy.identity.workspace_id
        || original["language"] != checker.language()
        || !original_binding_matches(&original, &policy, checker)
    {
        return Err("task_resolution_original_identity_mismatch");
    }
    let tool = request
        .tool
        .canonicalize()
        .map_err(|_| "task_resolution_tool_unavailable")?;
    if !request.tool.is_absolute() || tool_hash(&tool)? != policy.tool_sha256 {
        return Err("task_resolution_tool_mismatch");
    }
    let executable = std::env::current_exe()
        .map_err(|_| "task_resolution_adapter_unavailable")?
        .canonicalize()
        .map_err(|_| "task_resolution_adapter_unavailable")?;
    if artifact_hash(&executable)? != policy.adapter_sha256 {
        return Err("task_resolution_adapter_mismatch");
    }
    // 在进程启动前先识别已有历史冲突；不能靠追加一份新关闭遮蔽分叉或缺失证据。
    load(&root, &policy.identity, &policy.original_report_sha256)?;
    before_deadline(request.deadline)?;
    let lease = crate::task_lease_command::begin_verification(
        &root,
        request.task_id,
        request.borrowed_lease,
    )?;
    let result = execute(
        (request, checker),
        &root,
        &tool,
        &executable,
        &policy,
        approved.snapshot_sha256(),
        &lease,
    );
    let release = crate::task_lease_command::finish_verification(&root, request.task_id, &lease);
    match (result, release) {
        (_, Err(reason)) => Err(reason),
        (value, Ok(())) => value,
    }
}
fn original_binding_matches(
    original: &Value,
    policy: &TaskResolutionPolicyInput,
    checker: TaskResolutionChecker,
) -> bool {
    if matches!(original["schema_version"].as_str(), Some("0.2.0" | "0.5.0")) {
        matches!(checker.language(), "erlang" | "swift")
            && policy.grammar_sha256.is_none()
            && original["native_evidence"]["target"]["source_sha256"]
                == policy.original_source_sha256
            && original["native_evidence"]["native"]["tool_sha256"] == policy.tool_sha256
            && original["native_evidence"]["native"]["version"] == policy.native_version
    } else {
        policy.grammar_sha256.as_ref().is_some_and(|grammar| {
            original["observations"][0]["source_sha256"] == policy.original_source_sha256
                && original["observations"][0]["grammar_sha256"] == *grammar
        })
    }
}
fn execute(
    input: (&SyntaxTaskResolutionRequest<'_>, TaskResolutionChecker),
    root: &Path,
    tool: &Path,
    executable: &Path,
    policy: &TaskResolutionPolicyInput,
    policy_sha: &str,
    lease: &crate::task_lease_command::VerificationLease,
) -> Result<Value, &'static str> {
    let (request, checker) = input;
    let bound_attempt = crate::task_attempt_command::latest_ready_attempt(root, request.task_id)?;
    let brief = crate::next_command::read_task_brief(root, request.task_id)?;
    let source_path = root.join(&policy.identity.scope);
    let current = crate::plain_syntax_source::read_plain_source(&source_path)?;
    let original_native = checker.observe(tool, request.original_source, root, request.deadline);
    before_deadline(request.deadline)?;
    let current_report = checker.recheck(root, &brief, tool, request.deadline)?;
    let current_native = current_report["native"].clone();
    let _guard = crate::task_lease_command::lock_verification(root, request.task_id, lease)?;
    before_deadline(request.deadline)?;
    if crate::task_attempt_command::latest_ready_attempt(root, request.task_id)? != bound_attempt {
        return Err("attempt_changed_during_verification");
    }
    let inputs_current = crate::plain_syntax_source::read_plain_source(&source_path)
        .is_ok_and(|b| b == current)
        && tool_hash(tool).is_ok_and(|s| s == policy.tool_sha256)
        && artifact_hash(executable).is_ok_and(|s| s == policy.adapter_sha256);
    let brief = crate::next_command::read_task_brief(root, request.task_id)?;
    let original = crate::syntax_task_recheck::original(root, &brief)?;
    if brief["evidence_ref"]["first_report_sha256"] != policy.original_report_sha256
        || original["workspace_id"] != policy.identity.workspace_id
    {
        return Err("task_resolution_original_identity_mismatch");
    }
    crate::task_verify_command::persist_observation(
        root,
        &brief,
        &current_report,
        crate::syntax_task_recheck::classify(&current_report),
        bound_attempt.as_deref(),
    )?;
    before_deadline(request.deadline)?;
    let native_bound = [&original_native, &current_native]
        .iter()
        .all(|r| r["version"] == policy.native_version && r["tool_sha256"] == policy.tool_sha256);
    let current_completed = matches!(
        current_native["status"].as_str(),
        Some("completed" | "diagnostics_observed")
    );
    let original_completed = matches!(
        original_native["status"].as_str(),
        Some("completed" | "diagnostics_observed")
    ) && original_native["diagnostics"]
        .as_array()
        .is_some_and(|rows| {
            rows.iter().all(|row| {
                let line = row["line"].as_u64().unwrap_or(0) as usize;
                let column = row["column"].as_u64().unwrap_or(0) as usize;
                line > 0
                    && column > 0
                    && request
                        .original_source
                        .split(|b| *b == b'\n')
                        .nth(line - 1)
                        .is_some_and(|bytes| column <= bytes.len() + 1)
            })
        });
    let mut outcome = if !inputs_current {
        "inputs_stale"
    } else if !native_bound || !current_completed || !original_completed {
        "native_incomplete"
    } else if current_native["status"] == "diagnostics_observed" {
        "still_present"
    } else if original_native["status"] == "completed" {
        "false_positive_review_required"
    } else {
        "code_fixed"
    };
    let native_sha = digest(
        &serde_json::to_vec(&json!({"original":original_native,"current":current_native}))
            .map_err(|_| "task_resolution_encoding_failed")?,
    );
    let evidence = ResolutionEvidence {
        identity: policy.identity.clone(),
        original_source_sha256: policy.original_source_sha256.clone(),
        current_source_sha256: digest(&current),
        native_report_sha256: native_sha,
        tool_sha256: policy.tool_sha256.clone(),
        adapter_sha256: policy.adapter_sha256.clone(),
        // 原生首次任务的规则身份来自已批准策略字节；不用伪造 grammar 摘要填充证据。
        rulepack_sha256: policy
            .grammar_sha256
            .clone()
            .unwrap_or_else(|| policy_sha.into()),
        policy_sha256: policy_sha.into(),
        policy_revision: policy.policy_revision.clone(),
        native_completed: current_completed && original_completed && native_bound,
        original_rule_checked: original_native["status"] == "diagnostics_observed",
        target_covered: true,
        inputs_current,
        policy_verified: true,
        issue_still_present: current_native["status"] == "diagnostics_observed",
        suppression_changed: false,
        target_removed: false,
        cause: ResolutionCause::CodeFixed,
    };
    if outcome == "code_fixed"
        && evaluate_resolution(&policy.identity, &evidence)
            != ResolutionOutcome::Resolved(ResolutionCause::CodeFixed)
    {
        outcome = "resolution_evidence_incomplete";
    }
    let mut records = load(root, &policy.identity, &policy.original_report_sha256)?;
    let events: Vec<_> = records.iter().map(|r| r.event.clone()).collect();
    let tip = reduce_task_lifecycle(&policy.identity, &events, &[]).tip_event_id;
    let previous = tip
        .as_ref()
        .and_then(|id| records.iter().find(|r| r.event.event_id == *id));
    // 同样输入重检可确认已记录的关闭；输入变更不能追加第二份无父级协调的关闭。
    if outcome == "code_fixed"
        && previous.is_some_and(|r| matches!(r.event.kind, TaskLifecycleKind::Resolved { .. }))
        && previous.is_some_and(|r| r.policy_sha256.as_deref() != Some(policy_sha))
    {
        outcome = "resolution_history_binding_changed";
    }
    let raw_evidence=serde_json::to_vec_pretty(&json!({"schema_version":checker.evidence_version(),"report_type":"task_resolution_evidence","identity":policy.identity,"original_report_sha256":policy.original_report_sha256,"original_source_sha256":policy.original_source_sha256,"current_source_sha256":digest(&current),"grammar_sha256":policy.grammar_sha256,"tool_sha256":policy.tool_sha256,"adapter_sha256":policy.adapter_sha256,"policy_sha256":policy_sha,"policy_revision":policy.policy_revision,"original_native":original_native,"current_native":current_native,"outcome":outcome})).map_err(|_|"task_resolution_encoding_failed")?;
    let evidence_sha = digest(&raw_evidence);
    let same = previous.filter(|r| r.evidence_sha256.as_deref() == Some(&evidence_sha));
    if outcome == "code_fixed"
        && previous.is_some_and(|r| matches!(r.event.kind, TaskLifecycleKind::Resolved { .. }))
        && same.is_none()
    {
        return Err("task_lifecycle_reconciliation_required");
    }
    let evidence_ref = format!(".codeguard/state/resolution_evidence/{evidence_sha}.json");
    let dir = root.join(".codeguard/state/resolution_evidence");
    crate::task_lease_command::ensure_directory(&dir)?;
    safe_directory(&dir)?;
    crate::work_sync::write_once(
        &root.join(&evidence_ref),
        &raw_evidence,
        &root.join(".codeguard/state"),
    )?;
    let record = if let Some(same) = same {
        same.clone()
    } else {
        let kind = match outcome {
            "code_fixed" => TaskLifecycleKind::Resolved {
                cause: ResolutionCause::CodeFixed,
                evidence_sha256: evidence_sha.clone(),
            },
            "still_present"
                if previous.is_some_and(|r| {
                    matches!(r.event.kind, TaskLifecycleKind::Resolved { .. })
                }) =>
            {
                TaskLifecycleKind::Reopened
            }
            "still_present" => TaskLifecycleKind::Observed,
            _ => TaskLifecycleKind::VerificationRequired {
                evidence_sha256: evidence_sha.clone(),
                reason_code: outcome.into(),
            },
        };
        if records.is_empty() {
            let mut initial = TaskLifecycleRecord {
                schema_version: "0.1.0".into(),
                report_type: "task_lifecycle_record".into(),
                event: TaskLifecycleEvent {
                    event_id: String::new(),
                    parent_event_id: None,
                    identity: policy.identity.clone(),
                    kind: TaskLifecycleKind::Observed,
                },
                original_report_sha256: policy.original_report_sha256.clone(),
                evidence_sha256: None,
                policy_sha256: None,
            };
            save(root, &mut initial)?;
            records.push(initial);
        }
        let parent = reduce_task_lifecycle(
            &policy.identity,
            &records.iter().map(|r| r.event.clone()).collect::<Vec<_>>(),
            &[],
        )
        .tip_event_id
        .ok_or("task_lifecycle_reconciliation_required")?;
        let mut next = TaskLifecycleRecord {
            schema_version: "0.1.0".into(),
            report_type: "task_lifecycle_record".into(),
            event: TaskLifecycleEvent {
                event_id: String::new(),
                parent_event_id: Some(parent),
                identity: policy.identity.clone(),
                kind,
            },
            original_report_sha256: policy.original_report_sha256.clone(),
            evidence_sha256: Some(evidence_sha.clone()),
            policy_sha256: Some(policy_sha.into()),
        };
        next.event.event_id = crate::task_lifecycle_store::event_id(&next)?;
        let mut proposed = records.iter().map(|r| r.event.clone()).collect::<Vec<_>>();
        proposed.push(next.event.clone());
        if reduce_task_lifecycle(&policy.identity, &proposed, &[]).state
            == TaskLifecycleState::ReconciliationRequired
        {
            return Err("task_lifecycle_reconciliation_required");
        }
        save(root, &mut next)?;
        records.push(next.clone());
        next
    };
    let verified = if outcome == "code_fixed" {
        vec![record.event.event_id.clone()]
    } else {
        Vec::new()
    };
    let view = reduce_task_lifecycle(
        &policy.identity,
        &records.iter().map(|r| r.event.clone()).collect::<Vec<_>>(),
        &verified,
    );
    Ok(
        json!({"schema_version":"0.1.0","report_type":"task_resolution_receipt","identity":policy.identity,"state":view.state,"outcome":outcome,"authority":"host_context_verified","policy_sha256":policy_sha,"policy_revision":policy.policy_revision,"event_ref":format!(".codeguard/findings/{}/events/lifecycle-{}.json",request.task_id,record.event.event_id),"evidence_ref":evidence_ref,"evidence_sha256":evidence_sha,"delivery_decision":"not_evaluated"}),
    )
}
fn before_deadline(deadline: Instant) -> Result<(), &'static str> {
    if codeguard_runtime::sigint_cancellation_requested() {
        Err("request_cancelled")
    } else if Instant::now() >= deadline {
        Err("request_deadline_exceeded")
    } else {
        Ok(())
    }
}
fn tool_hash(path: &Path) -> Result<String, &'static str> {
    read_bounded_regular_file(path, 64 * 1024 * 1024)
        .map(|b| digest(&b))
        .map_err(|_| "task_resolution_tool_unavailable")
}
fn artifact_hash(path: &Path) -> Result<String, &'static str> {
    read_bounded_regular_file(path, 256 * 1024 * 1024)
        .map(|b| digest(&b))
        .map_err(|_| "task_resolution_adapter_unavailable")
}
