//! 显式调用原生 cargo-audit 的 Rust CVE 局部观察；数据库权威仍待接入。

use crate::check_budget::{
    budget_record, parse_check_timeout, resolve_project_default, select_check_timeout,
};
use crate::work_sync::{save_local_report, sync_local_workspace};
use crate::workspace_refresh::read_workspace_baseline;
use codeguard_adapters::{bind_cargo_audit_lockfile, parse_cargo_audit_json};
use codeguard_runtime::{ProcessSpec, Termination, read_bounded_regular_file, run_process};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
    process::ExitCode,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

static NEXT_RUN_ID: AtomicU64 = AtomicU64::new(0);

struct Arguments {
    root: PathBuf,
    tool: PathBuf,
    database: PathBuf,
    timeout: Option<u64>,
    json: bool,
}

/// 执行 `cve rust`；参数为该入口剩余 argv，始终保留未核验的数据库和策略状态。
pub fn run(args: &[String]) -> ExitCode {
    let arguments = match parse_args(args) {
        Ok(value) => value,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    };
    let root = match arguments.root.canonicalize() {
        Ok(path) if path.is_dir() => path,
        _ => {
            eprintln!("Rust 项目根目录不可用");
            return ExitCode::from(2);
        }
    };
    let (timeout, source) = match select_check_timeout(arguments.timeout)
        .and_then(|selection| resolve_project_default(&root, selection.0, selection.1))
    {
        Ok(value) => value,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    };
    let mut report = observe_for_check(
        &root,
        Some(&arguments.tool),
        Some(&arguments.database),
        timeout,
        source,
        Instant::now() + Duration::from_millis(timeout),
        &AtomicBool::new(false),
    );
    match persist_and_sync(&root, &report) {
        Ok(true) => report["next_actions"]
            .as_array_mut()
            .expect("固定下一步数组")
            .push(json!(
                "已同步稳定待处理任务；使用 codeguard next 获取原工具复检指引"
            )),
        Ok(false) => {}
        Err(reason) => report["next_actions"]
            .as_array_mut()
            .expect("固定下一步数组")
            .push(json!(format!(
                "工作台同步未完成：{reason}；修复记录目录后运行 codeguard work sync"
            ))),
    }
    if arguments.json {
        println!("{report}");
    } else {
        println!(
            "Rust CVE 检查：{}；原生漏洞 {} 项；数据库与完整覆盖尚未核验",
            report["reason"],
            report["findings"].as_array().map_or(0, Vec::len)
        );
        for finding in report["findings"].as_array().into_iter().flatten() {
            println!(
                "{} {} {}；CVE {}",
                finding["advisory_id"],
                finding["package_name"],
                finding["package_version"],
                finding["cve_aliases"]
            );
        }
        println!("下一步：核对漏洞库身份和时效，复核依赖图与原生 advisory，再决定修复或误报裁定");
    }
    ExitCode::from(if report["reason"] == "request_cancelled" {
        130
    } else {
        3
    })
}

/// 将本轮局部观察作为待审 CVE 完整性任务保存；不改变原始发现或质量门禁。
pub(crate) fn persist_and_sync(root: &Path, scan: &Value) -> Result<bool, &'static str> {
    let Some(report) = workbench_report(root, scan)? else {
        return Ok(false);
    };
    save_local_report(root, &report)?;
    let summary = sync_local_workspace(root)?;
    if summary.failed_reports != 0 {
        return Err("rust_cve_backlog_sync_incomplete");
    }
    Ok(true)
}

/// 包装本轮原生观察供持久工作台消费；包装本身没有批准权威。
pub(crate) fn workbench_report(root: &Path, scan: &Value) -> Result<Option<Value>, &'static str> {
    let Some(workspace_id) = read_workspace_baseline(root)
        .ok()
        .flatten()
        .and_then(|baseline| baseline.workspace_id().map(str::to_owned))
    else {
        return Ok(None);
    };
    let manifest = input_identity(&root.join("Cargo.toml"), 2 * 1024 * 1024);
    let lock = input_identity(&root.join("Cargo.lock"), 8 * 1024 * 1024);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    let run_id = format!(
        "rust-cve-{}-{nonce}-{}",
        std::process::id(),
        NEXT_RUN_ID.fetch_add(1, Ordering::Relaxed)
    );
    let report = json!({
        "schema_version":"0.1.0", "report_type":"rust_cve_workbench_observation",
        "workspace_binding":"bound", "workspace_id":workspace_id, "run_id":run_id,
        "checker_id":"rust.cargo_audit", "authority":"local_unverified",
        "coverage_proven":false, "delivery_decision":"not_evaluated",
        "manifest_state":manifest.0, "manifest_sha256":manifest.1,
        "lock_state":lock.0, "lock_sha256":lock.1,
        "scan":scan
    });
    Ok(Some(report))
}

