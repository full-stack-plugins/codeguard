//! Python宿主签名批准下的原始/当前语法确认；本地文件不提供批准权威。
use crate::{
    PythonTaskResolutionRequest,
    python_task_resolution_policy_input::PythonTaskResolutionPolicyInput,
};
use codeguard_core::{ResolutionCause, ResolutionEvidence};
use codeguard_runtime::{NativeObservation, read_bounded_regular_file};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::{Component, Path},
    sync::atomic::AtomicBool,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// 在独立宿主批准下对照Python原样本和当前源码，并提交限定任务解决或复发事件。
/// 参数中的信任根及上下文必须独立于项目固定；返回限定收据，不签发项目allow。
pub fn verify_python_task_resolution(
    request: &PythonTaskResolutionRequest<'_>,
) -> Result<Value, &'static str> {
    let started = Instant::now();
    deadline(request.deadline)?;
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
    let policy: PythonTaskResolutionPolicyInput =
        serde_json::from_value(raw).map_err(|_| "task_resolution_policy_invalid")?;
    if policy.schema_version != "1.5.0"
        || policy.report_type != "task_resolution_policy"
        || policy.policy_revision != approved.policy_revision()
        || policy.identity.workspace_id != request.context.workspace_id
        || policy.identity.task_id != request.task_id
        || policy.identity.checker_id != "python.ruff"
        || policy.native_rule_id != "invalid-syntax"
        || policy.native_version != "ruff 0.16.8"
        || !matches!(
            policy.target_version.as_str(),
            "py37" | "py38" | "py39" | "py310" | "py311" | "py312" | "py313" | "py314" | "py315"
        )
        || !safe_path(&policy.configuration_ref)
        || ![
            &policy.original_report_sha256,
            &policy.original_source_sha256,
            &policy.grammar_sha256,
            &policy.tool_sha256,
            &policy.adapter_sha256,
            &policy.configuration_sha256,
        ]
        .iter()
        .all(|s| valid_sha(s))
    {
        return Err("task_resolution_policy_scope_mismatch");
    }
    let root = request
        .root
        .canonicalize()
        .map_err(|_| "workspace_unreadable")?;
    let brief = crate::next_command::read_task_brief(&root, request.task_id)?;
    if brief["scope"] != policy.identity.scope || brief["checker_id"] != "python.ruff" {
        return Err("task_resolution_original_identity_mismatch");
    }
    let original = crate::validate_python_task_original_source(
        &root,
        request.task_id,
        request.original_source,
    )?;
    if original["report_sha256"] != policy.original_report_sha256
        || original["source_sha256"] != policy.original_source_sha256
        || original["grammar_sha256"] != policy.grammar_sha256
    {
        return Err("task_resolution_original_identity_mismatch");
    }
    let registry = codeguard_adapters::legacy_registry().map_err(|_| "registry_invalid")?;
    let selected = vec![policy.identity.scope.clone()];
    let discovery = crate::python_selected_discovery::discover_selected(
        &root,
        &selected,
        &registry,
        &NativeObservation,
        end,
    );
    let config_valid =
        crate::python_lint_scan::select_checker(&discovery, &policy.identity.scope).is_some_and(
            |c| c.configuration == "configured" && c.configuration_ref == policy.configuration_ref,
        ) && discovery
            .checker_config_sha256
            .get(&policy.configuration_ref)
            == Some(&policy.configuration_sha256);
    if !discovery.observation_complete || !config_valid {
        return Err("task_resolution_configuration_mismatch");
    }
    let tool = request
        .tool
        .canonicalize()
        .map_err(|_| "task_resolution_tool_unavailable")?;
    if !request.tool.is_absolute() || file_hash(&tool, 64 * 1024 * 1024)? != policy.tool_sha256 {
        return Err("task_resolution_tool_mismatch");
    }
    let adapter = std::env::current_exe()
        .map_err(|_| "task_resolution_adapter_unavailable")?
        .canonicalize()
        .map_err(|_| "task_resolution_adapter_unavailable")?;
    if file_hash(&adapter, 256 * 1024 * 1024)? != policy.adapter_sha256 {
        return Err("task_resolution_adapter_mismatch");
    }
    crate::task_lifecycle_store::load(&root, &policy.identity, &policy.original_report_sha256)?;
    deadline(end)?;
    let lease = crate::task_lease_command::begin_verification(
        &root,
        request.task_id,
        request.borrowed_lease,
    )?;
    let result = execute(
        request,
        (&root, &tool, &adapter),
        &policy,
        &original,
        &discovery,
        approved.snapshot_sha256(),
        (&lease, end),
    );
    let release = crate::task_lease_command::finish_verification(&root, request.task_id, &lease);
    match (result, release) {
        (_, Err(e)) => Err(e),
        (r, Ok(())) => r,
    }
}
fn execute(
    request: &PythonTaskResolutionRequest<'_>,
    paths: (&Path, &Path, &Path),
    policy: &PythonTaskResolutionPolicyInput,
    original: &Value,
    discovery: &crate::discovery::DiscoveryReport,
    policy_sha: &str,
    control: (&crate::task_lease_command::VerificationLease, Instant),
) -> Result<Value, &'static str> {
    let (root, tool, adapter) = paths;
    let (lease, end) = control;
    let attempt = crate::task_attempt_command::latest_ready_attempt(root, request.task_id)?;
    let source_path = root.join(&policy.identity.scope);
    let current = crate::plain_syntax_source::read_plain_source(&source_path)?;
    let original_native = crate::python_syntax_probe::observe_configured(
        tool,
        request.original_source,
        &policy.target_version,
        &root.join(&policy.configuration_ref),
        &source_path,
        end,
    );
    deadline(end)?;
    let scratch = crate::doctor_scratch::DoctorScratch::create("python-resolution")
        .ok_or("task_resolution_scratch_unavailable")?;
    let run = format!(
        "lint-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "clock_unavailable")?
            .as_nanos()
    );
    let selected = vec![policy.identity.scope.clone()];
    let bytes = read_bounded_regular_file(tool, 64 * 1024 * 1024)
        .map_err(|_| "task_resolution_tool_unavailable")?;
    let scan = crate::python_lint_scan::scan_python_lint(
        &crate::python_lint_scan::PythonLintScanRequest {
            root,
            discovery,
            selected_paths: Some(&selected),
            tool: Some(tool.to_path_buf()),
            tool_unavailable_reason: "task_resolution_tool_unavailable",
            expected_tool_sha256: Sha256::digest(&bytes).into(),
            expected_version: policy.native_version.clone(),
            evidence_dir: scratch.path().to_path_buf(),
            run_id: run.clone(),
            deadline: end,
        },
        &AtomicBool::new(false),
    );
    let file = scan
        .files
        .iter()
        .find(|f| f.path == policy.identity.scope)
        .ok_or("task_resolution_target_not_observed")?;
    let target_valid = file
        .rule_settings
        .as_ref()
        .is_some_and(|s| s.explicit_python_target() == Ok(policy.target_version.as_str()));
    let complete = file.completion
        && target_valid
        && file.source_sha256.as_deref() == Some(digest(&current).as_str());
    let rows:Vec<Value>=file.diagnostics.iter().filter(|r|r.code=="invalid-syntax").map(|r|json!({"line":r.location.row,"column":r.location.column,"column_unit":"ruff_reported","rule_id":"invalid-syntax"})).collect();
    let mut current_native = json!({"status":if complete {if rows.is_empty(){"completed"}else{"diagnostics_observed"}}else{"incomplete"},
        "reason":if complete {if rows.is_empty(){"python_native_syntax_no_diagnostics"}else{"python_native_syntax_diagnostics"}}else{"python_syntax_execution_incomplete"},
        "version":if complete {json!(policy.native_version)}else{Value::Null},"tool_sha256":file.tool_sha256,"target_version":policy.target_version,"diagnostics":rows});
    if !crate::python_syntax_probe::valid_native_observation(&current_native, Some(&current)) {
        current_native["status"] = json!("incomplete");
        current_native["reason"] = json!("python_syntax_report_invalid");
        current_native["diagnostics"] = json!([]);
    }
    let _guard = crate::task_lease_command::lock_verification(root, request.task_id, lease)?;
    deadline(end)?;
    if crate::task_attempt_command::latest_ready_attempt(root, request.task_id)? != attempt {
        return Err("attempt_changed_during_verification");
    }
    let inputs = crate::plain_syntax_source::read_plain_source(&source_path)
        .is_ok_and(|b| b == current)
        && file_hash(tool, 64 * 1024 * 1024).is_ok_and(|s| s == policy.tool_sha256)
        && request.tool.canonicalize().ok().as_deref() == Some(tool)
        && file_hash(adapter, 256 * 1024 * 1024).is_ok_and(|s| s == policy.adapter_sha256)
        && crate::ruff_verification_configuration::is_current(
            root,
            &policy.identity.scope,
            &policy.configuration_ref,
            &policy.configuration_sha256,
        );
    if crate::validate_python_task_original_source(root, request.task_id, request.original_source)?
        != *original
    {
        return Err("task_resolution_original_identity_mismatch");
    }
    let original_complete = crate::python_syntax_probe::valid_native_observation(
        &original_native,
        Some(request.original_source),
    ) && matches!(
        original_native["status"].as_str(),
        Some("completed" | "diagnostics_observed")
    );
    let current_complete = current_native["status"] != "incomplete";
    let native_bound = [&original_native, &current_native].iter().all(|n| {
        n["tool_sha256"] == policy.tool_sha256
            && n["version"] == policy.native_version
            && n["target_version"] == policy.target_version
    });
    let present = current_native["status"] == "diagnostics_observed";
    let outcome = if !inputs {
        "inputs_stale"
    } else if !native_bound || !original_complete || !current_complete {
        "native_incomplete"
    } else if present {
        "still_present"
    } else if original_native["status"] == "completed" {
        "false_positive_review_required"
    } else {
        "code_fixed"
    };
    let mut report = crate::python_lint_scan::python_lint_feedback(discovery, &scan);
    crate::python_lint_command::annotate_rulepack(&mut report, Some(&policy.native_version));
    let fields = json!({"schema_version":"0.19.0","operation":"lint","language":"python","run_id":run,"workspace_binding":"bound","workspace_id":policy.identity.workspace_id,"command_status":"incomplete","exit_code":3,"tool_approval":"unverified","native_tool_version":policy.native_version,"rulepack_approval":"unverified","adapter_sha256":policy.adapter_sha256,"task_scope":"single_python_confirmation_file","task_binding":{"task_id":request.task_id,"path":policy.identity.scope,"source_sha256":digest(&current),"original_report":original},"task_input_stable":inputs});
    report
        .as_object_mut()
        .ok_or("task_resolution_encoding_failed")?
        .extend(
            fields
                .as_object()
                .ok_or("task_resolution_encoding_failed")?
                .clone(),
        );
    let brief = crate::next_command::read_task_brief(root, request.task_id)?;
    if crate::python_confirmation_recheck::valid_binding(root, &report) {
        crate::task_verify_command::persist_observation(
            root,
            &brief,
            &report,
            crate::task_verify_command::classify(&brief, &report),
            attempt.as_deref(),
        )?;
    } else if inputs {
        return Err("task_resolution_observation_binding_invalid");
    }
    deadline(end)?;
    let evidence = ResolutionEvidence {
        identity: policy.identity.clone(),
        original_source_sha256: policy.original_source_sha256.clone(),
        current_source_sha256: digest(&current),
        native_report_sha256: digest(
            &serde_json::to_vec(&json!({"original":original_native,"current":current_native}))
                .map_err(|_| "task_resolution_encoding_failed")?,
        ),
        tool_sha256: policy.tool_sha256.clone(),
        adapter_sha256: policy.adapter_sha256.clone(),
        rulepack_sha256: policy.grammar_sha256.clone(),
        policy_sha256: policy_sha.into(),
        policy_revision: policy.policy_revision.clone(),
        native_completed: native_bound && original_complete && current_complete,
        original_rule_checked: original_native["status"] == "diagnostics_observed",
        target_covered: target_valid,
        inputs_current: inputs,
        policy_verified: true,
        issue_still_present: present,
        suppression_changed: false,
        target_removed: false,
        cause: ResolutionCause::CodeFixed,
    };
    crate::task_resolution_service::commit_resolution(
        root,
        request.task_id,
        &policy.original_report_sha256,
        &evidence,
        json!({"schema_version":"0.6.0","report_type":"task_resolution_evidence","identity":policy.identity,"original_report_sha256":policy.original_report_sha256,"original_source_sha256":policy.original_source_sha256,"current_source_sha256":digest(&current),"grammar_sha256":policy.grammar_sha256,"tool_sha256":policy.tool_sha256,"adapter_sha256":policy.adapter_sha256,"policy_sha256":policy_sha,"policy_revision":policy.policy_revision,"original_native":original_native,"current_native":current_native,"outcome":outcome,"target_version":policy.target_version,"configuration_ref":policy.configuration_ref,"configuration_sha256":policy.configuration_sha256}),
        outcome,
    )
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn file_hash(path: &Path, limit: u64) -> Result<String, &'static str> {
    read_bounded_regular_file(path, limit)
        .map(|b| digest(&b))
        .map_err(|_| "task_resolution_artifact_unavailable")
}
fn valid_sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}
fn safe_path(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('\\')
        && !value.chars().any(char::is_control)
        && Path::new(value)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}
