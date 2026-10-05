//! 固定ShellCheck0.11.0的受控stdin观察；原规则、配置和工具身份不能被通用exit猜测替代。
use crate::{doctor_scratch::DoctorScratch, shellcheck_config::ShellCheckConfig};
use codeguard_runtime::{ProcessSpec, Termination, read_bounded_regular_file, run_process};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap, ffi::OsString, path::Path, sync::atomic::AtomicBool, time::Instant,
};
/// 执行原生版本和json1检查；参数固定源码、方言、配置及共同截止时间，返回局部未批准事实。
pub(crate) fn observe(
    tool: &Path,
    source_path: &Path,
    source: &[u8],
    dialect: &str,
    config: &ShellCheckConfig,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    if cancelled.load(std::sync::atomic::Ordering::Relaxed)
        || codeguard_runtime::sigint_cancellation_requested()
    {
        return unavailable("request_cancelled");
    }
    if Instant::now() >= deadline {
        return unavailable("request_deadline_exceeded");
    }
    let mut report = unavailable("shellcheck_tool_unavailable");
    let Ok(executable) = tool.canonicalize() else {
        return report;
    };
    let Ok(tool_bytes) = read_bounded_regular_file(&executable, 128 * 1024 * 1024) else {
        return report;
    };
    let sha = digest(&tool_bytes);
    drop(tool_bytes);
    report["tool_sha256"] = json!(sha);
    let Some(scratch) = DoctorScratch::create("shellcheck") else {
        return unavailable("shellcheck_scratch_unavailable");
    };
    let frozen = scratch.path().join("shellcheckrc");
    if let Some(bytes) = config.bytes() {
        if std::fs::write(&frozen, bytes).is_err() {
            return unavailable("shellcheck_config_snapshot_unavailable");
        }
    }
    let current = || {
        tool.canonicalize().ok().as_deref() == Some(executable.as_path())
            && read_bounded_regular_file(&executable, 128 * 1024 * 1024)
                .is_ok_and(|b| digest(&b) == sha)
            && read_bounded_regular_file(source_path, 1024 * 1024).is_ok_and(|b| b == source)
            && config.current(source_path)
            && config.bytes().is_none_or(|b| {
                read_bounded_regular_file(&frozen, 32 * 1024).is_ok_and(|actual| actual == b)
            })
    };
    let invoke = |args: Vec<OsString>, stdin| {
        run_process(
            &ProcessSpec {
                executable: executable.clone(),
                args,
                cwd: scratch.path().into(),
                env: BTreeMap::new(),
                stdin,
                deadline,
                output_limit_bytes: 128 * 1024,
            },
            cancelled,
        )
    };
    let version = invoke(vec![OsString::from("--version")], None);
    if Instant::now() >= deadline {
        return unavailable("request_deadline_exceeded");
    }
    if !current() {
        return unavailable("shellcheck_inputs_changed_during_check");
    }
    if version.termination == Termination::Cancelled {
        return unavailable("request_cancelled");
    }
    if version.termination!=Termination::Exited(0)||!version.stderr.is_empty()||version.stdout!=b"ShellCheck - shell script analysis tool\nversion: 0.11.0\nlicense: GNU General Public License, version 3\nwebsite: https://www.shellcheck.net\n" {
        report["reason"]=json!("shellcheck_version_unverified");return report;
    }
    report["version"] = json!("0.11.0");
    let mut args = vec![
        OsString::from("--format=json1"),
        OsString::from(format!("--shell={dialect}")),
    ];
    if config.bytes().is_some() {
        args.push(OsString::from("--rcfile"));
        args.push(frozen.as_os_str().to_os_string());
    } else {
        args.push(OsString::from("--norc"));
    }
    args.push(OsString::from("-"));
    let outcome = invoke(args, Some(source.to_vec()));
    if Instant::now() >= deadline {
        return unavailable("request_deadline_exceeded");
    }
    if !current() {
        return unavailable("shellcheck_inputs_changed_during_check");
    }
    report["configuration_mode"] = json!(if config.bytes().is_some() {
        "frozen_project_rc"
    } else {
        "builtin_without_global_rc"
    });
    let code = match outcome.termination {
        Termination::Exited(code) => code,
        Termination::Cancelled => return unavailable("request_cancelled"),
        _ => {
            report["reason"] = json!("shellcheck_execution_incomplete");
            return report;
        }
    };
    if !outcome.stderr.is_empty() {
        report["reason"] = json!("shellcheck_execution_stderr_unverified");
        return report;
    }
    let parsed = codeguard_adapters::parse_shellcheck_json1(&outcome.stdout, source, code);
    report["diagnostics"] = json!(parsed.diagnostics);
    report["environment_codes"] = json!(parsed.environment_codes);
    report["report_valid"] = json!(parsed.report_valid);
    report["status"] = json!(if !parsed.local_scan_complete {
        "incomplete"
    } else if parsed.diagnostics.is_empty() {
        "completed"
    } else {
        "diagnostics_observed"
    });
    report["reason"] = json!(parsed.reason.unwrap_or(if parsed.diagnostics.is_empty() {
        "shellcheck_no_diagnostics"
    } else {
        "shellcheck_diagnostics"
    }));
    report
}
/// 构造环境未完成反馈，不创建源码违规或猜测位置。
pub(crate) fn unavailable(reason: &str) -> Value {
    json!({"status":"incomplete","reason":reason,"version":null,"tool_sha256":null,"configuration_mode":"not_applied","report_valid":false,"diagnostics":[],"environment_codes":[]})
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
