//! Claude Code 文件工具事件到 Rust 快检的受限转换；只回传脱敏对话摘要。

use crate::hook_execute_command::execute_host_input;
use codeguard_core::{HookEvent, HookTriggerInput, HookWriteOutcome};
use serde_json::{Value, json};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const MAX_HOST_INPUT_BYTES: u64 = 1024 * 1024;
const MAX_CONTEXT_CHARS: usize = 1200;

/// 从 Claude Code stdin 读取成功的文件工具事件并注入局部检查摘要。
/// 参数包含项目根和原 `hook execute` 的显式选项；返回宿主退出码而非质量结论。
pub fn run(args: &[String]) -> ExitCode {
    let [event, rest @ ..] = args else {
        eprintln!("需要 Claude Code 事件与项目路径");
        return ExitCode::from(2);
    };
    if event != "post-tool-use" || rest.first().is_none_or(|value| value.starts_with('-')) {
        eprintln!("目前仅支持 post-tool-use PATH --timeout DURATION --format=json");
        return ExitCode::from(2);
    }
    let root = match Path::new(&rest[0]).canonicalize() {
        Ok(root) if root.is_dir() => root,
        _ => {
            print_context("范围未确定：项目根不可读取；本次源码检查未运行，交付未评估。");
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
        print_context("范围未确定：宿主事件缺失或超出输入预算；本次源码检查未运行，交付未评估。");
        return ExitCode::SUCCESS;
    }
    let host = match codeguard_adapters::parse_unique_json(&raw) {
        Ok(host) => host,
        Err(_) => {
            print_context("范围未确定：宿主事件格式无效；本次源码检查未运行，交付未评估。");
            return ExitCode::SUCCESS;
        }
    };
    let path = match selected_file(&root, &host) {
        Some(path) => path,
        None => {
            print_context(
                "范围未确定：成功编辑事件没有可核对的项目内普通文件；本次源码检查未运行，交付未评估。",
            );
            return ExitCode::SUCCESS;
        }
    };
    let input = HookTriggerInput {
        event: HookEvent::FileChanged,
        changed_paths: vec![path.clone()],
        task_id: None,
        write_outcome: HookWriteOutcome::Confirmed,
        host_claims_blocking: false,
    };
    match execute_host_input(rest, &input) {
        Ok((report, _)) => print_context(&summarize(&path, &report)),
        Err(_) => {
            print_context("CodeGuard：宿主事件参数或路由无效；本次源码检查未运行，交付未评估。")
        }
    }
    ExitCode::SUCCESS
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
    let cwd = Path::new(host["cwd"].as_str()?);
    if !cwd.is_absolute() || !cwd.canonicalize().ok()?.starts_with(root) {
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

fn print_context(context: &str) {
    println!(
        "{}",
        json!({
            "hookSpecificOutput": {
                "hookEventName":"PostToolUse",
                "additionalContext":context
            },
            "systemMessage":"CodeGuard 局部检查反馈"
        })
    );
}
