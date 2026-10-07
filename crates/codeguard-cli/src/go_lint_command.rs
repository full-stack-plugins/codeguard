//! Go vet 的局部 CLI 原生观察；版本、源码与工具需同轮复核，始终不签发门禁。

use crate::check_budget::parse_check_timeout;
use crate::discovery::discover;
use crate::work_sync::{save_local_report, sync_local_workspace};
use crate::workspace_refresh::read_workspace_baseline;
use codeguard_adapters::go_finding_record;
use codeguard_adapters::{GoVetParseState, legacy_registry, parse_go_vet_json};
use codeguard_runtime::{
    NativeObservation, ProcessSpec, Termination, read_bounded_regular_file, run_process,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static NEXT_SCRATCH: AtomicU64 = AtomicU64::new(0);

struct Arguments {
    root: PathBuf,
    tool: Option<PathBuf>,
    json: bool,
    timeout_ms: u64,
}

struct Scratch(PathBuf);

#[derive(Eq, PartialEq)]
struct GoSourceSnapshot {
    hashes: BTreeMap<String, String>,
    modules: BTreeMap<String, GoModuleIdentity>,
}

#[derive(Eq, PartialEq)]
struct GoModuleIdentity {
    manifest_sha256: String,
    sum_sha256: Option<String>,
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// 执行 `codeguard lint go`，报告局部原生发现和明确的覆盖缺口。
pub fn run(args: &[String]) -> ExitCode {
    let arguments = match parse_args(args) {
        Ok(arguments) => arguments,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    };
    let started = Instant::now();
    let deadline = started + Duration::from_millis(arguments.timeout_ms);
    let root = match arguments.root.canonicalize() {
        Ok(path) if path.is_dir() => path,
        _ => {
            let mut feedback = empty_feedback();
            mark_request_cancellation(&mut feedback);
            return emit(feedback, arguments.json);
        }
    };
    let cancelled = AtomicBool::new(false);
    let selection = crate::go_tool_selection::GoToolSelection::discover(arguments.tool);
    let mut feedback = observe_for_check(&root, selection.tool(), deadline, &cancelled);
    persist_and_sync(&root, &mut feedback);
    if selection.tool().is_none() {
        return crate::go_lint_fallback::run(
            &root,
            feedback,
            selection.report(),
            deadline,
            arguments.json,
        );
    }
    // 持久化身份保持可导入的 incomplete/3；公开反馈按取消契约标 130。
    mark_request_cancellation(&mut feedback);
    emit(feedback, arguments.json)
}

/// 请求取消只改公开反馈的退出语义；不撤销已观察的原生结果或发现。
pub(crate) fn mark_request_cancellation(feedback: &mut Value) {
    if feedback["reason"] == "request_cancelled"
        || codeguard_runtime::sigint_cancellation_requested()
    {
        feedback["command_status"] = json!("cancelled");
        feedback["exit_code"] = json!(130);
    }
}

/// 与 `check all` 共用同一受控 Go 观察，不赋予策略或门禁权威。
pub(crate) fn observe_for_check(
    root: &Path,
    tool: Option<&Path>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    let mut feedback = empty_feedback();
    feedback["root"] = json!(root.to_string_lossy());
    let (binding, id) = match read_workspace_baseline(root) {
        Ok(Some(baseline)) => match baseline.workspace_id() {
            Some(id) => ("bound", json!(id)),
            None => ("legacy_unbound", Value::Null),
        },
        Ok(None) => ("uninitialized", Value::Null),
        Err(_) => ("invalid", Value::Null),
    };
    feedback["workspace_binding"] = json!(binding);
    feedback["workspace_id"] = id;
    observe(root, tool, deadline, cancelled, &mut feedback);
    feedback
}

/// 保存并幂等同步本轮局部报告；存储失败不改变原生结果或签发门禁。
pub(crate) fn persist_and_sync(root: &Path, report: &mut Value) {
    let (status, summary) = match save_local_report(root, report) {
        Ok(()) if report["workspace_binding"] == "bound" => match sync_local_workspace(root) {
            Ok(summary) => (
                if summary.failed_reports == 0 {
                    "synced_partial"
                } else {
                    "backlog_update_failed"
                },
                json!({"new_findings":summary.new_findings,"new_blockers":summary.new_blockers,
                    "imported_reports":summary.imported_reports,"historical_findings":summary.historical_findings,
                    "failed_reports":summary.failed_reports}),
            ),
            Err(_) => ("backlog_update_failed", Value::Null),
        },
        Ok(()) => ("not_initialized", Value::Null),
        Err(_) => ("backlog_update_failed", Value::Null),
    };
    report["backlog_status"] = json!(status);
    report["backlog_sync"] = summary;
}

fn empty_feedback() -> Value {
    json!({
        "schema_version":"0.6.0", "report_type":"go_lint_local_feedback",
        "operation":"lint", "language":"go", "checker_id":"go.vet",
        "workspace_binding":"uninitialized", "workspace_id":null,
        "run_id":format!("go-lint-{}-{}-{}", std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos()),
            NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed)),
        "module_identities":{}, "backlog_status":"not_attempted", "backlog_sync":null,
        "command_status":"incomplete", "exit_code":3,
        "delivery_decision":"not_evaluated", "coverage_proven":false,
        "authority":"local_unverified", "tool_approval":"unverified",
        "rulepack_approval":"unverified", "native_status":"incomplete",
        "reason":"project_path_unavailable", "native_tool_version":null,
        "tool_sha256":null, "manifest_sha256":null, "go_sum_sha256":null,
        "source_snapshot_sha256":null, "source_file_count":0, "module_count":0, "modules_completed":0,
        "module_results":[],
        "native_diagnostic_count":0, "findings":[],
        "native_recheck_argv":["go","vet","-json","./..."],
        "next_actions":["repair_go_tool_or_project_then_recheck"]
    })
}

