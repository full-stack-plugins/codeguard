//! 全项目检查入口的保守聚合：执行已接入的原生检查，候选类别不自升为必需义务。

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use codeguard_adapters::{capability_row, legacy_registry};
use codeguard_core::{CHECK_CATEGORIES, TaskGraph, TaskNode};
use codeguard_runtime::{
    NativeObservation, SourceSnapshot, TaskExecution, TaskOutcome, run_task_graph,
};
use serde_json::{Value, json};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::check_budget::{
    check_budget_record, parse_check_jobs, parse_check_timeout, resolve_check_runtime,
    select_check_jobs, select_check_timeout,
};
use crate::discovery::{DiscoveryReport, discover};
use crate::go_lint_command::observe_for_check as observe_go_vet;
use crate::java_checker_config_status::{checker_for_category, summarize};
use crate::java_cve_attribution::attach_candidates;
use crate::java_cve_scan::{
    NativeContext as CveNativeContext, observe_project as observe_cve_project,
};
use crate::java_dependency_scan::{
    NativeContext as DependencyNativeContext, observe_project as observe_dependency_project,
};
use crate::java_javadoc_scan::{
    NativeContext as JavadocNativeContext, observe_project as observe_javadoc_project,
};
use crate::java_p3c_scan::{NativeContext, observe_project};
use crate::next_command::{read_local_brief, read_local_brief_for_checker};
use crate::partial_sarif_feedback::partial_check_sarif;
use crate::python_lint_command::{
    annotate_conversation_budget, scan_and_sync_report_with_deadline,
};
use crate::report_export::export_report;
use crate::rust_lint_scan::observe_cargo_clippy;
use crate::work_sync::{save_local_report, sync_local_workspace};

struct Args {
    selection: Selection,
    root: PathBuf,
    ruff_tool: Option<PathBuf>,
    pip_audit_tool: Option<PathBuf>,
    pip_audit_version: Option<String>,
    npm_options: BTreeMap<String, String>,
    cargo_tool: Option<PathBuf>,
    cargo_audit_tool: Option<PathBuf>,
    rustsec_db: Option<PathBuf>,
    go_tool: Option<PathBuf>,
    erl_tool: Option<PathBuf>,
    maven_tool: Option<PathBuf>,
    java_home: Option<PathBuf>,
    maven_repo: Option<PathBuf>,
    repo_sha256: Option<String>,
    cve_data_dir: Option<PathBuf>,
    cve_data_sha256: Option<String>,
    format: OutputFormat,
    output: Option<PathBuf>,
    timeout_ms: u64,
    timeout_source: &'static str,
    jobs_limit: usize,
    jobs_source: &'static str,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Selection {
    All,
    Java,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum OutputFormat {
    Human,
    Json,
    Sarif,
}

impl Selection {
    fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Java => "java",
        }
    }
}

