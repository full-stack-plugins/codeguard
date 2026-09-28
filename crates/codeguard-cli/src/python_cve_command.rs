//! 标准 Python 锁的 pip-audit 局部原生观察；完整依赖和漏洞库门禁仍未实现。

use crate::check_budget::{
    budget_record, parse_check_timeout, resolve_project_default, select_check_timeout,
};
use crate::discovery::is_standard_python_lock_name;
use crate::doctor_scratch::DoctorScratch;
use crate::work_sync::{save_local_report, sync_local_workspace};
use crate::workspace_refresh::read_workspace_baseline;
use codeguard_adapters::{
    PipAuditCommand, bind_pip_audit_lockfile, parse_pip_audit_json, parse_pylock_package_identities,
};
use codeguard_runtime::{ProcessSpec, Termination, read_bounded_regular_file, run_process};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct Arguments {
    root: PathBuf,
    tool: PathBuf,
    version: String,
    timeout: Option<u64>,
    json: bool,
}

static NEXT_RUN_ID: AtomicU64 = AtomicU64::new(0);

/// 执行标准 pylock 的原生候选审计；结果只反馈给智能体，不签发交付结论。
pub fn run(args: &[String]) -> ExitCode {
    let arguments = match parse_args(args) {
        Ok(arguments) => arguments,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    };
    let root = match arguments.root.canonicalize() {
        Ok(root) if root.is_dir() => root,
        _ => {
            eprintln!("Python 项目根目录不可用");
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
    let deadline = Instant::now() + Duration::from_millis(timeout);
    let cancelled = AtomicBool::new(false);
    let mut report = observe(
        &root,
        Some(&arguments.tool),
        Some(&arguments.version),
        deadline,
        timeout,
        source,
        &cancelled,
    );
    match persist_and_sync(&root, ".", &report) {
        Ok(true) => {
            report["next_action"] =
                json!("已同步稳定 Python CVE 待处理任务；使用 codeguard next 查看原工具复检指引")
        }
        Ok(false) => {}
        Err(_) => {
            report["next_action"] =
                json!("工作台同步未完成；修复记录目录后运行 codeguard work sync，再核对原生诊断")
        }
    }
    if arguments.json {
        println!("{report}");
    } else {
        println!(
            "Python CVE 局部审计：{}；原生 advisory {} 项；交付未评估",
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
        println!("下一步：{}", report["next_action"]);
    }
    ExitCode::from(if report["reason"] == "request_cancelled" {
        130
    } else {
        3
    })
}

/// 将一轮局部观察同步到已初始化工作区；同步不代表漏洞已确认或门禁通过。
pub(crate) fn persist_and_sync(
    workspace_root: &Path,
    build_root: &str,
    scan: &Value,
) -> Result<bool, &'static str> {
    let Some(report) = workbench_report(workspace_root, build_root, scan)? else {
        return Ok(false);
    };
    save_local_report(workspace_root, &report)?;
    let summary = sync_local_workspace(workspace_root)?;
    if summary.failed_reports != 0 {
        return Err("python_cve_backlog_sync_incomplete");
    }
    Ok(true)
}

/// 包装当前构建根的输入身份；自写报告没有批准权威。
pub(crate) fn workbench_report(
    workspace_root: &Path,
    build_root: &str,
    scan: &Value,
) -> Result<Option<Value>, &'static str> {
    let Some(workspace_id) = read_workspace_baseline(workspace_root)
        .ok()
        .flatten()
        .and_then(|baseline| baseline.workspace_id().map(str::to_owned))
    else {
        return Ok(None);
    };
    let project = bounded_build_root(workspace_root, build_root)?;
    let manifest = input_identity(&project.join("pyproject.toml"), 2 * 1024 * 1024);
    let locks = lock_inputs(&project)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    let run_id = format!(
        "python-cve-{}-{nonce}-{}",
        std::process::id(),
        NEXT_RUN_ID.fetch_add(1, Ordering::Relaxed)
    );
    let mut native_scan = scan.clone();
    if let Some(object) = native_scan.as_object_mut() {
        object.remove("build_root");
    }
    Ok(Some(json!({
        "schema_version":"0.1.0", "report_type":"python_cve_workbench_observation",
        "workspace_binding":"bound", "workspace_id":workspace_id, "run_id":run_id,
        "checker_id":"python.pip_audit", "authority":"local_unverified",
        "coverage_proven":false, "delivery_decision":"not_evaluated",
        "build_root":build_root,
        "manifest_state":manifest.0, "manifest_sha256":manifest.1,
        "lock_inputs":locks,
        "scan":native_scan
    })))
}

/// 限定构建根为真实工作区内的普通目录，不沿符号链接越过项目范围。
pub(crate) fn bounded_build_root(root: &Path, build_root: &str) -> Result<PathBuf, &'static str> {
    if build_root.is_empty() {
        return Err("python_build_root_invalid");
    }
    if build_root == "." {
        return Ok(root.to_path_buf());
    }
    let relative = Path::new(build_root);
    if relative
        .components()
        .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err("python_build_root_invalid");
    }
    let mut path = root.to_path_buf();
    for component in relative.components() {
        path.push(component.as_os_str());
        if !fs::symlink_metadata(&path).is_ok_and(|meta| meta.file_type().is_dir()) {
            return Err("python_build_root_unavailable");
        }
    }
    Ok(path)
}