fn input_identity(path: &Path, limit: u64) -> (&'static str, Value) {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_file() => match read_bounded_regular_file(path, limit) {
            Ok(bytes) => ("present", json!(format!("{:x}", Sha256::digest(bytes)))),
            Err(_) => ("unavailable", Value::Null),
        },
        Ok(_) => ("not_regular", Value::Null),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => ("missing", Value::Null),
        Err(_) => ("unavailable", Value::Null),
    }
}

/// 项目检查只读选择已有cargo-audit；显式请求或首个存在入口固定后不尝试其它工具。
/// 参数为可选显式路径；返回原请求或绝对PATH中的固定入口，不启动、不安装、不推断数据库。
/// 独立cve与任务复检继续由调用者提供原工具参数。
pub(crate) fn discover_for_project_check(requested: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = requested {
        return Some(path.to_path_buf());
    }
    let path = std::env::var_os("PATH")?;
    let name = if cfg!(windows) {
        "cargo-audit.exe"
    } else {
        "cargo-audit"
    };
    for directory in std::env::split_paths(&path) {
        if !directory.is_absolute() {
            continue;
        }
        let entry = directory.join(name);
        match std::fs::symlink_metadata(&entry) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            _ => return Some(entry.canonicalize().unwrap_or(entry)),
        }
    }
    None
}

/// 在调用者的统一截止时间和取消标记内取得原生 CVE 观察，不保存或批准任务。
pub(crate) fn observe_for_check(
    root: &Path,
    tool: Option<&Path>,
    database: Option<&Path>,
    timeout: u64,
    source: &str,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    let mut report = json!({
        "schema_version":"0.1.0", "report_type":"rust_cve_local_observation", "operation":"cve", "language":"rust",
        "checker_id":"rust.cargo_audit", "command_status":"incomplete", "exit_code":3,
        "delivery_decision":"not_evaluated", "authority":"local_unverified", "coverage_proven":false,
        "database_freshness":"unverified", "local_scan_complete":false, "reason":"input_unavailable",
        "tool_sha256":null, "lock_sha256":null, "manifest_sha256":null,
        "dependency_count":null, "database_advisory_count":null, "database_last_commit":null, "database_last_updated":null,
        "native_warnings_present":false,
        "findings":[], "next_actions":["核对原生漏洞库来源、提交与时效；零漏洞不能证明安全", "复核解析依赖及原生 advisory 后修复或提出精确误报候选"],
        "execution_budget":budget_record(timeout,source)
    });
    if cancelled.load(std::sync::atomic::Ordering::Relaxed)
        || codeguard_runtime::sigint_cancellation_requested()
    {
        report["reason"] = json!("request_cancelled");
        report["command_status"] = json!("cancelled");
        report["exit_code"] = json!(130);
        return report;
    }
    let Some(tool) = tool else {
        report["reason"] = json!("cargo_audit_tool_not_selected");
        return report;
    };
    let Some(database) = database else {
        report["reason"] = json!("cargo_audit_database_not_selected");
        return report;
    };
    let tool = match tool.canonicalize() {
        Ok(path) if path == tool => path,
        _ => {
            report["reason"] = json!("cargo_audit_tool_unavailable");
            return report;
        }
    };
    let database = match database.canonicalize() {
        Ok(path) if path == database && path.is_dir() => path,
        _ => {
            report["reason"] = json!("cargo_audit_database_unavailable");
            return report;
        }
    };
    let (Ok(tool_before), Ok(manifest_before), Ok(lock_before)) = (
        read_bounded_regular_file(&tool, 128 * 1024 * 1024),
        read_bounded_regular_file(&root.join("Cargo.toml"), 2 * 1024 * 1024),
        read_bounded_regular_file(&root.join("Cargo.lock"), 8 * 1024 * 1024),
    ) else {
        return report;
    };
    report["tool_sha256"] = json!(format!("{:x}", Sha256::digest(&tool_before)));
    report["manifest_sha256"] = json!(format!("{:x}", Sha256::digest(&manifest_before)));
    report["lock_sha256"] = json!(format!("{:x}", Sha256::digest(&lock_before)));
    let output = run_process(
        &ProcessSpec {
            executable: tool.clone(),
            args: [
                OsString::from("audit"),
                OsString::from("--no-fetch"),
                OsString::from("--no-yanked"),
                OsString::from("--json"),
                OsString::from("--db"),
                database.as_os_str().to_os_string(),
                OsString::from("--file"),
                OsString::from("Cargo.lock"),
            ]
            .into(),
            cwd: root.to_path_buf(),
            env: BTreeMap::new(),
            stdin: None,
            deadline,
            output_limit_bytes: 16 * 1024 * 1024,
        },
        cancelled,
    );
    let termination_reason = match output.termination {
        Termination::Cancelled => "request_cancelled",
        Termination::TimedOut | Termination::DeadlineBeforeStart => "request_deadline_exceeded",
        Termination::OutputLimit => "cargo_audit_output_limit",
        _ => "cargo_audit_execution_incomplete",
    };
    if output.termination == Termination::Cancelled {
        report["reason"] = json!(termination_reason);
        report["command_status"] = json!("cancelled");
        report["exit_code"] = json!(130);
        return report;
    }
    if !matches!(output.termination, Termination::Exited(_)) && output.stdout.is_empty() {
        report["reason"] = json!(termination_reason);
        return report;
    }
    if read_bounded_regular_file(&tool, 128 * 1024 * 1024)
        .ok()
        .as_deref()
        != Some(&tool_before)
        || read_bounded_regular_file(&root.join("Cargo.toml"), 2 * 1024 * 1024)
            .ok()
            .as_deref()
            != Some(&manifest_before)
        || read_bounded_regular_file(&root.join("Cargo.lock"), 8 * 1024 * 1024)
            .ok()
            .as_deref()
            != Some(&lock_before)
    {
        report["reason"] = json!("cargo_audit_input_changed");
        return report;
    }
    let (parsed, exit_contract_valid) = match output.termination {
        Termination::Exited(native_exit) => {
            match parse_cargo_audit_json(&output.stdout, Some(native_exit)) {
                Ok(value) => (value, true),
                Err(reason) => match parse_cargo_audit_json(&output.stdout, Some(1)) {
                    Ok(value) => (value, false),
                    Err(_) => {
                        report["reason"] = json!(reason);
                        return report;
                    }
                },
            }
        }
        _ => match parse_cargo_audit_json(&output.stdout, Some(1)) {
            Ok(value) => (value, false),
            Err(_) => {
                report["reason"] = json!(termination_reason);
                return report;
            }
        },
    };
    if let Err(reason) = bind_cargo_audit_lockfile(&parsed, &lock_before) {
        report["reason"] = json!(reason);
        return report;
    }
    report["local_scan_complete"] = json!(exit_contract_valid);
    report["reason"] = json!(if exit_contract_valid {
        "database_freshness_unverified"
    } else {
        termination_reason
    });
    report["dependency_count"] = json!(parsed.dependency_count);
    report["database_advisory_count"] = json!(parsed.database_advisory_count);
    report["database_last_commit"] = json!(parsed.database_commit);
    report["database_last_updated"] = json!(parsed.database_updated);
    report["native_warnings_present"] = json!(parsed.warnings_present);
    report["findings"] = json!(parsed.findings.into_iter().map(|finding| json!({
        "advisory_id":finding.advisory_id,
        "package_name":finding.package_name,
        "package_version":finding.package_version,
        "package_source_sha256":format!("{:x}",Sha256::digest(finding.package_source.as_bytes())),
        "package_checksum":finding.package_checksum,
        "cve_aliases":finding.cve_aliases,
        "cvss_vector":finding.cvss,
        "severity":"unverified"
    })).collect::<Vec<_>>());
    report
}

