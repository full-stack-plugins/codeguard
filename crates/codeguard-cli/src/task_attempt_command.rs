//! 持久修复尝试：受租约保护，记录观察变化；成功结束仍需原检查器复检。

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use codeguard_runtime::TaskFileLock;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::next_command::{canonical_action_id, read_task_brief};
use crate::task_lease_command::{
    Lease, digest, ensure_directory, read_lease, real_directory, valid_owner, valid_task_id,
    valid_token,
};
use crate::task_verify_command::{
    classify, classify_cve, classify_doctor, classify_go, classify_java, classify_rust,
};
use crate::work_sync::write_once;
use crate::workspace_refresh::read_workspace_baseline;

const MAX_EVENT_BYTES: u64 = 4096;
const MAX_SOURCE_BYTES: u64 = 16 * 1024 * 1024;
const NO_PROGRESS_BUDGET: usize = 2;

struct Args {
    operation: String,
    task_id: String,
    root: PathBuf,
    owner: String,
    token: String,
    action_id: Option<String>,
    attempt_id: Option<String>,
    outcome: Option<String>,
    note_code: Option<String>,
    json: bool,
}

/// 修复动作开始事件；内容身份来自 CLI 观察，不接受调用方传入的摘要。
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AttemptStart {
    schema_version: String,
    task_id: String,
    attempt_id: String,
    action_id: String,
    action_fingerprint: String,
    owner: String,
    token_sha256: String,
    generation: u64,
    sequence: u64,
    before_sha256: String,
    started_at: u64,
}

/// 修复动作结束事件；不表示 finding 已关闭。
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AttemptFinish {
    schema_version: String,
    task_id: String,
    attempt_id: String,
    owner: String,
    token_sha256: String,
    generation: u64,
    outcome: String,
    note_code: String,
    after_sha256: String,
    observed_change: bool,
    finished_at: u64,
}

struct AttemptLedger {
    starts: BTreeMap<String, AttemptStart>,
    finishes: BTreeMap<String, AttemptFinish>,
}

/// 执行 start/finish；退出零仅表示本地尝试事件持久化，不代表质量通过。
pub fn run(args: &[String]) -> ExitCode {
    let parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    };
    let result = execute(&parsed);
    match result {
        Ok(report) => {
            if parsed.json {
                println!("{report}");
            } else {
                println!(
                    "任务 {} 尝试 {} 已记录；仍需原检查器复检。",
                    parsed.task_id, parsed.operation
                );
                println!("attempt_id={}", report["attempt_id"]);
            }
            ExitCode::SUCCESS
        }
        Err(reason) => {
            if parsed.json {
                println!(
                    "{}",
                    json!({"schema_version":"0.1.0", "report_type":"task_attempt",
                    "operation":format!("attempt_{}", parsed.operation), "task_id":parsed.task_id,
                    "command_status":"incomplete", "exit_code":3, "reason":reason,
                    "attempt_id":null, "authority":"local_unverified",
                    "delivery_decision":"not_evaluated"})
                );
            } else {
                eprintln!("修复尝试未完成：{reason}");
            }
            ExitCode::from(3)
        }
    }
}