fn parse_args(args: &[String]) -> Result<Arguments, String> {
    let mut root = None;
    let mut tool = None;
    let mut json = false;
    let mut timeout_ms = 180_000;
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--go-tool" => {
                cursor += 1;
                let value = args.get(cursor).ok_or("--go-tool 缺少绝对路径")?;
                if tool.replace(PathBuf::from(value)).is_some() {
                    return Err("--go-tool 不能重复".into());
                }
            }
            "--format=json" => json = true,
            "--format=human" => json = false,
            "--format" => {
                cursor += 1;
                let value = args.get(cursor).ok_or("--format 缺少 human|json")?;
                json = match value.as_str() {
                    "human" => false,
                    "json" => true,
                    _ => return Err("--format 仅支持 human|json".into()),
                };
            }
            "--timeout" => {
                cursor += 1;
                timeout_ms = parse_check_timeout(args.get(cursor).ok_or("--timeout 缺少值")?)?;
            }
            value if !value.starts_with('-') && root.is_none() => root = Some(PathBuf::from(value)),
            _ => return Err("lint go 参数无效".into()),
        }
        cursor += 1;
    }
    Ok(Arguments {
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        tool,
        json,
        timeout_ms,
    })
}

fn observe(
    root: &Path,
    tool: Option<&Path>,
    deadline: Instant,
    cancelled: &AtomicBool,
    feedback: &mut Value,
) {
    let Some(tool) = tool else {
        feedback["reason"] = json!("go_tool_not_selected");
        return;
    };
    if !tool.is_absolute() {
        feedback["reason"] = json!("go_tool_path_not_absolute");
        return;
    }
    let Ok(tool_path) = tool.canonicalize() else {
        feedback["reason"] = json!("go_tool_unavailable");
        return;
    };
    let Some(tool_sha) = file_sha256(&tool_path, 128 * 1024 * 1024) else {
        feedback["reason"] = json!("go_tool_unavailable");
        return;
    };
    feedback["tool_sha256"] = json!(tool_sha);
    let manifest = root.join("go.mod");
    let Some(manifest_sha) = file_sha256(&manifest, 256 * 1024) else {
        feedback["reason"] = json!("go_mod_unavailable");
        return;
    };
    feedback["manifest_sha256"] = json!(manifest_sha);
    let Some(sum_sha) = optional_file_sha256(&root.join("go.sum"), 4 * 1024 * 1024) else {
        feedback["reason"] = json!("go_sum_unavailable");
        return;
    };
    feedback["go_sum_sha256"] = json!(sum_sha);
    if root.join("go.work").exists() {
        feedback["reason"] = json!("go_workspace_not_validated");
        return;
    }
    let Some(source_before) = source_snapshot(root) else {
        feedback["reason"] = json!("go_source_discovery_incomplete");
        return;
    };
    feedback["source_file_count"] = json!(source_before.hashes.len());
    feedback["source_snapshot_sha256"] = json!(snapshot_sha256(&source_before));
    feedback["module_count"] = json!(source_before.modules.len());
    feedback["module_identities"] = json!(source_before.modules.iter().map(|(module, identity)| {
        (module.clone(), json!({"manifest_sha256":identity.manifest_sha256, "go_sum_sha256":identity.sum_sha256}))
    }).collect::<BTreeMap<_, _>>());
    if !source_before.modules.contains_key(".") {
        feedback["reason"] = json!("go_root_module_unavailable");
        return;
    }
    let Some(scratch) = private_scratch() else {
        feedback["reason"] = json!("private_go_cache_unavailable");
        return;
    };
    let environment = go_environment(&scratch, &tool_path);
    let version = run_process(
        &ProcessSpec {
            executable: tool_path.clone(),
            args: vec![OsString::from("version")],
            cwd: root.to_path_buf(),
            env: environment.clone(),
            stdin: None,
            deadline,
            output_limit_bytes: 4096,
        },
        cancelled,
    );
    if version.termination != Termination::Exited(0) || !version.stderr.is_empty() {
        feedback["reason"] = json!(process_reason(
            version.termination,
            "go_version_probe_incomplete"
        ));
        return;
    }
    let Some(observed_version) = parse_go_version(&version.stdout) else {
        feedback["reason"] = json!("go_tool_version_unvalidated");
        return;
    };
    feedback["native_tool_version"] = json!(observed_version);
    if observed_version != "go1.23.4" {
        feedback["reason"] = json!("go_tool_version_unvalidated");
        return;
    }
    if file_sha256(&tool_path, 128 * 1024 * 1024).as_deref() != Some(tool_sha.as_str()) {
        feedback["reason"] = json!("go_tool_changed");
        return;
    }
    let mut findings = Vec::<Value>::new();
    let mut module_results = Vec::<Value>::new();
    for module_root in source_before.modules.keys() {
        let cwd = if module_root == "." {
            root.to_path_buf()
        } else {
            root.join(module_root)
        };
        let outcome = run_process(
            &ProcessSpec {
                executable: tool_path.clone(),
                args: ["vet", "-json", "./..."]
                    .into_iter()
                    .map(OsString::from)
                    .collect(),
                cwd,
                env: environment.clone(),
                stdin: None,
                deadline,
                output_limit_bytes: 1024 * 1024,
            },
            cancelled,
        );
        if file_sha256(&tool_path, 128 * 1024 * 1024).as_deref() != Some(tool_sha.as_str())
            || source_snapshot(root).as_ref() != Some(&source_before)
        {
            feedback["reason"] = json!("go_inputs_changed_during_scan");
            feedback["findings"] = json!([]);
            feedback["native_diagnostic_count"] = json!(0);
            feedback["modules_completed"] = json!(0);
            feedback["module_results"] = json!([]);
            return;
        }
        let Termination::Exited(exit_code) = outcome.termination else {
            incomplete_module(
                feedback,
                &mut module_results,
                &findings,
                module_root,
                process_reason(outcome.termination, "go_vet_process_incomplete"),
            );
            return;
        };
        let parsed = parse_go_vet_json(
            root,
            "go1.23.4",
            observed_version,
            exit_code,
            &outcome.stdout,
            &outcome.stderr,
        );
        if parsed.state == GoVetParseState::Incomplete {
            incomplete_module(
                feedback,
                &mut module_results,
                &findings,
                module_root,
                parsed.reason.unwrap_or("go_vet_report_incomplete"),
            );
            return;
        }
        if parsed.findings.iter().any(|finding| {
            !source_before.hashes.contains_key(&finding.path)
                || owner_module(&finding.path, &source_before.modules) != Some(module_root.as_str())
        }) {
            incomplete_module(
                feedback,
                &mut module_results,
                &findings,
                module_root,
                "go_vet_diagnostic_outside_discovered_sources",
            );
            return;
        }
        // 同一模块全部位置和字节复核通过后才发布，避免坏诊断留下部分任务。
        let records = (|| -> Result<Vec<Value>, &'static str> {
            let mut records = Vec::new();
            let mut occurrences = BTreeMap::<String, u64>::new();
            let mut seen = BTreeSet::new();
            let mut native_findings = parsed.findings;
            // 原生 JSON 数组顺序不能决定重复锚点的身份序号。
            native_findings.sort_by(|left, right| {
                (
                    &left.path,
                    left.line,
                    left.column,
                    &left.native_rule_id,
                    &left.message,
                    &left.package,
                )
                    .cmp(&(
                        &right.path,
                        right.line,
                        right.column,
                        &right.native_rule_id,
                        &right.message,
                        &right.package,
                    ))
            });
            for finding in native_findings {
                if !seen.insert((
                    finding.path.clone(),
                    finding.line,
                    finding.column,
                    finding.native_rule_id.clone(),
                    finding.message.clone(),
                )) {
                    continue;
                }
                let source = read_bounded_regular_file(&root.join(&finding.path), 16 * 1024 * 1024)
                    .map_err(|_| "go_inputs_changed_during_scan")?;
                let source_sha = format!("{:x}", Sha256::digest(&source));
                if source_before.hashes.get(&finding.path) != Some(&source_sha) {
                    return Err("go_inputs_changed_during_scan");
                }
                let base = go_finding_record(module_root, &finding, &source, 0)
                    .ok_or("go_vet_source_location_invalid")?;
                let key = base["finding_fingerprint"]
                    .as_str()
                    .expect("generated fingerprint")
                    .to_owned();
                let ordinal = occurrences.entry(key).or_default();
                let record = go_finding_record(module_root, &finding, &source, *ordinal)
                    .ok_or("go_vet_source_location_invalid")?;
                *ordinal += 1;
                records.push(record);
            }
            Ok(records)
        })();
        let records = match records {
            Ok(records) => records,
            Err(reason) => {
                if reason == "go_inputs_changed_during_scan" {
                    findings.clear();
                    module_results.clear();
                    feedback["modules_completed"] = json!(0);
                }
                incomplete_module(
                    feedback,
                    &mut module_results,
                    &findings,
                    module_root,
                    reason,
                );
                return;
            }
        };
        if source_snapshot(root).as_ref() != Some(&source_before) {
            findings.clear();
            module_results.clear();
            feedback["modules_completed"] = json!(0);
            incomplete_module(
                feedback,
                &mut module_results,
                &findings,
                module_root,
                "go_inputs_changed_during_scan",
            );
            return;
        }
        let count = records.len();
        findings.extend(records);
        module_results.push(json!({
            "root":module_root,
            "status":if count == 0 {"clean_observed_unverified"} else {"findings_observed_unverified"},
            "diagnostic_count":count
        }));
        feedback["modules_completed"] = json!(module_results.len());
    }
    feedback["module_results"] = json!(module_results);
    feedback["native_diagnostic_count"] = json!(findings.len());
    feedback["native_status"] = json!(if findings.is_empty() {
        "clean_observed_unverified"
    } else {
        "findings_observed_unverified"
    });
    feedback["findings"] = json!(findings);
    feedback["reason"] = json!("project_scope_and_policy_unverified");
    feedback["next_actions"] = json!(
        if feedback["findings"].as_array().is_some_and(Vec::is_empty) {
            vec!["validate_go_scope_and_remaining_categories"]
        } else {
            vec![
                "inspect_native_go_vet_diagnostics",
                "repair_source_then_recheck",
            ]
        }
    );
}