fn parse_args(args: &[String]) -> Result<Arguments, String> {
    let mut root = None;
    let mut tool = None;
    let mut database = None;
    let mut timeout = None;
    let mut format = None;
    let mut index = 0;
    while index < args.len() {
        let current = &args[index];
        let (key, value) = if let Some((key, value)) = current.split_once('=') {
            (key, Some(value))
        } else if matches!(
            current.as_str(),
            "--cargo-audit-tool" | "--db" | "--timeout" | "--format"
        ) {
            index += 1;
            (
                current.as_str(),
                Some(args.get(index).ok_or("参数缺少值")?.as_str()),
            )
        } else {
            (current.as_str(), None)
        };
        match (key, value) {
            ("--cargo-audit-tool", Some(value))
                if tool.is_none() && Path::new(value).is_absolute() =>
            {
                tool = Some(PathBuf::from(value))
            }
            ("--db", Some(value)) if database.is_none() && Path::new(value).is_absolute() => {
                database = Some(PathBuf::from(value))
            }
            ("--timeout", Some(value)) if timeout.is_none() => {
                timeout = Some(parse_check_timeout(value)?)
            }
            ("--format", Some(value)) if format.is_none() && matches!(value, "json" | "human") => {
                format = Some(value)
            }
            (_, None) if !current.is_empty() && !current.starts_with('-') && root.is_none() => {
                root = Some(PathBuf::from(current))
            }
            _ => return Err("cve rust 参数非法".into()),
        }
        index += 1;
    }
    Ok(Arguments {
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        tool: tool.ok_or("必须显式指定 --cargo-audit-tool ABS_PATH")?,
        database: database.ok_or("必须显式指定 --db ABS_PATH")?,
        timeout,
        json: format == Some("json"),
    })
}