/// 执行已接入的原生检查并报告待确认的候选类别；当前绝不签发 allow。
pub fn run(args: &[String]) -> ExitCode {
    let mut parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    };
    let started = Instant::now();
    let root = match parsed.root.canonicalize() {
        Ok(root) if root.is_dir() => root,
        _ => {
            eprintln!("项目路径不可读取");
            return ExitCode::from(3);
        }
    };
    match resolve_check_runtime(
        &root,
        (parsed.timeout_ms, parsed.timeout_source),
        (parsed.jobs_limit, parsed.jobs_source),
    ) {
        Ok(((timeout_ms, timeout_source), (jobs_limit, jobs_source))) => {
            parsed.timeout_ms = timeout_ms;
            parsed.timeout_source = timeout_source;
            parsed.jobs_limit = jobs_limit;
            parsed.jobs_source = jobs_source;
        }
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    }
    let deadline = started + Duration::from_millis(parsed.timeout_ms);
    let registry = match legacy_registry() {
        Ok(registry) => registry,
        Err(_) => {
            eprintln!("内置语言清单损坏");
            return ExitCode::from(4);
        }
    };
    let discovery = discover(&root, &registry, &NativeObservation);
    let source_paths: BTreeSet<PathBuf> = discovery
        .languages
        .iter()
        .filter(|(language, _)| parsed.selection == Selection::All || language.as_str() == "java")
        .flat_map(|(_, evidence)| evidence.source_files.iter().map(PathBuf::from))
        .collect();
    let source_snapshot = if source_paths.is_empty() {
        Ok(None)
    } else if Instant::now() >= deadline {
        Err("project_source_snapshot_deadline_exceeded")
    } else {
        SourceSnapshot::capture(
            &root,
            source_paths,
            4096,
            16 * 1024 * 1024,
            256 * 1024 * 1024,
        )
        .map(Some)
        .map_err(|_| "project_source_snapshot_unavailable")
    };
    let python_present = parsed.selection == Selection::All
        && discovery
            .languages
            .get("python")
            .is_some_and(|evidence| !evidence.source_files.is_empty());
    let rust_sources = discovery
        .languages
        .get("rust")
        .map(|evidence| &evidence.source_files);
    let rust_present =
        parsed.selection == Selection::All && rust_sources.is_some_and(|files| !files.is_empty());
    let node_sources = if parsed.selection == Selection::All {
        crate::check_eslint_scan::sources(&discovery)
    } else {
        BTreeSet::new()
    };
    let node_present = !node_sources.is_empty();
    let go_present = parsed.selection == Selection::All
        && discovery
            .languages
            .get("go")
            .is_some_and(|evidence| !evidence.source_files.is_empty());
    let erlang_sources = if parsed.selection == Selection::All {
        discovery
            .languages
            .get("erlang")
            .map(|evidence| evidence.source_files.clone())
            .unwrap_or_default()
    } else {
        BTreeSet::new()
    };
    let erlang_present = !erlang_sources.is_empty();
    let java_sources = discovery
        .languages
        .get("java")
        .map(|evidence| &evidence.source_files);
    let java_present = java_sources.is_some_and(|files| !files.is_empty());
    let dependency_configured = discovery.checker_configurations.iter().any(|entry| {
        entry.checker_id == "java.maven.dependency" && entry.configuration == "configured"
    });
    let cve_configured = discovery.checker_configurations.iter().any(|entry| {
        entry.checker_id == "java.maven.dependency_check" && entry.configuration == "configured"
    });
    let javadoc_configured = java_present
        && discovery.checker_configurations.iter().any(|entry| {
            entry.checker_id == "java.maven.javadoc" && entry.configuration == "configured"
        });
    let javadoc_configuration_unresolved = java_present
        && !javadoc_configured
        && discovery.checker_configurations.iter().any(|entry| {
            entry.checker_id == "java.maven.javadoc"
                && matches!(entry.configuration.as_str(), "unknown" | "invalid")
        });
    let mut npm_roots: BTreeMap<String, (String, Option<String>)> = BTreeMap::new();
    let python_cve_roots: BTreeMap<String, String> = if parsed.selection == Selection::All {
        discovery
            .checker_configurations
            .iter()
            .filter(|entry| entry.checker_id == "python.pip_audit")
            .filter(|entry| {
                let manifest = if entry.build_root == "." {
                    "pyproject.toml".to_owned()
                } else {
                    format!("{}/pyproject.toml", entry.build_root)
                };
                discovery.manifest_sha256.contains_key(&manifest)
            })
            .enumerate()
            .map(|(index, entry)| (format!("python.cve.{index}"), entry.build_root.clone()))
            .collect()
    } else {
        BTreeMap::new()
    };
    let mut historical_npm_scope_error = None;
    let mut recorded_npm_scope_error = None;
    if parsed.selection == Selection::All && cfg!(unix) {
        let mut scopes = BTreeMap::new();
        for entry in discovery
            .checker_configurations
            .iter()
            .filter(|entry| entry.checker_id == "node.npm.audit")
        {
            let manifest = if entry.build_root == "." {
                "package.json".into()
            } else {
                format!("{}/package.json", entry.build_root)
            };
            if let Some(digest) = discovery.manifest_sha256.get(&manifest) {
                scopes.insert(entry.build_root.clone(), Some(digest.clone()));
            }
        }
        match crate::npm_historical_roots::collect(&root) {
            Ok(roots) => {
                for build in roots {
                    scopes.entry(build).or_insert(None);
                }
            }
            Err(reason) => historical_npm_scope_error = Some(reason),
        }
        match crate::npm_recorded_roots::collect(&root) {
            Ok(roots) => {
                for build in roots {
                    scopes.entry(build).or_insert(None);
                }
            }
            Err(reason) => recorded_npm_scope_error = Some(reason),
        }
        npm_roots = scopes
            .into_iter()
            .enumerate()
            .map(|(i, (build, digest))| (format!("npm.cve.{i}"), (build, digest)))
            .collect();
    }
    let mut npm_cve = Vec::<Value>::new();
    let mut npm_outcomes = Vec::new();
    let mut python_cve = Vec::<Value>::new();
    let mut python_cve_outcomes = Vec::new();
    let mut execution_tasks = Vec::new();
    let mut python_task_outcome = None;
    let mut rust_task_outcome = None;
    let mut rust_comments_outcome = None;
    let mut rust_build_outcome = None;
    let mut rust_cve_outcome = None;
    let mut node_task_outcome = None;
    let mut go_task_outcome = None;
    let mut erlang_task_outcome = None;
    let mut java_task_outcome = None;
    let mut javadoc_task_outcome = None;
    let mut dependency_task_outcome = None;
    let mut cve_task_outcome = None;
    let mut python_lint = Value::Null;
    let mut rust_lint = Value::Null;
    let mut rust_comments = Value::Null;
    let mut rust_build = Value::Null;
    let mut rust_cve = Value::Null;
    let mut node_lint = Value::Null;
    let mut go_lint = Value::Null;
    let mut erlang_lint = Value::Null;
    let mut java_p3c = Value::Null;
    let mut java_javadoc = Value::Null;
    let mut java_dependencies = Value::Null;
    let mut java_cve = Value::Null;
    let empty_java_sources = BTreeSet::new();
    if node_present
        || python_present
        || rust_present
        || go_present
        || erlang_present
        || java_present
        || dependency_configured
        || cve_configured
        || !npm_roots.is_empty()
        || !python_cve_roots.is_empty()
    {
        let mut nodes = Vec::new();
        if node_present {
            nodes.push(TaskNode {
                id: "node.lint".into(),
                dependencies: Vec::new(),
                resources: vec!["node.eslint".into()],
            });
        }
        if python_present {
            nodes.push(TaskNode {
                id: "python.lint".into(),
                dependencies: Vec::new(),
                resources: vec!["python.ruff".into()],
            });
        }
        if rust_present {
            nodes.push(TaskNode {
                id: "rust.cve".into(),
                dependencies: Vec::new(),
                resources: vec!["rust.advisory_db".into()],
            });
            nodes.push(TaskNode {
                id: "rust.build".into(),
                dependencies: Vec::new(),
                resources: vec!["rust.cargo_target".into()],
            });
            nodes.push(TaskNode {
                id: "rust.comments".into(),
                dependencies: Vec::new(),
                resources: vec!["rust.cargo_target".into()],
            });
            nodes.push(TaskNode {
                id: "rust.lint".into(),
                dependencies: Vec::new(),
                resources: vec!["rust.cargo_target".into()],
            });
        }
        if go_present {
            nodes.push(TaskNode {
                id: "go.lint".into(),
                dependencies: Vec::new(),
                resources: vec!["go.vet".into()],
            });
        }
        if erlang_present {
            nodes.push(TaskNode {
                id: "erlang.lint".into(),
                dependencies: Vec::new(),
                resources: vec!["erlang.otp.forms".into()],
            });
        }
        if java_present {
            nodes.push(TaskNode {
                id: "java.p3c".into(),
                dependencies: Vec::new(),
                resources: vec!["java.maven_p3c".into()],
            });
            if javadoc_configured {
                nodes.push(TaskNode {
                    id: "java.javadoc".into(),
                    dependencies: Vec::new(),
                    resources: vec!["java.jdk_javadoc".into()],
                });
            }
        }
        if dependency_configured {
            nodes.push(TaskNode {
                id: "java.dependencies".into(),
                dependencies: Vec::new(),
                resources: vec!["java.maven_dependency".into()],
            });
        }
        if cve_configured {
            nodes.push(TaskNode {
                id: "java.cve".into(),
                dependencies: Vec::new(),
                resources: vec!["java.maven_cve".into()],
            });
        }
        for (id, (build, _)) in &npm_roots {
            nodes.push(TaskNode {
                id: id.clone(),
                dependencies: Vec::new(),
                resources: vec![format!("npm.audit:{build}")],
            });
        }
        for (id, build) in &python_cve_roots {
            nodes.push(TaskNode {
                id: id.clone(),
                dependencies: Vec::new(),
                resources: vec![format!("python.pip_audit:{build}")],
            });
        }
        let graph = TaskGraph::new(nodes).expect("静态检查任务图必须有效");
        let cancelled = AtomicBool::new(false);
        let python_slot = Mutex::new(None);
        let rust_slot = Mutex::new(None);
        let rust_comments_slot = Mutex::new(None);
        let rust_build_slot = Mutex::new(None);
        let rust_cve_slot = Mutex::new(None);
        let node_slot = Mutex::new(None::<crate::check_eslint_scan::CheckEslintScan>);
        let go_slot = Mutex::new(None);
        let erlang_slot = Mutex::new(None);
        let java_slot = Mutex::new(None);
        let javadoc_slot = Mutex::new(None);
        let dependency_slot = Mutex::new(None);
        let cve_slot = Mutex::new(None);
        let npm_slots = Mutex::new(BTreeMap::<String, Value>::new());
        let python_cve_slots = Mutex::new(BTreeMap::<String, Value>::new());
        let mut outcomes = match run_task_graph(
            &graph,
            parsed.jobs_limit,
            deadline,
            &cancelled,
            |id, deadline, flag| {
                if let Some(build) = python_cve_roots.get(&id.id) {
                    let build_path = if build == "." {
                        root.clone()
                    } else {
                        root.join(build)
                    };
                    let report = crate::python_cve_command::observe_for_check(
                        &build_path,
                        parsed.pip_audit_tool.as_deref(),
                        parsed.pip_audit_version.as_deref(),
                        deadline,
                        parsed.timeout_ms,
                        parsed.timeout_source,
                        flag,
                    );
                    let outcome = if flag.load(Ordering::Relaxed)
                        || codeguard_runtime::sigint_cancellation_requested()
                        || report["reason"] == "request_cancelled"
                    {
                        TaskExecution::Cancelled
                    } else if report["reason"] == "request_deadline_exceeded" {
                        TaskExecution::TimedOut
                    } else if report["native_report_valid"] == true
                        && matches!(
                            report["reason"].as_str(),
                            Some(
                                "native_advisories_observed_unverified"
                                    | "native_zero_advisories_unverified"
                            )
                        )
                    {
                        TaskExecution::Succeeded
                    } else {
                        TaskExecution::Failed
                    };
                    python_cve_slots
                        .lock()
                        .expect("Python CVE结果槽未中毒")
                        .insert(id.id.clone(), report);
                    outcome
                } else if let Some((build, digest)) = npm_roots.get(&id.id) {
                    if flag.load(Ordering::Relaxed)
                        || codeguard_runtime::sigint_cancellation_requested()
                    {
                        return TaskExecution::Cancelled;
                    }
                    let report = crate::npm_check_scan::run(
                        &root,
                        build,
                        digest.as_deref(),
                        &parsed.npm_options,
                        deadline,
                    );
                    let outcome = if flag.load(Ordering::Relaxed)
                        || codeguard_runtime::sigint_cancellation_requested()
                        || report["feedback"]["reason"] == "cancelled"
                    {
                        TaskExecution::Cancelled
                    } else if report["feedback"]["reason"] == "deadline" {
                        TaskExecution::TimedOut
                    } else if report["feedback"]["local_coherent"] == true
                        && report["feedback"]["reason"] == "npm_database_and_policy_unverified"
                    {
                        TaskExecution::Succeeded
                    } else {
                        TaskExecution::Failed
                    };
                    npm_slots
                        .lock()
                        .expect("npm结果槽未中毒")
                        .insert(id.id.clone(), report);
                    outcome
                } else if id.id == "node.lint" {
                    let scan = crate::check_eslint_scan::CheckEslintScan::run(
                        &root,
                        &node_sources,
                        parsed
                            .npm_options
                            .get("--node-tool")
                            .map(std::path::Path::new),
                        deadline,
                        flag,
                    );
                    let outcome = if scan.feedback["reason"] == "request_cancelled" {
                        TaskExecution::Cancelled
                    } else if scan.feedback["reason"] == "request_deadline_exceeded" {
                        TaskExecution::TimedOut
                    } else if scan.feedback["status"] == "local_observation" {
                        TaskExecution::Succeeded
                    } else {
                        TaskExecution::Failed
                    };
                    *node_slot.lock().expect("ESLint结果槽未中毒") = Some(scan);
                    outcome
                } else if id.id == "erlang.lint" {
                    let report = crate::check_erlang_scan::observe(
                        &root,
                        &erlang_sources,
                        parsed.erl_tool.clone(),
                        deadline,
                        flag,
                    );
                    let outcome = if flag.load(Ordering::Relaxed)
                        || codeguard_runtime::sigint_cancellation_requested()
                    {
                        TaskExecution::Cancelled
                    } else if Instant::now() >= deadline {
                        TaskExecution::TimedOut
                    } else if report["local_forms_complete"] == true {
                        TaskExecution::Succeeded
                    } else {
                        TaskExecution::Failed
                    };
                    *erlang_slot.lock().expect("Erlang结果槽未中毒") = Some(report);
                    outcome
                } else if id.id == "go.lint" {
                    let report = observe_go_vet(&root, parsed.go_tool.as_deref(), deadline, flag);
                    let outcome = if flag.load(Ordering::Relaxed)
                        || codeguard_runtime::sigint_cancellation_requested()
                    {
                        TaskExecution::Cancelled
                    } else if report["reason"] == "request_deadline_exceeded" {
                        TaskExecution::TimedOut
                    } else if report["native_status"] == "incomplete" {
                        TaskExecution::Failed
                    } else {
                        TaskExecution::Succeeded
                    };
                    *go_slot.lock().expect("Go 结果槽位未中毒") = Some(report);
                    outcome
                } else if id.id == "rust.cve" {
                    let report = crate::cargo_audit_command::observe_for_check(
                        &root,
                        parsed.cargo_audit_tool.as_deref(),
                        parsed.rustsec_db.as_deref(),
                        parsed.timeout_ms,
                        parsed.timeout_source,
                        deadline,
                        flag,
                    );
                    let outcome = if flag.load(Ordering::Relaxed)
                        || codeguard_runtime::sigint_cancellation_requested()
                        || report["reason"] == "request_cancelled"
                    {
                        TaskExecution::Cancelled
                    } else if report["reason"] == "request_deadline_exceeded" {
                        TaskExecution::TimedOut
                    } else if report["local_scan_complete"] == true {
                        TaskExecution::Succeeded
                    } else {
                        TaskExecution::Failed
                    };
                    *rust_cve_slot.lock().expect("Rust CVE 结果槽未中毒") = Some(report);
                    outcome
                } else if id.id == "rust.build" {
                    let report = crate::rust_build_command::observe_for_verification(
                        &root,
                        parsed.cargo_tool.as_deref(),
                        parsed.timeout_ms,
                        parsed.timeout_source,
                        deadline,
                        flag,
                    );
                    let outcome = if flag.load(Ordering::Relaxed)
                        || codeguard_runtime::sigint_cancellation_requested()
                        || report["reason"] == "request_cancelled"
                    {
                        TaskExecution::Cancelled
                    } else if report["reason"] == "request_deadline_exceeded" {
                        TaskExecution::TimedOut
                    } else if report["local_scan_complete"] == true {
                        TaskExecution::Succeeded
                    } else {
                        TaskExecution::Failed
                    };
                    *rust_build_slot.lock().expect("Rust构建结果槽未中毒") = Some(report);
                    outcome
                } else if id.id == "rust.comments" {
                    let report = crate::rust_comments_command::observe_for_verification(
                        &root,
                        parsed.cargo_tool.as_deref(),
                        parsed.timeout_ms,
                        parsed.timeout_source,
                        deadline,
                        false,
                        flag,
                    );
                    let outcome = if flag.load(Ordering::Relaxed)
                        || codeguard_runtime::sigint_cancellation_requested()
                        || report["reason"] == "request_cancelled"
                    {
                        TaskExecution::Cancelled
                    } else if report["reason"] == "request_deadline_exceeded" {
                        TaskExecution::TimedOut
                    } else if report["local_scan_complete"] == true {
                        TaskExecution::Succeeded
                    } else {
                        TaskExecution::Failed
                    };
                    *rust_comments_slot.lock().expect("Rust文档结果槽未中毒") = Some(report);
                    outcome
                } else if id.id == "rust.lint" {
                    let report = observe_cargo_clippy(
                        &root,
                        rust_sources.expect("Rust 任务具备源码范围"),
                        parsed.cargo_tool.as_deref(),
                        deadline,
                        flag,
                    );
                    let outcome = if flag.load(Ordering::Relaxed)
                        || codeguard_runtime::sigint_cancellation_requested()
                        || report["reason"] == "request_cancelled"
                    {
                        TaskExecution::Cancelled
                    } else if report["reason"] == "request_deadline_exceeded" {
                        TaskExecution::TimedOut
                    } else if report["local_scan_complete"] == true {
                        TaskExecution::Succeeded
                    } else {
                        TaskExecution::Failed
                    };
                    *rust_slot.lock().expect("Rust 结果槽位未中毒") = Some(report);
                    outcome
                } else if id.id == "java.javadoc" {
                    let report = observe_javadoc_project(
                        &root,
                        java_sources.expect("Java 任务具备源码范围"),
                        &discovery.checker_configurations,
                        &JavadocNativeContext {
                            manifest_sha256: &discovery.manifest_sha256,
                            java_home: parsed.java_home.as_deref(),
                            maven_tool: parsed.maven_tool.as_deref(),
                            maven_repo: parsed.maven_repo.as_deref(),
                            repo_sha256: parsed.repo_sha256.as_deref(),
                            deadline,
                            cancelled: flag,
                        },
                    );
                    let outcome = if flag.load(Ordering::Relaxed)
                        || codeguard_runtime::sigint_cancellation_requested()
                    {
                        TaskExecution::Cancelled
                    } else if report["local_probe_complete"] == true {
                        TaskExecution::Succeeded
                    } else {
                        TaskExecution::Failed
                    };
                    *javadoc_slot.lock().expect("Javadoc 结果槽位未中毒") = Some(report);
                    outcome
                } else if id.id == "java.dependencies" {
                    let report = observe_dependency_project(
                        &root,
                        java_sources.unwrap_or(&empty_java_sources),
                        &discovery.checker_configurations,
                        &DependencyNativeContext {
                            manifest_sha256: &discovery.manifest_sha256,
                            maven_tool: parsed.maven_tool.as_deref(),
                            java_home: parsed.java_home.as_deref(),
                            maven_repo: parsed.maven_repo.as_deref(),
                            repo_sha256: parsed.repo_sha256.as_deref(),
                            deadline,
                            cancelled: flag,
                        },
                    );
                    let outcome = if flag.load(Ordering::Relaxed)
                        || codeguard_runtime::sigint_cancellation_requested()
                    {
                        TaskExecution::Cancelled
                    } else if report["local_probe_complete"] == true {
                        TaskExecution::Succeeded
                    } else {
                        TaskExecution::Failed
                    };
                    *dependency_slot.lock().expect("依赖图结果槽位未中毒") = Some(report);
                    outcome
                } else if id.id == "java.cve" {
                    let report = observe_cve_project(
                        &root,
                        &discovery.checker_configurations,
                        &CveNativeContext {
                            manifest_sha256: &discovery.manifest_sha256,
                            maven_tool: parsed.maven_tool.as_deref(),
                            java_home: parsed.java_home.as_deref(),
                            maven_repo: parsed.maven_repo.as_deref(),
                            repo_sha256: parsed.repo_sha256.as_deref(),
                            data_dir: parsed.cve_data_dir.as_deref(),
                            data_sha256: parsed.cve_data_sha256.as_deref(),
                            deadline,
                            cancelled: flag,
                        },
                    );
                    let outcome = if flag.load(Ordering::Relaxed)
                        || codeguard_runtime::sigint_cancellation_requested()
                    {
                        TaskExecution::Cancelled
                    } else if report["local_probe_complete"] == true {
                        TaskExecution::Succeeded
                    } else {
                        TaskExecution::Failed
                    };
                    *cve_slot.lock().expect("CVE 结果槽位未中毒") = Some(report);
                    outcome
                } else if id.id == "java.p3c" {
                    let report = observe_project(
                        &root,
                        java_sources.expect("Java 任务具备源码范围"),
                        &discovery.checker_configurations,
                        &NativeContext {
                            manifest_sha256: &discovery.manifest_sha256,
                            maven_tool: parsed.maven_tool.as_deref(),
                            java_home: parsed.java_home.as_deref(),
                            maven_repo: parsed.maven_repo.as_deref(),
                            repo_sha256: parsed.repo_sha256.as_deref(),
                            deadline,
                            cancelled: flag,
                        },
                    );
                    let timed_out = report["files"].as_array().is_some_and(|files| {
                        files.iter().any(|file| {
                            file["reason"] == "request_deadline_exceeded"
                                || file["observation"]["reason"] == "request_deadline_exceeded"
                        })
                    });
                    let outcome = if flag.load(Ordering::Relaxed)
                        || codeguard_runtime::sigint_cancellation_requested()
                    {
                        TaskExecution::Cancelled
                    } else if timed_out {
                        TaskExecution::TimedOut
                    } else if report["local_observation_complete"] == true {
                        TaskExecution::Succeeded
                    } else {
                        TaskExecution::Failed
                    };
                    *java_slot.lock().expect("Java 结果槽位未中毒") = Some(report);
                    outcome
                } else {
                    let report = scan_and_sync_report_with_deadline(
                        &root,
                        parsed.ruff_tool.as_deref(),
                        deadline,
                        flag,
                    );
                    let outcome = if flag.load(Ordering::Relaxed)
                        || codeguard_runtime::sigint_cancellation_requested()
                    {
                        TaskExecution::Cancelled
                    } else if report
                        .as_ref()
                        .is_ok_and(|report| report["local_scan_complete"] == true)
                    {
                        TaskExecution::Succeeded
                    } else {
                        TaskExecution::Failed
                    };
                    *python_slot.lock().expect("Python 结果槽位未中毒") = Some(report);
                    outcome
                }
            },
        ) {
            Ok(outcomes) => outcomes,
            Err(_) => {
                eprintln!("检查任务调度失败");
                return ExitCode::from(4);
            }
        };
        if python_present
            && python_slot
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .as_ref()
                .is_some_and(Result::is_err)
        {
            outcomes.insert("python.lint".into(), TaskOutcome::InternalFailure);
        }
        if outcomes
            .values()
            .any(|outcome| *outcome == TaskOutcome::InternalFailure)
        {
            let cancelled_with_failure = codeguard_runtime::sigint_cancellation_requested()
                || outcomes.values().any(|outcome| {
                    matches!(
                        outcome,
                        TaskOutcome::Cancelled | TaskOutcome::CancelledBeforeStart
                    )
                });
            let native_results = json!({
                "python_lint":python_slot.lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .as_ref().and_then(|result| result.as_ref().ok()).cloned().unwrap_or(Value::Null),
                "python_cve":python_cve_slots.lock().unwrap_or_else(std::sync::PoisonError::into_inner).values().cloned().collect::<Vec<_>>(),
                "rust_lint":snapshot_native_slot(&rust_slot),
                "rust_comments":snapshot_native_slot(&rust_comments_slot),
                "rust_build":snapshot_native_slot(&rust_build_slot),
                "rust_cve":snapshot_native_slot(&rust_cve_slot),
                "node_lint":node_slot.lock().unwrap_or_else(std::sync::PoisonError::into_inner).as_ref().map(|scan|scan.feedback.clone()).unwrap_or(Value::Null),
                "go_lint":snapshot_native_slot(&go_slot),
                "erlang_lint":snapshot_native_slot(&erlang_slot),
                "java_p3c":snapshot_native_slot(&java_slot),
                "java_javadoc":snapshot_native_slot(&javadoc_slot),
                "java_dependencies":snapshot_native_slot(&dependency_slot),
                "java_cve":snapshot_native_slot(&cve_slot),
                "npm_cve":npm_slots.lock().unwrap_or_else(std::sync::PoisonError::into_inner).values().cloned().collect::<Vec<_>>()
            });
            let report = aborted_task_report(
                parsed.selection,
                discovery.to_json(),
                &outcomes,
                native_results,
                cancelled_with_failure,
            );
            if parsed.format != OutputFormat::Human {
                emit_structured(&report, &parsed);
            } else {
                println!(
                    "CodeGuard 检查{}，退出 {}；已取得的原生结果如下，交付未完成：",
                    if cancelled_with_failure {
                        "已取消"
                    } else {
                        "发生内部故障"
                    },
                    if cancelled_with_failure { 130 } else { 4 }
                );
                println!("失败任务：{}", report["failed_task_ids"]);
                println!("任务状态：{}", report["execution_tasks"]);
                println!("{}", report["native_results"]);
            }
            return ExitCode::from(if cancelled_with_failure { 130 } else { 4 });
        }
        if node_present {
            let outcome = *outcomes.get("node.lint").expect("ESLint任务结果完整");
            node_task_outcome = Some(outcome);
            execution_tasks.push(json!({"id":"node.lint","status":task_status(outcome)}));
            if let Some(mut scan) = node_slot.into_inner().expect("ESLint结果槽未中毒") {
                scan.sync(&root, deadline);
                node_lint = scan.feedback;
            }
        }
        let mut npm_reports = npm_slots.into_inner().expect("npm结果槽未中毒");
        let mut python_cve_reports = python_cve_slots
            .into_inner()
            .expect("Python CVE结果槽未中毒");
        for (id, build) in &python_cve_roots {
            let outcome = *outcomes.get(id).expect("Python CVE任务图结果应完整");
            python_cve_outcomes.push(outcome);
            execution_tasks.push(json!({"id":id,"status":task_status(outcome)}));
            let mut report = python_cve_reports.remove(id).unwrap_or_else(|| {
                json!({
                    "reason":"python_cve_task_not_started",
                    "native_report_valid":false,"findings":[]
                })
            });
            report["build_root"] = json!(build);
            python_cve.push(report);
        }
        for (id, (build, _)) in &npm_roots {
            let outcome = *outcomes.get(id).expect("npm任务图结果应完整");
            npm_outcomes.push(outcome);
            execution_tasks.push(json!({"id":id,"status":task_status(outcome)}));
            let mut report = npm_reports.remove(id).unwrap_or_else(||json!({"build_root":build,"feedback":null,"observation":null,"backlog_status":"not_started"}));
            if report["feedback"].is_object() {
                report["feedback"]["execution_budget"] =
                    crate::check_budget::budget_record(parsed.timeout_ms, parsed.timeout_source);
            }
            crate::npm_check_scan::sync(&root, &mut report, deadline);
            npm_cve.push(report);
        }
        if python_present {
            let outcome = *outcomes.get("python.lint").expect("任务图结果应完整");
            if outcome == TaskOutcome::InternalFailure {
                eprintln!("Python 任务发生内部异常");
                return ExitCode::from(4);
            }
            python_task_outcome = Some(outcome);
            execution_tasks.push(json!({"id":"python.lint", "status":task_status(outcome)}));
            python_lint = match python_slot.into_inner().expect("扫描结果槽位未中毒") {
                Some(Ok(mut report)) => {
                    annotate_conversation_budget(
                        &mut report,
                        parsed.timeout_ms,
                        parsed.timeout_source,
                    );
                    report
                }
                Some(Err(_)) => {
                    eprintln!("Python 原生扫描无法建立完整结果");
                    return ExitCode::from(4);
                }
                None => Value::Null,
            };
        }
        if rust_present {
            let cve_outcome = *outcomes.get("rust.cve").expect("Rust CVE 任务结果完整");
            rust_cve_outcome = Some(cve_outcome);
            execution_tasks.push(json!({"id":"rust.cve","status":task_status(cve_outcome)}));
            rust_cve = rust_cve_slot
                .into_inner()
                .expect("Rust CVE 结果槽未中毒")
                .unwrap_or(Value::Null);
            let build_outcome = *outcomes.get("rust.build").expect("Rust构建任务结果完整");
            rust_build_outcome = Some(build_outcome);
            execution_tasks.push(json!({"id":"rust.build","status":task_status(build_outcome)}));
            rust_build = rust_build_slot
                .into_inner()
                .expect("Rust构建结果槽未中毒")
                .unwrap_or(Value::Null);
            let doc_outcome = *outcomes.get("rust.comments").expect("Rust文档任务结果完整");
            rust_comments_outcome = Some(doc_outcome);
            execution_tasks.push(json!({"id":"rust.comments","status":task_status(doc_outcome)}));
            rust_comments = rust_comments_slot
                .into_inner()
                .expect("Rust文档结果槽未中毒")
                .unwrap_or(Value::Null);
            let outcome = *outcomes.get("rust.lint").expect("任务图结果应完整");
            if outcome == TaskOutcome::InternalFailure {
                eprintln!("Rust 任务发生内部异常");
                return ExitCode::from(4);
            }
            rust_task_outcome = Some(outcome);
            execution_tasks.push(json!({"id":"rust.lint", "status":task_status(outcome)}));
            rust_lint = rust_slot
                .into_inner()
                .expect("Rust 结果槽位未中毒")
                .unwrap_or(Value::Null);
        }
        if erlang_present {
            let outcome = *outcomes.get("erlang.lint").expect("Erlang任务结果完整");
            erlang_task_outcome = Some(outcome);
            execution_tasks.push(json!({"id":"erlang.lint", "status":task_status(outcome)}));
            erlang_lint = erlang_slot
                .into_inner()
                .expect("Erlang结果槽未中毒")
                .unwrap_or(Value::Null);
        }
        if go_present {
            let outcome = *outcomes.get("go.lint").expect("Go 任务结果应完整");
            go_task_outcome = Some(outcome);
            execution_tasks.push(json!({"id":"go.lint", "status":task_status(outcome)}));
            go_lint = go_slot
                .into_inner()
                .expect("Go 结果槽位未中毒")
                .unwrap_or(Value::Null);
        }
        if java_present {
            let outcome = *outcomes.get("java.p3c").expect("任务图结果应完整");
            if outcome == TaskOutcome::InternalFailure {
                eprintln!("Java 任务发生内部异常");
                return ExitCode::from(4);
            }
            java_task_outcome = Some(outcome);
            execution_tasks.push(json!({"id":"java.p3c", "status":task_status(outcome)}));
            java_p3c = java_slot
                .into_inner()
                .expect("Java 结果槽位未中毒")
                .unwrap_or(Value::Null);
            if javadoc_configured {
                let outcome = *outcomes.get("java.javadoc").expect("任务图结果应完整");
                if outcome == TaskOutcome::InternalFailure {
                    eprintln!("Javadoc 任务发生内部异常");
                    return ExitCode::from(4);
                }
                javadoc_task_outcome = Some(outcome);
                execution_tasks.push(json!({"id":"java.javadoc", "status":task_status(outcome)}));
                java_javadoc = javadoc_slot
                    .into_inner()
                    .expect("Javadoc 结果槽位未中毒")
                    .unwrap_or(Value::Null);
            }
        }
        if dependency_configured {
            let outcome = *outcomes
                .get("java.dependencies")
                .expect("依赖图任务结果应完整");
            if outcome == TaskOutcome::InternalFailure {
                eprintln!("Java 依赖图任务发生内部异常");
                return ExitCode::from(4);
            }
            dependency_task_outcome = Some(outcome);
            execution_tasks.push(json!({"id":"java.dependencies","status":task_status(outcome)}));
            java_dependencies = dependency_slot
                .into_inner()
                .expect("依赖图结果槽位未中毒")
                .unwrap_or(Value::Null);
        }
        if cve_configured {
            let outcome = *outcomes.get("java.cve").expect("CVE 任务结果应完整");
            if outcome == TaskOutcome::InternalFailure {
                eprintln!("Java CVE 任务发生内部异常");
                return ExitCode::from(4);
            }
            cve_task_outcome = Some(outcome);
            execution_tasks.push(json!({"id":"java.cve","status":task_status(outcome)}));
            java_cve = cve_slot
                .into_inner()
                .expect("CVE 结果槽位未中毒")
                .unwrap_or(Value::Null);
            attach_candidates(&mut java_cve, &java_dependencies);
        }
    }
    if go_lint.is_object() {
        crate::go_lint_command::persist_and_sync(&root, &mut go_lint);
    }
    if rust_comments.is_object() {
        crate::rust_comments_command::persist_and_sync(&root, &mut rust_comments);
    }
    if rust_build.is_object() {
        crate::rust_build_command::persist_and_sync(&root, &mut rust_build);
    }
    let mut rust_cve_synced = false;
    let mut rust_cve_sync_failed = false;
    if rust_cve.is_object() {
        match crate::cargo_audit_command::persist_and_sync(&root, &rust_cve) {
            Ok(synced) => rust_cve_synced = synced,
            Err(_) => rust_cve_sync_failed = true,
        }
    }
    let mut python_cve_synced = false;
    let mut python_cve_sync_failed = false;
    for scan in &mut python_cve {
        let Some(build_root) = scan["build_root"].as_str().map(str::to_owned) else {
            python_cve_sync_failed = true;
            continue;
        };
        match crate::python_cve_command::persist_and_sync(&root, &build_root, scan) {
            Ok(true) => {
                python_cve_synced = true;
                scan["backlog_status"] = json!("synced_partial");
            }
            Ok(false) => scan["backlog_status"] = json!("not_initialized"),
            Err(_) => {
                python_cve_sync_failed = true;
                scan["backlog_status"] = json!("backlog_update_failed");
            }
        }
    }
    if rust_lint.is_object() {
        let (status, summary) = match save_local_report(&root, &rust_lint) {
            Ok(()) if rust_lint["workspace_binding"] == "bound" => {
                match sync_local_workspace(&root) {
                    Ok(summary) if summary.failed_reports == 0 => (
                        "synced_partial",
                        json!({
                            "new_findings":summary.new_findings,
                            "new_blockers":summary.new_blockers,
                            "imported_reports":summary.imported_reports,
                            "historical_findings":summary.historical_findings,
                            "failed_reports":summary.failed_reports
                        }),
                    ),
                    _ => ("backlog_update_failed", Value::Null),
                }
            }
            Ok(()) => ("not_initialized", Value::Null),
            Err(_) => ("backlog_update_failed", Value::Null),
        };
        rust_lint["backlog_status"] = json!(status);
        rust_lint["backlog_sync"] = summary;
        rust_lint["next"] = if status == "synced_partial" {
            read_local_brief(&root).unwrap_or(Value::Null)
        } else {
            Value::Null
        };
    }
    if java_p3c.is_object() {
        crate::java_p3c_workbench::persist_and_sync(&root, &mut java_p3c);
    }
    if java_cve.is_object() {
        let (status, summary) = match save_local_report(&root, &java_cve) {
            Ok(()) if java_cve["workspace_binding"] == "bound" => {
                match sync_local_workspace(&root) {
                    Ok(summary) if summary.failed_reports == 0 => (
                        "synced_partial",
                        json!({
                            "new_findings":summary.new_findings,
                            "new_blockers":summary.new_blockers,
                            "imported_reports":summary.imported_reports,
                            "historical_findings":summary.historical_findings,
                            "failed_reports":summary.failed_reports
                        }),
                    ),
                    _ => ("backlog_update_failed", Value::Null),
                }
            }
            Ok(()) => ("not_initialized", Value::Null),
            Err(_) => ("backlog_update_failed", Value::Null),
        };
        java_cve["backlog_status"] = json!(status);
        java_cve["backlog_sync"] = summary;
        java_cve["next"] = if status == "synced_partial" {
            read_local_brief_for_checker(&root, "java.maven.dependency_check")
                .unwrap_or(Value::Null)
        } else {
            Value::Null
        };
    }
    // 本轮原生工具可能创建或移动源码/清单；再次发现只用于识别本地观察范围漂移，
    // 不把第二轮候选提升为受保护策略义务，也不删除已取得的原生诊断。
    let scope_recheck = if Instant::now() < deadline {
        let after = discover(&root, &registry, &NativeObservation);
        if discovery_scope_changed(&discovery, &after) {
            Some("project_scope_changed_during_check")
        } else {
            None
        }
    } else {
        Some("project_scope_recheck_deadline_exceeded")
    };
    let source_recheck = match source_snapshot {
        Ok(Some(snapshot)) if Instant::now() < deadline => match snapshot.verify_source_unchanged()
        {
            Ok(true) => None,
            Ok(false) => Some("project_source_changed_during_check"),
            Err(_) => Some("project_source_recheck_unavailable"),
        },
        Ok(Some(_)) => Some("project_source_recheck_deadline_exceeded"),
        Ok(None) => None,
        Err(reason) => Some(reason),
    };
    if erlang_lint.is_object() {
        if source_recheck.or(scope_recheck).is_some() {
            crate::check_erlang_scan::invalidate_scope(&mut erlang_lint);
        }
        crate::check_erlang_scan::refresh(&root, &mut erlang_lint, deadline);
        crate::native_syntax_confirmation::connect(&root, &mut erlang_lint, deadline);
        crate::check_erlang_scan::refresh(&root, &mut erlang_lint, deadline);
    }
    let python_doc_observed = python_lint["files"].as_array().is_some_and(|files| {
        files.iter().any(|file| {
            file["run_status"] == "findings"
                && file["findings"].as_array().is_some_and(|findings| {
                    findings.iter().any(|finding| {
                        finding["rule_id"]
                            .as_str()
                            .is_some_and(codeguard_adapters::is_ruff_pydocstyle_rule)
                    })
                })
        })
    });
    let mut candidates = Vec::new();
    for (language, evidence) in &discovery.languages {
        if parsed.selection == Selection::Java && language != "java" {
            continue;
        }
        if evidence.source_files.is_empty() && evidence.manifests.is_empty() {
            continue;
        }
        let registry_language = registry
            .languages
            .iter()
            .find(|entry| entry.id == *language);
        let legacy_status = registry_language.map_or("unknown", |entry| entry.status.as_str());
        let capability_gap = registry_language.is_some_and(|entry| {
            capability_row(entry)
                .platforms
                .values()
                .all(|categories| categories.values().all(|cell| cell.status == "gap"))
        });
        let planned_language = legacy_status == "planned";
        for category in CHECK_CATEGORIES {
            let npm_selected = matches!(language.as_str(), "typescript" | "javascript")
                && category == "cve"
                && !npm_roots.is_empty();
            let python_lint_selected = language == "python" && category == "lint" && python_present;
            let python_comments_selected =
                language == "python" && category == "comments" && python_doc_observed;
            let python_cve_selected =
                language == "python" && category == "cve" && !python_cve_roots.is_empty();
            let rust_lint_selected = language == "rust" && category == "lint" && rust_present;
            let rust_comments_selected =
                language == "rust" && category == "comments" && rust_present;
            let rust_build_selected = language == "rust" && category == "build" && rust_present;
            let rust_cve_selected = language == "rust" && category == "cve" && rust_present;
            let go_lint_selected = language == "go" && category == "lint" && go_present;
            let java_lint_selected = language == "java" && category == "lint" && java_present;
            let java_comments_selected =
                language == "java" && category == "comments" && javadoc_configured;
            let observed = (npm_selected
                && npm_outcomes.iter().all(|o| *o == TaskOutcome::Succeeded))
                || (python_lint_selected
                    && python_task_outcome == Some(TaskOutcome::Succeeded)
                    && python_lint["local_scan_complete"] == true)
                || (python_comments_selected
                    && python_task_outcome == Some(TaskOutcome::Succeeded)
                    && python_lint["local_scan_complete"] == true)
                || (rust_lint_selected
                    && rust_task_outcome == Some(TaskOutcome::Succeeded)
                    && rust_lint["local_scan_complete"] == true)
                || (rust_comments_selected
                    && rust_comments_outcome == Some(TaskOutcome::Succeeded)
                    && rust_comments["local_scan_complete"] == true)
                || (rust_build_selected
                    && rust_build_outcome == Some(TaskOutcome::Succeeded)
                    && rust_build["local_scan_complete"] == true)
                || (rust_cve_selected
                    && rust_cve_outcome == Some(TaskOutcome::Succeeded)
                    && rust_cve["local_scan_complete"] == true)
                || (go_lint_selected
                    && go_task_outcome == Some(TaskOutcome::Succeeded)
                    && go_lint["native_status"] != "incomplete");
            let java_checker = if language == "java" && (java_present || cve_configured) {
                checker_for_category(category).map(|checker_id| {
                    summarize(
                        checker_id,
                        java_sources.unwrap_or(&empty_java_sources),
                        &discovery.checker_configurations,
                    )
                })
            } else {
                None
            };
            let cve_scope_configured = language == "java"
                && category == "cve"
                && cve_configured
                && java_checker.as_ref().is_some_and(|candidate| {
                    candidate.status == "native_incomplete"
                        && candidate.reason == "configured_checker_not_integrated"
                });
            let mut candidate = json!({
                "language": language,
                "legacy_status": legacy_status,
                "capability_status": if capability_gap { Some("gap") } else { None },
                "category": category,
                "checker_id": if rust_cve_selected {Some("rust.cargo_audit")} else if rust_build_selected {Some("rust.cargo_check")} else if rust_comments_selected {Some("rust.cargo_rustdoc")} else if npm_selected { Some("node.npm.audit") } else if go_lint_selected { Some("go.vet") } else if language == "java" && category == "dependencies" && dependency_configured { Some("java.maven.dependency") } else if cve_scope_configured { Some("java.maven.dependency_check") } else { java_checker.as_ref().and_then(|candidate| candidate.checker_id) },
                "next_action": if rust_cve_selected {Some("核对原生漏洞及解析版本；补齐离线数据库身份和时效、受批准工具与策略，再用原工具复检")} else if rust_build_selected {Some("核对原生编译错误与锁定输入，读取稳定任务并用同一Cargo复检；类型检查不执行测试，完整构建组合和策略仍须核验")} else if rust_comments_selected {Some("按文档原生规则修复或恢复锁定离线环境；读取稳定任务并用同一Cargo复检，核验原配置和完整目标范围")} else if npm_selected { Some("核对逐npm构建根的原生诊断、锁版本及漏洞源覆盖及时效，取得稳定任务并用原工具复检") } else if planned_language { Some("该语言尚无已验证的原生适配器；先确定原生检查工具、适用规则和正反例，再实现并验收检查链") } else if go_lint_selected { Some("提供匹配的 Go 1.23.4 工具、修复环境或源码后重跑逐模块 vet；仍须确认规则和平台覆盖") } else if language == "java" && category == "dependencies" && dependency_configured { Some("补齐 Maven/JDK/离线仓库身份或修正原生报告，再运行依赖图探针") } else if cve_scope_configured { Some("补齐原生工具和离线漏洞库身份，核验数据库时效及漏洞归属后重新检查") } else { java_checker.as_ref().map(|candidate| candidate.next_action) },
                "status": if observed { "observed_unverified" } else if npm_selected || python_lint_selected || rust_lint_selected || rust_comments_selected || rust_build_selected || rust_cve_selected || go_lint_selected || java_lint_selected || java_comments_selected || (language == "java" && category == "dependencies" && dependency_configured) || (cve_scope_configured) { "native_incomplete" } else if language == "java" && category == "comments" { if javadoc_configuration_unresolved { "configuration_unresolved" } else { "not_configured" } } else if let Some(candidate) = java_checker.as_ref() { candidate.status } else { "not_integrated" },
                "reason": if rust_cve_selected { if observed { "rust_cve_database_freshness_unverified" } else { "rust_cve_native_incomplete" } } else if npm_selected { "npm_advisory_coverage_and_freshness_unverified" } else if planned_language { "planned_language_adapter_gap" } else if observed { "trusted_policy_and_coverage_unavailable" } else if java_lint_selected && java_p3c["local_observation_complete"] == true { "p3c_declared_rulesets_unverified_coverage" } else if java_comments_selected && java_javadoc["maven_multifile_probes"].as_array().is_some_and(|probes| probes.iter().any(|probe| matches!(probe["observation"]["native_status"].as_str(), Some("findings_observed_untrusted" | "clean_log_unverified")))) { "javadoc_multifile_probe_unverified_project_coverage" } else if java_comments_selected && java_javadoc["local_probe_complete"] == true { "javadoc_single_file_probe_unverified_coverage" } else if language == "java" && category == "dependencies" && java_dependencies["observed_graph_count"].as_u64().is_some_and(|count| count > 0) { "dependency_graph_observed_unverified_coverage" } else if cve_scope_configured && java_cve["observed_report_count"].as_u64().is_some_and(|count| count > 0) { "cve_report_observed_database_unverified" } else if (language == "java" && category == "dependencies" && dependency_configured) || cve_scope_configured || python_lint_selected || rust_lint_selected || rust_comments_selected || rust_build_selected || go_lint_selected || java_lint_selected || java_comments_selected { "native_scan_incomplete" } else if language == "java" && category == "comments" { if javadoc_configuration_unresolved { "javadoc_configuration_unresolved" } else { "not_configured" } } else if let Some(candidate) = java_checker.as_ref() { candidate.reason } else { "native_adapter_not_integrated" }
            });
            if language == "erlang" && category == "lint" && erlang_present {
                candidate["checker_id"] = json!("erlang.otp.forms");
                candidate["status"] = json!("native_incomplete");
                candidate["reason"] = erlang_lint
                    .get("reason")
                    .cloned()
                    .unwrap_or(json!("erlang_native_task_not_started"));
                candidate["next_action"] = json!(
                    "读取 native_results.erlang_lint 的逐文件诊断、阻塞和原工具复检指令；单文件 forms 不代替项目完整 lint、预处理、编译和测试"
                );
            }
            if python_comments_selected {
                candidate["checker_id"] = json!("python.ruff");
                candidate["next_action"] = json!(
                    "核对 Ruff 原生 pydocstyle 规则与定位，按项目约定修正文档后运行同一原生工具复检；完整规则集与批准覆盖仍须核验"
                );
                candidate["status"] = json!(if observed {
                    "observed_unverified"
                } else {
                    "native_incomplete"
                });
                candidate["reason"] = json!(if observed {
                    "ruff_d100_native_finding_unverified_coverage"
                } else {
                    "ruff_d100_native_observation_incomplete"
                });
            }
            if language == "python" && category == "cve" {
                candidate["checker_id"] = json!("python.pip_audit");
                candidate["next_action"] = json!(
                    "查看逐 Python 构建根的原生结果与锁文件缺口；核验依赖组、环境和漏洞数据库身份及时效后复检"
                );
                let all_observed = python_cve_selected
                    && python_cve_outcomes
                        .iter()
                        .all(|outcome| *outcome == TaskOutcome::Succeeded)
                    && python_cve
                        .iter()
                        .all(|report| report["native_report_valid"] == true);
                candidate["status"] = json!(if all_observed {
                    "observed_unverified"
                } else if python_cve_selected {
                    "native_incomplete"
                } else {
                    "configuration_unresolved"
                });
                candidate["reason"] = json!(if all_observed {
                    "python_cve_advisory_coverage_and_freshness_unverified"
                } else if python_cve_selected {
                    "python_cve_native_incomplete"
                } else {
                    "checker_configuration_unresolved"
                });
            }
            if category == "lint"
                && node_sources
                    .iter()
                    .any(|path| evidence.source_files.contains(path))
            {
                candidate["checker_id"] = json!("node.eslint");
                candidate["status"] = json!(if node_lint["status"] == "local_observation" {
                    "observed_unverified"
                } else {
                    "native_incomplete"
                });
                candidate["reason"] = json!("eslint_project_coverage_unverified");
                candidate["next_action"] = json!(
                    "读取逐文件 ESLint 原生发现与准备原因，按项目原配置复检；缺工具或配置的模块继续语法初检"
                );
            }
            candidates.push(candidate);
        }
    }
    let mut unresolved: BTreeSet<String> = [
        "trusted_policy_unavailable".to_owned(),
        "required_obligations_unresolved".to_owned(),
        "tool_lock_unverified".to_owned(),
    ]
    .into_iter()
    .collect();
    if node_present && node_task_outcome != Some(TaskOutcome::Succeeded) {
        unresolved.insert("node_lint_incomplete".into());
    }
    if let Some(reason) = scope_recheck {
        unresolved.insert(reason.into());
    }
    if let Some(reason) = source_recheck {
        unresolved.insert(reason.into());
    }
    if !discovery.observation_complete {
        unresolved.insert("project_observation_incomplete".into());
    }
    for (language, evidence) in &discovery.languages {
        if (parsed.selection == Selection::All || language == "java")
            && (!evidence.source_files.is_empty() || !evidence.manifests.is_empty())
            && registry
                .languages
                .iter()
                .any(|entry| entry.id == *language && entry.status == "planned")
        {
            unresolved.insert(format!("planned_language_gap:{language}"));
        }
    }
    if Instant::now() >= deadline {
        unresolved.insert("request_deadline_exceeded".into());
    }
    if python_present && python_task_outcome != Some(TaskOutcome::Succeeded) {
        unresolved.insert("python_lint_task_incomplete".into());
    }
    if python_present {
        unresolved.insert("python_cve_dependency_graph_and_native_checker_unverified".into());
    }
    if !python_cve_roots.is_empty() {
        unresolved.insert("python_cve_advisory_coverage_and_freshness_unverified".into());
        if python_cve_sync_failed {
            unresolved.insert("python_cve_backlog_sync_incomplete".into());
        }
        if python_cve_outcomes
            .iter()
            .any(|outcome| *outcome != TaskOutcome::Succeeded)
        {
            unresolved.insert("python_cve_task_incomplete".into());
        }
    }
    if rust_present {
        unresolved.insert("rust_cve_database_freshness_unverified".into());
        if rust_cve_outcome != Some(TaskOutcome::Succeeded) {
            unresolved.insert("rust_cve_task_incomplete".into());
        }
        if rust_cve_sync_failed {
            unresolved.insert("rust_cve_backlog_sync_incomplete".into());
        }
        unresolved.insert("rust_build_policy_and_coverage_unverified".into());
        if rust_build_outcome != Some(TaskOutcome::Succeeded) {
            unresolved.insert("rust_build_task_incomplete".into());
        }
        if matches!(
            rust_build["backlog_status"].as_str(),
            Some("failed" | "sync_incomplete")
        ) {
            unresolved.insert("rust_build_backlog_sync_incomplete".into());
        }
        unresolved.insert("rust_comments_policy_and_coverage_unverified".into());
        if rust_comments_outcome != Some(TaskOutcome::Succeeded) {
            unresolved.insert("rust_comments_task_incomplete".into());
        }
        if matches!(
            rust_comments["backlog_status"].as_str(),
            Some("failed" | "sync_incomplete")
        ) {
            unresolved.insert("rust_comments_backlog_sync_incomplete".into());
        }
    }
    if rust_present && rust_task_outcome != Some(TaskOutcome::Succeeded) {
        unresolved.insert("rust_lint_task_incomplete".into());
    }
    if erlang_present {
        unresolved.insert("erlang_project_lint_preprocessing_and_build_coverage_unverified".into());
        if erlang_task_outcome != Some(TaskOutcome::Succeeded)
            || erlang_lint["local_forms_complete"] != true
        {
            unresolved.insert("erlang_lint_task_incomplete".into());
        }
    }
    if go_present && go_task_outcome != Some(TaskOutcome::Succeeded) {
        unresolved.insert("go_lint_task_incomplete".into());
    }
    if java_present && java_task_outcome != Some(TaskOutcome::Succeeded) {
        unresolved.insert("java_p3c_task_incomplete".into());
    }
    if javadoc_configured && javadoc_task_outcome != Some(TaskOutcome::Succeeded) {
        unresolved.insert("java_javadoc_task_incomplete".into());
    }
    if dependency_configured && dependency_task_outcome != Some(TaskOutcome::Succeeded) {
        unresolved.insert("java_dependency_task_incomplete".into());
    }
    if cve_configured {
        unresolved.insert("cve_database_freshness_unverified".into());
        if cve_task_outcome != Some(TaskOutcome::Succeeded) {
            unresolved.insert("java_cve_task_incomplete".into());
        }
    }
    if let Some(reason) = historical_npm_scope_error {
        unresolved.insert("historical_npm_scope_unavailable".into());
        unresolved.insert(reason.into());
    }
    if let Some(reason) = recorded_npm_scope_error {
        unresolved.insert("recorded_npm_scope_unavailable".into());
        unresolved.insert(reason.into());
    }
    if !npm_roots.is_empty() {
        unresolved.insert("npm_advisory_coverage_and_freshness_unverified".into());
        if npm_cve.iter().any(|report| {
            report["observation"].is_object() && report["backlog_status"] != "synced_partial"
        }) {
            unresolved.insert("npm_backlog_sync_incomplete".into());
        }
        if npm_outcomes.iter().any(|o| *o != TaskOutcome::Succeeded) {
            unresolved.insert("npm_cve_task_incomplete".into());
        }
    }
    let request_cancelled = python_cve_outcomes
        .iter()
        .chain(npm_outcomes.iter())
        .any(|o| {
            matches!(
                o,
                TaskOutcome::Cancelled | TaskOutcome::CancelledBeforeStart
            )
        })
        || codeguard_runtime::sigint_cancellation_requested()
        || [
            node_task_outcome,
            python_task_outcome,
            rust_task_outcome,
            rust_comments_outcome,
            rust_build_outcome,
            rust_cve_outcome,
            go_task_outcome,
            erlang_task_outcome,
            java_task_outcome,
            javadoc_task_outcome,
            dependency_task_outcome,
            cve_task_outcome,
        ]
        .into_iter()
        .flatten()
        .any(|outcome| {
            matches!(
                outcome,
                TaskOutcome::Cancelled | TaskOutcome::CancelledBeforeStart
            )
        });
    if request_cancelled {
        unresolved.insert("request_cancelled".into());
    }
    if parsed.selection == Selection::Java && !java_present {
        unresolved.insert("java_target_absent_or_unobserved".into());
    }
    unresolved.extend(discovery.unknown_conditions.iter().cloned());
    let next = (parsed.selection == Selection::All)
        .then(|| rust_lint.get("next").filter(|value| !value.is_null()))
        .flatten()
        .cloned()
        .or_else(|| {
            (parsed.selection == Selection::All)
                .then(|| {
                    if rust_build["backlog_status"] == "synced" {
                        read_local_brief_for_checker(&root, "rust.cargo_check")
                            .ok()
                            .filter(|value| !value.is_null())
                    } else {
                        None
                    }
                })
                .flatten()
        })
        .or_else(|| {
            (parsed.selection == Selection::All && rust_cve_synced)
                .then(|| read_local_brief_for_checker(&root, "rust.cargo_audit").ok())
                .flatten()
                .filter(|value| !value.is_null())
        })
        .or_else(|| {
            (parsed.selection == Selection::All)
                .then(|| python_lint.get("next").filter(|value| !value.is_null()))
                .flatten()
                .cloned()
        })
        .or_else(|| {
            (parsed.selection == Selection::All && python_cve_synced)
                .then(|| read_local_brief_for_checker(&root, "python.pip_audit").ok())
                .flatten()
                .filter(|value| !value.is_null())
        })
        .or_else(|| {
            java_p3c
                .get("next")
                .filter(|value| !value.is_null())
                .cloned()
        })
        .or_else(|| {
            java_cve
                .get("next")
                .filter(|value| !value.is_null())
                .cloned()
        })
        .or_else(|| {
            node_lint
                .get("next")
                .filter(|value| !value.is_null())
                .cloned()
        })
        .unwrap_or_else(|| {
            if npm_cve
                .iter()
                .any(|r| r["backlog_status"] == "synced_partial")
            {
                match read_local_brief_for_checker(&root, "node.npm.audit") {
                    Ok(brief) => brief,
                    Err(reason) => {
                        unresolved.insert(format!("npm_next_unavailable:{reason}"));
                        Value::Null
                    }
                }
            } else {
                Value::Null
            }
        });
    let started_native_task_count = [
        node_task_outcome,
        python_task_outcome,
        rust_task_outcome,
        rust_comments_outcome,
        rust_build_outcome,
        rust_cve_outcome,
        go_task_outcome,
        java_task_outcome,
        javadoc_task_outcome,
        dependency_task_outcome,
        cve_task_outcome,
    ]
    .into_iter()
    .flatten()
    .chain(npm_outcomes.iter().copied())
    .chain(python_cve_outcomes.iter().copied())
    .filter(|outcome| {
        !matches!(
            outcome,
            TaskOutcome::CancelledBeforeStart
                | TaskOutcome::DeadlineBeforeStart
                | TaskOutcome::DependencyFailed
        )
    })
    .count();
    #[cfg(feature = "wasm-precheck")]
    let syntax_candidates = crate::check_syntax_candidates::observe(
        &root,
        &discovery,
        crate::check_syntax_candidates::NativeCoverage {
            node_lint: &node_lint,
            python_lint: &python_lint,
            go_lint: &go_lint,
            erlang_lint: &erlang_lint,
            go_tool: parsed.go_tool.as_deref(),
        },
        parsed.selection == Selection::Java,
        parsed.jobs_limit,
        deadline,
        if request_cancelled {
            Some("request_cancelled")
        } else if scope_recheck.is_some() || source_recheck.is_some() {
            Some("source_or_scope_changed")
        } else {
            None
        },
    );
    #[cfg(not(feature = "wasm-precheck"))]
    let syntax_candidates = json!({
        "status":"not_run","reason":"binary_without_wasm_precheck","execution_phase":"after_native",
        "authority":"candidate_unqualified","delivery_decision":"incomplete",
        "source_file_count":discovery.languages.values().map(|item| item.source_files.len()).sum::<usize>() + discovery.ambiguous_source_files.len(),
        "skipped_count":0,"unrouted_count":0,"native_preferred_count":0,"observations":[],
        "next_action":"使用包含固定语法资产的发行包运行候选初检，并完成适用原生检查"
    });
    let report = json!({
        "schema_version":if erlang_lint["schema_version"] == "0.2.0" {"0.37.0"} else {"0.36.0"}, "report_type":"check_feedback",
        "operation":"check", "selection":parsed.selection.as_str(), "command_status":if request_cancelled { "cancelled" } else { "incomplete" },
        "exit_code":if request_cancelled { 130 } else { 3 }, "delivery_decision":if parsed.selection == Selection::All { "incomplete" } else { "not_evaluated" }, "authority":"local_unverified",
        "reason":if request_cancelled { "request_cancelled" } else if parsed.selection == Selection::All { "full_project_obligations_and_trusted_policy_unavailable" } else { "java_selection_obligations_and_trusted_policy_unavailable" },
        "discovery":discovery.to_json(),
        "native_results":{"node_lint":node_lint,"python_lint":python_lint,"python_cve":python_cve,"rust_lint":rust_lint,"rust_comments":rust_comments,"rust_build":rust_build,"rust_cve":rust_cve,"go_lint":go_lint,"erlang_lint":erlang_lint,"java_p3c":java_p3c,"java_javadoc":java_javadoc,"java_dependencies":java_dependencies,"java_cve":java_cve,"npm_cve":npm_cve}, "execution_tasks":execution_tasks,
        "obligation_status":"unresolved", "required_obligations":null,
        "category_candidates":candidates, "unresolved_conditions":unresolved,
        "syntax_candidates":syntax_candidates,
        "execution_budget":check_budget_record(
            parsed.timeout_ms, parsed.timeout_source, parsed.jobs_limit, parsed.jobs_source,
            usize::from(node_present) + usize::from(python_present) + python_cve_roots.len() + 4 * usize::from(rust_present) + usize::from(go_present) + usize::from(erlang_present) + usize::from(java_present) + usize::from(javadoc_configured) + usize::from(dependency_configured) + usize::from(cve_configured) + npm_roots.len(), started_native_task_count
        ),
        "next":next,
        "export":{"status":"not_requested","reason_code":null}
    });
    if parsed.format != OutputFormat::Human {
        emit_structured(&report, &parsed);
    } else {
        println!(
            "CodeGuard {}检查{}；已观察的原生结果与待确认类别如下：",
            if parsed.selection == Selection::All {
                "全项目"
            } else {
                "Java "
            },
            if request_cancelled {
                "已取消"
            } else {
                "未完成"
            }
        );
        if let Some(checkers) = report["discovery"]["checker_configurations"].as_array() {
            for checker in checkers {
                if parsed.selection == Selection::Java
                    && !checker["checker_id"]
                        .as_str()
                        .is_some_and(|id| id.starts_with("java."))
                {
                    continue;
                }
                println!(
                    "配置 {} [{}]: {}",
                    checker["checker_id"].as_str().unwrap_or("未知检查器"),
                    checker["category"].as_str().unwrap_or("未知类别"),
                    checker["configuration"].as_str().unwrap_or("unknown")
                );
                if checker["checker_id"] == "node.npm.audit"
                    || checker["checker_id"] == "python.pip_audit"
                {
                    println!(
                        "  构建根：{:?}；依据：{}；下一步：{}",
                        checker["build_root"].as_str().unwrap_or("未知"),
                        checker["reason"].as_str().unwrap_or("unknown"),
                        checker["next_action"]
                            .as_str()
                            .unwrap_or("核对项目原审计配置")
                    );
                }
            }
        }
        if let Some(tools) = report["discovery"]["native_tool_candidates"].as_array() {
            for tool in tools {
                if parsed.selection == Selection::Java && tool["checker_id"] != "java.maven" {
                    continue;
                }
                println!(
                    "原生工具候选 {} [{:?}]：{}；版本 {}；下一步：{}（未执行）",
                    tool["checker_id"].as_str().unwrap_or("未知"),
                    tool["build_root"].as_str().unwrap_or("未知"),
                    tool["state"].as_str().unwrap_or("unknown"),
                    tool["observed_version"].as_str().unwrap_or("未知"),
                    tool["next_action"]
                        .as_str()
                        .unwrap_or("核对原生工具及项目配置")
                );
            }
        }
        if let Some(files) = report["native_results"]["python_lint"]["files"].as_array() {
            for file in files {
                let path = file["path"].as_str().unwrap_or("未知目标");
                let status = file["run_status"].as_str().unwrap_or("incomplete");
                let reason = file["reason"].as_str().unwrap_or("none");
                println!("Python/Ruff {path}: {status} ({reason})");
                for finding in file["findings"].as_array().into_iter().flatten() {
                    println!("  {}", finding["rule_id"].as_str().unwrap_or("未知规则"));
                }
            }
        }
        if let Some(scans) = report["native_results"]["python_cve"].as_array() {
            for scan in scans {
                println!(
                    "Python/pip-audit 构建根 {}：{}；原生 advisory {} 项；交付未评估",
                    scan["build_root"].as_str().unwrap_or("未知"),
                    scan["reason"].as_str().unwrap_or("未完成"),
                    scan["findings"].as_array().map_or(0, Vec::len)
                );
                for finding in scan["findings"].as_array().into_iter().flatten() {
                    println!(
                        "  {} {} {}；CVE {}",
                        finding["advisory_id"],
                        finding["package_name"],
                        finding["package_version"],
                        finding["cve_aliases"]
                    );
                }
            }
        }
        if report["native_results"]["rust_comments"].is_object() {
            let docs = &report["native_results"]["rust_comments"];
            println!(
                "Rust文档观察：{}；任务同步：{}；原配置与完整目标覆盖待核验",
                docs["reason"], docs["backlog_status"]
            );
            for finding in docs["findings"].as_array().into_iter().flatten() {
                println!(
                    "原生文档规则 {}：{}:{}；{}",
                    finding["rule_id"],
                    finding["path"],
                    finding["line"],
                    finding["repair_brief"]["rule_basis"]
                );
            }
        }
        if report["native_results"]["rust_build"].is_object() {
            let build = &report["native_results"]["rust_build"];
            println!(
                "Rust原生类型检查：{}；任务同步：{}；测试执行：否；完整构建组合与策略待核验",
                build["reason"], build["backlog_status"]
            );
            for finding in build["findings"].as_array().into_iter().flatten() {
                println!(
                    "原生编译规则 {}：{}:{}；{}",
                    finding["rule_id"],
                    finding["path"],
                    finding["line"],
                    finding["repair_brief"]["rule_basis"]
                );
            }
        }
        if report["native_results"]["rust_cve"].is_object() {
            let cve = &report["native_results"]["rust_cve"];
            println!("Rust原生CVE观察：{}；漏洞库时效未核验", cve["reason"]);
            for finding in cve["findings"].as_array().into_iter().flatten() {
                println!(
                    "原生advisory {}：{} {}；CVE {}",
                    finding["advisory_id"],
                    finding["package_name"],
                    finding["package_version"],
                    finding["cve_aliases"]
                );
            }
        }
        if let Some(findings) = report["native_results"]["rust_lint"]["findings"].as_array() {
            println!(
                "Rust/Cargo Clippy 配置: {}；原生状态: {}",
                report["native_results"]["rust_lint"]["configuration"]
                    .as_str()
                    .unwrap_or("unknown"),
                report["native_results"]["rust_lint"]["reason"]
                    .as_str()
                    .unwrap_or("unknown")
            );
            println!(
                "Rust/Cargo Clippy: {} 条原生诊断（尚未核验完整范围与策略）",
                findings.len()
            );
            for finding in findings {
                println!(
                    "  {}:{} {}",
                    finding["path"], finding["line"], finding["rule_id"]
                );
            }
            println!(
                "复检: {}",
                report["native_results"]["rust_lint"]["recheck_command"]
                    .as_str()
                    .unwrap_or(
                        "cargo clippy --locked --offline --all-targets --message-format=json"
                    )
            );
        }
        if let Some(files) = report["native_results"]["erlang_lint"]["files"].as_array() {
            println!("Erlang 原生 forms；项目完整 lint、预处理、编译和测试仍待核验");
            for file in files {
                println!(
                    "  {}：{}；{}",
                    file["path"].as_str().unwrap_or("未知路径"),
                    file["native"]["reason"].as_str().unwrap_or("未观察"),
                    file["next_action"].as_str().unwrap_or("重跑原工具")
                );
                for finding in file["findings"].as_array().into_iter().flatten() {
                    println!(
                        "  原生语法错误：{}:{}:{}",
                        file["path"].as_str().unwrap_or("未知路径"),
                        finding["line"],
                        finding["column"]
                    );
                }
                if !file["recheck_argv"].is_null() {
                    println!("  复检 argv：{}", file["recheck_argv"]);
                }
                if !file["task_id"].is_null() {
                    println!(
                        "  稳定修复任务：{}；运行 codeguard next 获取当前动作",
                        file["task_id"]
                    );
                }
                if let Some(reason) = file["task_sync_reason"].as_str() {
                    println!("  任务同步未完成：{reason}；原生观察仍保留");
                }
            }
        }
        if let Some(findings) = report["native_results"]["go_lint"]["findings"].as_array() {
            println!(
                "Go/vet：{}；已完成模块 {}/{}；原生诊断 {} 条（未核验完整范围与策略）",
                report["native_results"]["go_lint"]["native_status"],
                report["native_results"]["go_lint"]["modules_completed"],
                report["native_results"]["go_lint"]["module_count"],
                findings.len()
            );
            for finding in findings {
                println!(
                    "  {}:{} {}",
                    finding["path"], finding["line"], finding["rule_id"]
                );
            }
        }
        if let Some(files) = report["native_results"]["java_p3c"]["files"].as_array() {
            for file in files {
                println!(
                    "Java/P3C {}: 配置 {}；原生状态 {}",
                    file["path"].as_str().unwrap_or("未知目标"),
                    file["configuration"].as_str().unwrap_or("unknown"),
                    file["observation"]["reason"]
                        .as_str()
                        .or_else(|| file["reason"].as_str())
                        .unwrap_or("unknown")
                );
                for finding in file["observation"]["findings"]
                    .as_array()
                    .into_iter()
                    .flatten()
                {
                    println!(
                        "  {}:{} {}",
                        finding["path"], finding["line"], finding["rule_id"]
                    );
                }
            }
        }
        if let Some(files) = report["native_results"]["java_javadoc"]["files"].as_array() {
            for file in files {
                println!(
                    "Java/Javadoc {}: 配置 {}；局部状态 {}",
                    file["path"].as_str().unwrap_or("未知目标"),
                    file["configuration"].as_str().unwrap_or("unknown"),
                    file["observation"]["reason"]
                        .as_str()
                        .or_else(|| file["reason"].as_str())
                        .unwrap_or("unknown")
                );
                for finding in file["observation"]["findings"]
                    .as_array()
                    .into_iter()
                    .flatten()
                {
                    println!(
                        "  {}:{} {}",
                        finding["path"].as_str().unwrap_or("未知目标"),
                        finding["line"].as_u64().unwrap_or(0),
                        finding["rule_id"].as_str().unwrap_or("未知规则")
                    );
                }
            }
        }
        if let Some(probes) =
            report["native_results"]["java_javadoc"]["maven_multifile_probes"].as_array()
        {
            for probe in probes {
                println!(
                    "Java/Maven Javadoc 多文件探针 {}: {} ({})；项目归属未核验",
                    probe["build_root"].as_str().unwrap_or("."),
                    probe["observation"]["native_status"]
                        .as_str()
                        .unwrap_or("incomplete"),
                    probe["observation"]["reason"].as_str().unwrap_or("unknown")
                );
                for finding in probe["observation"]["findings"]
                    .as_array()
                    .into_iter()
                    .flatten()
                {
                    println!(
                        "  {}:{} {}",
                        finding["path"], finding["line"], finding["rule_id"]
                    );
                }
            }
        }
        if let Some(probes) = report["native_results"]["java_dependencies"]["probes"].as_array() {
            for probe in probes {
                let observation = &probe["observation"];
                println!(
                    "Java/Maven 依赖图 {}: {} ({})；仅为图观察，未检查 CVE/许可证",
                    probe["build_root"].as_str().unwrap_or("."),
                    observation["native_status"]
                        .as_str()
                        .unwrap_or("incomplete"),
                    observation["reason"].as_str().unwrap_or("unknown")
                );
                for component in observation["nodes"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .skip(1)
                    .take(20)
                {
                    println!(
                        "  {}:{}:{} [{}]",
                        component["group_id"].as_str().unwrap_or("?"),
                        component["artifact_id"].as_str().unwrap_or("?"),
                        component["version"].as_str().unwrap_or("?"),
                        component["scope"].as_str().unwrap_or("?")
                    );
                }
            }
        }
        if let Some(probes) = report["native_results"]["java_cve"]["probes"].as_array() {
            for probe in probes {
                let observation = &probe["observation"];
                println!(
                    "Java/OWASP CVE {}: {} ({})；漏洞库时效未核验",
                    probe["build_root"].as_str().unwrap_or("."),
                    observation["native_status"]
                        .as_str()
                        .unwrap_or("incomplete"),
                    observation["reason"].as_str().unwrap_or("unknown")
                );
                for advisory in observation["advisories"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .take(20)
                {
                    println!(
                        "  {} {} 分数 {}{}",
                        advisory["source"].as_str().unwrap_or("?"),
                        advisory["advisory_id"].as_str().unwrap_or("?"),
                        advisory["score"]
                            .as_f64()
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "UNKNOWN".into()),
                        if advisory["suppressed_by_native_tool"] == true {
                            " [原生抑制]"
                        } else {
                            ""
                        }
                    );
                }
            }
        }
        if let Some(probes) = report["native_results"]["java_cve"]["attribution_probes"].as_array()
        {
            for probe in probes {
                println!(
                    "Java/CVE 依赖归属 {}: {}；坐标和制品摘要仅为本地候选，漏洞库时效未核验",
                    probe["build_root"].as_str().unwrap_or("."),
                    probe["status"].as_str().unwrap_or("native_report_invalid")
                );
                for binding in probe["bindings"].as_array().into_iter().flatten().take(20) {
                    println!(
                        "  漏洞观察 #{}: {} / {}",
                        binding["observation_index"],
                        binding["coordinate_state"],
                        binding["artifact_state"]
                    );
                }
            }
        }
        for scan in report["native_results"]["npm_cve"]
            .as_array()
            .into_iter()
            .flatten()
        {
            println!(
                "npm审计根 {}：{}；原因 {}；组件 {}；任务同步 {}；下一步 {}",
                scan["build_root"],
                scan["feedback"]["status"],
                scan["feedback"]["reason"],
                scan["feedback"]["dependency_total"],
                scan["backlog_status"],
                scan["feedback"]["next_action"]
            );
            for component in scan["feedback"]["findings"]
                .as_array()
                .into_iter()
                .flatten()
            {
                println!(
                    "  组件引用 {}；严重度 {}；advisory {}；锁节点版本 {}",
                    component["component_ref"],
                    component["severity"],
                    component["advisory_sources"],
                    component["nodes"]
                );
            }
        }
        for task in &execution_tasks {
            println!("执行任务 {}: {}", task["id"], task["status"]);
        }
        if let Some(files) = report["native_results"]["node_lint"]["files"].as_array() {
            for file in files {
                println!(
                    "ESLint 文件：{}；原因：{}",
                    file["path"], file["feedback"]["reason"]
                );
                for finding in file["feedback"]["findings"]
                    .as_array()
                    .into_iter()
                    .flatten()
                {
                    println!(
                        "原生规则 {}，行 {}，列 {}，严重度 {}",
                        finding["rule_id"], finding["line"], finding["column"], finding["severity"]
                    );
                }
                println!("下一步：{}", file["feedback"]["next_action"]);
            }
            println!(
                "ESLint 工作台：{}",
                report["native_results"]["node_lint"]["backlog_status"]
            );
        }
        if let Some(observations) = report["syntax_candidates"]["observations"].as_array() {
            if report["syntax_candidates"]["status"] == "not_run" {
                if report["syntax_candidates"]["reason"] == "native_preferred" {
                    println!(
                        "候选语法初检未运行：{} 个文件已由本轮原生工具检查；其余交付义务仍待核验。",
                        report["syntax_candidates"]["native_preferred_count"]
                    );
                } else {
                    println!(
                        "候选语法初检未运行：{}；仍需适用原生检查。",
                        report["syntax_candidates"]["reason"]
                    );
                }
            } else {
                let suspected = observations
                    .iter()
                    .filter(|item| {
                        item["recovery_count"]
                            .as_u64()
                            .is_some_and(|count| count > 0)
                    })
                    .count();
                println!(
                    "候选语法初检：{} 个片段已观察，{} 个存在待原生确认的恢复节点，{} 个文件由原生检查覆盖，{} 个范围未执行；仍未完成完整检查。",
                    observations
                        .iter()
                        .filter(|item| item["status"] == "candidate_observed")
                        .count(),
                    suspected,
                    report["syntax_candidates"]["native_preferred_count"],
                    report["syntax_candidates"]["skipped_count"]
                );
                for item in observations
                    .iter()
                    .filter(|item| {
                        item["recovery_count"]
                            .as_u64()
                            .is_some_and(|count| count > 0)
                    })
                    .take(8)
                {
                    println!(
                        "  {} [{}] 候选恢复 {} 处；先运行适用原生工具确认。",
                        item["path"], item["language"], item["recovery_count"]
                    );
                }
                for item in observations
                    .iter()
                    .filter(|item| item["reason"] == "syntax_recovery_incomplete")
                    .take(8)
                {
                    println!(
                        "  {} [{}] grammar 报告错误但恢复位置不完整；初检未完成，需适用原生工具确认。",
                        item["path"].as_str().unwrap_or("?"),
                        item["language"].as_str().unwrap_or("unknown")
                    );
                }
                for item in observations
                    .iter()
                    .filter(|item| item["known_limitations"][0].is_string())
                    .take(8)
                {
                    if let Some(limitation) = item["known_limitations"][0].as_str() {
                        println!(
                            "  {} [{}] 已知 grammar 限制：{}；仍需适用原生工具确认。",
                            item["path"].as_str().unwrap_or("?"),
                            item["language"].as_str().unwrap_or("unknown"),
                            limitation
                        );
                    }
                }
            }
        }
        if report["unresolved_conditions"]
            .as_array()
            .is_some_and(|reasons| {
                reasons
                    .iter()
                    .any(|reason| reason == "project_scope_changed_during_check")
            })
        {
            println!("项目源码或检查配置在本轮执行期间变化；重新发现项目并运行原工具复检。");
        }
        if report["unresolved_conditions"]
            .as_array()
            .is_some_and(|reasons| {
                reasons
                    .iter()
                    .any(|reason| reason == "project_scope_recheck_deadline_exceeded")
            })
        {
            println!("本轮截止时间已用尽，未完成项目范围复核；延长预算后重新检查。");
        }
        if report["unresolved_conditions"]
            .as_array()
            .is_some_and(|reasons| {
                reasons
                    .iter()
                    .any(|reason| reason == "project_source_changed_during_check")
            })
        {
            println!(
                "项目源码字节在本轮执行期间变化；核对并发编辑或检查器副作用，再运行原工具复检。"
            );
        }
        if report["unresolved_conditions"]
            .as_array()
            .is_some_and(|reasons| {
                reasons.iter().any(|reason| {
                    matches!(
                        reason.as_str(),
                        Some(
                            "project_source_snapshot_unavailable"
                                | "project_source_recheck_unavailable"
                        )
                    )
                })
            })
        {
            println!("本轮源码快照不可安全读取或超过预算；核对路径、权限和规模后重新检查。");
        }
        if report["unresolved_conditions"]
            .as_array()
            .is_some_and(|reasons| {
                reasons.iter().any(|reason| {
                    matches!(
                        reason.as_str(),
                        Some(
                            "project_source_snapshot_deadline_exceeded"
                                | "project_source_recheck_deadline_exceeded"
                        )
                    )
                })
            })
        {
            println!("本轮截止时间不足以完成源码字节复核；延长预算后重新检查。");
        }
        for candidate in &candidates {
            println!(
                "候选 {} / {}{}: {}（{}）；下一步：{}",
                candidate["language"].as_str().unwrap_or("未知语言"),
                candidate["category"].as_str().unwrap_or("未知类别"),
                candidate["checker_id"]
                    .as_str()
                    .map(|id| format!(" ({id})"))
                    .unwrap_or_default(),
                candidate["status"].as_str().unwrap_or("not_integrated"),
                candidate["reason"].as_str().unwrap_or("unknown"),
                candidate["next_action"]
                    .as_str()
                    .unwrap_or("核对该类别的项目配置和适配器能力")
            );
        }
        if let Some(brief) = report["next"]["repair_brief"].as_object() {
            println!(
                "下一步任务 {}：{}；范围 {}；复检 {}",
                brief["task_id"], brief["step"], brief["scope"], brief["recheck_argv"]
            );
        }
        println!(
            "候选类别：{}；正式必需义务未确认；交付决策：{}",
            candidates.len(),
            report["delivery_decision"].as_str().unwrap_or("incomplete")
        );
    }
    ExitCode::from(if request_cancelled { 130 } else { 3 })
}