/// 返回当前已发现 Go 源码的整组字节身份；未知或不完整时返回空。
pub(crate) fn current_source_snapshot_sha256(root: &Path) -> Option<String> {
    source_snapshot(root).map(|snapshot| snapshot_sha256(&snapshot))
}

/// 返回本轮原生 vet 与默认包清单共同确认的源码摘要；任一身份或范围不完整则不跳过 WASM。
#[cfg(feature = "wasm-precheck")]
pub(crate) fn selected_sources_for_candidate(
    root: &Path,
    tool: Option<&Path>,
    scan: &Value,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Option<BTreeMap<String, String>> {
    let tool = tool?.canonicalize().ok()?;
    let before = source_snapshot(root)?;
    let tool_sha = file_sha256(&tool, 128 * 1024 * 1024)?;
    let identities = before
        .modules
        .iter()
        .map(|(module, identity)| {
            (
                module.clone(),
                json!({"manifest_sha256":identity.manifest_sha256,"go_sum_sha256":identity.sum_sha256}),
            )
        })
        .collect::<BTreeMap<_, _>>();
    if scan["root"] != root.to_string_lossy().as_ref()
        || scan["native_tool_version"] != "go1.23.4"
        || !matches!(
            scan["native_status"].as_str(),
            Some("clean_observed_unverified" | "findings_observed_unverified")
        )
        || scan["tool_sha256"] != tool_sha
        || scan["source_snapshot_sha256"] != snapshot_sha256(&before)
        || scan["source_file_count"] != before.hashes.len()
        || scan["module_count"] != before.modules.len()
        || scan["modules_completed"] != before.modules.len()
        || scan["module_identities"] != json!(identities)
        || scan["module_results"].as_array().is_none_or(|results| {
            let roots = results
                .iter()
                .filter(|result| {
                    matches!(
                        result["status"].as_str(),
                        Some("clean_observed_unverified" | "findings_observed_unverified")
                    )
                })
                .filter_map(|result| result["root"].as_str())
                .collect::<BTreeSet<_>>();
            results.len() != before.modules.len()
                || roots
                    != before
                        .modules
                        .keys()
                        .map(String::as_str)
                        .collect::<BTreeSet<_>>()
        })
    {
        return None;
    }
    let scratch = private_scratch()?;
    let environment = go_environment(&scratch, &tool);
    let mut selected = BTreeMap::new();
    for module in before.modules.keys() {
        let directory = if module == "." {
            root.to_path_buf()
        } else {
            root.join(module)
        };
        let outcome = run_process(
            &ProcessSpec {
                executable: tool.clone(),
                args: ["list", "-json", "./..."]
                    .into_iter()
                    .map(OsString::from)
                    .collect(),
                cwd: directory.clone(),
                env: environment.clone(),
                stdin: None,
                deadline,
                output_limit_bytes: 8 * 1024 * 1024,
            },
            cancelled,
        );
        if outcome.termination != Termination::Exited(0) || !outcome.stderr.is_empty() {
            return None;
        }
        let listed = codeguard_adapters::parse_go_list_scope(&directory, &outcome.stdout).ok()?;
        for path in listed.active_files.iter().chain(&listed.ignored_files) {
            let path = if module == "." {
                path.clone()
            } else {
                format!("{module}/{path}")
            };
            if !before.hashes.contains_key(&path)
                || owner_module(&path, &before.modules) != Some(module.as_str())
            {
                return None;
            }
        }
        for path in listed.active_files {
            let path = if module == "." {
                path
            } else {
                format!("{module}/{path}")
            };
            if selected
                .insert(path.clone(), before.hashes[&path].clone())
                .is_some()
            {
                return None;
            }
        }
    }
    if source_snapshot(root).as_ref() != Some(&before)
        || file_sha256(&tool, 128 * 1024 * 1024).as_deref() != Some(tool_sha.as_str())
    {
        return None;
    }
    Some(selected)
}

fn snapshot_sha256(snapshot: &GoSourceSnapshot) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&snapshot.hashes).expect("源码摘要可编码"))
    )
}

