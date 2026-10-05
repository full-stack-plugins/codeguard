//! 使用任务对应的原生检查器进行观察复检；未经可信策略核验不关闭任务。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use codeguard_adapters::legacy_registry;
use codeguard_runtime::{NativeObservation, read_bounded_regular_file};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::check_budget::{
    budget_record, parse_check_timeout, resolve_project_default, select_check_timeout,
};
use crate::discovery::discover;
use crate::java_cve_attribution::attach_candidates;
use crate::java_cve_scan::{
    NativeContext as CveNativeContext, observe_project as observe_cve_project,
};
use crate::java_p3c_command::has_projectable_findings;
use crate::java_p3c_scan::{NativeContext, observe_project};
use crate::next_command::read_task_brief;
use crate::python_lint_command::scan_local_report_with_deadline;
use crate::rust_lint_scan::{observe_cargo_clippy, observe_cargo_clippy_force_warn};
use crate::task_attempt_command::latest_ready_attempt;
use crate::task_lease_command::{
    begin_verification, finish_verification, lock_verification, valid_owner, valid_token,
};
use crate::work_sync::{save_local_report, sync_local_workspace, write_once};

struct Arguments {
    task_id: String,
    eslint_options: std::collections::BTreeMap<String, String>,
    npm_options: std::collections::BTreeMap<String, String>,
    root: PathBuf,
    ruff_tool: Option<PathBuf>,
    zig_tool: Option<PathBuf>,
    erl_tool: Option<PathBuf>,
    swift_tool: Option<PathBuf>,
    ruby_tool: Option<PathBuf>,
    kotlinc_tool: Option<PathBuf>,
    cargo_tool: Option<PathBuf>,
    cargo_audit_tool: Option<PathBuf>,
    rustsec_db: Option<PathBuf>,
    pip_audit_tool: Option<PathBuf>,
    pip_audit_version: Option<String>,
    go_tool: Option<PathBuf>,
    maven_tool: Option<PathBuf>,
    java_home: Option<PathBuf>,
    java_tool: Option<PathBuf>,
    checkstyle_jar: Option<PathBuf>,
    checkstyle_config: Option<PathBuf>,
    maven_repo: Option<PathBuf>,
    repo_sha256: Option<String>,
    cve_data_dir: Option<PathBuf>,
    cve_data_sha256: Option<String>,
    owner: Option<String>,
    lease_token: Option<String>,
    json: bool,
    timeout_ms: u64,
    timeout_source: &'static str,
}