fn execute(args: &Args) -> Result<Value, &'static str> {
    let root = args.root.canonicalize().map_err(|_| "project_unreadable")?;
    if !root.is_dir() {
        return Err("project_unreadable");
    }
    let state = root.join(".codeguard/state");
    if !real_directory(&state) {
        return Err("workspace_state_unavailable");
    }
    read_task_brief(&root, &args.task_id)?;
    let locks = state.join("task_locks");
    ensure_directory(&locks)?;
    let _lock = TaskFileLock::acquire(&locks.join(format!("{}.lock", args.task_id)))
        .map_err(|_| "lease_lock_unavailable")?;
    let brief = read_task_brief(&root, &args.task_id)?;
    let events = events_dir(&root, &args.task_id)?;
    let ledger = read_ledger(&events, &args.task_id)?;
    let rechecks = verified_rechecks(&root, &args.task_id, &brief, &ledger)?;
    let now = now()?;
    if args.operation == "finish" {
        let attempt_id = args.attempt_id.as_deref().ok_or("attempt_id_missing")?;
        let start = ledger.starts.get(attempt_id).ok_or("attempt_not_found")?;
        if start.owner != args.owner || start.token_sha256 != digest(args.token.as_bytes()) {
            return Err("lease_token_mismatch");
        }
        if let Some(finish) = ledger.finishes.get(attempt_id) {
            if finish.outcome == args.outcome.as_deref().unwrap_or("")
                && finish.note_code == args.note_code.as_deref().unwrap_or("")
            {
                return Ok(finish_report(finish));
            }
            return Err("attempt_finish_conflict");
        }
    }
    let lease = read_lease(&state.join(format!("leases/{}.json", args.task_id)))?
        .ok_or("lease_not_found")?;
    validate_lease(&lease, args, now)?;
    let before_or_after = input_digest(&root, &brief)?;
    match args.operation.as_str() {
        "start" => {
            if ledger.open_attempt().is_some() {
                return Err("attempt_already_open");
            }
            let action_id = args.action_id.as_deref().ok_or("action_id_missing")?;
            if action_id != canonical_action_id(&brief)? {
                return Err("action_id_invalid");
            }
            if awaiting_verification(&ledger, &rechecks, &before_or_after) {
                return Err("verification_required_before_retry");
            }
            let history = history_from_ledger(
                &ledger,
                &rechecks,
                action_id,
                &before_or_after,
                confirmation_action_budget(&brief),
            );
            if history.no_progress_count >= NO_PROGRESS_BUDGET {
                return Err("no_progress_budget_exhausted");
            }
            if brief["disposition"] != "actionable" {
                return Err("task_not_actionable");
            }
            let attempt_id = format!("CG-A-{}", &crate::task_lease_command::random_token()?[..32]);
            let start = AttemptStart {
                schema_version: "0.1.0".into(),
                task_id: args.task_id.clone(),
                attempt_id: attempt_id.clone(),
                action_id: action_id.into(),
                action_fingerprint: action_fingerprint(&args.task_id, action_id),
                owner: args.owner.clone(),
                token_sha256: lease.token_sha256,
                generation: lease.generation,
                sequence: ledger
                    .starts
                    .len()
                    .checked_add(1)
                    .ok_or("attempt_sequence_exhausted")? as u64,
                before_sha256: before_or_after,
                started_at: now,
            };
            write_event(&events, &state, &attempt_id, "start", &start)?;
            Ok(
                json!({"schema_version":"0.1.0", "report_type":"task_attempt",
                "operation":"attempt_start", "task_id":args.task_id,
                "command_status":"complete", "exit_code":0, "reason":null,
                "attempt_id":attempt_id, "action_id":action_id,
                "action_fingerprint":start.action_fingerprint,
                "before_sha256":start.before_sha256,
                "authority":"local_unverified", "delivery_decision":"not_evaluated"}),
            )
        }
        "finish" => {
            let attempt_id = args.attempt_id.as_deref().ok_or("attempt_id_missing")?;
            let start = ledger.starts.get(attempt_id).ok_or("attempt_not_found")?;
            if start.owner != args.owner
                || start.token_sha256 != lease.token_sha256
                || start.generation != lease.generation
            {
                return Err("attempt_lease_conflict");
            }
            let outcome = args.outcome.as_deref().ok_or("outcome_missing")?;
            let note_code = args.note_code.as_deref().ok_or("note_code_missing")?;
            if outcome == "no-change" && before_or_after != start.before_sha256 {
                return Err("outcome_observation_conflict");
            }
            let finish = AttemptFinish {
                schema_version: "0.1.0".into(),
                task_id: args.task_id.clone(),
                attempt_id: attempt_id.into(),
                owner: args.owner.clone(),
                token_sha256: lease.token_sha256,
                generation: lease.generation,
                outcome: outcome.into(),
                note_code: note_code.into(),
                observed_change: before_or_after != start.before_sha256,
                after_sha256: before_or_after,
                finished_at: now,
            };
            write_event(&events, &state, attempt_id, "finish", &finish)?;
            Ok(finish_report(&finish))
        }
        _ => Err("operation_invalid"),
    }
}

fn validate_lease(lease: &Lease, args: &Args, now: u64) -> Result<(), &'static str> {
    if lease.task_id != args.task_id
        || lease.owner != args.owner
        || lease.token_sha256 != digest(args.token.as_bytes())
    {
        return Err("lease_token_mismatch");
    }
    if lease.status != "active" || lease.expires_at <= now {
        return Err("lease_expired_or_inactive");
    }
    Ok(())
}

fn finish_report(finish: &AttemptFinish) -> Value {
    json!({"schema_version":"0.1.0", "report_type":"task_attempt",
        "operation":"attempt_finish", "task_id":finish.task_id,
        "command_status":"complete", "exit_code":0, "reason":null,
        "attempt_id":finish.attempt_id, "outcome":finish.outcome,
        "note_code":finish.note_code, "after_sha256":finish.after_sha256,
        "observed_change":finish.observed_change,
        "authority":"local_unverified", "delivery_decision":"not_evaluated"})
}

/// 读取同一受控动作在当前输入上的历史，供 RepairBrief 展示。
pub(crate) fn attempt_history(root: &Path, id: &str, brief: &Value) -> Result<Value, &'static str> {
    let events = events_dir(root, id)?;
    let ledger = read_ledger(&events, id)?;
    let rechecks = verified_rechecks(root, id, brief, &ledger)?;
    let action = canonical_action_id(brief)?;
    let input = input_digest(root, brief)?;
    let history = history_from_ledger(
        &ledger,
        &rechecks,
        action,
        &input,
        confirmation_action_budget(brief),
    );
    let mut recent: Vec<_> = ledger.starts.values().collect();
    recent.sort_by_key(|start| start.sequence);
    let recent: Vec<Value> = recent
        .into_iter()
        .rev()
        .take(5)
        .map(|start| {
            let finish = ledger.finishes.get(&start.attempt_id);
            json!({"attempt_id":start.attempt_id, "action_id":start.action_id,
            "outcome":finish.map(|value| value.outcome.as_str()),
            "note_code":finish.map(|value| value.note_code.as_str()),
            "observed_change":finish.map(|value| value.observed_change)})
        })
        .collect();
    Ok(json!({"attempt_count":history.attempt_count,
        "no_progress_count":history.no_progress_count,
        "open_attempt_id":ledger.open_attempt().map(|start| start.attempt_id.as_str()),
        "awaiting_verification":awaiting_verification(&ledger, &rechecks, &input),
        "budget":NO_PROGRESS_BUDGET, "recent":recent}))
}