/// 用同一受控工具与默认构建环境观察目标是否进入原生包清单；不认证全部覆盖。
pub(crate) fn observe_target_scope(
    root: &Path,
    target: &str,
    tool: Option<&Path>,
    scan: &Value,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    let mut scope = json!({"schema_version":"0.1.0","status":"incomplete",
        "reason":"go_scope_prerequisite_unavailable","module_root":null,
        "source_sha256":null,"source_snapshot_sha256":null,"tool_sha256":null,
        "manifest_sha256":null,"go_sum_sha256":null,"package_count":0,
        "active_file_count":0,"ignored_file_count":0,"source_map_sha256":null,
        "build_conditions":{"cgo_enabled":false,"tags":"default","platform":"tool_host_default"},
        "native_scope_argv":["go","list","-json","./..."]});
    if scan["native_tool_version"] != "go1.23.4"
        || !matches!(
            scan["native_status"].as_str(),
            Some("clean_observed_unverified" | "findings_observed_unverified")
        )
    {
        scope["reason"] = json!("go_vet_scope_prerequisite_incomplete");
        return scope;
    }
    let Some(tool) = tool else {
        return scope;
    };
    let Some(before) = source_snapshot(root) else {
        return scope;
    };
    let Some(source_sha) = before.hashes.get(target) else {
        return scope;
    };
    let Some(module) = owner_module(target, &before.modules) else {
        return scope;
    };
    let Some(tool_sha) = file_sha256(tool, 128 * 1024 * 1024) else {
        return scope;
    };
    if scan["source_snapshot_sha256"] != snapshot_sha256(&before) || scan["tool_sha256"] != tool_sha
    {
        scope["reason"] = json!("go_scope_inputs_changed");
        return scope;
    }
    let Some(scratch) = private_scratch() else {
        return scope;
    };
    let identity = &before.modules[module];
    scope["module_root"] = json!(module);
    scope["source_sha256"] = json!(source_sha);
    scope["source_snapshot_sha256"] = json!(snapshot_sha256(&before));
    scope["tool_sha256"] = json!(tool_sha);
    scope["manifest_sha256"] = json!(identity.manifest_sha256);
    scope["go_sum_sha256"] = json!(identity.sum_sha256);
    let directory = if module == "." {
        root.to_path_buf()
    } else {
        root.join(module)
    };
    let outcome = run_process(
        &ProcessSpec {
            executable: tool.to_path_buf(),
            args: ["list", "-json", "./..."]
                .into_iter()
                .map(OsString::from)
                .collect(),
            cwd: directory.clone(),
            env: go_environment(&scratch, tool),
            stdin: None,
            deadline,
            output_limit_bytes: 8 * 1024 * 1024,
        },
        cancelled,
    );
    if source_snapshot(root).as_ref() != Some(&before)
        || file_sha256(tool, 128 * 1024 * 1024).as_deref() != Some(tool_sha.as_str())
    {
        scope["reason"] = json!("go_scope_inputs_changed");
        return scope;
    }
    if outcome.termination != Termination::Exited(0) || !outcome.stderr.is_empty() {
        scope["reason"] = json!(process_reason(
            outcome.termination,
            "go_list_execution_incomplete"
        ));
        return scope;
    }
    let listed = match codeguard_adapters::parse_go_list_scope(&directory, &outcome.stdout) {
        Ok(listed) => listed,
        Err(reason) => {
            scope["reason"] = json!(reason);
            return scope;
        }
    };
    let project_path = |path: &str| {
        if module == "." {
            path.to_owned()
        } else {
            format!("{module}/{path}")
        }
    };
    if listed
        .active_files
        .iter()
        .chain(&listed.ignored_files)
        .any(|path| {
            let path = project_path(path);
            !before.hashes.contains_key(&path)
                || owner_module(&path, &before.modules) != Some(module)
        })
    {
        scope["reason"] = json!("go_list_source_not_discovered");
        return scope;
    }
    let relative = Path::new(target)
        .strip_prefix(Path::new(if module == "." { "" } else { module }))
        .ok()
        .and_then(|path| path.to_str())
        .unwrap_or("");
    let (status, reason) = if listed.active_files.contains(relative) {
        ("selected", "go_target_selected_default_build")
    } else if listed.ignored_files.contains(relative) {
        ("excluded", "go_target_excluded_by_build_constraints")
    } else {
        ("not_selected", "go_target_not_in_native_package_scope")
    };
    scope["status"] = json!(status);
    scope["reason"] = json!(reason);
    scope["package_count"] = json!(listed.package_count);
    scope["active_file_count"] = json!(listed.active_files.len());
    scope["ignored_file_count"] = json!(listed.ignored_files.len());
    scope["source_map_sha256"] = json!(format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&json!({
        "active":listed.active_files,"ignored":listed.ignored_files}))
            .expect("原生选择集合可编码")
        )
    ));
    scope
}