/// 按任务检查器调用对应原生扫描，持久记录局部观察结果。
pub fn run(args: &[String]) -> ExitCode {
    let mut parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let started = Instant::now();
    let root = match parsed.root.canonicalize() {
        Ok(root) if root.is_dir() => root,
        _ => return print_unavailable(&parsed, "project_unreadable"),
    };
    match resolve_project_default(&root, parsed.timeout_ms, parsed.timeout_source) {
        Ok((timeout_ms, source)) => {
            parsed.timeout_ms = timeout_ms;
            parsed.timeout_source = source;
        }
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    }
    let deadline = started + Duration::from_millis(parsed.timeout_ms);
    let mut brief = match read_task_brief(&root, &parsed.task_id) {
        Ok(brief) => brief,
        Err(reason) => return print_unavailable(&parsed, reason),
    };
    let python_confirmation = brief["checker_id"] == "python.ruff"
        && brief["reason_code"] == "python_syntax_confirmation_needed";
    let python_original = if python_confirmation {
        match crate::python_confirmation_recheck::original_reference(&root, &brief) {
            Ok(reference) => Some(reference),
            Err(reason) => return print_unavailable(&parsed, reason),
        }
    } else {
        None
    };
    let syntax_task = brief["checker_id"] == "syntax.native_confirmation";
    if (parsed.zig_tool.is_some()
        || parsed.erl_tool.is_some()
        || parsed.swift_tool.is_some()
        || parsed.ruby_tool.is_some()
        || parsed.kotlinc_tool.is_some())
        && !syntax_task
    {
        eprintln!("语法工具参数仅用于对应的原生语法确认任务");
        return ExitCode::from(2);
    }
    // 语言与工具在租约及原生启动前核对，不能先取得租约再发现错参。
    if syntax_task
        && (parsed.zig_tool.is_some()
            || parsed.erl_tool.is_some()
            || parsed.swift_tool.is_some()
            || parsed.ruby_tool.is_some()
            || parsed.kotlinc_tool.is_some()
            || parsed.go_tool.is_some())
    {
        let original = match crate::syntax_task_recheck::original(&root, &brief) {
            Ok(original) => original,
            Err(reason) => return print_unavailable(&parsed, reason),
        };
        if (parsed.zig_tool.is_some() && original["language"] != "zig")
            || (parsed.erl_tool.is_some() && original["language"] != "erlang")
            || (parsed.swift_tool.is_some() && original["language"] != "swift")
            || (parsed.ruby_tool.is_some() && original["language"] != "ruby")
            || (parsed.kotlinc_tool.is_some() && original["language"] != "kotlin")
            || (parsed.go_tool.is_some() && original["language"] != "go")
        {
            eprintln!("原生语法工具不匹配任务语言");
            return ExitCode::from(2);
        }
    }
    let npm_task = brief["checker_id"] == "node.npm.audit";
    if !npm_task && !parsed.npm_options.is_empty() {
        eprintln!("npm 参数仅用于对应任务");
        return ExitCode::from(2);
    }
    let eslint_task = matches!(
        brief["checker_id"].as_str(),
        Some("node.eslint" | "node.eslint.preparation")
    );
    if !eslint_task && !npm_task && !parsed.eslint_options.is_empty() {
        eprintln!("ESLint 参数仅用于对应任务");
        return ExitCode::from(2);
    }
    if matches!(
        brief["checker_id"].as_str(),
        Some("java.checkstyle" | "java.checkstyle.preparation")
    ) {
        brief["evidence_workspace_id"] = crate::workspace_refresh::read_workspace_baseline(&root)
            .ok()
            .flatten()
            .and_then(|b| b.workspace_id().map(str::to_owned))
            .map_or(Value::Null, |id| json!(id));
    } else if parsed.java_tool.is_some()
        || parsed.checkstyle_jar.is_some()
        || (parsed.checkstyle_config.is_some() && !eslint_task)
    {
        eprintln!("Checkstyle 参数仅用于相应任务复检");
        return ExitCode::from(2);
    }
    if parsed.go_tool.is_some() && brief["checker_id"] != "go.vet" && !syntax_task {
        eprintln!("--go-tool 仅用于 Go 任务复检");
        return ExitCode::from(2);
    }
    if (parsed.cargo_audit_tool.is_some() || parsed.rustsec_db.is_some())
        && brief["checker_id"] != "rust.cargo_audit"
    {
        eprintln!("Rust CVE 参数仅用于对应任务复检");
        return ExitCode::from(2);
    }
    if (parsed.pip_audit_tool.is_some() || parsed.pip_audit_version.is_some())
        && brief["checker_id"] != "python.pip_audit"
    {
        eprintln!("Python CVE 参数仅用于对应任务复检");
        return ExitCode::from(2);
    }
    let lease = match begin_verification(
        &root,
        &parsed.task_id,
        parsed.owner.as_deref().zip(parsed.lease_token.as_deref()),
    ) {
        Ok(lease) => lease,
        Err(reason) => return print_unavailable(&parsed, reason),
    };
    let bound_attempt = match latest_ready_attempt(&root, &parsed.task_id) {
        Ok(attempt) => attempt,
        Err(reason) => {
            let release = finish_verification(&root, &parsed.task_id, &lease);
            return print_unavailable(&parsed, release.err().unwrap_or(reason));
        }
    };
    let cve_pom = if brief["checker_id"] == "java.maven.dependency_check" {
        let Some(path) = brief["affected_paths"][0].as_str() else {
            let release = finish_verification(&root, &parsed.task_id, &lease);
            return print_unavailable(&parsed, release.err().unwrap_or("task_scope_invalid"));
        };
        match read_bounded_regular_file(&root.join(path), 4 * 1024 * 1024) {
            Ok(bytes) => Some((path.to_owned(), format!("{:x}", Sha256::digest(bytes)))),
            Err(_) => {
                let release = finish_verification(&root, &parsed.task_id, &lease);
                return print_unavailable(&parsed, release.err().unwrap_or("cve_pom_unavailable"));
            }
        }
    } else {
        None
    };
    let source_target_before = if matches!(
        brief["checker_id"].as_str(),
        Some("rust.cargo_clippy" | "go.vet")
    ) && brief["kind"] == "finding"
    {
        let Some(path) = brief["scope"].as_str() else {
            let release = finish_verification(&root, &parsed.task_id, &lease);
            return print_unavailable(&parsed, release.err().unwrap_or("task_scope_invalid"));
        };
        match read_bounded_regular_file(&root.join(path), 16 * 1024 * 1024) {
            Ok(bytes) => Some(bytes),
            Err(_) => {
                let release = finish_verification(&root, &parsed.task_id, &lease);
                return print_unavailable(
                    &parsed,
                    release.err().unwrap_or("rust_source_unavailable"),
                );
            }
        }
    } else {
        None
    };
    let mut scan = if syntax_task {
        let recheck = if crate::syntax_task_recheck::original(&root, &brief)
            .is_ok_and(|original| original["language"] == "go")
        {
            crate::syntax_task_recheck::run_go(&root, &brief, parsed.go_tool.as_deref(), deadline)
        } else if crate::syntax_task_recheck::original(&root, &brief)
            .is_ok_and(|original| original["language"] == "ruby")
        {
            crate::syntax_task_recheck::run_ruby(
                &root,
                &brief,
                parsed.ruby_tool.as_deref(),
                deadline,
            )
        } else {
            crate::syntax_task_recheck::run(
                &root,
                &brief,
                parsed.zig_tool.as_deref(),
                parsed.erl_tool.as_deref(),
                parsed.swift_tool.as_deref(),
                parsed.kotlinc_tool.as_deref(),
                deadline,
            )
        };
        match recheck {
            Ok(report) => report,
            Err(reason) => {
                let release = finish_verification(&root, &parsed.task_id, &lease);
                return print_unavailable(&parsed, release.err().unwrap_or(reason));
            }
        }
    } else if npm_task {
        let mut options = parsed.npm_options.clone();
        options.extend(parsed.eslint_options.clone());
        match crate::npm_task_recheck::run(&root, &brief, &options, deadline) {
            Ok(report) => report,
            Err(reason) => {
                let release = finish_verification(&root, &parsed.task_id, &lease);
                return print_unavailable(&parsed, release.err().unwrap_or(reason));
            }
        }
    } else if eslint_task {
        let mut options = parsed.eslint_options.clone();
        if let Some(config) = &parsed.checkstyle_config {
            options.insert("--config".into(), config.to_string_lossy().into_owned());
        }
        crate::eslint_task_recheck::run(&root, &brief, &options, deadline)
    } else if matches!(
        brief["checker_id"].as_str(),
        Some("java.checkstyle" | "java.checkstyle.preparation")
    ) {
        let options = [
            ("--java-tool", parsed.java_tool.as_ref()),
            ("--checkstyle-jar", parsed.checkstyle_jar.as_ref()),
            ("--config", parsed.checkstyle_config.as_ref()),
        ]
        .into_iter()
        .filter_map(|(key, value)| {
            value.map(|p| (key.to_owned(), p.to_string_lossy().into_owned()))
        })
        .collect();
        if brief["checker_id"] == "java.checkstyle.preparation" {
            crate::checkstyle_preparation_recheck::run(&root, &brief, &options, deadline)
        } else {
            crate::checkstyle_task_recheck::run(&root, &brief, &options, deadline)
        }
    } else if brief["checker_id"] == "go.vet" {
        crate::go_lint_command::observe_for_check(
            &root,
            parsed.go_tool.as_deref(),
            deadline,
            &std::sync::atomic::AtomicBool::new(false),
        )
    } else if brief["checker_id"] == "java.maven.p3c" {
        let Some(registry) = legacy_registry().ok() else {
            let release = finish_verification(&root, &parsed.task_id, &lease);
            return print_unavailable(
                &parsed,
                release.err().unwrap_or("language_registry_invalid"),
            );
        };
        let discovery = discover(&root, &registry, &NativeObservation);
        if !discovery.observation_complete {
            let release = finish_verification(&root, &parsed.task_id, &lease);
            return print_unavailable(&parsed, release.err().unwrap_or("discovery_incomplete"));
        }
        let Some(sources) = discovery
            .languages
            .get("java")
            .map(|language| &language.source_files)
        else {
            let release = finish_verification(&root, &parsed.task_id, &lease);
            return print_unavailable(
                &parsed,
                release.err().unwrap_or("java_source_scope_unavailable"),
            );
        };
        observe_project(
            &root,
            sources,
            &discovery.checker_configurations,
            &NativeContext {
                manifest_sha256: &discovery.manifest_sha256,
                maven_tool: parsed.maven_tool.as_deref(),
                java_home: parsed.java_home.as_deref(),
                maven_repo: parsed.maven_repo.as_deref(),
                repo_sha256: parsed.repo_sha256.as_deref(),
                deadline,
                cancelled: &std::sync::atomic::AtomicBool::new(false),
            },
        )
    } else if brief["checker_id"] == "java.maven.dependency_check" {
        let Some(registry) = legacy_registry().ok() else {
            let release = finish_verification(&root, &parsed.task_id, &lease);
            return print_unavailable(
                &parsed,
                release.err().unwrap_or("language_registry_invalid"),
            );
        };
        let discovery = discover(&root, &registry, &NativeObservation);
        if !discovery.observation_complete {
            let release = finish_verification(&root, &parsed.task_id, &lease);
            return print_unavailable(&parsed, release.err().unwrap_or("discovery_incomplete"));
        }
        let mut report = observe_cve_project(
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
                cancelled: &std::sync::atomic::AtomicBool::new(false),
            },
        );
        attach_candidates(&mut report, &Value::Null);
        report
    } else if brief["checker_id"] == "rust.cargo_check" {
        match crate::rust_build_task_recheck::run(
            &root,
            &brief,
            parsed.cargo_tool.as_deref(),
            parsed.timeout_ms,
            parsed.timeout_source,
            deadline,
        ) {
            Ok(report) => report,
            Err(reason) => {
                let release = finish_verification(&root, &parsed.task_id, &lease);
                return print_unavailable(&parsed, release.err().unwrap_or(reason));
            }
        }
    } else if brief["checker_id"] == "rust.cargo_audit" {
        let local = crate::cargo_audit_command::observe_for_check(
            &root,
            parsed.cargo_audit_tool.as_deref(),
            parsed.rustsec_db.as_deref(),
            parsed.timeout_ms,
            parsed.timeout_source,
            deadline,
            &std::sync::atomic::AtomicBool::new(false),
        );
        match crate::cargo_audit_command::workbench_report(&root, &local) {
            Ok(Some(report)) => report,
            Ok(None) | Err(_) => {
                let release = finish_verification(&root, &parsed.task_id, &lease);
                return print_unavailable(
                    &parsed,
                    release.err().unwrap_or("rust_cve_workbench_unavailable"),
                );
            }
        }
    } else if brief["checker_id"] == "python.pip_audit" {
        let Some(build_root) = brief["build_root"].as_str() else {
            let release = finish_verification(&root, &parsed.task_id, &lease);
            return print_unavailable(
                &parsed,
                release.err().unwrap_or("python_build_root_invalid"),
            );
        };
        let project = match crate::python_cve_command::bounded_build_root(&root, build_root) {
            Ok(project) => project,
            Err(reason) => {
                let release = finish_verification(&root, &parsed.task_id, &lease);
                return print_unavailable(&parsed, release.err().unwrap_or(reason));
            }
        };
        let local = crate::python_cve_command::observe_for_check(
            &project,
            parsed.pip_audit_tool.as_deref(),
            parsed.pip_audit_version.as_deref(),
            deadline,
            parsed.timeout_ms,
            parsed.timeout_source,
            &std::sync::atomic::AtomicBool::new(false),
        );
        match crate::python_cve_command::workbench_report(&root, build_root, &local) {
            Ok(Some(report)) => report,
            Ok(None) | Err(_) => {
                let release = finish_verification(&root, &parsed.task_id, &lease);
                return print_unavailable(
                    &parsed,
                    release.err().unwrap_or("python_cve_workbench_unavailable"),
                );
            }
        }
    } else if brief["checker_id"] == "rust.cargo_rustdoc" {
        match crate::rustdoc_task_recheck::run(
            &root,
            &brief,
            parsed.cargo_tool.as_deref(),
            parsed.timeout_ms,
            parsed.timeout_source,
            deadline,
        ) {
            Ok(report) => report,
            Err(reason) => {
                let release = finish_verification(&root, &parsed.task_id, &lease);
                return print_unavailable(&parsed, release.err().unwrap_or(reason));
            }
        }
    } else if brief["checker_id"] == "rust.cargo_clippy" {
        let Some(registry) = legacy_registry().ok() else {
            let release = finish_verification(&root, &parsed.task_id, &lease);
            return print_unavailable(
                &parsed,
                release.err().unwrap_or("language_registry_invalid"),
            );
        };
        let discovery = discover(&root, &registry, &NativeObservation);
        if !discovery.observation_complete {
            let release = finish_verification(&root, &parsed.task_id, &lease);
            return print_unavailable(&parsed, release.err().unwrap_or("discovery_incomplete"));
        }
        let Some(sources) = discovery
            .languages
            .get("rust")
            .map(|language| &language.source_files)
        else {
            let release = finish_verification(&root, &parsed.task_id, &lease);
            return print_unavailable(
                &parsed,
                release.err().unwrap_or("rust_source_scope_unavailable"),
            );
        };
        observe_cargo_clippy(
            &root,
            sources,
            parsed.cargo_tool.as_deref(),
            deadline,
            &std::sync::atomic::AtomicBool::new(false),
        )
    } else if brief["checker_id"] == "python.ruff.doctor" {
        match crate::doctor_command::observe_for_verification(
            &root,
            parsed.ruff_tool.as_deref(),
            parsed.timeout_ms,
            deadline,
        ) {
            Ok(report) => report,
            Err(reason) => {
                let release = finish_verification(&root, &parsed.task_id, &lease);
                return print_unavailable(&parsed, release.err().unwrap_or(reason));
            }
        }
    } else if python_confirmation {
        let path = brief["scope"].as_str().expect("已核验首次Python范围");
        let source = root.join(path);
        let before = if source.canonicalize().ok().as_deref() == Some(source.as_path()) {
            read_bounded_regular_file(&source, 1024 * 1024).ok()
        } else {
            None
        };
        let Some(before) = before else {
            let release = finish_verification(&root, &parsed.task_id, &lease);
            return print_unavailable(
                &parsed,
                release
                    .err()
                    .unwrap_or("python_confirmation_source_unavailable"),
            );
        };
        match crate::python_lint_command::scan_local_report_scoped_with_deadline(
            &root,
            parsed.ruff_tool.as_deref(),
            Some(&[path.to_owned()]),
            deadline,
            &std::sync::atomic::AtomicBool::new(false),
        ) {
            Ok(mut report) => {
                report["schema_version"] = json!("0.19.0");
                report["task_scope"] = json!("single_python_confirmation_file");
                report["task_binding"] = json!({"task_id":parsed.task_id,"path":path,
                    "source_sha256":crate::python_confirmation_recheck::digest(&before),
                    "original_report":python_original});
                report["task_input_stable"] = json!(
                    crate::python_confirmation_recheck::inputs_current(&root, &report)
                );
                report
            }
            Err(reason) => {
                let release = finish_verification(&root, &parsed.task_id, &lease);
                return print_unavailable(&parsed, release.err().unwrap_or(reason));
            }
        }
    } else if brief["checker_id"] == "python.ruff" {
        match scan_local_report_with_deadline(
            &root,
            parsed.ruff_tool.as_deref(),
            deadline,
            &std::sync::atomic::AtomicBool::new(false),
        ) {
            Ok(scan) => scan,
            Err(reason) => {
                let release = finish_verification(&root, &parsed.task_id, &lease);
                return print_unavailable(&parsed, release.err().unwrap_or(reason));
            }
        }
    } else {
        let release = finish_verification(&root, &parsed.task_id, &lease);
        return print_unavailable(&parsed, release.err().unwrap_or("task_checker_unsupported"));
    };
    if let Some((path, digest)) = cve_pom.as_ref() {
        scan["task_config_sha256"] = json!(digest);
        scan["task_input_stable"] = json!(cve_pom_stable(&root, path, digest));
    }
    if brief["checker_id"] == "go.vet" {
        if let Some(before) = source_target_before.as_deref() {
            let path = brief["scope"].as_str().expect("已核验任务范围");
            scan["schema_version"] = json!("0.7.0");
            scan["task_target"] = json!({"task_id":brief["task_id"],"path":path,
                "source_sha256":format!("{:x}", Sha256::digest(before))});
            scan["task_scope"] = crate::go_lint_command::observe_target_scope(
                &root,
                path,
                parsed.go_tool.as_deref(),
                &scan,
                deadline,
                &std::sync::atomic::AtomicBool::new(false),
            );
            scan["task_input_stable"] = json!(go_task_inputs_stable(&root, &scan));
        }
    }
    if let Some(before) = source_target_before
        .as_deref()
        .filter(|_| brief["checker_id"] == "rust.cargo_clippy")
    {
        let found = scan["findings"].as_array().is_some_and(|findings| {
            findings
                .iter()
                .any(|finding| finding["finding_id"] == parsed.task_id)
        });
        if scan["local_scan_complete"] == true && !found {
            let path = brief["scope"].as_str().expect("已核验 Rust 任务范围");
            let rule = brief["native_rule_id"].as_str().expect("已核验 Rust 规则");
            let stable_after_normal = read_bounded_regular_file(&root.join(path), 16 * 1024 * 1024)
                .ok()
                .is_some_and(|bytes| bytes == before);
            let probe = if stable_after_normal && Instant::now() < deadline {
                match legacy_registry() {
                    Ok(registry) => {
                        let discovery = discover(&root, &registry, &NativeObservation);
                        if discovery.observation_complete {
                            discovery.languages.get("rust").map(|language| {
                                observe_cargo_clippy_force_warn(
                                    &root,
                                    &language.source_files,
                                    parsed.cargo_tool.as_deref(),
                                    rule,
                                    deadline,
                                    &std::sync::atomic::AtomicBool::new(false),
                                )
                            })
                        } else {
                            None
                        }
                    }
                    Err(_) => None,
                }
            } else {
                None
            };
            let stable_after_probe = read_bounded_regular_file(&root.join(path), 16 * 1024 * 1024)
                .ok()
                .is_some_and(|bytes| bytes == before);
            scan["schema_version"] = json!("0.3.0");
            scan["suppression_probe"] = json!({
                "rule_id":rule, "target_path":path,
                "target_finding_id":parsed.task_id,
                "source_sha256":format!("{:x}", Sha256::digest(before)),
                "input_stable":stable_after_normal && stable_after_probe,
                "forced_scan":probe
            });
        }
    }
    if brief["checker_id"] == "java.maven.p3c" {
        scan["task_input_stable"] = json!(java_task_inputs_stable(&root, &brief, &scan));
    }
    let outcome = if Instant::now() >= deadline || scan["task_input_stable"] == false {
        "incomplete"
    } else {
        if syntax_task {
            crate::syntax_task_recheck::classify(&scan)
        } else if npm_task {
            crate::npm_task_recheck::classify(&root, &brief, &scan)
        } else if eslint_task {
            crate::eslint_task_recheck::classify(&brief, &scan)
        } else if brief["checker_id"] == "java.checkstyle.preparation" {
            crate::checkstyle_preparation_recheck::classify(&brief, &scan)
        } else if brief["checker_id"] == "java.checkstyle" {
            crate::checkstyle_task_recheck::classify(&brief, &scan)
        } else if brief["checker_id"] == "python.ruff.doctor" {
            classify_doctor(&brief, &scan)
        } else if brief["checker_id"] == "java.maven.p3c" {
            classify_java(&brief, &scan)
        } else if brief["checker_id"] == "java.maven.dependency_check" {
            classify_cve(&brief, &scan)
        } else if brief["checker_id"] == "go.vet" {
            classify_go(&brief, &scan)
        } else if brief["checker_id"] == "rust.cargo_check" {
            crate::rust_build_task_recheck::classify(&brief, &scan)
        } else if brief["checker_id"] == "rust.cargo_audit" {
            crate::rust_cve_task_recheck::classify(&brief, &scan)
        } else if brief["checker_id"] == "python.pip_audit" {
            crate::python_cve_task_recheck::classify(&brief, &scan)
        } else if brief["checker_id"] == "rust.cargo_rustdoc" {
            crate::rustdoc_task_recheck::classify(&brief, &scan)
        } else if brief["checker_id"] == "rust.cargo_clippy" {
            classify_rust(&brief, &scan)
        } else {
            classify(&brief, &scan)
        }
    };
    let mut report = json!({
        "schema_version":"0.6.0", "report_type":"task_verification_preview",
        "operation":"task_verify", "command_status":"incomplete", "exit_code":3,
        "task_id":parsed.task_id, "kind":brief["kind"],
        "observation":outcome, "event_persisted":false,
        "reason":null, "native_scan":scan,
        "authority":"local_unverified", "delivery_decision":"not_evaluated",
        "execution_budget":budget_record(parsed.timeout_ms, parsed.timeout_source),
        "next_actions":["inspect_native_recheck_and_policy_before_closure"]
    });
    if syntax_task {
        report["schema_version"] = json!(match report["native_scan"]["schema_version"].as_str() {
            Some("0.10.0") => "0.23.0",
            Some("0.9.0") => "0.22.0",
            Some("0.8.0") => "0.19.0",
            Some("0.7.0") => "0.18.0",
            Some("0.6.0") => "0.17.0",
            Some("0.5.0") => "0.16.0",
            Some("0.4.0") => "0.15.0",
            Some("0.3.0") => "0.14.0",
            Some("0.2.0") => "0.13.0",
            _ => "0.12.0",
        });
        report["next_actions"] = json!([
            "inspect_native_syntax_observation",
            "repair_only_current_native_diagnostics",
            "verify_policy_and_capability_before_closure"
        ]);
    }
    if npm_task {
        report["next_actions"] = json!([
            "inspect_npm_native_diagnostic",
            "verify_advisory_source_coverage_and_freshness",
            "recheck_original_npm_after_dependency_or_environment_repair"
        ]);
    }
    if brief["checker_id"] == "rust.cargo_check" {
        report["schema_version"] = json!("0.8.0");
    } else if brief["checker_id"] == "rust.cargo_audit" {
        report["schema_version"] = json!("0.10.0");
    } else if brief["checker_id"] == "python.pip_audit" {
        report["schema_version"] = json!("0.11.0");
    } else if brief["checker_id"] == "rust.cargo_rustdoc" {
        report["schema_version"] = json!("0.7.0");
    } else if brief["checker_id"] == "python.ruff" {
        report["schema_version"] = json!(if python_confirmation {
            "0.21.0"
        } else {
            "0.9.0"
        });
    }
    let persist = if codeguard_runtime::sigint_cancellation_requested() {
        Err("request_cancelled")
    } else if Instant::now() >= deadline {
        Err("request_deadline_exceeded")
    } else {
        match lock_verification(&root, &parsed.task_id, &lease) {
            Ok(_guard) => match latest_ready_attempt(&root, &parsed.task_id) {
                Ok(current) if current == bound_attempt => {
                    if (python_confirmation
                        && (!crate::python_confirmation_recheck::valid_binding(&root, &scan)
                            || (scan["task_input_stable"] == true
                                && !crate::python_confirmation_recheck::inputs_current(
                                    &root, &scan,
                                ))))
                        || (syntax_task
                            && scan["input_stable"] == true
                            && !crate::syntax_task_recheck::inputs_current(&root, &scan))
                        || (npm_task && !crate::npm_task_recheck::inputs_current(&root, &scan))
                        || (brief["checker_id"] == "rust.cargo_check"
                            && scan["input_stable"] == true
                            && !crate::rust_build_task_recheck::inputs_current(&root, &scan))
                        || (brief["checker_id"] == "rust.cargo_audit"
                            && !crate::rust_cve_task_recheck::inputs_current(&root, &scan))
                        || (brief["checker_id"] == "python.pip_audit"
                            && !crate::python_cve_task_recheck::inputs_current(&root, &scan))
                        || (brief["checker_id"] == "rust.cargo_rustdoc"
                            && scan["input_stable"] == true
                            && !crate::rustdoc_task_recheck::inputs_current(&root, &scan))
                        || (brief["checker_id"] == "java.checkstyle.preparation"
                            && !crate::checkstyle_preparation_recheck::inputs_current(&scan))
                        || (brief["checker_id"] == "java.checkstyle"
                            && !crate::checkstyle_task_recheck::inputs_current(&scan))
                        || (brief["checker_id"] == "go.vet" && !go_task_inputs_stable(&root, &scan))
                        || (brief["checker_id"] == "java.maven.p3c"
                            && !java_task_inputs_stable(&root, &brief, &scan))
                        || cve_pom
                            .as_ref()
                            .is_some_and(|(path, digest)| !cve_pom_stable(&root, path, digest))
                    {
                        Err("source_changed_before_verification_record")
                    } else {
                        persist_observation(&root, &brief, &scan, outcome, bound_attempt.as_deref())
                    }
                }
                Ok(_) => Err("attempt_changed_during_verification"),
                Err(reason) => Err(reason),
            },
            Err(reason) => Err(reason),
        }
    };
    if let Err(reason) = persist {
        if reason == "source_changed_before_verification_record" {
            report["observation"] = json!("incomplete");
            report["native_scan"][if syntax_task {
                "input_stable"
            } else {
                "task_input_stable"
            }] = json!(false);
        }
        report["reason"] = json!(reason);
    } else {
        report["event_persisted"] = json!(true);
    }
    if let Err(reason) = finish_verification(&root, &parsed.task_id, &lease) {
        report["reason"] = json!(reason);
    }
    if parsed.json {
        println!("{report}");
    } else {
        println!(
            "任务 {} 原工具复检：{}；记录 {}；正式门禁未评估。",
            report["task_id"],
            report["observation"],
            if report["event_persisted"] == true {
                "已保存"
            } else {
                "未保存"
            }
        );
        if matches!(
            brief["checker_id"].as_str(),
            Some("java.checkstyle" | "java.checkstyle.preparation")
        ) {
            println!(
                "原工具说明：{}",
                report["native_scan"]["reason"]
                    .as_str()
                    .unwrap_or("unknown")
            );
            for finding in report["native_scan"]["scan"]["findings"]
                .as_array()
                .into_iter()
                .flatten()
            {
                println!(
                    "{}:{} {}；{}",
                    finding["path"],
                    finding["line"],
                    finding["rule_id"],
                    finding["native"]["rule_summary"]
                );
            }
        }
        if !report["reason"].is_null() {
            println!("记录原因：{}", report["reason"]);
        }
        if python_confirmation {
            println!(
                "复检范围：{}（仅首次Python确认文件）；首次报告：{}；输入一致：{}",
                report["native_scan"]["task_binding"]["path"],
                report["native_scan"]["task_binding"]["original_report"]["run_id"],
                report["native_scan"]["task_input_stable"]
            );
            for file in report["native_scan"]["files"]
                .as_array()
                .into_iter()
                .flatten()
            {
                println!("原生状态：{}；原因：{}", file["run_status"], file["reason"]);
                for finding in file["findings"].as_array().into_iter().flatten().take(8) {
                    println!(
                        "原生规则 {}；行 {}，Ruff原生列 {}",
                        finding["rule_id"], finding["line"], finding["column"]
                    );
                }
            }
        }
        if syntax_task {
            println!(
                "原生语法说明：{}；报告引用：.codeguard/reports/{}.json",
                report["native_scan"]["native"]["reason"],
                report["native_scan"]["run_id"]
                    .as_str()
                    .unwrap_or("unknown")
            );
            if report["native_scan"]["input_stable"] == true {
                for position in report["native_scan"]["native"]["diagnostics"]
                    .as_array()
                    .into_iter()
                    .flatten()
                {
                    println!(
                        "原生规则 {}；行 {}，{} {}",
                        position["rule_id"],
                        position["line"],
                        if report["native_scan"]["target"]["language"] == "erlang" {
                            "字符列"
                        } else {
                            "字节列"
                        },
                        position["column"]
                    );
                }
            }
        }
        if npm_task {
            println!(
                "npm 原工具诊断：{}；组件观察数：{}；下一步核对漏洞源覆盖及时效，按原工具复检。",
                report["native_scan"]["diagnostic_reason"],
                report["native_scan"]["component_count"]
            );
            for component in report["native_scan"]["advisories"]
                .as_array()
                .into_iter()
                .flatten()
            {
                println!(
                    "组件引用 {}；严重度 {}；原生 advisory {}",
                    component["component_ref"],
                    component["severity"],
                    component["advisory_sources"]
                );
            }
        }
        if eslint_task {
            println!("ESLint 原工具说明：{}", report["native_scan"]["reason"]);
            if report["native_scan"]["effective_rule"].is_object() {
                println!(
                    "原规则有效配置：{}；级别 {}；原因 {}",
                    report["native_scan"]["effective_rule"]["status"],
                    report["native_scan"]["effective_rule"]["severity"],
                    report["native_scan"]["effective_rule"]["reason"]
                );
            }
            for finding in report["native_scan"]["scan"]["findings"]
                .as_array()
                .into_iter()
                .flatten()
            {
                println!(
                    "原生规则 {}，行 {}；局部观察仍需完整覆盖核验",
                    finding["rule_id"], finding["line"]
                );
            }
        }
    }
    ExitCode::from(3)
}

