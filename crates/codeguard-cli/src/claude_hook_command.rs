//! Claude Code 生命周期事件到 Rust 路由的受限转换；只回传脱敏对话摘要。

use crate::hook_execute_command::execute_host_input;
use codeguard_core::{HookEvent, HookTriggerInput, HookWriteOutcome};
use serde_json::{Value, json};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const MAX_HOST_INPUT_BYTES: u64 = 1024 * 1024;
const MAX_CONTEXT_CHARS: usize = 1200;

/// 从 Claude Code stdin 读取生命周期事件并注入受限检查摘要。
/// 参数包含项目根和原 `hook execute` 的显式选项；返回宿主退出码而非质量结论。
pub fn run(args: &[String]) -> ExitCode {
    let [event, rest @ ..] = args else {
        eprintln!("需要 Claude Code 事件与项目路径");
        return ExitCode::from(2);
    };
    let Some((host_event, route_event)) = mapped_event(event) else {
        eprintln!(
            "支持 session-start、user-prompt-submit、post-tool-use、post-tool-use-failure、stop"
        );
        return ExitCode::from(2);
    };
    if rest.first().is_none_or(|value| value.starts_with('-')) {
        eprintln!("需要 PATH --timeout DURATION --format=json");
        return ExitCode::from(2);
    }
    let root = match Path::new(&rest[0]).canonicalize() {
        Ok(root) if root.is_dir() => root,
        _ => {
            print_context(
                host_event,
                "范围未确定：项目根不可读取；本次源码检查未运行，交付未评估。",
            );
            return ExitCode::SUCCESS;
        }
    };
    let mut raw = Vec::new();
    if std::io::stdin()
        .lock()
        .take(MAX_HOST_INPUT_BYTES + 1)
        .read_to_end(&mut raw)
        .is_err()
        || raw.is_empty()
        || raw.len() as u64 > MAX_HOST_INPUT_BYTES
    {
        print_context(
            host_event,
            "范围未确定：宿主事件缺失或超出输入预算；本次源码检查未运行，交付未评估。",
        );
        return ExitCode::SUCCESS;
    }
    let host = match codeguard_adapters::parse_unique_json(&raw) {
        Ok(host) => host,
        Err(_) => {
            print_context(
                host_event,
                "范围未确定：宿主事件格式无效；本次源码检查未运行，交付未评估。",
            );
            return ExitCode::SUCCESS;
        }
    };
    if host["hook_event_name"] != host_event || !valid_cwd(&root, &host) {
        print_context(
            host_event,
            "范围未确定：宿主事件或工作目录不匹配；本次源码检查未运行，交付未评估。",
        );
        return ExitCode::SUCCESS;
    }
    if route_event == HookEvent::SessionStart
        && !matches!(
            host["source"].as_str(),
            Some("startup" | "resume" | "clear" | "compact" | "fork")
        )
    {
        print_context(
            host_event,
            "范围未确定：会话来源无效；只读发现未运行，交付未评估。",
        );
        return ExitCode::SUCCESS;
    }
    if route_event == HookEvent::PromptSubmitted && !host["prompt"].is_string() {
        print_context(
            host_event,
            "CodeGuard：用户提示事件无效；源码检查未运行，交付未评估。",
        );
        return ExitCode::SUCCESS;
    }
    if route_event == HookEvent::FileChanged
        && host_event == "PostToolUseFailure"
        && (!matches!(
            host["tool_name"].as_str(),
            Some("Write" | "Edit" | "MultiEdit")
        ) || !host["tool_input"].is_object()
            || !host["error"].is_string())
    {
        print_context(
            host_event,
            "范围未确定：失败编辑事件无效；本次源码检查未运行，交付未评估。",
        );
        return ExitCode::SUCCESS;
    }
    let path = if host_event == "PostToolUse" {
        match selected_file(&root, &host) {
            Some(path) => Some(path),
            None => {
                print_context(
                    host_event,
                    "范围未确定：成功编辑事件没有可核对的项目内普通文件；本次源码检查未运行，交付未评估。",
                );
                return ExitCode::SUCCESS;
            }
        }
    } else {
        None
    };
    if route_event == HookEvent::Stop && !host["stop_hook_active"].is_boolean() {
        print_context(
            host_event,
            "范围未确定：Stop 状态缺失；只读指引未运行，交付未评估。",
        );
        return ExitCode::SUCCESS;
    }
    let input = HookTriggerInput {
        event: route_event,
        changed_paths: path.iter().cloned().collect(),
        task_id: None,
        write_outcome: if route_event != HookEvent::FileChanged {
            HookWriteOutcome::Unknown
        } else if host_event == "PostToolUseFailure" {
            HookWriteOutcome::Failed
        } else {
            HookWriteOutcome::Confirmed
        },
        host_claims_blocking: false,
    };
    match execute_host_input(rest, &input) {
        Ok((report, _)) => {
            let context = match route_event {
                HookEvent::SessionStart => summarize_discovery(&report),
                HookEvent::PromptSubmitted => summarize_intent_guidance(&report),
                HookEvent::Stop => summarize_stop(&report, host["stop_hook_active"] == true),
                HookEvent::FileChanged if host_event == "PostToolUseFailure" => {
                    "CodeGuard：写入失败，本次源码检查未运行；请先处理工具错误，交付未评估。"
                        .to_owned()
                }
                HookEvent::FileChanged => {
                    summarize(path.as_deref().unwrap_or("<changed-file>"), &report)
                }
                _ => "CodeGuard：宿主事件未运行，交付未评估。".to_owned(),
            };
            if route_event == HookEvent::Stop {
                print_stop_context(
                    &context,
                    safe_task_id(&report).is_some() && host["stop_hook_active"] == false,
                );
            } else {
                print_context(host_event, &context);
            }
        }
        Err(_) => print_context(
            host_event,
            "CodeGuard：宿主事件参数或路由无效；本次源码检查未运行，交付未评估。",
        ),
    }
    ExitCode::SUCCESS
}