fn source_snapshot(root: &Path) -> Option<GoSourceSnapshot> {
    let registry = legacy_registry().ok()?;
    let discovery = discover(root, &registry, &NativeObservation);
    if !discovery.observation_complete {
        return None;
    }
    let evidence = discovery.languages.get("go")?;
    let sources: &BTreeSet<String> = &evidence.source_files;
    if sources.is_empty() || sources.len() > 10_000 || evidence.manifests.len() > 64 {
        return None;
    }
    let mut hashes = BTreeMap::new();
    for source in sources {
        hashes.insert(
            source.clone(),
            file_sha256(&root.join(source), 16 * 1024 * 1024)?,
        );
    }
    let mut modules = BTreeMap::new();
    for manifest in &evidence.manifests {
        if Path::new(manifest)
            .file_name()
            .and_then(|name| name.to_str())
            != Some("go.mod")
        {
            return None;
        }
        let module_root = Path::new(manifest)
            .parent()
            .and_then(|path| path.to_str())
            .filter(|path| !path.is_empty())
            .unwrap_or(".")
            .replace('\\', "/");
        let manifest_sha256 = file_sha256(&root.join(manifest), 256 * 1024)?;
        let sum_sha256 =
            optional_file_sha256(&root.join(&module_root).join("go.sum"), 4 * 1024 * 1024)?;
        if modules
            .insert(
                module_root,
                GoModuleIdentity {
                    manifest_sha256,
                    sum_sha256,
                },
            )
            .is_some()
        {
            return None;
        }
    }
    Some(GoSourceSnapshot { hashes, modules })
}

