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

/// 验签 Kotlin 限定语法任务，沿用同一原反例/当前字节对照与关闭父链。
/// 参数为宿主固定的工具和批准上下文，返回限定收据；上下文未完成不批准关闭。
pub fn verify_kotlin_task_resolution(
    request: &crate::KotlinTaskResolutionRequest<'_>,
) -> Result<Value, &'static str> {
    verify_task_resolution(request, TaskResolutionChecker::Kotlin)
}

/// 以签名策略绑定Go与同SDK gofmt，对原反例和当前源码复检并维护同一任务父链。
/// 参数来自受保护宿主；返回限定任务收据，原生反证和工具变化不能批准关闭。
pub fn verify_go_task_resolution(
    request: &crate::GoTaskResolutionRequest<'_>,
) -> Result<Value, &'static str> {
    verify_task_resolution(request, TaskResolutionChecker::Go)
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
    let go_policy = matches!(checker, TaskResolutionChecker::Go);
    let companion_fields = ["gofmt_sha256", "companion_binding_sha256"];
    if (go_policy
        && !companion_fields.iter().all(|key| {
            raw[*key].as_str().is_some_and(|sha| {
                sha.len() == 64
                    && sha
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
        }))
        || (!go_policy && companion_fields.iter().any(|key| raw.get(*key).is_some()))
    {
        return Err("task_resolution_policy_invalid");
    }
    let policy: TaskResolutionPolicyInput =
        serde_json::from_value(raw).map_err(|_| "task_resolution_policy_invalid")?;
    if !checker.accepts_policy_version(&policy.schema_version)
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
    if go_policy && !go_companion_matches(&tool, &policy) {
        return Err("task_resolution_companion_mismatch");
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
    if matches!(
        original["schema_version"].as_str(),
        Some("0.2.0" | "0.4.0" | "0.5.0" | "0.6.0")
    ) {
        (matches!(checker.language(), "erlang" | "swift" | "kotlin")
            || (checker.language() == "zig"
                && policy.schema_version == "1.4.0"
                && original["schema_version"] == "0.6.0"))
            && policy.grammar_sha256.is_none()
            && original["native_evidence"]["target"]["source_sha256"]
                == policy.original_source_sha256
            && original["native_evidence"]["native"]["tool_sha256"] == policy.tool_sha256
            && original["native_evidence"]["native"]["version"] == policy.native_version
    } else {
        policy.schema_version != "1.4.0"
            && policy.grammar_sha256.as_ref().is_some_and(|grammar| {
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
        && artifact_hash(executable).is_ok_and(|s| s == policy.adapter_sha256)
        && (checker.language() != "go" || go_companion_matches(tool, policy));
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
    let native_bound = [&original_native, &current_native].iter().all(|r| {
        r["version"] == policy.native_version
            && r["tool_sha256"] == policy.tool_sha256
            && (checker.language() != "go"
                || (r["gofmt_sha256"].as_str() == policy.gofmt_sha256.as_deref()
                    && r["companion_binding_sha256"].as_str()
                        == policy.companion_binding_sha256.as_deref()))
    });
    let current_completed = matches!(
        current_native["status"].as_str(),
        Some("completed" | "diagnostics_observed")
    );
    let current_completed = current_completed
        && (checker.language() != "go"
            || crate::go_syntax_probe::valid_observation(&current_native, Some(&current)));
    let original_completed = checker.original_completed(&original_native, request.original_source);
    let current_issue_present = current_native["status"] == "diagnostics_observed"
        || (checker.language() == "kotlin"
            && crate::task_resolution_evidence_shape::kotlin_syntax_present(&current_native));
    let mut outcome = if !inputs_current {
        "inputs_stale"
    } else if checker.language() == "kotlin" && native_bound && current_issue_present {
        // 语法正向证据不因另一个上下文阻塞而消失；完整性仍保留为 false。
        "still_present"
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
        issue_still_present: current_issue_present,
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
    let mut raw_evidence = json!({"schema_version":checker.evidence_version(&policy.schema_version),"report_type":"task_resolution_evidence","identity":policy.identity,"original_report_sha256":policy.original_report_sha256,"original_source_sha256":policy.original_source_sha256,"current_source_sha256":digest(&current),"grammar_sha256":policy.grammar_sha256,"tool_sha256":policy.tool_sha256,"adapter_sha256":policy.adapter_sha256,"policy_sha256":policy_sha,"policy_revision":policy.policy_revision,"original_native":original_native,"current_native":current_native,"outcome":outcome});
    if checker.language() == "go" {
        raw_evidence["gofmt_sha256"] = json!(policy.gofmt_sha256);
        raw_evidence["companion_binding_sha256"] = json!(policy.companion_binding_sha256);
    }
    commit_resolution(
        root,
        request.task_id,
        &policy.original_report_sha256,
        &evidence,
        raw_evidence,
        outcome,
    )
}

/// 提交限定任务的原生复检证据，保留幂等、父链冲突和复发语义。
/// 参数必须来自已验签且输入仍稳定的宿主复检服务；本函数不从本地文件取得批准。
/// 返回限定任务收据，不改变项目门禁。
pub(crate) fn commit_resolution(
    root: &Path,
    task_id: &str,
    original_report_sha256: &str,
    evidence: &ResolutionEvidence,
    mut raw_evidence: Value,
    mut outcome: &'static str,
) -> Result<Value, &'static str> {
    let mut keys = vec![
        "schema_version",
        "report_type",
        "identity",
        "original_report_sha256",
        "original_source_sha256",
        "current_source_sha256",
        "grammar_sha256",
        "tool_sha256",
        "adapter_sha256",
        "policy_sha256",
        "policy_revision",
        "original_native",
        "current_native",
        "outcome",
    ];
    if raw_evidence["schema_version"] == "0.7.0" {
        keys.extend(["gofmt_sha256", "companion_binding_sha256"]);
        if !crate::task_resolution_evidence_shape::valid_go_binding(&raw_evidence) {
            return Err("task_resolution_evidence_binding_invalid");
        }
    }
    if raw_evidence["schema_version"] == "0.6.0" {
        keys.extend([
            "target_version",
            "configuration_ref",
            "configuration_sha256",
        ]);
        if !crate::task_resolution_evidence_shape::valid_python_binding(&raw_evidence) {
            return Err("task_resolution_evidence_binding_invalid");
        }
    }
    if !raw_evidence.as_object().is_some_and(|object| {
        object.len() == keys.len() && keys.iter().all(|key| object.contains_key(*key))
    }) || !matches!(
        raw_evidence["schema_version"].as_str(),
        Some("0.1.0" | "0.2.0" | "0.3.0" | "0.4.0" | "0.5.0" | "0.6.0" | "0.7.0")
    ) {
        return Err("task_resolution_evidence_binding_invalid");
    }
    let policy_sha = evidence.policy_sha256.as_str();
    let identity =
        serde_json::to_value(&evidence.identity).map_err(|_| "task_resolution_encoding_failed")?;
    let native_sha = digest(
        &serde_json::to_vec(&json!({
            "original":raw_evidence["original_native"],"current":raw_evidence["current_native"]
        }))
        .map_err(|_| "task_resolution_encoding_failed")?,
    );
    let grammar_matches = if raw_evidence["grammar_sha256"].is_null() {
        evidence.rulepack_sha256 == evidence.policy_sha256
    } else {
        raw_evidence["grammar_sha256"] == evidence.rulepack_sha256
    };
    // 写入之前核对两种表示，不允许未来语言入口拼接另一份报告或批准身份。
    if task_id != evidence.identity.task_id
        || raw_evidence["report_type"] != "task_resolution_evidence"
        || raw_evidence["identity"] != identity
        || raw_evidence["original_report_sha256"] != original_report_sha256
        || raw_evidence["original_source_sha256"] != evidence.original_source_sha256
        || raw_evidence["current_source_sha256"] != evidence.current_source_sha256
        || raw_evidence["tool_sha256"] != evidence.tool_sha256
        || raw_evidence["adapter_sha256"] != evidence.adapter_sha256
        || raw_evidence["policy_sha256"] != evidence.policy_sha256
        || raw_evidence["policy_revision"] != evidence.policy_revision
        || raw_evidence["outcome"] != outcome
        || native_sha != evidence.native_report_sha256
        || !grammar_matches
    {
        return Err("task_resolution_evidence_binding_invalid");
    }
    if outcome == "code_fixed"
        && evaluate_resolution(&evidence.identity, evidence)
            != ResolutionOutcome::Resolved(ResolutionCause::CodeFixed)
    {
        outcome = "resolution_evidence_incomplete";
    }
    let mut records = load(root, &evidence.identity, original_report_sha256)?;
    let events: Vec<_> = records.iter().map(|r| r.event.clone()).collect();
    let tip = reduce_task_lifecycle(&evidence.identity, &events, &[]).tip_event_id;
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
    raw_evidence["outcome"] = json!(outcome);
    let raw_evidence =
        serde_json::to_vec_pretty(&raw_evidence).map_err(|_| "task_resolution_encoding_failed")?;
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
                    identity: evidence.identity.clone(),
                    kind: TaskLifecycleKind::Observed,
                },
                original_report_sha256: original_report_sha256.to_owned(),
                evidence_sha256: None,
                policy_sha256: None,
            };
            save(root, &mut initial)?;
            records.push(initial);
        }
        let parent = reduce_task_lifecycle(
            &evidence.identity,
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
                identity: evidence.identity.clone(),
                kind,
            },
            original_report_sha256: original_report_sha256.to_owned(),
            evidence_sha256: Some(evidence_sha.clone()),
            policy_sha256: Some(policy_sha.into()),
        };
        next.event.event_id = crate::task_lifecycle_store::event_id(&next)?;
        let mut proposed = records.iter().map(|r| r.event.clone()).collect::<Vec<_>>();
        proposed.push(next.event.clone());
        if reduce_task_lifecycle(&evidence.identity, &proposed, &[]).state
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
        &evidence.identity,
        &records.iter().map(|r| r.event.clone()).collect::<Vec<_>>(),
        &verified,
    );
    Ok(
        json!({"schema_version":"0.1.0","report_type":"task_resolution_receipt","identity":evidence.identity,"state":view.state,"outcome":outcome,"authority":"host_context_verified","policy_sha256":policy_sha,"policy_revision":evidence.policy_revision,"event_ref":format!(".codeguard/findings/{}/events/lifecycle-{}.json",task_id,record.event.event_id),"evidence_ref":evidence_ref,"evidence_sha256":evidence_sha,"delivery_decision":"not_evaluated"}),
    )
}
fn go_companion_matches(tool: &Path, policy: &TaskResolutionPolicyInput) -> bool {
    crate::go_syntax_probe::companion_current(
        tool,
        &json!({
            "gofmt_sha256":policy.gofmt_sha256,
            "companion_binding_sha256":policy.companion_binding_sha256
        }),
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

#[cfg(test)]
mod commit_tests {
    use codeguard_core::{ResolutionCause, ResolutionEvidence, TaskIdentity};
    use serde_json::json;
    use std::path::Path;
    #[test]
    fn mismatched_domain_and_redacted_evidence_is_rejected_before_history_access() {
        let identity = TaskIdentity {
            workspace_id: "commit-boundary".into(),
            task_id: format!("CG-B-{}", "a".repeat(32)),
            checker_id: "syntax.native_confirmation".into(),
            scope: "app.zig".into(),
        };
        let native = json!({"status":"incomplete","reason":"zig_ast_check_incomplete","version":"0.16.0","tool_sha256":"b".repeat(64),"diagnostics":[]});
        let pair = json!({"original":native,"current":native});
        let evidence = ResolutionEvidence {
            identity: identity.clone(),
            original_source_sha256: "c".repeat(64),
            current_source_sha256: "d".repeat(64),
            native_report_sha256: super::digest(&serde_json::to_vec(&pair).unwrap()),
            tool_sha256: "b".repeat(64),
            adapter_sha256: "e".repeat(64),
            rulepack_sha256: "f".repeat(64),
            policy_sha256: "1".repeat(64),
            policy_revision: "p1".into(),
            native_completed: false,
            original_rule_checked: false,
            target_covered: true,
            inputs_current: true,
            policy_verified: true,
            issue_still_present: false,
            suppression_changed: false,
            target_removed: false,
            cause: ResolutionCause::CodeFixed,
        };
        let raw = json!({"schema_version":"0.1.0","report_type":"task_resolution_evidence","identity":identity,
            "original_report_sha256":"2".repeat(64),"original_source_sha256":evidence.original_source_sha256,"current_source_sha256":evidence.current_source_sha256,
            "grammar_sha256":evidence.rulepack_sha256,"tool_sha256":evidence.tool_sha256,"adapter_sha256":evidence.adapter_sha256,
            "policy_sha256":evidence.policy_sha256,"policy_revision":evidence.policy_revision,"original_native":native,"current_native":native,"outcome":"native_incomplete"});
        assert_eq!(
            super::commit_resolution(
                Path::new("/codeguard-commit-no-workspace"),
                &identity.task_id,
                &"2".repeat(64),
                &evidence,
                raw.clone(),
                "native_incomplete"
            ),
            Err("task_lifecycle_directory_invalid")
        );
        for key in [
            "schema_version",
            "report_type",
            "outcome",
            "identity",
            "original_report_sha256",
            "original_source_sha256",
            "current_source_sha256",
            "tool_sha256",
            "adapter_sha256",
            "grammar_sha256",
            "policy_sha256",
            "policy_revision",
            "current_native",
        ] {
            let mut forged = raw.clone();
            forged[key] = json!("forged");
            assert_eq!(
                super::commit_resolution(
                    Path::new("/codeguard-commit-no-workspace"),
                    &identity.task_id,
                    &"2".repeat(64),
                    &evidence,
                    forged,
                    "native_incomplete"
                ),
                Err("task_resolution_evidence_binding_invalid"),
                "{key}"
            );
        }
    }
}