pub(crate) fn classify_doctor(brief: &Value, report: &Value) -> &'static str {
    if brief["kind"] != "blocker"
        || brief["checker_id"] != "python.ruff.doctor"
        || !crate::work_sync::valid_doctor_report(report)
    {
        return "incomplete";
    }
    match report["ruff_version"]["status"].as_str() {
        Some("observed_untrusted") => "environment_restored_unverified_policy",
        Some("incomplete") => "still_blocked",
        _ => "incomplete",
    }
}

fn java_task_inputs_stable(root: &Path, brief: &Value, scan: &Value) -> bool {
    let Some(files) = scan["files"].as_array() else {
        return false;
    };
    let paths: Vec<&str> = if brief["kind"] == "finding" {
        brief["scope"].as_str().into_iter().collect()
    } else {
        brief["affected_paths"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect()
    };
    if paths.is_empty() {
        return false;
    }
    paths.into_iter().all(|path| {
        let Some(file) = files.iter().find(|file| file["path"] == path) else {
            return false;
        };
        if file["reason"] != "native_probe_returned" {
            return true;
        }
        let Some(config_ref) = file["configuration_ref"].as_str() else {
            return false;
        };
        let Some(config_sha) = file["configuration_sha256"].as_str() else {
            return false;
        };
        if !read_bounded_regular_file(&root.join(config_ref), 4 * 1024 * 1024)
            .ok()
            .is_some_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == config_sha)
        {
            return false;
        }
        let Some(expected) = file["observation"]["source_sha256"].as_str() else {
            return false;
        };
        read_bounded_regular_file(&root.join(path), 16 * 1024 * 1024)
            .ok()
            .is_some_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == expected)
    })
}