fn discovery_scope_changed(before: &DiscoveryReport, after: &DiscoveryReport) -> bool {
    before.observation_complete != after.observation_complete
        || before.languages != after.languages
        || before.build_roots != after.build_roots
        || before.declared_versions != after.declared_versions
        || before.manifest_sha256 != after.manifest_sha256
        || before.lock_sha256 != after.lock_sha256
        || before.checker_config_sha256 != after.checker_config_sha256
        || before.lockfiles != after.lockfiles
        || before.checker_configurations != after.checker_configurations
        || before.source_set_candidates != after.source_set_candidates
        || before.blocked_paths != after.blocked_paths
        || before.unknown_conditions != after.unknown_conditions
}

fn task_status(outcome: TaskOutcome) -> &'static str {
    match outcome {
        TaskOutcome::Succeeded => "native_observed_unverified",
        TaskOutcome::Failed => "native_incomplete",
        TaskOutcome::Cancelled => "cancelled",
        TaskOutcome::TimedOut => "deadline_exceeded",
        TaskOutcome::DependencyFailed => "dependency_failed",
        TaskOutcome::CancelledBeforeStart => "cancelled_before_start",
        TaskOutcome::DeadlineBeforeStart => "deadline_before_start",
        TaskOutcome::InternalFailure => "internal_failure",
    }
}