fn owner_module<'a>(
    path: &str,
    modules: &'a BTreeMap<String, GoModuleIdentity>,
) -> Option<&'a str> {
    modules
        .keys()
        .filter(|root| {
            root.as_str() == "." || Path::new(path).starts_with(Path::new(root.as_str()))
        })
        .max_by_key(|root| root.len())
        .map(String::as_str)
}

fn incomplete_module(
    feedback: &mut Value,
    module_results: &mut Vec<Value>,
    findings: &[Value],
    module_root: &str,
    reason: &str,
) {
    module_results.push(json!({
        "root":module_root,
        "status":"incomplete",
        "diagnostic_count":0,
        "reason":reason
    }));
    feedback["module_results"] = json!(module_results);
    feedback["findings"] = json!(findings);
    feedback["native_diagnostic_count"] = json!(findings.len());
    feedback["reason"] = json!(reason);
    feedback["next_actions"] = json!(["repair_go_module_environment_then_recheck"]);
}

fn process_reason(termination: Termination, fallback: &'static str) -> &'static str {
    match termination {
        Termination::TimedOut | Termination::DeadlineBeforeStart => "request_deadline_exceeded",
        Termination::Cancelled => "request_cancelled",
        _ => fallback,
    }
}

fn file_sha256(path: &Path, limit: u64) -> Option<String> {
    let bytes = read_bounded_regular_file(path, limit).ok()?;
    Some(format!("{:x}", Sha256::digest(bytes)))
}

fn optional_file_sha256(path: &Path, limit: u64) -> Option<Option<String>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => file_sha256(path, limit).map(Some),
        Ok(_) => None,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some(None),
        Err(_) => None,
    }
}