fn cve_pom_stable(root: &Path, path: &str, expected: &str) -> bool {
    read_bounded_regular_file(&root.join(path), 4 * 1024 * 1024)
        .ok()
        .is_some_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == expected)
}

pub(crate) fn classify_cve(brief: &Value, scan: &Value) -> &'static str {
    if brief["kind"] != "blocker"
        || scan["report_type"] != "java_cve_project_probe"
        || scan["schema_version"] != "0.3.0"
        || scan["checker_id"] != "java.maven.dependency_check"
        || scan["database_freshness"] != "unverified"
        || scan["coverage_proven"] != false
        || scan["task_input_stable"] != true
        || !scan["task_config_sha256"]
            .as_str()
            .is_some_and(valid_sha256)
    {
        return "incomplete";
    }
    let Some(probes) = scan["probes"].as_array() else {
        return "incomplete";
    };
    let Some(probe) = probes.iter().find(|probe| {
        probe["build_root"] == brief["build_root"]
            && probe["configuration_ref"] == brief["affected_paths"][0]
    }) else {
        return "still_blocked";
    };
    match probe["observation"]["native_status"].as_str() {
        Some("findings_observed_untrusted" | "empty_report_unverified") => {
            if probe["observation"]["native_plan_sha256"] != scan["task_config_sha256"] {
                "incomplete"
            } else if brief["reason_code"] == "cve_database_freshness_unverified" {
                "still_blocked"
            } else {
                "environment_restored_unverified_policy"
            }
        }
        Some("incomplete") => "still_blocked",
        _ => "incomplete",
    }
}