fn mapped_event(event: &str) -> Option<(&'static str, HookEvent)> {
    match event {
        "session-start" => Some(("SessionStart", HookEvent::SessionStart)),
        "user-prompt-submit" => Some(("UserPromptSubmit", HookEvent::PromptSubmitted)),
        "post-tool-use" => Some(("PostToolUse", HookEvent::FileChanged)),
        "post-tool-use-failure" => Some(("PostToolUseFailure", HookEvent::FileChanged)),
        "stop" => Some(("Stop", HookEvent::Stop)),
        _ => None,
    }
}

fn valid_cwd(root: &Path, host: &Value) -> bool {
    host["cwd"].as_str().is_some_and(|cwd| {
        let cwd = Path::new(cwd);
        cwd.is_absolute() && cwd.canonicalize().is_ok_and(|cwd| cwd.starts_with(root))
    })
}

fn selected_file(root: &Path, host: &Value) -> Option<String> {
    if host["hook_event_name"] != "PostToolUse"
        || !matches!(
            host["tool_name"].as_str(),
            Some("Write" | "Edit" | "MultiEdit")
        )
    {
        return None;
    }
    let path = PathBuf::from(host["tool_input"]["file_path"].as_str()?);
    if !host["tool_response"].is_object() {
        return None;
    }
    if !path.is_absolute() || !fs::symlink_metadata(&path).ok()?.file_type().is_file() {
        return None;
    }
    if let Some(response_path) = host["tool_response"]["filePath"].as_str() {
        if response_path != path.to_str()? {
            return None;
        }
    }
    let canonical = path.canonicalize().ok()?;
    let relative = canonical.strip_prefix(root).ok()?.to_str()?;
    if relative.is_empty() || relative.contains('\\') || relative.chars().any(char::is_control) {
        return None;
    }
    Some(relative.to_owned())
}

fn summarize_discovery(report: &Value) -> String {
    if report["execution"] != "read_only_discovery" {
        return "CodeGuard：会话启动只读发现未运行；源码检查未运行，交付未评估。".into();
    }
    let languages = report["local_feedback"]["languages"]
        .as_array()
        .into_iter()
        .flat_map(|items| items.iter())
        .filter_map(Value::as_str)
        .filter(|language| {
            language.len() <= 24
                && language
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
        })
        .take(8)
        .collect::<Vec<_>>();
    let languages = if languages.is_empty() {
        "未识别".to_owned()
    } else {
        languages.join(", ")
    };
    let count = report["local_feedback"]["checker_count"]
        .as_u64()
        .unwrap_or(0);
    let complete = report["local_feedback"]["observation_complete"] == true;
    let status = if complete { "完成" } else { "不完整" };
    format!(
        "CodeGuard：会话启动只读发现{status}；语言 {languages}，检查器配置观察 {count} 项。源码检查未运行，交付未评估。"
    )
}