fn private_scratch() -> Option<Scratch> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_nanos();
    let sequence = NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "codeguard-go-lint-{}-{stamp}-{sequence}",
        std::process::id()
    ));
    fs::DirBuilder::new().mode(0o700).create(&path).ok()?;
    let scratch = Scratch(path);
    for name in ["cache", "gopath", "tmp"] {
        fs::DirBuilder::new()
            .mode(0o700)
            .create(scratch.0.join(name))
            .ok()?;
    }
    Some(scratch)
}

fn go_environment(scratch: &Scratch, tool: &Path) -> BTreeMap<OsString, OsString> {
    let mut env = BTreeMap::new();
    for (key, value) in [
        ("GOPROXY", "off"),
        ("GOSUMDB", "off"),
        ("GOTOOLCHAIN", "local"),
        ("GOWORK", "off"),
        ("GOENV", "off"),
        ("CGO_ENABLED", "0"),
        ("GOFLAGS", ""),
    ] {
        env.insert(OsString::from(key), OsString::from(value));
    }
    for (key, path) in [
        ("GOCACHE", scratch.0.join("cache")),
        ("GOPATH", scratch.0.join("gopath")),
        ("TMPDIR", scratch.0.join("tmp")),
        ("HOME", scratch.0.clone()),
    ] {
        env.insert(OsString::from(key), path.into_os_string());
    }
    if let Some(parent) = tool.parent() {
        env.insert(OsString::from("PATH"), parent.as_os_str().to_os_string());
    }
    env
}