pub(crate) fn classify_java(brief: &Value, scan: &Value) -> &'static str {
    if scan["report_type"] != "java_p3c_project_observation"
        || scan["schema_version"] != "0.2.0"
        || scan["checker_id"] != "java.maven.p3c"
        || scan["coverage_proven"] != false
        || scan["task_input_stable"] == false
    {
        return "incomplete";
    }
    let Some(files) = scan["files"].as_array() else {
        return "incomplete";
    };
    let valid_file = |path: &str| {
        files
            .iter()
            .find(|file| file["path"] == path)
            .is_some_and(|file| {
                file["configuration"] == "configured"
                    && file["reason"] == "native_probe_returned"
                    && matches!(
                        file["observation"]["local_status"].as_str(),
                        Some("findings_observed_untrusted" | "clean_scope_unproven")
                    )
            })
    };
    if brief["kind"] == "blocker" {
        return if brief["affected_paths"].as_array().is_some_and(|paths| {
            !paths.is_empty()
                && paths
                    .iter()
                    .all(|path| path.as_str().is_some_and(valid_file))
        }) {
            "environment_restored_unverified_policy"
        } else {
            "still_blocked"
        };
    }
    if brief["kind"] != "finding" {
        return "incomplete";
    }
    let Some(path) = brief["scope"].as_str() else {
        return "incomplete";
    };
    let partial_positive = files.iter().any(|file| {
        file["path"] == path
            && file["configuration"] == "configured"
            && file["reason"] == "native_probe_returned"
            && has_projectable_findings(&file["observation"])
    });
    if !valid_file(path) && !partial_positive {
        return "incomplete";
    }
    if scan["findings"].as_array().is_some_and(|findings| {
        findings.iter().any(|finding| {
            finding["finding_id"] == brief["task_id"]
                && finding["path"] == path
                && finding["rule_id"] == brief["native_rule_id"]
        })
    }) {
        "still_present"
    } else if valid_file(path) {
        "rule_coverage_requires_review"
    } else {
        "incomplete"
    }
}

