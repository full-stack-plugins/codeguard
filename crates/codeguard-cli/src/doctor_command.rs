//! 局部配置/Ruff 版本诊断，不安装或扫描源码。
use crate::{
    check_budget::parse_check_timeout, discovery::discover, doctor_scratch::DoctorScratch,
};
use codeguard_adapters::legacy_registry;
use codeguard_runtime::{
    NativeObservation, NativeVersionRequest, ProcessSpec, observe_native_version,
    read_bounded_regular_file,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    process::ExitCode,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
/// 仅接受显式工具路径的诊断选项。
struct Args {
    root: PathBuf,
    ruff: Option<PathBuf>,
    timeout_ms: u64,
    json: bool,
}

/// 观察配置和固定 Ruff 版本，输出局部诊断；无策略批准时退出 3。
/// 参数支持显式工具与预算；结果不代表质量通过。
pub fn run(arguments: &[String]) -> ExitCode {
    let args = match parse_args(arguments) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let deadline = Instant::now() + Duration::from_millis(args.timeout_ms);
    let mut report = new_report(args.timeout_ms);
    let run_id = report["run_id"].as_str().expect("内部运行身份").to_owned();
    match args.root.canonicalize() {
        Ok(root) if root.is_dir() => {
            let registry = match legacy_registry() {
                Ok(r) => r,
                Err(_) => {
                    eprintln!("语言清单损坏");
                    return ExitCode::from(4);
                }
            };
            report["profile_summary"] = crate::init_profile_feedback::render(&discover(
                &root,
                &registry,
                &NativeObservation,
            ));
            if let Some(tool) = args.ruff {
                report["ruff_version"] = diagnose(tool, &run_id, deadline);
            }
            persist_and_sync(&root, &mut report);
        }
        _ => {
            report["ruff_version"] = observation("incomplete", Some("project_unavailable"));
            report["next_actions"] = json!(["provide_readable_project_directory"]);
        }
    }
    let cancelled = codeguard_runtime::sigint_cancellation_requested()
        || report["ruff_version"]["reason"] == "request_cancelled";
    if args.json {
        println!("{report}");
    } else {
        println!("环境诊断：局部未完成；准备 unknown；交付 not_evaluated；门禁效力 none。");
        println!(
            "报告保存与任务同步：{}；工作区绑定：{}；同步结果：{}",
            report["persistence"], report["workspace_binding"], report["backlog_sync"]
        );
        println!(
            "Ruff 版本：{}；原因：{}；下一步：{}",
            report["ruff_version"]["status"],
            report["ruff_version"]["reason"],
            report["ruff_version"]["next_action"]
        );
        for checker in report["profile_summary"]["checkers"]
            .as_array()
            .into_iter()
            .flatten()
        {
            println!(
                "检查配置 {} @ {}：{}；原因 {}；下一步 {}；源码检查 not_run",
                checker["checker_id"],
                checker["build_root"],
                checker["configuration"],
                checker["reason"],
                checker["next_action"]
            );
        }
    }
    ExitCode::from(if cancelled { 130 } else { 3 })
}

fn persist_and_sync(root: &std::path::Path, report: &mut Value) {
    // 拒绝工作区根链接，避免局部报告写入另一个项目。
    if std::fs::symlink_metadata(root.join("codeguard")).is_ok_and(|m| !m.file_type().is_dir()) {
        report["workspace_binding"] = "invalid".into();
        return;
    }
    match crate::workspace_refresh::read_workspace_baseline(root) {
        Ok(Some(baseline)) => match baseline.workspace_id() {
            Some(id) => {
                report["workspace_binding"] = "bound".into();
                report["workspace_id"] = id.into();
            }
            None => {
                report["workspace_binding"] = "legacy_unbound".into();
                return;
            }
        },
        Ok(None) => return,
        Err(_) => {
            report["workspace_binding"] = "invalid".into();
            return;
        }
    }
    report["persistence"] = "queued".into();
    match crate::work_sync::save_local_report(root, report) {
        Ok(()) => match crate::work_sync::sync_local_workspace(root) {
            Ok(summary) => {
                report["persistence"] = if summary.failed_reports == 0 {
                    "synced_partial"
                } else {
                    "backlog_update_failed"
                }
                .into();
                report["backlog_sync"] = json!({"imported_reports":summary.imported_reports,"new_blockers":summary.new_blockers,"failed_reports":summary.failed_reports});
            }
            Err(_) => report["persistence"] = "backlog_update_failed".into(),
        },
        Err(_) => report["persistence"] = "backlog_update_failed".into(),
    }
}
fn observation(status: &str, reason: Option<&str>) -> Value {
    json!({"status":status,"reason":reason,"version":null,"tool_sha256":null,"required_by_policy":null,"next_action":"选择明确 Ruff 原生入口并核对版本/身份；按 reason 修复环境后重新 doctor，不修改无关源码。"})
}
fn diagnose(tool: PathBuf, id: &str, deadline: Instant) -> Value {
    let tool = match tool.canonicalize() {
        Ok(t) => t,
        Err(_) => return observation("incomplete", Some("tool_unavailable")),
    };
    let bytes = match read_bounded_regular_file(&tool, 128 * 1024 * 1024) {
        Ok(b) => b,
        Err(_) => return observation("incomplete", Some("tool_unavailable")),
    };
    // 脚本入口尚不能隔离项目代码/联网，启动前拒绝。
    if bytes.starts_with(b"#!") {
        return observation("incomplete", Some("script_launcher_requires_isolation"));
    }
    if !native_header(&bytes) {
        return observation("incomplete", Some("native_launcher_unrecognized"));
    }
    let Some(scratch) = DoctorScratch::create(id) else {
        return observation("incomplete", Some("private_scratch_unavailable"));
    };
    let digest: [u8; 32] = Sha256::digest(&bytes).into();
    let result = observe_native_version(
        &NativeVersionRequest {
            process: ProcessSpec {
                executable: tool,
                args: vec!["--version".into()],
                cwd: scratch.path().to_path_buf(),
                env: BTreeMap::new(),
                stdin: None,
                deadline: deadline.min(Instant::now() + Duration::from_secs(10)),
                output_limit_bytes: 4096,
            },
            expected_tool_sha256: digest,
            expected_stdout: b"ruff 0.16.8\n".to_vec(),
            evidence_root: scratch.path().to_path_buf(),
            log_name: "ruff-version.log".into(),
        },
        &AtomicBool::new(false),
    );
    let mut report = observation(
        if result.complete {
            "observed_untrusted"
        } else {
            "incomplete"
        },
        result.reason,
    );
    if result.complete {
        report["version"] = "ruff 0.16.8".into();
        report["tool_sha256"] = format!("{:x}", Sha256::digest(&bytes)).into();
        report["next_action"] =
            "仅观察 Ruff 版本；核验批准工具锁及义务，补齐其它前置后执行 check 与任务复检。".into();
    }
    report
}
fn native_header(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\x7fELF")
        || bytes.get(..4).is_some_and(|h| {
            matches!(
                h,
                [0xfe, 0xed, 0xfa, 0xce]
                    | [0xce, 0xfa, 0xed, 0xfe]
                    | [0xfe, 0xed, 0xfa, 0xcf]
                    | [0xcf, 0xfa, 0xed, 0xfe]
                    | [0xca, 0xfe, 0xba, 0xbe]
                    | [0xbe, 0xba, 0xfe, 0xca]
                    | [0xca, 0xfe, 0xba, 0xbf]
                    | [0xbf, 0xba, 0xfe, 0xca]
            )
        })
}
fn parse_args(arguments: &[String]) -> Result<Args, String> {
    let (mut root, mut ruff, mut timeout, mut format) = (None, None, None, None);
    let mut i = 0;
    while i < arguments.len() {
        let arg = &arguments[i];
        if let Some(v) = arg.strip_prefix("--format=") {
            if format.replace(v.to_owned()).is_some() {
                return Err("格式重复".into());
            }
        } else if matches!(arg.as_str(), "--ruff-tool" | "--timeout" | "--format") {
            i += 1;
            let v = arguments.get(i).ok_or("选项缺少值")?;
            match arg.as_str() {
                "--ruff-tool" => {
                    let p = PathBuf::from(v);
                    if !p.is_absolute() || ruff.replace(p).is_some() {
                        return Err("Ruff 须是唯一绝对路径".into());
                    }
                }
                "--timeout" => {
                    if timeout.replace(parse_check_timeout(v)?).is_some() {
                        return Err("预算重复".into());
                    }
                }
                _ => {
                    if format.replace(v.clone()).is_some() {
                        return Err("格式重复".into());
                    }
                }
            }
        } else if arg.starts_with('-') || root.replace(PathBuf::from(arg)).is_some() {
            return Err("doctor 参数或路径无效".into());
        }
        i += 1;
    }
    if format
        .as_deref()
        .is_some_and(|v| !matches!(v, "human" | "json"))
    {
        return Err("格式只支持 human/json".into());
    }
    Ok(Args {
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        ruff,
        timeout_ms: timeout.unwrap_or(120000),
        json: format.as_deref() == Some("json"),
    })
}

fn new_report(timeout_ms: u64) -> Value {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |v| v.as_nanos());
    let run_id = format!(
        "doctor-{}-{stamp}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    json!({"schema_version":"0.2.0","report_type":"doctor_observation","run_id":run_id,"workspace_binding":"uninitialized","workspace_id":null,"backlog_sync":null,"scope":"configuration_and_ruff_version_only","inspection_status":"incomplete","authority":"unverified","readiness":"unknown","delivery_decision":"not_evaluated","gate_effect":"none","quality_checks":"not_run","profile_summary":null,"budget":{"timeout_ms":timeout_ms,"probe_limit_ms":10000,"enforcement":"native_execution_only"},"ruff_version":observation("not_selected",Some("ruff_tool_not_selected")),"next_actions":["review_checker_configuration","bind_approved_prerequisites","complete_remaining_tool_diagnostics"],"persistence":"not_saved"})
}

/// 用任务复检的剩余预算观察版本，返回待同步报告；不自行保存或输出。
/// root 为已核验工作区，tool 为用户显式选择的入口；无选择保持未完成。
pub(crate) fn observe_for_verification(
    root: &std::path::Path,
    tool: Option<&std::path::Path>,
    timeout_ms: u64,
    deadline: Instant,
) -> Result<Value, &'static str> {
    let registry = legacy_registry().map_err(|_| "language_registry_invalid")?;
    let baseline = crate::workspace_refresh::read_workspace_baseline(root)
        .map_err(|_| "workspace_profile_invalid")?
        .ok_or("workspace_uninitialized")?;
    let workspace_id = baseline.workspace_id().ok_or("workspace_binding_invalid")?;
    let mut report = new_report(timeout_ms);
    report["workspace_binding"] = "bound".into();
    report["workspace_id"] = workspace_id.into();
    report["persistence"] = "queued".into();
    report["profile_summary"] =
        crate::init_profile_feedback::render(&discover(root, &registry, &NativeObservation));
    if let Some(tool) = tool {
        report["ruff_version"] = diagnose(
            tool.to_path_buf(),
            report["run_id"].as_str().expect("内部运行身份"),
            deadline,
        );
    }
    let after = crate::workspace_refresh::read_workspace_baseline(root)
        .map_err(|_| "workspace_profile_invalid")?
        .ok_or("workspace_uninitialized")?;
    if after.workspace_id() != Some(workspace_id) {
        return Err("workspace_changed_during_verification");
    }
    Ok(report)
}