struct History {
    attempt_count: usize,
    no_progress_count: usize,
}

fn history_from_ledger(
    ledger: &AttemptLedger,
    rechecks: &BTreeMap<String, String>,
    action: &str,
    input: &str,
    confirmation_actions: bool,
) -> History {
    let mut matching: Vec<_> = ledger
        .starts
        .values()
        .filter(|start| {
            start.before_sha256 == input
                && (start.action_id == action
                    || (confirmation_actions
                        && matches!(
                            start.action_id.as_str(),
                            "repair-source" | "restore-checker-environment"
                        )
                        && matches!(action, "repair-source" | "restore-checker-environment")))
        })
        .collect();
    matching.sort_by_key(|start| start.sequence);
    let attempt_count = matching.len();
    let mut no_progress_count = 0;
    for start in matching {
        if let Some(finish) = ledger.finishes.get(&start.attempt_id) {
            if matches!(
                finish.outcome.as_str(),
                "no-change" | "failed" | "blocked" | "abandoned"
            ) {
                no_progress_count += 1;
            } else if finish.outcome == "ready-to-verify" {
                match rechecks.get(&start.attempt_id).map(String::as_str) {
                    Some(
                        "still_present"
                        | "still_blocked"
                        | "incomplete"
                        | "suppression_requires_review"
                        | "rule_coverage_requires_review",
                    ) => {
                        no_progress_count += 1;
                    }
                    Some(_) => no_progress_count = 0,
                    None => {}
                }
            } else {
                no_progress_count = 0;
            }
        }
    }
    History {
        attempt_count,
        no_progress_count,
    }
}

/// 返回最近一次待复检的受控尝试 ID；仅用于将新原生复检与该尝试关联。
pub(crate) fn latest_ready_attempt(root: &Path, id: &str) -> Result<Option<String>, &'static str> {
    let ledger = read_ledger(&events_dir(root, id)?, id)?;
    let brief = read_task_brief(root, id)?;
    let current_input = input_digest(root, &brief)?;
    Ok(ledger
        .starts
        .values()
        .max_by_key(|start| start.sequence)
        .and_then(|start| {
            ledger
                .finishes
                .get(&start.attempt_id)
                .filter(|finish| {
                    finish.outcome == "ready-to-verify" && finish.after_sha256 == current_input
                })
                .map(|_| start.attempt_id.clone())
        }))
}

fn awaiting_verification(
    ledger: &AttemptLedger,
    rechecks: &BTreeMap<String, String>,
    current_input: &str,
) -> bool {
    ledger
        .starts
        .values()
        .max_by_key(|start| start.sequence)
        .and_then(|start| {
            ledger
                .finishes
                .get(&start.attempt_id)
                .map(|finish| (start, finish))
        })
        .is_some_and(|(start, finish)| {
            finish.outcome == "ready-to-verify"
                && finish.after_sha256 == current_input
                && !rechecks.contains_key(&start.attempt_id)
        })
}