/// 解释原生 Go 局部复检；无发现不证明构建标签、平台或策略完整。
pub(crate) fn classify_go(brief: &Value, scan: &Value) -> &'static str {
    if scan["report_type"] != "go_lint_local_feedback"
        || scan["checker_id"] != "go.vet"
        || scan["coverage_proven"] != false
        || scan["delivery_decision"] != "not_evaluated"
        || !matches!(
            scan["native_status"].as_str(),
            Some("clean_observed_unverified" | "findings_observed_unverified")
        )
    {
        return if brief["kind"] == "blocker" {
            "still_blocked"
        } else {
            "incomplete"
        };
    }
    if brief["kind"] == "blocker" {
        return if brief["reason_code"] == "go_policy_and_coverage_unverified" {
            "still_blocked"
        } else {
            "environment_restored_unverified_policy"
        };
    }
    if brief["kind"] != "finding"
        || !matches!(scan["schema_version"].as_str(), Some("0.5.0" | "0.7.0"))
        || scan["task_target"]["task_id"] != brief["task_id"]
        || scan["task_target"]["path"] != brief["scope"]
        || !scan["task_target"]["source_sha256"]
            .as_str()
            .is_some_and(valid_sha256)
        || scan["task_input_stable"] != true
    {
        return "incomplete";
    }
    if scan["schema_version"] == "0.7.0" {
        let scope = &scan["task_scope"];
        let Some(module) = scope["module_root"].as_str() else {
            return "incomplete";
        };
        if scope["status"] == "incomplete"
            || scope["source_sha256"] != scan["task_target"]["source_sha256"]
            || !scan["source_snapshot_sha256"]
                .as_str()
                .is_some_and(valid_sha256)
            || scope["source_snapshot_sha256"] != scan["source_snapshot_sha256"]
            || scope["tool_sha256"] != scan["tool_sha256"]
            || scope["manifest_sha256"] != scan["module_identities"][module]["manifest_sha256"]
            || scope["go_sum_sha256"] != scan["module_identities"][module]["go_sum_sha256"]
        {
            return "incomplete";
        }
        if matches!(scope["status"].as_str(), Some("excluded" | "not_selected")) {
            return "rule_coverage_requires_review";
        }
        if scope["status"] != "selected" {
            return "incomplete";
        }
    }
    let Some(findings) = scan["findings"].as_array() else {
        return "incomplete";
    };
    if findings.iter().any(|finding| {
        finding["finding_id"] == brief["task_id"]
            && finding["path"] == brief["scope"]
            && finding["rule_id"] == brief["native_rule_id"]
    }) {
        "still_present"
    } else if findings.iter().any(|finding| {
        finding["path"] == brief["scope"] && finding["rule_id"] == brief["native_rule_id"]
    }) {
        // 同文件同规则仍有诊断，仅指纹变化不能解释为原问题已修复。
        "rule_coverage_requires_review"
    } else {
        "candidate_absent_unverified_policy"
    }
}

pub(crate) fn go_task_inputs_stable(root: &Path, scan: &Value) -> bool {
    if let Some(expected) = scan["source_snapshot_sha256"].as_str() {
        if !valid_sha256(expected)
            || crate::go_lint_command::current_source_snapshot_sha256(root).as_deref()
                != Some(expected)
        {
            return false;
        }
    }
    if let Some(target) = scan.get("task_target") {
        let Some(path) = target["path"].as_str() else {
            return false;
        };
        let Some(expected) = target["source_sha256"]
            .as_str()
            .filter(|sha| valid_sha256(sha))
        else {
            return false;
        };
        if !read_bounded_regular_file(&root.join(path), 16 * 1024 * 1024)
            .is_ok_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == expected)
        {
            return false;
        }
    }
    let Some(identities) = scan["module_identities"].as_object() else {
        return false;
    };
    identities.iter().all(|(module, identity)| {
        let directory = if module == "." {
            root.to_path_buf()
        } else {
            root.join(module)
        };
        let manifest = read_bounded_regular_file(&directory.join("go.mod"), 16 * 1024 * 1024)
            .is_ok_and(|bytes| {
                identity["manifest_sha256"] == format!("{:x}", Sha256::digest(bytes))
            });
        let sum = match identity["go_sum_sha256"].as_str() {
            Some(expected) => {
                read_bounded_regular_file(&directory.join("go.sum"), 16 * 1024 * 1024)
                    .is_ok_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == expected)
            }
            None if identity["go_sum_sha256"].is_null() => {
                fs::symlink_metadata(directory.join("go.sum"))
                    .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
            }
            _ => false,
        };
        manifest && sum
    })
}

pub(crate) fn classify_rust(brief: &Value, scan: &Value) -> &'static str {
    if scan["report_type"] != "rust_clippy_local_observation"
        || scan["checker_id"] != "rust.cargo_clippy"
        || scan["local_scan_complete"] != true
        || scan["coverage_proven"] != false
    {
        return if brief["kind"] == "blocker" {
            "still_blocked"
        } else {
            "incomplete"
        };
    }
    if brief["kind"] == "blocker" {
        return "environment_restored_unverified_policy";
    }
    if brief["kind"] != "finding" {
        return "incomplete";
    }
    let Some(findings) = scan["findings"].as_array() else {
        return "incomplete";
    };
    if findings.iter().any(|finding| {
        finding["finding_id"] == brief["task_id"]
            && finding["path"] == brief["scope"]
            && finding["rule_id"] == brief["native_rule_id"]
    }) {
        "still_present"
    } else {
        let audit = &scan["suppression_probe"];
        let probe = &audit["forced_scan"];
        let expected_command = format!(
            "cargo clippy --locked --offline --all-targets --message-format=json -- --force-warn {}",
            brief["native_rule_id"].as_str().unwrap_or("")
        );
        // 历史局部报告保持可读取；两个精确已知命令均不能签发可信关闭。
        let legacy_command = format!(
            "cargo clippy --offline --all-targets --message-format=json -- --force-warn {}",
            brief["native_rule_id"].as_str().unwrap_or("")
        );
        if scan["schema_version"] != "0.3.0"
            || audit["input_stable"] != true
            || audit["target_finding_id"] != brief["task_id"]
            || audit["target_path"] != brief["scope"]
            || audit["rule_id"] != brief["native_rule_id"]
            || !audit["source_sha256"].as_str().is_some_and(valid_sha256)
            || probe["report_type"] != "rust_clippy_local_observation"
            || probe["schema_version"] != "0.2.0"
            || probe["checker_id"] != "rust.cargo_clippy"
            || probe["local_scan_complete"] != true
            || probe["reason"] != "native_observed_unverified"
            || probe["workspace_id"] != scan["workspace_id"]
            || probe["tool_sha256"] != scan["tool_sha256"]
            || probe["manifest_sha256"] != scan["manifest_sha256"]
            || probe["scope"] != scan["scope"]
            || (probe["recheck_command"] != expected_command
                && probe["recheck_command"] != legacy_command)
        {
            return "incomplete";
        }
        if probe["findings"].as_array().is_some_and(|items| {
            items.iter().any(|finding| {
                finding["finding_id"] == brief["task_id"]
                    && finding["path"] == brief["scope"]
                    && finding["rule_id"] == brief["native_rule_id"]
                    && finding["source_sha256"] == audit["source_sha256"]
            })
        }) {
            "suppression_requires_review"
        } else {
            "candidate_absent_unverified_policy"
        }
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

pub(crate) fn classify(brief: &Value, scan: &Value) -> &'static str {
    if matches!(scan["schema_version"].as_str(), Some("0.18.0" | "0.19.0"))
        && scan["task_input_stable"] != true
    {
        return "incomplete";
    }
    let Some(files) = scan["files"].as_array() else {
        return "incomplete";
    };
    if scan["schema_version"] == "0.19.0"
        && brief["kind"] == "blocker"
        && brief["checker_id"] == "python.ruff"
        && brief["reason_code"] == "python_syntax_confirmation_needed"
    {
        let Some(file) = files.iter().find(|file| file["path"] == brief["scope"]) else {
            return "still_blocked";
        };
        if !matches!(
            file["run_status"].as_str(),
            Some("passed" | "findings" | "suppressed")
        ) || !file["source_sha256"].as_str().is_some_and(valid_sha256)
        {
            return "still_blocked";
        }
        return if file["findings"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| row["rule_id"] == "invalid-syntax"))
        {
            "still_present"
        } else {
            "candidate_absent_unverified_policy"
        };
    }
    if brief["kind"] == "finding" {
        let Some(path) = brief["scope"].as_str() else {
            return "incomplete";
        };
        let Some(file) = files.iter().find(|file| file["path"] == path) else {
            return "incomplete";
        };
        if !matches!(
            file["run_status"].as_str(),
            Some("passed" | "findings" | "suppressed")
        ) || !file["source_sha256"].is_string()
        {
            return "incomplete";
        }
        if file["findings"].as_array().is_some_and(|items| {
            items
                .iter()
                .any(|item| item["finding_id"] == brief["task_id"])
        }) {
            "still_present"
        } else if file["suppression_audit"]["suppressed_diagnostic_count"]
            .as_u64()
            .is_some_and(|count| count > 0)
            && file["suppression_audit"]["suppressed_rule_ids"]
                .as_array()
                .is_some_and(|rules| rules.iter().any(|rule| *rule == brief["native_rule_id"]))
        {
            "suppression_requires_review"
        } else if brief["native_rule_id"]
            .as_str()
            .is_some_and(|rule| matches!(rule, "F401" | "E501"))
            && file["rule_settings"]["globally_enabled_mapped_rules"]
                .as_array()
                .is_some_and(|rules| !rules.iter().any(|rule| *rule == brief["native_rule_id"]))
        {
            "rule_coverage_requires_review"
        } else if file["rule_settings"]["per_file_ignores_present"] == true {
            "suppression_requires_review"
        } else {
            "candidate_absent_unverified_policy"
        }
    } else if brief["kind"] == "blocker" {
        let Some(paths) = brief["affected_paths"].as_array() else {
            return "incomplete";
        };
        if paths.iter().all(|path| {
            files.iter().any(|file| {
                file["path"] == *path
                    && matches!(
                        file["run_status"].as_str(),
                        Some("passed" | "findings" | "suppressed")
                    )
                    && file["source_sha256"].is_string()
            })
        }) {
            "environment_restored_unverified_policy"
        } else {
            "still_blocked"
        }
    } else {
        "incomplete"
    }
}