/// 重读一份标准锁的身份；缺失、链接和读取失败均与正常普通文件区分。
pub(crate) fn input_identity(path: &Path, limit: u64) -> (&'static str, Value) {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_file() => match read_bounded_regular_file(path, limit) {
            Ok(bytes) => ("present", json!(sha256(&bytes))),
            Err(_) => ("unavailable", Value::Null),
        },
        Ok(_) => ("not_regular", Value::Null),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => ("missing", Value::Null),
        Err(_) => ("unavailable", Value::Null),
    }
}

/// 列出全部标准锁身份；多锁保留选择歧义，不擅自挑选一份。
pub(crate) fn lock_inputs(project: &Path) -> Result<Value, &'static str> {
    let mut locks = Vec::new();
    for entry in fs::read_dir(project).map_err(|_| "python_lock_directory_unavailable")? {
        let entry = entry.map_err(|_| "python_lock_directory_unavailable")?;
        let name = entry.file_name();
        let name = name.to_str().ok_or("python_lock_directory_unavailable")?;
        if is_standard_python_lock_name(name) {
            let (state, digest) = input_identity(&entry.path(), 8 * 1024 * 1024);
            locks.push(json!({"name":name,"state":state,"sha256":digest}));
        }
    }
    locks.sort_by(|left, right| left["name"].as_str().cmp(&right["name"].as_str()));
    Ok(Value::Array(locks))
}

/// 在共享检查任务中观察一个 Python 构建根；缺少显式原生上下文只形成阻塞。
pub(crate) fn observe_for_check(
    root: &Path,
    selected_tool: Option<&Path>,
    version: Option<&str>,
    deadline: Instant,
    timeout: u64,
    source: &str,
    cancelled: &AtomicBool,
) -> Value {
    observe(
        root,
        selected_tool,
        version,
        deadline,
        timeout,
        source,
        cancelled,
    )
}

