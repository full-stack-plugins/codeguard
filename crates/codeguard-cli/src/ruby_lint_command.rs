//! Ruby 单文件原生优先入口；只做固定 Ruby 2.6.10p210 的 ruby -c，不执行源码、gems 或项目插件。
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::atomic::AtomicBool;
use std::{
    path::{Path, PathBuf},
    process::ExitCode,
    time::{Duration, Instant},
};

/// 检查普通 Ruby 文件；参数含显式语法工具、共享超时和反馈格式，返回码保留项目未完成状态。
pub fn run(args: &[String]) -> ExitCode {
    let mut file = None;
    let mut tool = None;
    let mut timeout = 30_000;
    let mut output_json = false;
    let mut seen_timeout = false;
    let mut seen_format = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--ruby-tool" | "--timeout" | "--format" => {
                let key = &args[index];
                index += 1;
                let Some(value) = args.get(index) else {
                    return invalid("参数值缺失");
                };
                match key.as_str() {
                    "--ruby-tool" if tool.is_none() => {
                        let path = PathBuf::from(value);
                        if !path.is_absolute() {
                            return invalid("工具必须为绝对路径");
                        };
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
                        output_json = value == "json";
                        seen_format = true;
                    }
                    _ => return invalid("重复或不支持的参数"),
                }
            }
            "--format=json" if !seen_format => {
                output_json = true;
                seen_format = true;
            }
            "--format=human" if !seen_format => {
                seen_format = true;
            }
            value if !value.starts_with('-') && file.is_none() => {
                file = Some(PathBuf::from(value));
            }
            _ => return invalid("重复或不支持的参数"),
        }
        index += 1;
    }
    let Some(file) = file else {
        return invalid("必须指定普通 Ruby 文件");
    };
    if file.extension().and_then(|s| s.to_str()) != Some("rb") {
        return invalid("仅支持 Ruby 文件");
    }
    let selection = crate::ruby_tool_selection::RubyToolSelection::discover(tool);
    let selected = selection.tool();
    // 在原生调用前冻结入口目标，避免符号链接重定向后保留旧诊断。
    let resolved = selected.and_then(|p| p.canonicalize().ok());
    let tool_sha = resolved
        .as_ref()
        .and_then(|p| read_bounded_regular_file(p, 64 * 1024 * 1024).ok())
        .map(|b| digest(&b));
    let deadline = Instant::now() + Duration::from_millis(timeout);
    let source = read_bounded_regular_file(&file, 1024 * 1024).ok();
    let valid_source = source.as_ref().filter(|b| std::str::from_utf8(b).is_ok());
    let mut native = match (valid_source, selected) {
        (Some(bytes), Some(tool)) => crate::ruby_project_version::source_root(&file)
            .map(|root| {
                crate::ruby_project_version::observe(
                    &root,
                    &if file.is_absolute() {
                        file.clone()
                    } else {
                        std::env::current_dir().unwrap_or_default().join(&file)
                    },
                    tool,
                    bytes,
                    deadline,
                    &AtomicBool::new(false),
                )
            })
            .unwrap_or_else(|| unavailable("ruby_project_version_unresolved")),
        (Some(_), None) => unavailable("ruby_tool_not_found"),
        _ => unavailable("ruby_source_unavailable_or_invalid"),
    };
    let mut syntax_precheck = if selected.is_none() {
        valid_source.map(|b| candidate_precheck(b, deadline))
    } else {
        None
    };
    let input_stable = source.as_ref().is_some_and(|bytes| {
        read_bounded_regular_file(&file, 1024 * 1024).ok().as_ref() == Some(bytes)
    });
    if source.is_some() && !input_stable {
        native = unavailable("ruby_source_changed_during_check");
        syntax_precheck = None;
    } else if selected.is_some()
        && native["tool_sha256"].is_string()
        && !launcher_current(
            selected.unwrap(),
            resolved.as_deref(),
            tool_sha.as_deref(),
            &native,
        )
    {
        native = unavailable("ruby_selected_tool_changed_during_check");
    }
    let confirmation_required = if selected.is_some() {
        native["status"] == "incomplete"
    } else {
        valid_source.is_none()
            || syntax_precheck.as_ref().is_none_or(|r| {
                r["reason"].is_string()
                    || r["precheck"]["truncated_files"].as_u64().unwrap_or(0) > 0
                    || r["recoveries"]
                        .as_array()
                        .is_some_and(|rows| !rows.is_empty())
            })
    };
    let cancelled = codeguard_runtime::sigint_cancellation_requested()
        || native["reason"] == "request_cancelled";
    let mut report = json!({"schema_version":"0.1.0","report_type":"ruby_lint_feedback","operation":"lint","language":"ruby",
        "command_status":if cancelled {"cancelled"} else {"incomplete"},"exit_code":if cancelled {130} else {3},
        "source_path":file,"scope":"single_frozen_ruby_file","source_sha256":source.as_ref().map(|b|digest(b)),"input_stable":input_stable,
        "tool_selection":selection.report(),"native_identity_scope":"interpreter_entry_only","native":native,"syntax_precheck":syntax_precheck,
        "setup":{"native_confirmation_required":confirmation_required,"automatic_installation":false,"full_project_checks_required":true},
        "project_version_compatibility":"unverified","coverage_proven":false,"authority":"local_unverified","delivery_decision":"not_evaluated",
        "next_actions":["核对 Ruby 2.6.10p210 工具与当前诊断；使用 --ruby-tool 绝对路径复检，不自动安装",
        "ruby -c 仅做语法确认；RuboCop、注释、类型、依赖、安全及完整构建仍须检查"]});
    if input_stable {
        crate::ruby_lint_workbench::connect(&file, &mut report, deadline);
    }
    if output_json {
        println!("{report}")
    } else {
        println!(
            "Ruby 原生单文件 parse：{}；原因 {}；完整项目检查未完成，交付未评估。",
            report["native"]["status"], report["native"]["reason"]
        );
        for row in report["native"]["diagnostics"]
            .as_array()
            .into_iter()
            .flatten()
        {
            println!(
                "{}:{} {}（原生工具仅提供行号）",
                report["source_path"], row["line"], row["rule_id"]
            );
        }
        if report["syntax_precheck"].is_object() {
            println!(
                "候选初检：{}；原生确认要求 {}；完整语言资格未验收。",
                report["syntax_precheck"], report["setup"]["native_confirmation_required"]
            );
        }
        if report["task_status"].is_string() {
            println!(
                "修复任务 {}；同步状态 {}；原因 {}；使用 codeguard next 获取原工具复检步骤",
                report["task_id"], report["task_status"], report["task_sync_reason"]
            );
        }
        println!("下一步：{}", report["next_actions"]);
    }
    ExitCode::from(if cancelled { 130 } else { 3 })
}
fn invalid(reason: &str) -> ExitCode {
    eprintln!("{reason}");
    ExitCode::from(2)
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(crate) fn unavailable(reason: &str) -> Value {
    json!({"status":"incomplete","reason":reason,"version":null,"tool_sha256":null,"diagnostics":[]})
}
fn launcher_current(
    selected: &Path,
    resolved: Option<&Path>,
    sha: Option<&str>,
    native: &Value,
) -> bool {
    let Some(resolved) = resolved else {
        return false;
    };
    selected.canonicalize().ok().as_deref() == Some(resolved)
        && native["tool_sha256"].as_str() == sha
        && read_bounded_regular_file(resolved, 64 * 1024 * 1024)
            .ok()
            .map(|b| digest(&b))
            .as_deref()
            == sha
}
#[cfg(feature = "wasm-precheck")]
fn candidate_precheck(source: &[u8], deadline: Instant) -> Value {
    let result = std::env::current_exe().ok().and_then(|exe| {
        crate::syntax_worker_runner::run_syntax_worker_candidate(
            &exe,
            "ruby",
            "Sample.rb",
            source,
            deadline,
            &AtomicBool::new(false),
        )
        .ok()
    });
    match result {
        Some(o) => {
            json!({"status":"incomplete","grammar_qualified":false,"grammar_sha256":o.grammar_sha256,"precheck":o.precheck,"recoveries":o.recoveries})
        }
        None => {
            json!({"status":"incomplete","reason":"ruby_syntax_worker_incomplete","grammar_qualified":false})
        }
    }
}
#[cfg(not(feature = "wasm-precheck"))]
fn candidate_precheck(_source: &[u8], _deadline: Instant) -> Value {
    json!({"status":"incomplete","reason":"wasm_feature_not_built","grammar_qualified":false})
}