pub(crate) fn persist_observation(
    root: &Path,
    brief: &Value,
    scan: &Value,
    outcome: &str,
    attempt_id: Option<&str>,
) -> Result<(), &'static str> {
    if scan["workspace_binding"] != "bound" {
        return Err("workspace_binding_invalid");
    }
    save_local_report(root, scan)?;
    let summary = sync_local_workspace(root)?;
    if summary.failed_reports != 0 || scan["workspace_id"] != summary.workspace_id {
        return Err("backlog_sync_incomplete");
    }
    let report_bytes = serde_json::to_vec_pretty(scan).map_err(|_| "report_encoding_failed")?;
    let report_sha256 = format!("{:x}", Sha256::digest(&report_bytes));
    let run_id = scan["run_id"].as_str().ok_or("run_id_invalid")?;
    let task_id = brief["task_id"].as_str().ok_or("task_id_invalid")?;
    let events = root.join(format!(".codeguard/findings/{task_id}/events"));
    let state = root.join(".codeguard/state");
    if !real_directory(&events) || !real_directory(&state) {
        return Err("task_events_unavailable");
    }
    let event = serde_json::to_vec_pretty(&json!({
        "schema_version":"0.3.0", "event":"verification_observed",
        "task_id":task_id, "kind":brief["kind"],
        "attempt_id":attempt_id,
        "workspace_id":summary.workspace_id, "run_id":run_id,
        "report_sha256":report_sha256, "observation":outcome,
        "authority":"local_unverified", "state_after":"open"
    }))
    .map_err(|_| "event_encoding_failed")?;
    write_once(
        &events.join(format!("verify-{run_id}.json")),
        &event,
        &state,
    )
    .map_err(|_| "verification_event_write_failed")?;
    crate::task_lifecycle_store::record_native_recurrence(root, brief, scan)
}

fn real_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_dir())
}

fn print_unavailable(parsed: &Arguments, reason: &str) -> ExitCode {
    if parsed.json {
        println!(
            "{}",
            json!({
                "schema_version":"0.6.0", "report_type":"task_verification_preview",
                "operation":"task_verify", "command_status":"incomplete", "exit_code":3,
                "task_id":null, "kind":null, "observation":"incomplete",
                "event_persisted":false, "reason":reason, "native_scan":null,
                "authority":"local_unverified", "delivery_decision":"not_evaluated",
                "execution_budget":budget_record(parsed.timeout_ms, parsed.timeout_source),
                "next_actions":["inspect_task_and_restore_verification_prerequisites"]
            })
        );
    } else {
        eprintln!("任务复检不可用：{reason}");
    }
    ExitCode::from(3)
}