fn verified_rechecks(
    root: &Path,
    id: &str,
    brief: &Value,
    ledger: &AttemptLedger,
) -> Result<BTreeMap<String, String>, &'static str> {
    let workspace_id = read_workspace_baseline(root)
        .map_err(|_| "workspace_identity_unavailable")?
        .and_then(|baseline| baseline.workspace_id().map(str::to_owned))
        .ok_or("workspace_identity_unavailable")?;
    let events = events_dir(root, id)?;
    let mut matched = BTreeMap::<String, (u128, String)>::new();
    for entry in fs::read_dir(events).map_err(|_| "attempt_events_unreadable")? {
        let entry = entry.map_err(|_| "attempt_events_unreadable")?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "verification_event_invalid")?;
        let Some(run_id) = name
            .strip_prefix("verify-")
            .and_then(|name| name.strip_suffix(".json"))
        else {
            continue;
        };
        let nanos = if run_id.starts_with("rust-cve-") || run_id.starts_with("python-cve-") {
            run_id
                .rsplit('-')
                .nth(1)
                .and_then(|s| s.parse::<u128>().ok())
        } else if run_id.starts_with("syntax-native-")
            || run_id.starts_with("checkstyle-")
            || run_id.starts_with("eslint-")
            || run_id.starts_with("npm-")
            || run_id.starts_with("rustdoc-")
            || run_id.starts_with("cargo-build-")
        {
            run_id
                .rsplit('-')
                .next()
                .and_then(|s| s.parse::<u128>().ok())
        } else if let Some(value) = run_id
            .strip_prefix("rust-lint-")
            .or_else(|| run_id.strip_prefix("go-lint-"))
            .or_else(|| run_id.strip_prefix("java-p3c-"))
            .or_else(|| run_id.strip_prefix("java-cve-"))
            .or_else(|| run_id.strip_prefix("doctor-"))
        {
            value
                .rsplit_once('-')
                .and_then(|(prefix, _)| prefix.rsplit_once('-'))
                .and_then(|(_, nanos)| nanos.parse::<u128>().ok())
        } else {
            run_id
                .strip_prefix("lint-")
                .and_then(|value| value.rsplit_once('-'))
                .and_then(|(_, nanos)| nanos.parse::<u128>().ok())
        }
        .ok_or("verification_event_invalid")?;
        let bytes = bounded_regular(&entry.path(), MAX_EVENT_BYTES)?;
        let event: Value =
            serde_json::from_slice(&bytes).map_err(|_| "verification_event_invalid")?;
        if event["schema_version"] != "0.3.0" {
            continue;
        }
        let Some(attempt_id) = event["attempt_id"].as_str() else {
            if event["attempt_id"].is_null() {
                continue;
            }
            return Err("verification_event_invalid");
        };
        let start = ledger
            .starts
            .get(attempt_id)
            .ok_or("verification_event_invalid")?;
        let finish = ledger
            .finishes
            .get(attempt_id)
            .ok_or("verification_event_invalid")?;
        if finish.outcome != "ready-to-verify"
            || nanos < (finish.finished_at as u128).saturating_mul(1_000_000_000)
            || event["event"] != "verification_observed"
            || event["task_id"] != id
            || event["kind"] != brief["kind"]
            || event["workspace_id"] != workspace_id
            || event["run_id"] != run_id
            || event["state_after"] != "open"
            || event["authority"] != "local_unverified"
            || start.attempt_id != attempt_id
        {
            return Err("verification_event_invalid");
        }
        let report_path = root.join(format!(".codeguard/reports/{run_id}.json"));
        let report_bytes = match bounded_regular(&report_path, 16 * 1024 * 1024) {
            Ok(bytes) => bytes,
            Err("verification_evidence_unavailable") if !report_path.exists() => {
                // reports 默认不入 Git；缺少本地字节时不能确认旧复检，可重新运行原工具。
                continue;
            }
            Err(reason) => return Err(reason),
        };
        let report: Value =
            serde_json::from_slice(&report_bytes).map_err(|_| "verification_event_invalid")?;
        if brief["checker_id"] == "node.npm.audit" {
            codeguard_adapters::parse_unique_json(&report_bytes)
                .map_err(|_| "verification_event_invalid")?;
            if !crate::npm_task_recheck::matches_task(brief, &report) {
                return Err("verification_event_invalid");
            }
        }
        if brief["checker_id"] == "node.npm.audit"
            && event["report_sha256"] == digest(&report_bytes)
            && crate::npm_task_recheck::inputs_stale(root, &report)
        {
            continue;
        }
        if brief["checker_id"] == "rust.cargo_audit"
            && event["report_sha256"] == digest(&report_bytes)
            && !crate::rust_cve_task_recheck::inputs_current(root, &report)
        {
            continue;
        }
        if brief["checker_id"] == "python.pip_audit"
            && event["report_sha256"] == digest(&report_bytes)
            && !crate::python_cve_task_recheck::inputs_current(root, &report)
        {
            continue;
        }
        if brief["checker_id"] == "syntax.native_confirmation"
            && event["report_sha256"] == digest(&report_bytes)
            && !crate::syntax_task_recheck::inputs_current(root, &report)
        {
            continue;
        }
        let report_matches = if brief["checker_id"] == "syntax.native_confirmation" {
            crate::syntax_task_recheck::valid_shape(root, &report)
                && event["observation"] == crate::syntax_task_recheck::classify(&report)
        } else if brief["checker_id"] == "python.ruff.doctor" {
            crate::work_sync::valid_doctor_report(&report)
                && event["observation"] == classify_doctor(brief, &report)
        } else if matches!(
            brief["checker_id"].as_str(),
            Some("node.eslint" | "node.eslint.preparation")
        ) {
            crate::eslint_task_recheck::valid_shape(&report)
                && event["observation"] == crate::eslint_task_recheck::classify(brief, &report)
        } else if brief["checker_id"] == "node.npm.audit" {
            crate::work_sync::valid_npm_observation(root, &report)
                && event["observation"] == crate::npm_task_recheck::classify(root, brief, &report)
        } else if brief["checker_id"] == "go.vet" {
            report["report_type"] == "go_lint_local_feedback"
                && matches!(
                    report["schema_version"].as_str(),
                    Some("0.4.0" | "0.5.0" | "0.6.0" | "0.7.0")
                )
                && report["checker_id"] == "go.vet"
                && event["observation"] == classify_go(brief, &report)
        } else if brief["checker_id"] == "java.checkstyle.preparation" {
            crate::checkstyle_preparation_recheck::valid_shape(&report)
                && event["observation"]
                    == crate::checkstyle_preparation_recheck::classify(brief, &report)
        } else if brief["checker_id"] == "java.checkstyle" {
            crate::checkstyle_task_recheck::valid_shape(&report)
                && event["observation"] == crate::checkstyle_task_recheck::classify(brief, &report)
        } else if brief["checker_id"] == "java.maven.p3c" {
            report["report_type"] == "java_p3c_project_observation"
                && report["schema_version"] == "0.2.0"
                && report["checker_id"] == "java.maven.p3c"
                && event["observation"] == classify_java(brief, &report)
        } else if brief["checker_id"] == "java.maven.dependency_check" {
            report["report_type"] == "java_cve_project_probe"
                && report["schema_version"] == "0.3.0"
                && report["checker_id"] == "java.maven.dependency_check"
                && event["observation"] == classify_cve(brief, &report)
        } else if brief["checker_id"] == "rust.cargo_check" {
            crate::rust_build_task_recheck::valid_shape(&report)
                && event["observation"] == crate::rust_build_task_recheck::classify(brief, &report)
        } else if brief["checker_id"] == "rust.cargo_audit" {
            crate::rust_cve_task_recheck::valid_shape(&report)
                && event["observation"] == crate::rust_cve_task_recheck::classify(brief, &report)
        } else if brief["checker_id"] == "python.pip_audit" {
            crate::work_sync::valid_python_cve_observation(root, &report)
                && event["observation"] == crate::python_cve_task_recheck::classify(brief, &report)
        } else if brief["checker_id"] == "rust.cargo_rustdoc" {
            crate::rustdoc_task_recheck::valid_shape(&report)
                && event["observation"] == crate::rustdoc_task_recheck::classify(brief, &report)
        } else if brief["checker_id"] == "rust.cargo_clippy" {
            report["report_type"] == "rust_clippy_local_observation"
                && matches!(report["schema_version"].as_str(), Some("0.2.0" | "0.3.0"))
                && report["checker_id"] == "rust.cargo_clippy"
                && event["observation"] == classify_rust(brief, &report)
        } else {
            report["report_type"] == "python_lint_feedback"
                && event["observation"] == classify(brief, &report)
        };
        if event["report_sha256"] != digest(&report_bytes)
            || report["run_id"] != run_id
            || report["workspace_id"] != workspace_id
            || report["delivery_decision"] != "not_evaluated"
            || !report_matches
        {
            if brief["checker_id"] == "rust.cargo_rustdoc" {
                return Err("rustdoc_attempt_verification_invalid");
            }
            return Err("verification_event_invalid");
        }
        let observation = event["observation"]
            .as_str()
            .ok_or("verification_event_invalid")?;
        if matched.get(attempt_id).is_none_or(|(old, _)| nanos > *old) {
            matched.insert(attempt_id.into(), (nanos, observation.into()));
        }
    }
    Ok(matched
        .into_iter()
        .map(|(id, (_, observation))| (id, observation))
        .collect())
}