fn observe(
    root: &Path,
    selected_tool: Option<&Path>,
    version: Option<&str>,
    deadline: Instant,
    timeout: u64,
    source: &str,
    cancelled: &AtomicBool,
) -> Value {
    let mut report = json!({
        "schema_version":"0.1.0", "report_type":"python_cve_local_observation",
        "operation":"cve", "language":"python", "checker_id":"python.pip_audit",
        "command_status":"incomplete", "reason":"python_cve_context_incomplete", "exit_code":3,
        "delivery_decision":"not_evaluated", "authority":"local_unverified",
        "coverage_proven":false, "advisory_coverage":"not_evaluated",
        "native_report_valid":false, "local_scan_complete":false,
        "tool_sha256":null, "manifest_sha256":null, "lock_sha256":null, "lock_file":null,
        "native_version":null, "native_exit_code":null, "dependency_count":null,
        "findings":[],
        "next_action":"确认项目标准锁、原生工具、完整依赖图与漏洞数据库身份和时效后重查",
        "execution_budget":budget_record(timeout,source)
    });
    if cancelled.load(std::sync::atomic::Ordering::Relaxed)
        || codeguard_runtime::sigint_cancellation_requested()
        || Instant::now() >= deadline
    {
        return interrupted(report, deadline, cancelled);
    }
    let manifest_path = root.join("pyproject.toml");
    let manifest = match read_bounded_regular_file(&manifest_path, 2 * 1024 * 1024) {
        Ok(bytes) => bytes,
        Err(_) => {
            report["reason"] = json!("python_manifest_unavailable");
            return report;
        }
    };
    report["manifest_sha256"] = json!(sha256(&manifest));
    let locks = match fs::read_dir(root) {
        Ok(entries) => {
            let mut locks = Vec::new();
            for entry in entries {
                let Ok(entry) = entry else {
                    report["reason"] = json!("python_lock_directory_unavailable");
                    return report;
                };
                let file_name = entry.file_name();
                let Some(name) = file_name.to_str() else {
                    report["reason"] = json!("python_lock_directory_unavailable");
                    return report;
                };
                if is_standard_python_lock_name(name) {
                    locks.push(name.to_owned());
                }
            }
            locks
        }
        Err(_) => {
            report["reason"] = json!("python_lock_directory_unavailable");
            return report;
        }
    };
    if locks.is_empty() {
        report["reason"] = json!("standard_python_lock_missing");
        report["next_action"] =
            json!("提供本项目的 PEP 751 标准 pylock，或选择可审计现有 uv/Poetry 锁的原生工具");
        return report;
    }
    if locks.len() != 1 {
        report["reason"] = json!("multiple_python_lock_inputs_unresolved");
        return report;
    }
    let lock_name = &locks[0];
    let lock_path = root.join(lock_name);
    let lock = match read_bounded_regular_file(&lock_path, 8 * 1024 * 1024) {
        Ok(bytes) => bytes,
        Err(_) => {
            report["reason"] = json!("python_lock_input_unavailable");
            return report;
        }
    };
    report["lock_file"] = json!(lock_name);
    report["lock_sha256"] = json!(sha256(&lock));
    let lock_identities = match parse_pylock_package_identities(&lock) {
        Ok(identities) => Some(identities),
        Err("python_lock_selection_unresolved") => None,
        Err(reason) => {
            report["reason"] = json!(reason);
            return report;
        }
    };
    let (Some(selected_tool), Some(version)) = (selected_tool, version) else {
        report["reason"] = json!("pip_audit_native_context_missing");
        report["next_action"] =
            json!("提供显式 pip-audit 可执行文件和版本，核对标准锁后运行原生审计");
        return report;
    };
    let tool = match (
        fs::symlink_metadata(selected_tool),
        selected_tool.canonicalize(),
    ) {
        (Ok(metadata), Ok(path)) if metadata.file_type().is_file() => path,
        _ => {
            report["reason"] = json!("pip_audit_tool_unavailable");
            return report;
        }
    };
    let tool_bytes = match read_bounded_regular_file(&tool, 128 * 1024 * 1024) {
        Ok(bytes) => bytes,
        Err(_) => {
            report["reason"] = json!("pip_audit_tool_unavailable");
            return report;
        }
    };
    report["tool_sha256"] = json!(sha256(&tool_bytes));
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let Some(scratch) =
        DoctorScratch::create(&format!("python-cve-{}-{nonce}", std::process::id()))
    else {
        report["reason"] = json!("python_private_workspace_unavailable");
        return report;
    };
    let snapshot = scratch.path().join("project");
    let cache = scratch.path().join("cache");
    if fs::create_dir(&snapshot).is_err()
        || fs::create_dir(&cache).is_err()
        || fs::write(snapshot.join("pyproject.toml"), &manifest).is_err()
        || fs::write(snapshot.join(lock_name), &lock).is_err()
    {
        report["reason"] = json!("python_private_workspace_unavailable");
        return report;
    }
    let version_output = run_process(
        &ProcessSpec {
            executable: tool.clone(),
            args: vec![OsString::from("--version")],
            cwd: snapshot.clone(),
            env: BTreeMap::new(),
            stdin: None,
            deadline,
            output_limit_bytes: 4096,
        },
        cancelled,
    );
    if version_output.termination != Termination::Exited(0) {
        report["reason"] = json!(termination_reason(version_output.termination));
        return interrupted(report, deadline, cancelled);
    }
    let observed = String::from_utf8_lossy(&version_output.stdout);
    let observed = observed.trim().strip_prefix("pip-audit ");
    if observed != Some(version) {
        report["reason"] = json!("pip_audit_version_unverified");
        return report;
    }
    report["native_version"] = json!(version);
    let command = PipAuditCommand {
        tool: tool.clone(),
        project_root: snapshot.clone(),
        cache,
    };
    let args = match command.args() {
        Ok(args) => args,
        Err(reason) => {
            report["reason"] = json!(reason);
            return report;
        }
    };
    let output = run_process(
        &ProcessSpec {
            executable: tool.clone(),
            args,
            cwd: snapshot.clone(),
            env: BTreeMap::new(),
            stdin: None,
            deadline,
            output_limit_bytes: 16 * 1024 * 1024,
        },
        cancelled,
    );
    let failure_reason = termination_reason(output.termination);
    let exit = match output.termination {
        Termination::Exited(code) => Some(code),
        Termination::Cancelled => {
            report["reason"] = json!(failure_reason);
            return interrupted(report, deadline, cancelled);
        }
        _ if output.stdout.is_empty() => {
            report["reason"] = json!(failure_reason);
            return report;
        }
        _ => None,
    };
    if let Some(exit) = exit {
        report["native_exit_code"] = json!(exit);
    }
    if read_bounded_regular_file(&manifest_path, 2 * 1024 * 1024)
        .ok()
        .as_deref()
        != Some(manifest.as_slice())
        || read_bounded_regular_file(&lock_path, 8 * 1024 * 1024)
            .ok()
            .as_deref()
            != Some(lock.as_slice())
        || read_bounded_regular_file(&snapshot.join("pyproject.toml"), 2 * 1024 * 1024)
            .ok()
            .as_deref()
            != Some(manifest.as_slice())
        || read_bounded_regular_file(&snapshot.join(lock_name), 8 * 1024 * 1024)
            .ok()
            .as_deref()
            != Some(lock.as_slice())
        || read_bounded_regular_file(&tool, 128 * 1024 * 1024)
            .ok()
            .as_deref()
            != Some(tool_bytes.as_slice())
    {
        report["reason"] = json!("python_cve_input_changed");
        return report;
    }
    let (parsed, exit_contract_valid) = match exit {
        Some(exit) => match parse_pip_audit_json(&output.stdout, version, version, Some(exit)) {
            Ok(parsed) => (parsed, true),
            Err(reason) => match parse_pip_audit_json(&output.stdout, version, version, Some(1)) {
                Ok(parsed) => (parsed, false),
                Err(_) => {
                    report["reason"] = json!(reason);
                    return report;
                }
            },
        },
        None => match parse_pip_audit_json(&output.stdout, version, version, Some(1)) {
            Ok(parsed) => (parsed, false),
            Err(_) => {
                report["reason"] = json!(failure_reason);
                return report;
            }
        },
    };
    let Some(lock_identities) = lock_identities else {
        report["reason"] = json!("python_lock_selection_unresolved");
        report["next_action"] = json!(format!(
            "原生审计已运行，观察到 {} 个待归属 advisory；须先确认目标 Python 环境、extras 与依赖组，不能把多环境锁的全部包或本轮 advisory 当作项目实际依赖漏洞",
            parsed.findings.len()
        ));
        return report;
    };
    if let Err(reason) = bind_pip_audit_lockfile(&parsed, &lock_identities) {
        report["reason"] = json!(reason);
        return report;
    }
    report["native_report_valid"] = json!(true);
    report["dependency_count"] = json!(parsed.dependencies.len());
    report["findings"] = json!(
        parsed
            .findings
            .iter()
            .map(|finding| json!({
                "advisory_id":finding.advisory_id,
                "package_name":finding.package_name,
                "package_version":finding.package_version,
                "aliases":finding.aliases,
                "cve_aliases":finding.cve_aliases,
                "fix_versions":finding.fix_versions
            }))
            .collect::<Vec<_>>()
    );
    report["command_status"] = json!("native_observed");
    report["reason"] = json!(if !exit_contract_valid {
        failure_reason
    } else if parsed.findings.is_empty() {
        "native_zero_advisories_unverified"
    } else {
        "native_advisories_observed_unverified"
    });
    report["next_action"] = json!(if exit_contract_valid {
        "核对标准锁的环境、依赖组、全部解析包与原生结果，再验证漏洞数据库身份和时效；逐 advisory 修复后用原工具复检"
    } else {
        "原生审计未完成；保留已归属 advisory 供调查，先核对原生故障、标准锁与漏洞数据库，再用原工具完整复检"
    });
    report
}