fn parse_args(args: &[String]) -> Result<Arguments, String> {
    if args.first().map(String::as_str) != Some("verify") {
        return Err("task 当前仅支持 verify".into());
    }
    let task_id = args.get(1).ok_or("verify 缺少任务 ID")?;
    let valid_id = task_id
        .strip_prefix("CG-B-")
        .or_else(|| task_id.strip_prefix("CG-"))
        .is_some_and(|suffix| {
            suffix.len() == 32
                && suffix
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        });
    if !valid_id {
        return Err("任务 ID 无效".into());
    }
    let mut root = None;
    let mut eslint_options = std::collections::BTreeMap::new();
    let mut npm_options = std::collections::BTreeMap::new();
    let mut ruff_tool = None;
    let mut zig_tool = None;
    let mut erl_tool = None;
    let mut swift_tool = None;
    let mut ruby_tool = None;
    let mut kotlinc_tool = None;
    let mut cargo_tool = None;
    let mut cargo_audit_tool = None;
    let mut rustsec_db = None;
    let mut pip_audit_tool = None;
    let mut pip_audit_version = None;
    let mut go_tool = None;
    let mut maven_tool = None;
    let mut java_home = None;
    let mut java_tool = None;
    let mut checkstyle_jar = None;
    let mut checkstyle_config = None;
    let mut maven_repo = None;
    let mut repo_sha256 = None;
    let mut cve_data_dir = None;
    let mut cve_data_sha256 = None;
    let mut owner = None;
    let mut lease_token = None;
    let mut json = false;
    let mut timeout_ms = 0;
    let mut timeout_seen = false;
    let mut index = 2;
    while index < args.len() {
        let arg = &args[index];
        if matches!(
            arg.as_str(),
            "--node-tool"
                | "--npm-entry"
                | "--npm-version"
                | "--userconfig"
                | "--globalconfig"
                | "--registry"
                | "--eslint-entry"
                | "--eslint-version"
                | "--cwd"
                | "--format"
                | "--ruff-tool"
                | "--zig-tool"
                | "--erl-tool"
                | "--swift-tool"
                | "--ruby-tool"
                | "--kotlinc-tool"
                | "--cargo-tool"
                | "--cargo-audit-tool"
                | "--rustsec-db"
                | "--pip-audit-tool"
                | "--pip-audit-version"
                | "--go-tool"
                | "--maven-tool"
                | "--java-home"
                | "--java-tool"
                | "--checkstyle-jar"
                | "--config"
                | "--maven-repo"
                | "--repo-sha256"
                | "--cve-data-dir"
                | "--cve-data-sha256"
                | "--owner"
                | "--lease-token"
                | "--timeout"
        ) {
            index += 1;
            let value = args.get(index).ok_or_else(|| format!("{arg} 缺少值"))?;
            match arg.as_str() {
                "--npm-entry" | "--npm-version" | "--userconfig" | "--globalconfig"
                | "--registry"
                    if npm_options.insert(arg.clone(), value.clone()).is_none() => {}
                "--node-tool" | "--eslint-entry" | "--eslint-version" | "--cwd"
                    if eslint_options.insert(arg.clone(), value.clone()).is_none() => {}
                "--format" => json = parse_format(value)?,
                "--timeout" if !timeout_seen => {
                    timeout_ms = parse_check_timeout(value)?;
                    timeout_seen = true;
                }
                "--ruff-tool" if ruff_tool.replace(PathBuf::from(value)).is_none() => {}
                "--zig-tool" if zig_tool.replace(PathBuf::from(value)).is_none() => {}
                "--erl-tool" if erl_tool.replace(PathBuf::from(value)).is_none() => {}
                "--swift-tool" if swift_tool.replace(PathBuf::from(value)).is_none() => {}
                "--ruby-tool" if ruby_tool.replace(PathBuf::from(value)).is_none() => {}
                "--kotlinc-tool" if kotlinc_tool.replace(PathBuf::from(value)).is_none() => {}
                "--cargo-tool" if cargo_tool.replace(PathBuf::from(value)).is_none() => {}
                "--cargo-audit-tool"
                    if cargo_audit_tool.replace(PathBuf::from(value)).is_none() => {}
                "--rustsec-db" if rustsec_db.replace(PathBuf::from(value)).is_none() => {}
                "--pip-audit-tool" if pip_audit_tool.replace(PathBuf::from(value)).is_none() => {}
                "--pip-audit-version" if pip_audit_version.replace(value.clone()).is_none() => {}
                "--go-tool" if go_tool.replace(PathBuf::from(value)).is_none() => {}
                "--maven-tool" if maven_tool.replace(PathBuf::from(value)).is_none() => {}
                "--java-home" if java_home.replace(PathBuf::from(value)).is_none() => {}
                "--java-tool" if java_tool.replace(PathBuf::from(value)).is_none() => {}
                "--checkstyle-jar" if checkstyle_jar.replace(PathBuf::from(value)).is_none() => {}
                "--config" if checkstyle_config.replace(PathBuf::from(value)).is_none() => {}
                "--maven-repo" if maven_repo.replace(PathBuf::from(value)).is_none() => {}
                "--repo-sha256" if repo_sha256.replace(value.clone()).is_none() => {}
                "--cve-data-dir" if cve_data_dir.replace(PathBuf::from(value)).is_none() => {}
                "--cve-data-sha256" if cve_data_sha256.replace(value.clone()).is_none() => {}
                "--owner" if valid_owner(value) && owner.replace(value.clone()).is_none() => {}
                "--lease-token"
                    if valid_token(value) && lease_token.replace(value.clone()).is_none() => {}
                _ => return Err(format!("参数无效或重复：{arg}")),
            }
        } else if let Some(value) = arg.strip_prefix("--format=") {
            json = parse_format(value)?;
        } else if arg.starts_with('-') || root.is_some() {
            return Err(format!("不支持的参数：{arg}"));
        } else {
            root = Some(PathBuf::from(arg));
        }
        index += 1;
    }
    if zig_tool.as_ref().is_some_and(|tool| !tool.is_absolute()) {
        return Err("--zig-tool 必须是绝对路径".into());
    }
    if erl_tool.as_ref().is_some_and(|tool| !tool.is_absolute()) {
        return Err("--erl-tool 必须是绝对路径".into());
    }
    if kotlinc_tool
        .as_ref()
        .is_some_and(|tool| !tool.is_absolute())
    {
        return Err("--kotlinc-tool 必须是绝对路径".into());
    }
    if ruby_tool.as_ref().is_some_and(|tool| !tool.is_absolute()) {
        return Err("Ruby工具必须为绝对路径".into());
    }
    if swift_tool.as_ref().is_some_and(|tool| !tool.is_absolute()) {
        return Err("--swift-tool 必须是绝对路径".into());
    }
    if ruff_tool.as_ref().is_some_and(|tool| !tool.is_absolute()) {
        return Err("--ruff-tool 必须是绝对路径".into());
    }
    if go_tool.as_ref().is_some_and(|tool| !tool.is_absolute()) {
        return Err("--go-tool 必须是绝对路径".into());
    }
    if cargo_tool.as_ref().is_some_and(|tool| !tool.is_absolute()) {
        return Err("--cargo-tool 必须是绝对路径".into());
    }
    if cargo_audit_tool
        .as_ref()
        .is_some_and(|tool| !tool.is_absolute())
        || rustsec_db.as_ref().is_some_and(|db| !db.is_absolute())
    {
        return Err("Rust CVE 工具与数据库必须是绝对路径".into());
    }
    if pip_audit_tool
        .as_ref()
        .is_some_and(|tool| !tool.is_absolute())
        || pip_audit_tool.is_some() != pip_audit_version.is_some()
        || pip_audit_version
            .as_ref()
            .is_some_and(|version: &String| version.trim().is_empty())
    {
        return Err("Python CVE 工具与版本必须成对提供，工具路径必须为绝对路径".into());
    }
    if [
        maven_tool.as_ref(),
        java_home.as_ref(),
        java_tool.as_ref(),
        checkstyle_jar.as_ref(),
        checkstyle_config.as_ref(),
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
    if owner.is_some() != lease_token.is_some() {
        return Err("--owner 与 --lease-token 必须同时提供".into());
    }
    if npm_options.is_empty() && !eslint_options.is_empty() {
        let mut request = vec!["validation.ts".to_owned()];
        for (key, value) in &eslint_options {
            request.extend([key.clone(), value.clone()]);
        }
        if let Some(config) = &checkstyle_config {
            request.extend(["--config".into(), config.to_string_lossy().into_owned()]);
        }
        crate::eslint_lint_arguments::EslintLintArguments::parse(&request)?;
    }
    let (timeout_ms, timeout_source) = select_check_timeout(timeout_seen.then_some(timeout_ms))?;
    Ok(Arguments {
        task_id: task_id.clone(),
        eslint_options,
        npm_options,
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        ruff_tool,
        zig_tool,
        erl_tool,
        swift_tool,
        ruby_tool,
        kotlinc_tool,
        cargo_tool,
        cargo_audit_tool,
        rustsec_db,
        pip_audit_tool,
        pip_audit_version,
        go_tool,
        maven_tool,
        java_home,
        java_tool,
        checkstyle_jar,
        checkstyle_config,
        maven_repo,
        repo_sha256,
        cve_data_dir,
        cve_data_sha256,
        owner,
        lease_token,
        json,
        timeout_ms,
        timeout_source,
    })
}

fn parse_format(value: &str) -> Result<bool, String> {
    match value {
        "json" => Ok(true),
        "human" => Ok(false),
        _ => Err(format!("不支持的格式：{value}")),
    }
}

#[cfg(test)]
mod tests {
    use super::classify;
    use serde_json::json;
    #[test]
    fn python_confirmation_distinguishes_syntax_from_environment_and_style() {
        let brief = json!({"kind":"blocker","checker_id":"python.ruff","reason_code":"python_syntax_confirmation_needed",
            "scope":"broken.py","affected_paths":["broken.py"]});
        let mut scan = json!({"schema_version":"0.19.0","task_input_stable":true,
            "files":[{"path":"broken.py","run_status":"findings","source_sha256":"a".repeat(64),
                "findings":[{"rule_id":"invalid-syntax"}]}]});
        assert_eq!(classify(&brief, &scan), "still_present");
        let mut historical = scan.clone();
        historical["schema_version"] = json!("0.18.0");
        assert_eq!(
            classify(&brief, &historical),
            "environment_restored_unverified_policy"
        );
        scan["files"][0]["findings"][0]["rule_id"] = json!("F401");
        assert_eq!(
            classify(&brief, &scan),
            "candidate_absent_unverified_policy"
        );
        scan["files"][0]["run_status"] = json!("incomplete");
        assert_eq!(classify(&brief, &scan), "still_blocked");
        scan["task_input_stable"] = json!(false);
        assert_eq!(classify(&brief, &scan), "incomplete");
    }
}