fn bounded_regular(path: &Path, limit: u64) -> Result<Vec<u8>, &'static str> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "verification_evidence_unavailable")?;
    if !metadata.file_type().is_file() || metadata.nlink() != 1 || metadata.len() > limit {
        return Err("verification_evidence_invalid");
    }
    let bytes = fs::read(path).map_err(|_| "verification_evidence_unavailable")?;
    if bytes.len() as u64 > limit {
        return Err("verification_evidence_invalid");
    }
    Ok(bytes)
}

/// 正常 release 前拒绝未结束的尝试。
pub(crate) fn ensure_no_open_attempt(root: &Path, id: &str) -> Result<(), &'static str> {
    let ledger = read_ledger(&events_dir(root, id)?, id)?;
    if ledger.open_attempt().is_some() {
        Err("attempt_still_open")
    } else {
        Ok(())
    }
}

/// 过期接管前记录唯一 abandoned 结束事件；落盘失败不得发新租约。
pub(crate) fn recover_abandoned(
    root: &Path,
    id: &str,
    old: &Lease,
    now: u64,
) -> Result<(), &'static str> {
    let events = events_dir(root, id)?;
    let ledger = read_ledger(&events, id)?;
    let Some(start) = ledger.open_attempt() else {
        return Ok(());
    };
    if start.generation != old.generation || start.token_sha256 != old.token_sha256 {
        return Err("attempt_lease_conflict");
    }
    let brief = read_task_brief(root, id)?;
    let after = input_digest(root, &brief)?;
    let finish = AttemptFinish {
        schema_version: "0.1.0".into(),
        task_id: id.into(),
        attempt_id: start.attempt_id.clone(),
        owner: start.owner.clone(),
        token_sha256: start.token_sha256.clone(),
        generation: start.generation,
        outcome: "abandoned".into(),
        note_code: "lease_expired".into(),
        observed_change: after != start.before_sha256,
        after_sha256: after,
        finished_at: now,
    };
    write_event(
        &events,
        &root.join(".codeguard/state"),
        &start.attempt_id,
        "finish",
        &finish,
    )
}