fn emit_structured(report: &Value, args: &Args) {
    let mut feedback = report.clone();
    if let Some(path) = &args.output {
        feedback["export"] = json!({"status":"saved","reason_code":null});
        let intended = serialize_structured(&feedback, args.format);
        match export_report(path, intended.as_bytes()) {
            Ok(()) => {
                print!("{intended}");
                return;
            }
            Err(reason) => {
                feedback["export"] = json!({"status":"failed","reason_code":reason});
                eprintln!(
                    "报告导出失败（{reason}）；完整检查反馈已保留在 stdout，请修复目标路径后重试"
                );
            }
        }
    }
    print!("{}", serialize_structured(&feedback, args.format));
}

fn serialize_structured(report: &Value, format: OutputFormat) -> String {
    let document = if format == OutputFormat::Sarif {
        partial_check_sarif(report)
    } else {
        report.clone()
    };
    format!("{document}\n")
}

fn snapshot_native_slot(slot: &Mutex<Option<Value>>) -> Value {
    slot.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
        .unwrap_or(Value::Null)
}

fn aborted_task_report(
    selection: Selection,
    discovery: Value,
    outcomes: &BTreeMap<String, TaskOutcome>,
    mut native_results: Value,
    cancelled: bool,
) -> Value {
    if native_results.get("erlang_lint").is_none() {
        native_results["erlang_lint"] = Value::Null;
    }
    if native_results.get("node_lint").is_none() {
        native_results["node_lint"] = Value::Null;
    }
    if native_results.get("rust_cve").is_none() {
        native_results["rust_cve"] = Value::Null;
    }
    if native_results.get("rust_comments").is_none() {
        native_results["rust_comments"] = Value::Null;
    }
    if native_results.get("npm_cve").is_none() {
        native_results["npm_cve"] = json!([]);
    }
    if native_results.get("python_cve").is_none() {
        native_results["python_cve"] = json!([]);
    }
    let failed_task_ids: Vec<_> = outcomes
        .iter()
        .filter(|(_, outcome)| **outcome == TaskOutcome::InternalFailure)
        .map(|(id, _)| id.clone())
        .collect();
    let execution_tasks: Vec<_> = outcomes
        .iter()
        .map(|(id, outcome)| json!({"id":id,"status":task_status(*outcome)}))
        .collect();
    json!({
        "schema_version":"0.13.0", "report_type":"check_aborted",
        "operation":"check", "selection":selection.as_str(),
        "command_status":if cancelled { "cancelled" } else { "internal_error" },
        "exit_code":if cancelled { 130 } else { 4 },
        "delivery_decision":if selection == Selection::All { "incomplete" } else { "not_evaluated" },
        "authority":"local_unverified", "reason":if cancelled { "request_cancelled" } else { "native_task_internal_failure" },
        "failed_task_ids":failed_task_ids, "execution_tasks":execution_tasks,
        "discovery":discovery, "native_results":native_results,
        "export":{"status":"not_requested","reason_code":null}
    })
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let selection = match args.first().map(String::as_str) {
        Some("all") => Selection::All,
        Some("java") => Selection::Java,
        _ => return Err("当前 check 支持 all 或 java".into()),
    };
    let mut root = None;
    let mut ruff_tool = None;
    let mut pip_audit_tool = None;
    let mut pip_audit_version = None;
    let mut npm_options = BTreeMap::new();
    let mut cargo_tool = None;
    let mut cargo_audit_tool = None;
    let mut rustsec_db = None;
    let mut go_tool = None;
    let mut erl_tool = None;
    let mut maven_tool = None;
    let mut java_home = None;
    let mut maven_repo = None;
    let mut repo_sha256 = None;
    let mut cve_data_dir = None;
    let mut cve_data_sha256 = None;
    let mut format = OutputFormat::Human;
    let mut output = None;
    let mut timeout_ms = 0;
    let mut timeout_seen = false;
    let mut jobs = None;
    let mut index = 1;
    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "--node-tool" | "--npm-entry" | "--npm-version" | "--userconfig" | "--globalconfig"
            | "--registry" => {
                index += 1;
                let value = args.get(index).ok_or("npm审计参数缺少值")?;
                if npm_options.insert(arg.clone(), value.clone()).is_some() {
                    return Err("npm审计参数重复".into());
                }
            }
            "--ruff-tool" => {
                index += 1;
                if ruff_tool
                    .replace(PathBuf::from(args.get(index).ok_or("缺少 Ruff 路径")?))
                    .is_some()
                {
                    return Err("--ruff-tool 重复".into());
                }
            }
            "--pip-audit-tool" => {
                index += 1;
                if pip_audit_tool
                    .replace(PathBuf::from(args.get(index).ok_or("缺少 pip-audit 路径")?))
                    .is_some()
                {
                    return Err("--pip-audit-tool 重复".into());
                }
            }
            "--pip-audit-version" => {
                index += 1;
                if pip_audit_version
                    .replace(args.get(index).ok_or("缺少 pip-audit 版本")?.clone())
                    .is_some()
                {
                    return Err("--pip-audit-version 重复".into());
                }
            }
            "--cargo-tool" => {
                index += 1;
                if cargo_tool
                    .replace(PathBuf::from(args.get(index).ok_or("缺少 Cargo 路径")?))
                    .is_some()
                {
                    return Err("--cargo-tool 重复".into());
                }
            }
            "--cargo-audit-tool" => {
                index += 1;
                if cargo_audit_tool
                    .replace(PathBuf::from(
                        args.get(index).ok_or("缺少 cargo-audit 路径")?,
                    ))
                    .is_some()
                {
                    return Err("--cargo-audit-tool 重复".into());
                }
            }
            "--rustsec-db" => {
                index += 1;
                if rustsec_db
                    .replace(PathBuf::from(
                        args.get(index).ok_or("缺少 RustSec 数据库路径")?,
                    ))
                    .is_some()
                {
                    return Err("--rustsec-db 重复".into());
                }
            }
            "--erl-tool" => {
                index += 1;
                if erl_tool
                    .replace(PathBuf::from(
                        args.get(index).ok_or("缺少 Erlang 工具路径")?,
                    ))
                    .is_some()
                {
                    return Err("--erl-tool 重复".into());
                }
            }
            "--go-tool" => {
                index += 1;
                if go_tool
                    .replace(PathBuf::from(args.get(index).ok_or("缺少 Go 路径")?))
                    .is_some()
                {
                    return Err("--go-tool 重复".into());
                }
            }
            "--maven-tool" => {
                index += 1;
                if maven_tool
                    .replace(PathBuf::from(args.get(index).ok_or("缺少 Maven 路径")?))
                    .is_some()
                {
                    return Err("--maven-tool 重复".into());
                }
            }
            "--java-home" => {
                index += 1;
                if java_home
                    .replace(PathBuf::from(args.get(index).ok_or("缺少 JAVA_HOME 路径")?))
                    .is_some()
                {
                    return Err("--java-home 重复".into());
                }
            }
            "--maven-repo" => {
                index += 1;
                if maven_repo
                    .replace(PathBuf::from(
                        args.get(index).ok_or("缺少离线 Maven 仓路径")?,
                    ))
                    .is_some()
                {
                    return Err("--maven-repo 重复".into());
                }
            }
            "--repo-sha256" => {
                index += 1;
                if repo_sha256
                    .replace(args.get(index).ok_or("缺少 Maven 仓摘要")?.clone())
                    .is_some()
                {
                    return Err("--repo-sha256 重复".into());
                }
            }
            "--cve-data-dir" => {
                index += 1;
                if cve_data_dir
                    .replace(PathBuf::from(args.get(index).ok_or("缺少 CVE 数据目录")?))
                    .is_some()
                {
                    return Err("--cve-data-dir 重复".into());
                }
            }
            "--cve-data-sha256" => {
                index += 1;
                if cve_data_sha256
                    .replace(args.get(index).ok_or("缺少 CVE 数据摘要")?.clone())
                    .is_some()
                {
                    return Err("--cve-data-sha256 重复".into());
                }
            }
            "--timeout" => {
                index += 1;
                if timeout_seen {
                    return Err("--timeout 重复".into());
                }
                timeout_ms = parse_check_timeout(args.get(index).ok_or("缺少执行预算")?)?;
                timeout_seen = true;
            }
            "--jobs" => {
                index += 1;
                if jobs.is_some() {
                    return Err("--jobs 重复".into());
                }
                jobs = Some(parse_check_jobs(args.get(index).ok_or("缺少并行上限")?)?);
            }
            _ if arg.starts_with("--jobs=") => {
                if jobs.is_some() {
                    return Err("--jobs 重复".into());
                }
                jobs = Some(parse_check_jobs(
                    arg.strip_prefix("--jobs=").expect("匹配前缀"),
                )?);
            }
            "--format" => {
                index += 1;
                format = parse_format(args.get(index).ok_or("缺少输出格式")?)?;
            }
            "--format=json" => format = OutputFormat::Json,
            "--format=human" => format = OutputFormat::Human,
            "--format=sarif" => format = OutputFormat::Sarif,
            "--output" => {
                index += 1;
                if output
                    .replace(PathBuf::from(args.get(index).ok_or("缺少报告目标路径")?))
                    .is_some()
                {
                    return Err("--output 重复".into());
                }
            }
            _ if arg.starts_with('-') || root.is_some() => {
                return Err(format!("不支持的参数：{arg}"));
            }
            _ => root = Some(PathBuf::from(arg)),
        }
        index += 1;
    }
    if !npm_options.is_empty() {
        if selection != Selection::All {
            return Err("npm参数仅用于check all".into());
        }
        crate::npm_check_scan::validate_options(&npm_options)?;
    }
    if ruff_tool.as_ref().is_some_and(|path| !path.is_absolute()) {
        return Err("--ruff-tool 必须是绝对路径".into());
    }
    if pip_audit_tool
        .as_ref()
        .is_some_and(|path| !path.is_absolute())
        || pip_audit_tool.is_some() != pip_audit_version.is_some()
        || pip_audit_version
            .as_ref()
            .is_some_and(|version: &String| version.trim().is_empty())
    {
        return Err("pip-audit 需要成对提供绝对工具路径与非空版本".into());
    }
    if output.is_some() && format == OutputFormat::Human {
        return Err("--output 当前要求 --format json|sarif".into());
    }
    if cargo_tool.as_ref().is_some_and(|path| !path.is_absolute()) {
        return Err("--cargo-tool 必须是绝对路径".into());
    }
    if cargo_audit_tool
        .as_ref()
        .is_some_and(|path| !path.is_absolute())
        || rustsec_db.as_ref().is_some_and(|path| !path.is_absolute())
    {
        return Err("Rust CVE 工具和数据库必须是绝对路径".into());
    }
    if erl_tool.as_ref().is_some_and(|path| !path.is_absolute()) {
        return Err("--erl-tool 必须是绝对路径".into());
    }
    if go_tool.as_ref().is_some_and(|path| !path.is_absolute()) {
        return Err("--go-tool 必须是绝对路径".into());
    }
    if selection == Selection::Java
        && (ruff_tool.is_some()
            || pip_audit_tool.is_some()
            || cargo_tool.is_some()
            || cargo_audit_tool.is_some()
            || rustsec_db.is_some()
            || go_tool.is_some()
            || erl_tool.is_some())
    {
        return Err("check java 不接受其它语言的原生工具参数".into());
    }
    if [
        maven_tool.as_ref(),
        java_home.as_ref(),
        maven_repo.as_ref(),
        cve_data_dir.as_ref(),
    ]
    .into_iter()
    .flatten()
    .any(|path| !path.is_absolute())
    {
        return Err("Java 原生工具路径必须是绝对路径".into());
    }
    if repo_sha256.as_ref().is_some_and(|value: &String| {
        value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    }) {
        return Err("--repo-sha256 必须是小写 SHA-256".into());
    }
    if cve_data_sha256.as_ref().is_some_and(|value: &String| {
        value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    }) {
        return Err("--cve-data-sha256 必须是小写 SHA-256".into());
    }
    let (timeout_ms, timeout_source) = select_check_timeout(timeout_seen.then_some(timeout_ms))?;
    let (jobs_limit, jobs_source) = select_check_jobs(jobs)?;
    Ok(Args {
        selection,
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        ruff_tool,
        pip_audit_tool,
        pip_audit_version,
        npm_options,
        cargo_tool,
        cargo_audit_tool,
        rustsec_db,
        go_tool,
        erl_tool,
        maven_tool,
        java_home,
        maven_repo,
        repo_sha256,
        cve_data_dir,
        cve_data_sha256,
        format,
        output,
        timeout_ms,
        timeout_source,
        jobs_limit,
        jobs_source,
    })
}