fn summarize_intent_guidance(report: &Value) -> String {
    if report["execution"] != "read_only_intent_guidance"
        || report["local_feedback"]["report_type"] != "hook_intent_guidance"
    {
        return "CodeGuard：用户提示指引未运行；源码检查未运行，交付未评估。".into();
    }
    "CodeGuard：本事件只提供检查时机提示；代码变更后执行局部检查，真实提交或 CI 时执行完整门禁。源码检查未运行，交付未评估。".into()
}

fn safe_task_id(report: &Value) -> Option<&str> {
    report["local_feedback"]["task_id"].as_str().filter(|task| {
        task.len() <= 64
            && task.starts_with("CG-")
            && task[3..]
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}

fn summarize_stop(report: &Value, already_continuing: bool) -> String {
    if report["execution"] != "read_only_guidance" {
        return "CodeGuard：结束时只读指引未运行；请显式执行 codeguard next . --format=json，交付未评估。".into();
    }
    match safe_task_id(report) {
        Some(task) if !already_continuing => format!(
            "CodeGuard：待处理任务 {task}；请读取 codeguard next . --format=json 并按原检查器复检。结束指引不运行源码检查，交付未评估。"
        ),
        Some(task) => format!(
            "CodeGuard：待处理任务 {task}；本轮已在 Stop 后继续，避免重复唤醒。结束指引不运行源码检查，交付未评估。"
        ),
        None => "CodeGuard：没有可展示的待处理任务不代表检查通过；交付前仍需新鲜完整检查。结束指引不运行源码检查，交付未评估。".into(),
    }
}

fn summarize(path: &str, report: &Value) -> String {
    let label = if path.len() <= 128
        && path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._/-".contains(&byte))
    {
        path
    } else {
        "<changed-file>"
    };
    if report["execution"] != "local_observation" {
        let reason = report["reason"]
            .as_str()
            .unwrap_or("local_check_unavailable");
        return format!(
            "CodeGuard：{label} 的局部检查未运行（{reason}）；请按项目原生检查要求复检，交付未评估。"
        );
    }
    let files = report["local_feedback"]["files"].as_array();
    let incomplete = report["local_feedback"]["local_scan_complete"] != true;
    let mut rules = Vec::new();
    if let Some(files) = files {
        for finding in files.iter().flat_map(|file| {
            file["findings"]
                .as_array()
                .into_iter()
                .flat_map(|findings| findings.iter())
        }) {
            let Some(rule) = finding["rule_id"].as_str() else {
                continue;
            };
            if rule.len() <= 32
                && rule
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
            {
                rules.push(rule.to_owned());
                if rules.len() == 3 {
                    break;
                }
            }
        }
    }
    let count = files.map_or(0, |files| {
        files
            .iter()
            .map(|file| file["findings"].as_array().map_or(0, Vec::len))
            .sum::<usize>()
    });
    let status = if incomplete {
        "局部检查未完成"
    } else {
        "局部检查已运行"
    };
    let detail = if count == 0 {
        "未取得可展示的原生诊断".to_owned()
    } else {
        format!("原生诊断 {count} 项；规则 {}", rules.join(", "))
    };
    let summary = format!(
        "CodeGuard：{label} {status}，{detail}。请核对原生报告并在修复后复检；完整项目与交付未评估。"
    );
    summary.chars().take(MAX_CONTEXT_CHARS).collect()
}

fn print_context(event: &str, context: &str) {
    if event == "Stop" {
        println!("{}", json!({"systemMessage":context}));
    } else {
        let label = match event {
            "SessionStart" => "CodeGuard 项目发现反馈",
            "UserPromptSubmit" => "CodeGuard 检查时机提示",
            "PostToolUseFailure" => "CodeGuard 写入失败反馈",
            _ => "CodeGuard 局部检查反馈",
        };
        println!(
            "{}",
            json!({
                "hookSpecificOutput": {"hookEventName":event,"additionalContext":context},
                "systemMessage":label
            })
        );
    }
}

fn print_stop_context(context: &str, continue_once: bool) {
    if continue_once {
        println!(
            "{}",
            json!({
                "hookSpecificOutput":{"hookEventName":"Stop","additionalContext":context},
                "systemMessage":"CodeGuard 待处理任务指引"
            })
        );
    } else {
        println!("{}", json!({"systemMessage":context}));
    }
}