impl AttemptLedger {
    fn open_attempt(&self) -> Option<&AttemptStart> {
        self.starts
            .values()
            .find(|start| !self.finishes.contains_key(&start.attempt_id))
    }
}

fn events_dir(root: &Path, id: &str) -> Result<PathBuf, &'static str> {
    let events = root.join(".codeguard/findings").join(id).join("events");
    if !real_directory(&events) {
        return Err("attempt_events_unavailable");
    }
    Ok(events)
}

fn read_ledger(events: &Path, id: &str) -> Result<AttemptLedger, &'static str> {
    let mut ledger = AttemptLedger {
        starts: BTreeMap::new(),
        finishes: BTreeMap::new(),
    };
    for entry in fs::read_dir(events).map_err(|_| "attempt_events_unreadable")? {
        let entry = entry.map_err(|_| "attempt_events_unreadable")?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "attempt_event_invalid")?;
        if !name.starts_with("attempt-") {
            continue;
        }
        let (attempt_id, kind) = parse_event_name(&name).ok_or("attempt_event_invalid")?;
        let metadata = fs::symlink_metadata(entry.path()).map_err(|_| "attempt_event_invalid")?;
        if !metadata.file_type().is_file()
            || metadata.nlink() != 1
            || metadata.len() > MAX_EVENT_BYTES
        {
            return Err("attempt_event_invalid");
        }
        let bytes = fs::read(entry.path()).map_err(|_| "attempt_event_unreadable")?;
        if bytes.len() as u64 > MAX_EVENT_BYTES {
            return Err("attempt_event_invalid");
        }
        if kind == "start" {
            let start: AttemptStart =
                serde_json::from_slice(&bytes).map_err(|_| "attempt_event_invalid")?;
            if start.schema_version != "0.1.0"
                || start.task_id != id
                || start.attempt_id != attempt_id
                || !matches!(
                    start.action_id.as_str(),
                    "repair-source" | "restore-checker-environment" | "review-project-policy"
                )
                || start.action_fingerprint != action_fingerprint(id, &start.action_id)
                || !valid_owner(&start.owner)
                || !valid_token(&start.token_sha256)
                || !valid_token(&start.before_sha256)
                || start.generation == 0
                || start.sequence == 0
                || start.started_at == 0
                || ledger.starts.insert(attempt_id.into(), start).is_some()
            {
                return Err("attempt_event_invalid");
            }
        } else {
            let finish: AttemptFinish =
                serde_json::from_slice(&bytes).map_err(|_| "attempt_event_invalid")?;
            if finish.schema_version != "0.1.0"
                || finish.task_id != id
                || finish.attempt_id != attempt_id
                || !valid_token(&finish.after_sha256)
                || !valid_token(&finish.token_sha256)
                || !valid_owner(&finish.owner)
                || !valid_outcome(&finish.outcome, true)
                || !valid_note_code(&finish.note_code)
                || finish.generation == 0
                || finish.finished_at == 0
                || ledger.finishes.insert(attempt_id.into(), finish).is_some()
            {
                return Err("attempt_event_invalid");
            }
        }
    }
    let mut sequences: Vec<u64> = ledger.starts.values().map(|start| start.sequence).collect();
    sequences.sort_unstable();
    if sequences
        .iter()
        .enumerate()
        .any(|(index, sequence)| *sequence != (index + 1) as u64)
        || ledger.finishes.iter().any(|(id, finish)| {
            ledger.starts.get(id).is_none_or(|start| {
                start.owner != finish.owner
                    || start.token_sha256 != finish.token_sha256
                    || start.generation != finish.generation
                    || (start.before_sha256 != finish.after_sha256) != finish.observed_change
                    || (finish.outcome == "no-change" && finish.observed_change)
            })
        })
        || ledger
            .starts
            .values()
            .filter(|start| !ledger.finishes.contains_key(&start.attempt_id))
            .count()
            > 1
    {
        return Err("attempt_event_invalid");
    }
    Ok(ledger)
}

fn parse_event_name(name: &str) -> Option<(&str, &str)> {
    let suffix = name.strip_prefix("attempt-")?.strip_suffix(".json")?;
    let (id, kind) = suffix.rsplit_once('-')?;
    if !matches!(kind, "start" | "finish") || !valid_attempt_id(id) {
        return None;
    }
    Some((id, kind))
}

