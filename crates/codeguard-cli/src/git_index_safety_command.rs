//! `gate pre-commit` 的真实 index 路径安全反馈；完整门禁尚未实现。

use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde_json::json;

use crate::git_index_safety::observe_index_safety;

struct Arguments {
    root: PathBuf,
    git_tool: Option<PathBuf>,
    json: bool,
}

/// 观察真实 index 路径并反馈违规；当前必定返回 incomplete，不签发 allow。
pub fn run(args: &[String]) -> ExitCode {
    let parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let git_tool = parsed.git_tool.or_else(resolve_git);
    let alternate_index = env::var_os("GIT_INDEX_FILE").map(PathBuf::from);
    let root = parsed.root.canonicalize();
    let observation = match (root, git_tool) {
        (Ok(root), Some(tool)) => {
            let index = alternate_index.as_ref().map(|index| {
                if index.is_absolute() {
                    index.clone()
                } else {
                    root.join(index)
                }
            });
            observe_index_safety(&root, &tool, index.as_deref())
        }
        _ => Err("项目或 Git 工具不可读取".into()),
    };
    let feedback = match observation {
        Ok(result) => {
            let object_status = if result.objects_verified {
                "verified"
            } else if result.object_verification_reason.is_some() {
                "failed"
            } else {
                "unresolved"
            };
            let mut incomplete_reasons = vec![
                "full_quality_obligations_not_run",
                "git_tool_identity_unverified",
            ];
            if !result.objects_verified {
                incomplete_reasons.push("index_objects_unresolved");
            }
            json!({
                "schema_version":"0.2.0",
                "report_type":"git_index_safety_preview",
                "operation":"gate_pre_commit",
                "command_status":"incomplete",
                "exit_code":3,
                "delivery_decision":"not_evaluated",
                "index_observation":"complete",
                "index_object_format":result.object_format,
                "index_listing_sha256":result.listing_sha256,
                "staged_entry_count":result.entries.len(),
                "object_status":object_status,
                "verified_object_count":result.object_evidence.len(),
                "unresolved_object_count":result.unresolved_object_paths.len(),
                "object_verification_reason":result.object_verification_reason,
                "violations":result.violations.iter().map(|item| json!({"path":item.path,"rule_id":item.rule_id})).collect::<Vec<_>>(),
                "tool_approval":"unverified",
                "incomplete_reasons":incomplete_reasons,
                "next_actions":["remove_sensitive_staged_paths", "run_full_codeguard_gate_when_available"]
            })
        }
        Err(_) => json!({
            "schema_version":"0.2.0",
            "report_type":"git_index_safety_preview",
            "operation":"gate_pre_commit",
            "command_status":"incomplete",
            "exit_code":3,
            "delivery_decision":"not_evaluated",
            "index_observation":"incomplete",
            "index_object_format":null,
            "index_listing_sha256":null,
            "staged_entry_count":null,
            "object_status":"not_observed",
            "verified_object_count":null,
            "unresolved_object_count":null,
            "object_verification_reason":null,
            "violations":[],
            "tool_approval":"unverified",
            "incomplete_reasons":["git_index_observation_failed", "full_quality_obligations_not_run"],
            "next_actions":["check_git_repository_and_index", "run_full_codeguard_gate_when_available"]
        }),
    };
    if parsed.json {
        println!("{feedback}");
    } else {
        println!(
            "Git index 路径观察：{}",
            feedback["index_observation"]
                .as_str()
                .unwrap_or("incomplete")
        );
        for violation in feedback["violations"].as_array().expect("固定数组") {
            let path = serde_json::to_string(&violation["path"]).expect("固定字符串");
            println!(
                "拟提交路径违规：{path} ({})",
                violation["rule_id"].as_str().unwrap_or("unknown")
            );
        }
        println!(
            "暂存对象核验：{}；完整质量门禁：未完成，Git 工具身份尚未核验。",
            feedback["object_status"].as_str().unwrap_or("not_observed")
        );
    }
    ExitCode::from(3)
}

fn parse_args(args: &[String]) -> Result<Arguments, String> {
    if args.first().map(String::as_str) != Some("pre-commit") {
        return Err("当前 gate 仅提供 pre-commit 路径安全预览".into());
    }
    let mut root = None;
    let mut git_tool = None;
    let mut json = false;
    let mut index = 1;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--format" || arg == "--git-tool" {
            index += 1;
            let value = args.get(index).ok_or_else(|| format!("{arg} 缺少值"))?;
            if arg == "--format" {
                json = parse_format(value)?;
            } else if git_tool.replace(PathBuf::from(value)).is_some() {
                return Err("--git-tool 重复".into());
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
    if git_tool.as_ref().is_some_and(|tool| !tool.is_absolute()) {
        return Err("--git-tool 必须是绝对路径".into());
    }
    Ok(Arguments {
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        git_tool,
        json,
    })
}

fn parse_format(value: &str) -> Result<bool, String> {
    match value {
        "json" => Ok(true),
        "human" => Ok(false),
        _ => Err(format!("不支持的格式：{value}")),
    }
}

fn resolve_git() -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join("git"))
        .find_map(|candidate| {
            candidate
                .canonicalize()
                .ok()
                .filter(|path| executable(path))
        })
}

fn executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}