fn parse_format(value: &str) -> Result<OutputFormat, String> {
    match value {
        "json" => Ok(OutputFormat::Json),
        "human" => Ok(OutputFormat::Human),
        "sarif" => Ok(OutputFormat::Sarif),
        _ => Err(format!("不支持的输出格式：{value}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use codeguard_runtime::TaskOutcome;
    use serde_json::json;

    use super::{Selection, aborted_task_report};

    #[test]
    fn internal_task_failure_keeps_sibling_native_findings_visible() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../schemas/check-aborted.schema.json"))
                .unwrap();
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(schema["properties"]["exit_code"]["enum"], json!([4, 130]));
        let outcomes = BTreeMap::from([
            ("python.lint".into(), TaskOutcome::Succeeded),
            ("rust.lint".into(), TaskOutcome::InternalFailure),
        ]);
        let native_results = json!({
            "python_lint":{"files":[{"path":"app.py","findings":[{"rule_id":"F401"}]}]},
            "rust_lint":null,"go_lint":null,"java_p3c":null,"java_javadoc":null,
            "java_dependencies":null,"java_cve":null
        });
        let report = aborted_task_report(
            Selection::All,
            json!({"observation_complete":true}),
            &outcomes,
            native_results,
            false,
        );
        assert_eq!(report["command_status"], "internal_error");
        assert_eq!(report["exit_code"], 4);
        assert_eq!(report["delivery_decision"], "incomplete");
        assert_eq!(
            report["native_results"]["python_lint"]["files"][0]["findings"][0]["rule_id"],
            "F401"
        );
        assert_eq!(report["failed_task_ids"], json!(["rust.lint"]));
        assert_eq!(
            report["execution_tasks"][0]["status"],
            "native_observed_unverified"
        );
        assert_eq!(report["execution_tasks"][1]["status"], "internal_failure");
    }

    #[test]
    fn cancellation_precedes_internal_failure_and_preserves_prior_finding() {
        let outcomes = BTreeMap::from([
            ("python.lint".into(), TaskOutcome::Succeeded),
            ("rust.lint".into(), TaskOutcome::InternalFailure),
            ("java.p3c".into(), TaskOutcome::Cancelled),
        ]);
        let report = aborted_task_report(
            Selection::All,
            json!({"observation_complete":true}),
            &outcomes,
            json!({
                "python_lint":{"files":[{"findings":[{"rule_id":"F401"}]}]},
                "rust_lint":null,"go_lint":null,"java_p3c":null,"java_javadoc":null,
                "java_dependencies":null,"java_cve":null
            }),
            true,
        );
        assert_eq!(report["command_status"], "cancelled");
        assert_eq!(report["exit_code"], 130);
        assert_eq!(report["reason"], "request_cancelled");
        assert_eq!(report["failed_task_ids"], json!(["rust.lint"]));
        assert_eq!(
            report["native_results"]["python_lint"]["files"][0]["findings"][0]["rule_id"],
            "F401"
        );
    }
}