fn interrupted(mut report: Value, deadline: Instant, cancelled: &AtomicBool) -> Value {
    let reason = if report["reason"] == "request_cancelled"
        || cancelled.load(std::sync::atomic::Ordering::Relaxed)
        || codeguard_runtime::sigint_cancellation_requested()
    {
        "request_cancelled"
    } else if report["reason"] == "request_deadline_exceeded" || Instant::now() >= deadline {
        "request_deadline_exceeded"
    } else {
        return report;
    };
    report["reason"] = json!(reason);
    if reason == "request_cancelled" {
        report["exit_code"] = json!(130);
        report["command_status"] = json!("cancelled");
    }
    report
}

fn termination_reason(termination: Termination) -> &'static str {
    match termination {
        Termination::Cancelled => "request_cancelled",
        Termination::TimedOut | Termination::DeadlineBeforeStart => "request_deadline_exceeded",
        Termination::OutputLimit => "pip_audit_output_limit",
        _ => "pip_audit_execution_incomplete",
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn parse_args(args: &[String]) -> Result<Arguments, String> {
    let mut root = None;
    let mut tool = None;
    let mut version = None;
    let mut timeout = None;
    let mut format = None;
    let mut index = 0;
    while index < args.len() {
        let current = &args[index];
        let (key, value) = if let Some((key, value)) = current.split_once('=') {
            (key, Some(value))
        } else if matches!(
            current.as_str(),
            "--pip-audit-tool" | "--pip-audit-version" | "--timeout" | "--format"
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
            ("--pip-audit-tool", Some(value))
                if tool.is_none() && Path::new(value).is_absolute() =>
            {
                tool = Some(PathBuf::from(value));
            }
            ("--pip-audit-version", Some(value))
                if version.is_none()
                    && value.len() <= 64
                    && value
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'.') =>
            {
                version = Some(value.to_owned());
            }
            ("--timeout", Some(value)) if timeout.is_none() => {
                timeout = Some(parse_check_timeout(value)?);
            }
            ("--format", Some(value)) if format.is_none() && matches!(value, "json" | "human") => {
                format = Some(value);
            }
            (_, None) if !current.is_empty() && !current.starts_with('-') && root.is_none() => {
                root = Some(PathBuf::from(current));
            }
            _ => return Err("cve python 参数非法".into()),
        }
        index += 1;
    }
    Ok(Arguments {
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        tool: tool.ok_or("必须显式指定 --pip-audit-tool ABS_PATH")?,
        version: version.ok_or("必须显式指定 --pip-audit-version VERSION")?,
        timeout,
        json: format == Some("json"),
    })
}
