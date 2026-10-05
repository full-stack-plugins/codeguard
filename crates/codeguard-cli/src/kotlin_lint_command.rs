//! Kotlin 单文件原生编译观察；不运行 Gradle、Maven、脚本或项目插件。
use crate::doctor_scratch::DoctorScratch;
use codeguard_runtime::{ProcessSpec, Termination, read_bounded_regular_file, run_process};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    path::PathBuf,
    process::ExitCode,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

/// 调用显式 Kotlin/JVM 工具检查冻结的单一 kt 文件，反馈仍不授予完整 lint 或交付许可。
/// 参数为源码路径及 --kotlinc-tool/--timeout/--format；返回码 3 表示完整项目义务未完成。
pub fn run(args: &[String]) -> ExitCode {
    let mut file = None;
    let mut tool = None;
    let mut timeout = 30_000;
    let mut json_output = false;
    let mut seen_timeout = false;
    let mut seen_format = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--kotlinc-tool" | "--timeout" | "--format" => {
                let key = &args[index];
                index += 1;
                let Some(value) = args.get(index) else {
                    return invalid("参数值缺失");
                };
                match key.as_str() {
                    "--kotlinc-tool" if tool.is_none() => {
                        let path = PathBuf::from(value);
                        if !path.is_absolute() {
                            return invalid("工具必须为绝对路径");
                        }
                        tool = Some(path);
                    }
                    "--timeout" if !seen_timeout => {
                        let Ok(ms) = crate::check_budget::parse_check_timeout(value) else {
                            return invalid("超时参数无效");
                        };
                        timeout = ms;
                        seen_timeout = true;
                    }
                    "--format" if !seen_format && matches!(value.as_str(), "json" | "human") => {
                        json_output = value == "json";
                        seen_format = true;
                    }
                    _ => return invalid("重复或不支持的参数"),
                }
            }
            "--format=json" if !seen_format => {
                json_output = true;
                seen_format = true;
            }
            "--format=human" if !seen_format => {
                seen_format = true;
            }
            value if !value.starts_with('-') && file.is_none() => file = Some(PathBuf::from(value)),
            _ => return invalid("重复或不支持的参数"),
        }
        index += 1;
    }
    let Some(file) = file else {
        return invalid("必须指定普通 kt 文件");
    };
    if file.extension().and_then(|s| s.to_str()) != Some("kt") {
        return invalid("仅支持普通 kt 文件，不执行 kts 脚本");
    }
    let selection = crate::kotlin_tool_selection::KotlinToolSelection::discover(tool);
    let tool = selection.tool();
    let deadline = Instant::now() + Duration::from_millis(timeout);
    let source = fs::symlink_metadata(&file)
        .ok()
        .filter(|m| m.file_type().is_file())
        .and_then(|_| read_bounded_regular_file(&file, 1024 * 1024).ok());
    let native = match source.as_ref().filter(|s| std::str::from_utf8(s).is_ok()) {
        Some(source) => match tool {
            Some(tool) => observe(tool, source, deadline),
            None => unavailable("kotlin_tool_not_found"),
        },
        None => unavailable("kotlin_source_unavailable_or_invalid"),
    };
    let mut syntax_precheck = if tool.is_none() {
        source
            .as_ref()
            .filter(|bytes| std::str::from_utf8(bytes).is_ok())
            .map(|bytes| candidate_precheck(bytes, deadline))
    } else {
        None
    };
    let input_stable = source.as_ref().is_some_and(|bytes| {
        read_bounded_regular_file(&file, 1024 * 1024).ok().as_ref() == Some(bytes)
    });
    let native = if source.is_some() && !input_stable {
        syntax_precheck = None;
        unavailable("kotlin_source_changed_during_check")
    } else {
        native
    };
    let native_confirmation_required = (native["status"] == "incomplete"
        && native["reason"] != "kotlin_tool_not_found")
        || (tool.is_none()
            && syntax_precheck.as_ref().is_some_and(|precheck| {
                precheck["reason"].is_string()
                    || precheck["precheck"]["truncated_files"]
                        .as_u64()
                        .unwrap_or(0)
                        > 0
                    || precheck["recoveries"]
                        .as_array()
                        .is_some_and(|rows| !rows.is_empty())
            }));
    let request_cancelled = codeguard_runtime::sigint_cancellation_requested()
        || native["reason"] == "request_cancelled";
    let report = json!({"schema_version":"0.1.0","report_type":"kotlin_lint_feedback","operation":"lint","language":"kotlin",
        "command_status":if request_cancelled {"cancelled"} else {"incomplete"},"exit_code":if request_cancelled {130} else {3},"source_path":file.to_str(),"scope":"single_frozen_kt_file","source_sha256":source.as_ref().map(|bytes| digest(bytes)),
        "input_stable":input_stable,"tool_selection":selection.report(),"native":native,"syntax_precheck":syntax_precheck,"setup":{"native_confirmation_required":native_confirmation_required,"automatic_installation":false,"full_project_checks_required":true},"coverage_proven":false,"authority":"local_unverified","delivery_decision":"not_evaluated",
        "next_actions":["核对已安装 Kotlin/JVM 2.4.10 工具；用 --kotlinc-tool 绝对路径原生复检，不自动安装",
        "按语法诊断核对当前源码；上下文诊断需完整项目编译确认，零诊断不证明完整 lint、注释、安全或交付通过"]});
    if json_output {
        println!("{report}");
    } else {
        println!(
            "Kotlin 原生单文件观察：{}；原因 {}；语法诊断 {} 项，上下文诊断 {} 项。完整项目未完成，交付未评估。",
            report["native"]["status"],
            report["native"]["reason"],
            report["native"]["diagnostics"]
                .as_array()
                .map_or(0, Vec::len),
            report["native"]["context_diagnostics"]
                .as_array()
                .map_or(0, Vec::len)
        );
        // 仅输出已验证的位置和规则，路径使用 JSON 转义避免控制字符污染终端。
        for category in ["diagnostics", "context_diagnostics"] {
            if let Some(rows) = report["native"][category].as_array() {
                for row in rows {
                    println!(
                        "{}:{}:{} {}（UTF-16 列 {}）",
                        report["source_path"],
                        row["line"],
                        row["column_byte"],
                        row["rule_id"],
                        row["column_utf16"]
                    );
                }
            }
        }
        if report["syntax_precheck"].is_object() {
            println!(
                "候选初检：{}；原因 {}；原生确认要求 {}；WASM 无完整语言资格。",
                report["syntax_precheck"]["precheck"],
                report["syntax_precheck"]["reason"],
                report["setup"]["native_confirmation_required"]
            );
        }
    }
    ExitCode::from(if request_cancelled { 130 } else { 3 })
}
fn invalid(reason: &str) -> ExitCode {
    eprintln!("{reason}");
    ExitCode::from(2)
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(crate) fn unavailable(reason: &str) -> Value {
    json!({"status":"incomplete","reason":reason,"version":null,"tool_sha256":null,"tool_identity_scope":"launcher_only","diagnostics":[],"context_diagnostics":[]})
}
pub(crate) fn observe(tool: &std::path::Path, source: &[u8], deadline: Instant) -> Value {
    observe_with_cancellation(tool, source, deadline, &AtomicBool::new(false))
}

/// 使用调用方取消令牌观察同一冻结输入；版本探测与源码检查共用令牌和截止时间。
pub(crate) fn observe_with_cancellation(
    tool: &std::path::Path,
    source: &[u8],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    let mut report = unavailable("kotlin_tool_unavailable_or_untrusted");
    let Ok(executable) = tool.canonicalize() else {
        return report;
    };
    let Ok(bytes) = read_bounded_regular_file(&executable, 64 * 1024 * 1024) else {
        return report;
    };
    let sha = digest(&bytes);
    report["tool_sha256"] = json!(sha);
    let Some(scratch) = DoctorScratch::create("kotlin-native") else {
        report["reason"] = json!("kotlin_private_directory_unavailable");
        return report;
    };
    let input = scratch.path().join("Sample.kt");
    if fs::write(&input, source).is_err() {
        report["reason"] = json!("kotlin_frozen_input_unavailable");
        return report;
    }
    let mut env = BTreeMap::from([
        (OsString::from("PATH"), OsString::from("/usr/bin:/bin")),
        (
            OsString::from("HOME"),
            scratch.path().as_os_str().to_owned(),
        ),
        (
            OsString::from("TMPDIR"),
            scratch.path().as_os_str().to_owned(),
        ),
    ]);
    if let Some(home) = std::env::var_os("JAVA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute() && p.join("bin/java").is_file())
    {
        env.insert(OsString::from("JAVA_HOME"), home.into_os_string());
    }
    let invoke = |args| {
        run_process(
            &ProcessSpec {
                executable: executable.clone(),
                args,
                cwd: scratch.path().to_path_buf(),
                env: env.clone(),
                stdin: None,
                deadline,
                output_limit_bytes: 64 * 1024,
            },
            cancelled,
        )
    };
    let version = invoke(vec![OsString::from("-version")]);
    if version.termination != Termination::Exited(0) {
        report["reason"] = json!(execution_reason(version.termination));
        return report;
    }
    let banner = std::str::from_utf8(&version.stderr).ok().map(str::trim);
    if !version.stdout.is_empty()
        || !banner.is_some_and(|s| {
            s.len() <= 1024
                && !s.contains('\n')
                && s.starts_with("info: kotlinc-jvm 2.4.10 (JRE ")
                && s.ends_with(')')
        })
    {
        report["reason"] = json!("kotlin_version_unverified_or_unsupported");
        return report;
    }
    report["version"] = json!("kotlinc-jvm 2.4.10");
    // 版本调用可以写私有工作目录；冻结源码在编译前后都必须保持同字节。
    let input_current = || {
        read_bounded_regular_file(&input, 1024 * 1024).is_ok_and(|bytes| bytes.as_slice() == source)
    };
    if !launcher_current(tool, &executable, &sha) || !input_current() {
        report["reason"] = json!("kotlin_input_or_launcher_changed_during_check");
        return report;
    }
    let outcome = invoke(vec![
        input.as_os_str().to_owned(),
        OsString::from("-nowarn"),
        OsString::from("-Xrender-internal-diagnostic-names"),
        OsString::from("-d"),
        scratch.path().join("classes").into_os_string(),
    ]);
    if !launcher_current(tool, &executable, &sha) || !input_current() {
        report["reason"] = json!("kotlin_input_or_launcher_changed_during_check");
        return report;
    }
    if !matches!(outcome.termination, Termination::Exited(0 | 1)) {
        report["reason"] = json!(execution_reason(outcome.termination));
        return report;
    }
    let parsed = input.to_str().and_then(|path| {
        codeguard_adapters::parse_kotlin_diagnostics(
            &outcome.stderr,
            path,
            std::str::from_utf8(source).ok()?,
        )
        .or_else(|| {
            codeguard_adapters::parse_kotlin_diagnostics(
                &outcome.stderr,
                "Sample.kt",
                std::str::from_utf8(source).ok()?,
            )
        })
    });
    let Some(parsed) = parsed else {
        report["reason"] = json!("kotlin_native_report_invalid");
        return report;
    };
    let clean = parsed.syntax.is_empty() && parsed.context.is_empty();
    if !outcome.stdout.is_empty() || (outcome.termination == Termination::Exited(0)) != clean {
        report["reason"] = json!("kotlin_exit_diagnostics_inconsistent");
        return report;
    }
    report["status"] = json!(if !parsed.context.is_empty() {
        "incomplete"
    } else if clean {
        "completed"
    } else {
        "diagnostics_observed"
    });
    report["reason"] = json!(if !parsed.context.is_empty() {
        "kotlin_project_context_unresolved"
    } else if clean {
        "kotlin_native_single_file_no_diagnostics"
    } else {
        "kotlin_native_syntax_diagnostics"
    });
    report["diagnostics"] = json!(parsed.syntax);
    report["context_diagnostics"] = json!(parsed.context);
    report
}
fn execution_reason(termination: Termination) -> &'static str {
    match termination {
        Termination::TimedOut | Termination::DeadlineBeforeStart => "kotlin_native_timeout",
        Termination::OutputLimit => "kotlin_native_output_limit",
        Termination::Cancelled => "request_cancelled",
        _ => "kotlin_native_execution_failed",
    }
}

#[cfg(feature = "wasm-precheck")]
fn candidate_precheck(source: &[u8], deadline: Instant) -> Value {
    let result = std::env::current_exe().ok().and_then(|exe| {
        crate::syntax_worker_runner::run_syntax_worker_candidate(
            &exe,
            "kotlin",
            "Sample.kt",
            source,
            deadline,
            &AtomicBool::new(false),
        )
        .ok()
    });
    match result {
        Some(observation) => {
            json!({"status":"incomplete","grammar_qualified":false,"grammar_sha256":observation.grammar_sha256,"precheck":observation.precheck,"recoveries":observation.recoveries})
        }
        None => {
            json!({"status":"incomplete","reason":"kotlin_syntax_worker_incomplete","grammar_qualified":false})
        }
    }
}
#[cfg(not(feature = "wasm-precheck"))]
fn candidate_precheck(_source: &[u8], _deadline: Instant) -> Value {
    json!({"status":"incomplete","reason":"wasm_feature_not_built","grammar_qualified":false})
}

fn launcher_current(selected: &std::path::Path, resolved: &std::path::Path, sha: &str) -> bool {
    selected.canonicalize().ok().as_deref() == Some(resolved)
        && read_bounded_regular_file(resolved, 64 * 1024 * 1024)
            .ok()
            .map(|bytes| digest(&bytes))
            .as_deref()
            == Some(sha)
}
