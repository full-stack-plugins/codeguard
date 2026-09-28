//! 公开npm局部CVE反馈；完整数据库、策略和任务关闭尚未授权。
use crate::{
    doctor_scratch::DoctorScratch, npm_audit_arguments::NpmAuditArguments,
    npm_audit_probe::run_npm_audit_probe, npm_audit_probe_request::NpmAuditProbeRequest,
};
use codeguard_adapters::{NpmAuditCommand, inspect_npm_audit_config};
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    process::ExitCode,
    sync::atomic::AtomicBool,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// 执行显式npm审计并反馈脱敏原生观察；局部调用始终不签发完整CVE通过。
pub fn run(args: &[String]) -> ExitCode {
    let started = Instant::now();
    let args = match NpmAuditArguments::parse(args) {
        Ok(a) => a,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    };
    let cli_timeout = args
        .options
        .contains_key("--timeout")
        .then_some(args.timeout_ms);
    let (timeout_ms, source) = match crate::check_budget::select_check_timeout(cli_timeout) {
        Ok(value) => value,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    };
    let mut report = match crate::npm_workspace_scope::NpmWorkspaceScope::resolve(&args) {
        Err(reason) => feedback(reason),
        Ok(scope) => {
            match crate::check_budget::resolve_project_default(&scope.workspace, timeout_ms, source)
            {
                Ok((timeout_ms, source)) => {
                    let mut report = observe(
                        &args,
                        &scope,
                        started + Duration::from_millis(timeout_ms),
                        true,
                    )
                    .0;
                    report["execution_budget"] =
                        crate::check_budget::budget_record(timeout_ms, source);
                    report
                }
                Err(_) => {
                    let mut report = feedback("npm_runtime_defaults_invalid");
                    report["next_action"] = json!(
                        "修正工作区codeguard/runtime.json的版本、类型和预算字段，禁止加入规则排除或白名单；随后重新执行原生审计"
                    );
                    report
                }
            }
        }
    };
    report["schema_version"] = json!("0.3.0");
    if args.json {
        println!("{report}");
    } else {
        println!("npm审计：{}；完整CVE检查尚未判定", report["status"]);
        println!(
            "原因：{}；依赖观察数量：{}",
            report["reason"], report["dependency_total"]
        );
        for finding in report["findings"].as_array().into_iter().flatten() {
            println!(
                "组件引用 {}；原生严重度 {}；advisory {}",
                finding["component_ref"], finding["severity"], finding["advisory_sources"]
            );
        }
        println!("下一步：{}", report["next_action"]);
    }
    ExitCode::from(if report["reason"] == "cancelled" {
        130
    } else {
        3
    })
}
pub(crate) fn observe(
    args: &NpmAuditArguments,
    scope: &crate::npm_workspace_scope::NpmWorkspaceScope,
    deadline: Instant,
    connect: bool,
) -> (Value, Option<Value>) {
    let mut report = feedback("npm_execution_context_missing");
    if codeguard_runtime::sigint_cancellation_requested() {
        return (feedback("cancelled"), None);
    }
    if Instant::now() >= deadline {
        return (feedback("deadline"), None);
    }
    let initial_root = scope.project.clone();
    let mut initial_manifest_sha256 = None;
    if let Ok(bytes) = read_bounded_regular_file(&initial_root.join("package.json"), 256 * 1024) {
        initial_manifest_sha256 = Some(format!("{:x}", Sha256::digest(&bytes)));
        report["configuration"] = json!(inspect_npm_audit_config(&bytes, ".", "package.json"));
    }
    let manifest_invalid = matches!(
        report["configuration"]["reason"].as_str(),
        Some("npm_manifest_invalid" | "npm_scripts_invalid")
    );
    let Some(id) = next_run_id(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos()),
    ) else {
        return (feedback("npm_run_identity_exhausted"), None);
    };
    let get = |name: &str| args.options.get(name);
    let context = (
        get("--node-tool"),
        get("--npm-entry"),
        get("--npm-version"),
        get("--userconfig"),
        get("--globalconfig"),
    );
    let missing_lock = matches!(std::fs::symlink_metadata(scope.project.join("package-lock.json")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound);
    let lock_unavailable =
        crate::npm_input_state::observe(&scope.project.join("package-lock.json"), 8 * 1024 * 1024)
            .0
            != "present";
    if manifest_invalid
        || initial_manifest_sha256.is_none()
        || lock_unavailable
        || missing_lock
        || !matches!(context, (Some(_), Some(_), Some(_), Some(_), Some(_)))
    {
        if missing_lock {
            report["reason"] = json!("npm_lock_missing");
        }
        let observation =
            crate::npm_workbench::prepare_preparation(&scope.workspace, &scope.project, &id);
        match observation {
            Ok(observation) => {
                if observation["manifest_sha256"].as_str() != initial_manifest_sha256.as_deref() {
                    return (feedback("npm_input_changed"), None);
                }
                if missing_lock && observation["lock_state"] != "missing" {
                    return (feedback("npm_input_changed"), None);
                }
                report["reason"] = observation["diagnostic_reason"].clone();
                if codeguard_runtime::sigint_cancellation_requested() {
                    return (feedback("cancelled"), None);
                }
                if Instant::now() >= deadline {
                    return (feedback("deadline"), None);
                }
                if connect {
                    let mut pending = json!({"feedback":report,"observation":observation,
                        "backlog_status":"not_connected"});
                    crate::npm_check_scan::sync(&scope.workspace, &mut pending, deadline);
                    return (
                        pending["feedback"].take(),
                        Some(pending["observation"].take()),
                    );
                }
                return (report, Some(observation));
            }
            Err(reason) => {
                report["workbench_status"] = json!(reason);
                return (report, None);
            }
        }
    }
    let (Some(node), Some(entry), Some(version), Some(user), Some(global)) = context else {
        unreachable!("上下文完整性已检查");
    };
    let cwd = scope.project.clone();
    let Some(scratch) = DoctorScratch::create(&id) else {
        return (feedback("npm_private_workspace_unavailable"), None);
    };
    let command = NpmAuditCommand {
        node: node.into(),
        entry: entry.into(),
        cache: scratch.path().join("cache"),
        user_config: user.into(),
        global_config: global.into(),
    };
    if get("--registry")
        .map_or_else(|| command.args(), |r| command.args_for_registry(r))
        .is_err()
    {
        return (feedback("npm_command_invalid"), None);
    }
    let mut inputs = vec![
        (command.node.clone(), 128 * 1024 * 1024),
        (command.entry.clone(), 16 * 1024 * 1024),
        (cwd.join("package.json"), 256 * 1024),
        (cwd.join("package-lock.json"), 8 * 1024 * 1024),
        (command.user_config.clone(), 1024 * 1024),
        (command.global_config.clone(), 1024 * 1024),
    ];
    let npmrc = cwd.join(".npmrc");
    match std::fs::symlink_metadata(&npmrc) {
        Ok(_) => inputs.push((npmrc, 1024 * 1024)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return (feedback("npm_project_config_unreadable"), None),
    };
    let mut hashes = BTreeMap::<PathBuf, [u8; 32]>::new();
    for (path, limit) in inputs {
        if codeguard_runtime::sigint_cancellation_requested() {
            return (feedback("cancelled"), None);
        }
        if Instant::now() >= deadline {
            return (feedback("deadline"), None);
        }
        if path.canonicalize().ok().as_ref() != Some(&path) {
            return (feedback("npm_input_path_invalid"), None);
        }
        let Ok(bytes) = read_bounded_regular_file(&path, limit) else {
            return (feedback("npm_input_unavailable"), None);
        };
        if path == cwd.join("package.json") {
            if Some(format!("{:x}", Sha256::digest(&bytes))) != initial_manifest_sha256 {
                return (feedback("npm_input_changed"), None);
            }
            report["configuration"] = json!(inspect_npm_audit_config(&bytes, ".", "package.json"));
        }
        if hashes.insert(path, Sha256::digest(bytes).into()).is_some() {
            return (feedback("npm_input_identity_missing"), None);
        }
    }
    let request = NpmAuditProbeRequest {
        command,
        cwd,
        evidence_dir: scratch.path().into(),
        run_id: id,
        expected_version: version.clone(),
        registry: get("--registry").cloned(),
        expected_sha256: hashes,
        deadline,
    };
    let result = run_npm_audit_probe(&request, &AtomicBool::new(false));
    let observation = crate::npm_workbench::prepare(&scope.workspace, &request, &result).ok();
    let workbench = if connect {
        crate::npm_workbench::connect(&scope.workspace, &request, &result)
    } else {
        json!({"status":"not_connected"})
    };
    report["workbench_status"] = workbench["status"].clone();
    report["workbench"] = workbench;
    if report["workbench_status"] == "synced_partial" {
        report["next_action"] = json!(
            "运行codeguard next取得稳定npm修复任务；先核验环境与漏洞源覆盖，再按原工具复检，零发现不自动关闭任务"
        );
    }

    report["local_coherent"] = json!(result.local_coherent);
    report["reason"] = json!(
        result
            .reason
            .unwrap_or("npm_database_and_policy_unverified")
    );
    if result.local_coherent {
        report["status"] = json!("local_observation");
    }
    report["findings"] = json!(crate::npm_observation_projection::project(&result));
    if let Some(parsed) = result.parsed {
        report["dependency_total"] = json!(parsed.native_dependency_total);
    }

    (report, observation)
}
pub(crate) fn feedback(reason: &str) -> Value {
    json!({"schema_version":"0.3.0","report_type":"npm_cve_feedback","status":"incomplete","reason":reason,"local_coherent":false,"configuration":null,"dependency_total":null,"findings":[],"advisory_coverage":"not_evaluated","tool_approval":"unverified","delivery_decision":"not_evaluated","workbench_status":"not_connected","workbench":null,"execution_budget":null,"next_action":"提供明确Node/npm版本、原用户及全局配置、锁文件和审计源；核对漏洞数据覆盖及时效，按原生建议修订依赖并用同一工具复检；未完成时先诊断环境，不添加白名单跳过"})
}

/// 为任务复检返回待持久化原生观察，持久化仍由任务租约保护。
pub(crate) fn observe_for_task(
    root: &std::path::Path,
    args: &NpmAuditArguments,
    deadline: Instant,
) -> Result<Value, &'static str> {
    let scope = crate::npm_workspace_scope::NpmWorkspaceScope::resolve(args)?;
    if scope.workspace != root {
        return Err("npm_workspace_mismatch");
    }
    let (feedback, report) = observe(args, &scope, deadline, false);
    report.ok_or_else(|| match feedback["reason"].as_str() {
        Some("cancelled") => "request_cancelled",
        Some("deadline") => "request_deadline_exceeded",
        Some("npm_input_unavailable") => "npm_input_unavailable",
        Some("npm_input_path_invalid") => "npm_input_path_invalid",
        Some("npm_execution_context_missing") => "npm_execution_context_missing",
        _ => "npm_recheck_observation_unavailable",
    })
}

static NEXT_RUN_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn next_run_id(nanos: u128) -> Option<String> {
    let sequence = NEXT_RUN_SEQUENCE
        .fetch_update(
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed,
            |v| v.checked_add(1),
        )
        .ok()?;
    Some(format!("npm-{}-{sequence}-{nanos}", std::process::id()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn concurrent_identity_uses_sequence_even_when_clock_ticks_are_equal() {
        let ids: std::collections::BTreeSet<_> = (0..64)
            .map(|_| super::next_run_id(123456).unwrap())
            .collect();
        assert_eq!(ids.len(), 64);
        assert!(
            ids.iter()
                .all(|id| id.len() <= 64 && id.ends_with("-123456"))
        );
    }
}