fn parse_go_version(stdout: &[u8]) -> Option<&str> {
    let raw = std::str::from_utf8(stdout).ok()?;
    let parts: Vec<&str> = raw.split_whitespace().collect();
    if parts.len() != 4 || parts[0] != "go" || parts[1] != "version" {
        return None;
    }
    Some(parts[2])
}

fn emit(feedback: Value, json_format: bool) -> ExitCode {
    let exit = feedback["exit_code"].as_u64().unwrap_or(3) as u8;
    if json_format {
        println!("{feedback}");
    } else {
        if feedback["command_status"] == "cancelled" {
            println!("Go 检查已取消（退出 130）；保留已观察结果，不签发任何通过结论");
        }
        println!(
            "Go vet：{}",
            feedback["native_status"].as_str().unwrap_or("incomplete")
        );
        println!(
            "原生诊断：{}；交付门禁：未评估",
            feedback["native_diagnostic_count"]
        );
        for finding in feedback["findings"].as_array().into_iter().flatten() {
            println!(
                "{}:{}:{} {}",
                finding["path"].as_str().unwrap_or("?"),
                finding["line"],
                finding["column"],
                finding["rule_id"].as_str().unwrap_or("?")
            );
        }
        println!(
            "修复工作台：{}；下一步可查询 codeguard next",
            feedback["backlog_status"]
                .as_str()
                .unwrap_or("not_attempted")
        );
        println!(
            "待完成：{}",
            feedback["reason"].as_str().unwrap_or("unknown")
        );
    }
    ExitCode::from(exit)
}