fn valid_attempt_id(value: &str) -> bool {
    value.strip_prefix("CG-A-").is_some_and(|suffix| {
        suffix.len() == 32
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn write_event<T: Serialize>(
    events: &Path,
    state: &Path,
    id: &str,
    kind: &str,
    event: &T,
) -> Result<(), &'static str> {
    let bytes = serde_json::to_vec_pretty(event).map_err(|_| "attempt_event_encoding_failed")?;
    write_once(
        &events.join(format!("attempt-{id}-{kind}.json")),
        &bytes,
        state,
    )
    .map_err(|_| "attempt_event_write_failed")
}

fn action_fingerprint(task_id: &str, action_id: &str) -> String {
    digest(format!("{task_id}\0{action_id}").as_bytes())
}

fn input_digest(root: &Path, brief: &Value) -> Result<String, &'static str> {
    let paths: Vec<&str> = if brief["kind"] == "finding" {
        vec![brief["scope"].as_str().ok_or("attempt_scope_invalid")?]
    } else {
        brief["affected_paths"]
            .as_array()
            .ok_or("attempt_scope_invalid")?
            .iter()
            .map(|item| item.as_str().ok_or("attempt_scope_invalid"))
            .collect::<Result<_, _>>()?
    };
    let mut input = Vec::new();
    for path in paths {
        if path.starts_with('/') || path.contains("..") || path.contains('\\') || path.contains(':')
        {
            return Err("attempt_scope_invalid");
        }
        input.extend_from_slice(path.as_bytes());
        input.push(0);
        let target = root.join(path);
        if brief["checker_id"] == "node.npm.audit" && brief["kind"] == "blocker" {
            let limit = match target.file_name().and_then(|name| name.to_str()) {
                Some("package.json") => 256 * 1024,
                Some("package-lock.json") => 8 * 1024 * 1024,
                _ => return Err("attempt_scope_invalid"),
            };
            let (state, hash) = crate::npm_input_state::observe(&target, limit);
            if let Some(hash) = hash {
                input.extend_from_slice(hash.as_bytes());
            } else if state == "missing" {
                input.extend_from_slice(b"missing");
            } else {
                // 不可读输入仍可记录环境修复尝试，但不是有效源码摘要或关闭证据。
                input.extend_from_slice(b"npm-input-state-v1:");
                input.extend_from_slice(state.as_bytes());
            }
            input.push(0);
            continue;
        }
        match fs::symlink_metadata(&target) {
            Ok(metadata)
                if metadata.file_type().is_file()
                    && metadata.len() <= MAX_SOURCE_BYTES
                    && metadata.nlink() == 1 =>
            {
                input.extend_from_slice(
                    digest(&fs::read(target).map_err(|_| "attempt_input_unreadable")?).as_bytes(),
                );
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                input.extend_from_slice(b"missing")
            }
            _ => return Err("attempt_input_unreadable"),
        }
        input.push(0);
    }
    Ok(digest(&input))
}

fn now() -> Result<u64, &'static str> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| time.as_secs())
        .map_err(|_| "clock_unavailable")
}

fn valid_outcome(value: &str, allow_abandoned: bool) -> bool {
    matches!(
        value,
        "ready-to-verify" | "no-change" | "failed" | "blocked"
    ) || (allow_abandoned && value == "abandoned")
}

fn valid_note_code(value: &str) -> bool {
    matches!(
        value,
        "source_edit"
            | "tool_restored"
            | "no_change"
            | "execution_failed"
            | "blocked"
            | "lease_expired"
    )
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let [attempt, operation, task_id, rest @ ..] = args else {
        return Err("task attempt 缺少操作或任务 ID".into());
    };
    if attempt != "attempt"
        || !matches!(operation.as_str(), "start" | "finish")
        || !valid_task_id(task_id)
    {
        return Err("task attempt 操作或任务 ID 无效".into());
    }
    let (root, options) = if rest.first().is_some_and(|value| !value.starts_with('-')) {
        (PathBuf::from(&rest[0]), &rest[1..])
    } else {
        (PathBuf::from("."), rest)
    };
    let mut owner = None;
    let mut token = None;
    let mut action_id = None;
    let mut attempt_id = None;
    let mut outcome = None;
    let mut note_code = None;
    let mut json = false;
    let mut index = 0;
    while index < options.len() {
        let option = options[index].as_str();
        let (key, value) = if let Some((key, value)) = option.split_once('=') {
            (key, value.to_owned())
        } else {
            index += 1;
            (
                option,
                options
                    .get(index)
                    .ok_or(format!("{option} 缺少值"))?
                    .clone(),
            )
        };
        match key {
            "--owner" if owner.is_none() && valid_owner(&value) => owner = Some(value),
            "--lease-token" if token.is_none() && valid_token(&value) => token = Some(value),
            "--action-id"
                if action_id.is_none()
                    && matches!(
                        value.as_str(),
                        "repair-source" | "restore-checker-environment" | "review-project-policy"
                    ) =>
            {
                action_id = Some(value)
            }
            "--attempt-id" if attempt_id.is_none() && valid_attempt_id(&value) => {
                attempt_id = Some(value)
            }
            "--outcome" if outcome.is_none() && valid_outcome(&value, false) => {
                outcome = Some(value)
            }
            "--note-code"
                if note_code.is_none() && valid_note_code(&value) && value != "lease_expired" =>
            {
                note_code = Some(value)
            }
            "--format" if matches!(value.as_str(), "human" | "json") => json = value == "json",
            _ => return Err(format!("task attempt 参数无效或重复：{key}")),
        }
        index += 1;
    }
    if (operation == "start"
        && (action_id.is_none()
            || attempt_id.is_some()
            || outcome.is_some()
            || note_code.is_some()))
        || (operation == "finish"
            && (action_id.is_some()
                || attempt_id.is_none()
                || outcome.is_none()
                || note_code.is_none()))
    {
        return Err("task attempt 参数与操作不匹配".into());
    }
    Ok(Args {
        operation: operation.clone(),
        task_id: task_id.clone(),
        root,
        owner: owner.ok_or("缺少或无效的 --owner")?,
        token: token.ok_or("缺少或无效的 --lease-token")?,
        action_id,
        attempt_id,
        outcome,
        note_code,
        json,
    })
}