fn deadline(end: Instant) -> Result<(), &'static str> {
    if codeguard_runtime::sigint_cancellation_requested() {
        Err("request_cancelled")
    } else if Instant::now() >= end {
        Err("request_deadline_exceeded")
    } else {
        Ok(())
    }
}

/// 普通限定Python复检只能重开同一历史任务，不沿用历史批准签发新关闭。
/// 调用方持有任务锁；输入、配置和原生制品必须与已记录关闭证据相符。
pub(crate) fn record_recurrence(
    root: &Path,
    brief: &Value,
    scan: &Value,
) -> Result<(), &'static str> {
    use codeguard_core::{
        TaskIdentity, TaskLifecycleEvent, TaskLifecycleKind, TaskLifecycleRecord,
        reduce_task_lifecycle,
    };
    if brief["reason_code"] != "python_syntax_confirmation_needed"
        || scan["schema_version"] != "0.19.0"
        || scan["task_input_stable"] != true
        || !crate::python_confirmation_recheck::valid_binding(root, scan)
        || !crate::python_confirmation_recheck::inputs_current(root, scan)
    {
        return Ok(());
    }
    let identity = TaskIdentity {
        workspace_id: scan["workspace_id"]
            .as_str()
            .ok_or("workspace_identity_unavailable")?
            .into(),
        task_id: brief["task_id"].as_str().ok_or("task_id_invalid")?.into(),
        checker_id: "python.ruff".into(),
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
    let file = &scan["files"][0];
    // 配置原字节相同才沿用历史明确目标；逐文件目标未经解析的配置不会产生关闭证据。
    if evidence["schema_version"] != "0.6.0"
        || file["run_status"] != "findings"
        || file["configuration_ref"] != evidence["configuration_ref"]
        || file["config_sha256"] != evidence["configuration_sha256"]
        || file["tool_sha256"] != evidence["tool_sha256"]
        || scan["native_tool_version"] != "ruff 0.16.8"
    {
        return Ok(());
    }
    let rows = file["findings"].as_array().ok_or("task_lifecycle_evidence_invalid")?.iter()
        .filter(|r| r["rule_id"] == "invalid-syntax")
        .map(|r| json!({"line":r["line"],"column":r["column"],"column_unit":"ruff_reported","rule_id":"invalid-syntax"})).collect::<Vec<_>>();
    if rows.is_empty() {
        return Ok(());
    }
    let source = crate::plain_syntax_source::read_plain_source(&root.join(&identity.scope))?;
    let native = json!({"status":"diagnostics_observed","reason":"python_native_syntax_diagnostics","version":"ruff 0.16.8","tool_sha256":file["tool_sha256"],"target_version":evidence["target_version"],"diagnostics":rows});
    if !crate::python_syntax_probe::valid_native_observation(&native, Some(&source))
        || file["source_sha256"] != digest(&source)
    {
        return Err("task_lifecycle_recurrence_inputs_stale");
    }
    evidence["current_native"] = native;
    evidence["current_source_sha256"] = file["source_sha256"].clone();
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