/// 语法确认任务在工具恢复与源码修复间切换时共享同输入重试预算。
/// 其他任务保留既有按动作区分的历史，不因这项兼容规则扩大关联范围。
fn confirmation_action_budget(brief: &Value) -> bool {
    brief["kind"] == "blocker"
        && (brief["checker_id"] == "syntax.native_confirmation"
            || (brief["checker_id"] == "python.ruff"
                && brief["reason_code"] == "python_syntax_confirmation_needed"))
}

#[cfg(test)]
mod confirmation_budget_tests {
    use super::{
        AttemptFinish, AttemptLedger, AttemptStart, action_fingerprint, confirmation_action_budget,
        history_from_ledger,
    };
    use serde_json::json;
    use std::collections::BTreeMap;
    fn ledger() -> AttemptLedger {
        let mut starts = BTreeMap::new();
        let mut finishes = BTreeMap::new();
        for (index, action) in [
            "restore-checker-environment",
            "restore-checker-environment",
            "repair-source",
        ]
        .iter()
        .enumerate()
        {
            let id = format!("CG-A-{index:032x}");
            starts.insert(
                id.clone(),
                AttemptStart {
                    schema_version: "0.1.0".into(),
                    task_id: format!("CG-B-{}", "a".repeat(32)),
                    attempt_id: id.clone(),
                    action_id: (*action).into(),
                    action_fingerprint: action_fingerprint("task", action),
                    owner: "worker".into(),
                    token_sha256: "a".repeat(64),
                    generation: 1,
                    sequence: index as u64 + 1,
                    before_sha256: "b".repeat(64),
                    started_at: 100,
                },
            );
            finishes.insert(
                id.clone(),
                AttemptFinish {
                    schema_version: "0.1.0".into(),
                    task_id: format!("CG-B-{}", "a".repeat(32)),
                    attempt_id: id,
                    owner: "worker".into(),
                    token_sha256: "a".repeat(64),
                    generation: 1,
                    outcome: "no-change".into(),
                    note_code: "no_change".into(),
                    after_sha256: "b".repeat(64),
                    observed_change: false,
                    finished_at: 101,
                },
            );
        }
        AttemptLedger { starts, finishes }
    }
    #[test]
    fn action_transition_cannot_reset_confirmation_budget() {
        let ledger = ledger();
        let rechecks = BTreeMap::new();
        let input = "b".repeat(64);
        for action in ["repair-source", "restore-checker-environment"] {
            let history = history_from_ledger(&ledger, &rechecks, action, &input, true);
            assert_eq!(history.attempt_count, 3);
            assert_eq!(history.no_progress_count, 3);
        }
        assert_eq!(
            history_from_ledger(&ledger, &rechecks, "repair-source", &input, false)
                .no_progress_count,
            1
        );
        assert_eq!(
            history_from_ledger(&ledger, &rechecks, "repair-source", &"c".repeat(64), true)
                .no_progress_count,
            0
        );
    }
    #[test]
    fn complete_progress_retains_existing_budget_reset_semantics() {
        let mut ledger = ledger();
        let id = format!("CG-A-{:032x}", 2);
        ledger.finishes.get_mut(&id).unwrap().outcome = "ready-to-verify".into();
        let rechecks = BTreeMap::from([(id, "candidate_absent_unverified_policy".into())]);
        assert_eq!(
            history_from_ledger(&ledger, &rechecks, "repair-source", &"b".repeat(64), true)
                .no_progress_count,
            0
        );
    }
    #[test]
    fn sharing_is_limited_to_confirmation_tasks() {
        let mut brief = json!({"kind":"blocker","checker_id":"python.ruff","reason_code":"python_syntax_confirmation_needed"});
        assert!(confirmation_action_budget(&brief));
        brief["reason_code"] = json!("ruff_tool_not_found");
        assert!(!confirmation_action_budget(&brief));
        brief["checker_id"] = json!("syntax.native_confirmation");
        assert!(confirmation_action_budget(&brief));
        brief["kind"] = json!("finding");
        assert!(!confirmation_action_budget(&brief));
    }
}
