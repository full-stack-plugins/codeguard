//! 从受限本地事实构建 status 与 task show 只读视图；不运行诊断或签发门禁。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

use crate::next_command::{read_local_brief, read_task_brief};
use crate::workspace_refresh::read_workspace_baseline;

struct Arguments {
    root: PathBuf,
    json: bool,
}

/// 返回工作区所有当前开放任务的只读概况；退出零仅说明查询完成。
pub fn run_status(args: &[String]) -> ExitCode {
    let parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(reason) => return output_error("status", true, "invalid_request", 2, &reason),
    };
    let root = match canonical_root(&parsed.root) {
        Ok(root) => root,
        Err(reason) => return output_error("status", parsed.json, "incomplete", 3, reason),
    };
    let next = match read_local_brief(&root) {
        Ok(next) => next,
        Err(reason) => return output_error("status", parsed.json, "incomplete", 3, reason),
    };
    let baseline = match read_workspace_baseline(&root) {
        Ok(value) => value,
        Err(_) => return output_error("status", parsed.json, "incomplete", 3, "workspace_invalid"),
    };
    let workspace_id = baseline
        .as_ref()
        .and_then(|baseline| baseline.workspace_id());
    let initialized = workspace_id.is_some();
    let mut tasks = Vec::new();
    let mut findings = 0_u64;
    let mut blockers = 0_u64;
    if initialized {
        let directory = root.join(".codeguard/findings");
        if !real_directory(&directory) {
            return output_error(
                "status",
                parsed.json,
                "incomplete",
                3,
                "findings_unavailable",
            );
        }
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(_) => {
                return output_error(
                    "status",
                    parsed.json,
                    "incomplete",
                    3,
                    "findings_unreadable",
                );
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    return output_error(
                        "status",
                        parsed.json,
                        "incomplete",
                        3,
                        "findings_unreadable",
                    );
                }
            };
            let Some(id) = entry.file_name().to_str().map(str::to_owned) else {
                return output_error("status", parsed.json, "incomplete", 3, "finding_id_invalid");
            };
            let brief = match read_task_brief(&root, &id) {
                Ok(brief) => brief,
                Err(reason) => return output_error("status", parsed.json, "incomplete", 3, reason),
            };
            if brief["kind"] == "finding" {
                findings += 1;
            } else if brief["kind"] == "blocker" {
                blockers += 1;
            } else {
                return output_error("status", parsed.json, "incomplete", 3, "task_kind_invalid");
            }
            tasks.push(json!({
                "task_id":id,"kind":brief["kind"],"checker_id":brief["checker_id"],
                "scope":brief["scope"],"disposition":brief["disposition"],
                "reason_code":brief["reason_code"],"verification_observation":brief["verification_observation"],
                "verification_reason":brief["verification_reason"],
                "verification_invalidated_reason":brief["verification_invalidated_reason"],
                "state":"open","authority":"local_unverified"
            }));
        }
    }
    tasks.sort_by(|left, right| left["task_id"].as_str().cmp(&right["task_id"].as_str()));
    let report = json!({
        "schema_version":"0.1.0","report_type":"workspace_status_preview",
        "operation":"status","request_id":request_id(),"command_status":"complete","exit_code":0,
        "initialized":initialized,"workspace_id":workspace_id,
        "profile_freshness":"unverified","evidence_freshness":"unverified",
        "pending_reports":next["reason"] == "pending_reports_require_sync" || next["reason"] == "failed_report_requires_repair",
        "open_task_count":tasks.len(),"finding_count":findings,"blocker_count":blockers,
        "tasks":tasks,"next":next,"next_actions":next["next_actions"],
        "authority":"local_unverified","delivery_decision":"not_evaluated"
    });
    if parsed.json {
        println!("{report}");
    } else {
        println!(
            "CodeGuard 工作区：{}；开放任务 {}（发现 {}、环境阻塞 {}）；证据新鲜度未核验。",
            if initialized {
                "已初始化"
            } else {
                "未初始化"
            },
            report["open_task_count"],
            findings,
            blockers
        );
        println!("下一步：{}", report["next"]["reason"]);
    }
    ExitCode::SUCCESS
}

/// 返回指定任务的受限 RepairBrief；任务 Markdown 仅用于存在性核对。
pub fn run_show(args: &[String]) -> ExitCode {
    let Some(id) = args.first() else {
        return output_error("task_show", true, "invalid_request", 2, "task_id_missing");
    };
    let parsed = match parse_args(&args[1..]) {
        Ok(parsed) => parsed,
        Err(reason) => return output_error("task_show", true, "invalid_request", 2, &reason),
    };
    if !safe_id(id) {
        return output_error(
            "task_show",
            parsed.json,
            "invalid_request",
            2,
            "task_id_invalid",
        );
    }
    let root = match canonical_root(&parsed.root) {
        Ok(root) => root,
        Err(reason) => return output_error("task_show", parsed.json, "incomplete", 3, reason),
    };
    let brief = match read_task_brief(&root, id) {
        Ok(brief) => brief,
        Err(reason) => return output_error("task_show", parsed.json, "incomplete", 3, reason),
    };
    let c_structure = matches!(
        brief["checker_id"].as_str(),
        Some("c.clang.documentation_structure" | "cpp.clang.documentation_structure")
    );
    let c_documentation = matches!(
        brief["checker_id"].as_str(),
        Some("c.clang.documentation" | "cpp.clang.documentation")
    );
    // 首次原生确认必须保留简报绑定的工具选择参数，避免外层动作丢失必要输入。
    let next_actions = if c_structure
        || c_documentation
        || brief["checker_id"] == "shell.shellcheck"
        || matches!(
            brief["schema_version"].as_str(),
            Some("0.7.0" | "0.8.0" | "0.9.0" | "0.10.0" | "0.11.0" | "0.12.0")
        ) {
        json!([brief["recheck_argv"]])
    } else {
        json!([["codeguard", "task", "verify", id, "."]])
    };
    let report = json!({
        "schema_version":if c_structure {"0.5.0"}else if c_documentation {"0.4.0"}else if brief["checker_id"] == "shell.shellcheck" {"0.3.0"}else if brief["schema_version"] == "0.12.0" {"0.2.0"}else{"0.1.0"},"report_type":"task_show_preview",
        "operation":"task_show","request_id":request_id(),"command_status":"complete","exit_code":0,
        "task_id":id,"task":brief,"state":"open",
        "evidence_freshness":"unverified","event_chain_status":"unverified",
        "next_actions":next_actions,
        "authority":"local_unverified","delivery_decision":"not_evaluated"
    });
    if parsed.json {
        println!("{report}");
    } else {
        println!(
            "任务 {}：{}；范围 {}；步骤 {}；复检 {}。正式门禁未评估。",
            id,
            report["task"]["disposition"],
            report["task"]["scope"],
            report["task"]["step"],
            report["task"]["recheck_argv"]
        );
    }
    ExitCode::SUCCESS
}

fn parse_args(args: &[String]) -> Result<Arguments, String> {
    let mut root = None;
    let mut json = false;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--format" {
            index += 1;
            json = parse_format(args.get(index).ok_or("format_missing")?)?;
        } else if let Some(value) = arg.strip_prefix("--format=") {
            json = parse_format(value)?;
        } else if arg.starts_with('-') || root.is_some() {
            return Err("arguments_invalid".into());
        } else {
            root = Some(PathBuf::from(arg));
        }
        index += 1;
    }
    Ok(Arguments {
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        json,
    })
}

fn parse_format(value: &str) -> Result<bool, String> {
    match value {
        "json" => Ok(true),
        "human" => Ok(false),
        _ => Err("format_invalid".into()),
    }
}

fn canonical_root(root: &Path) -> Result<PathBuf, &'static str> {
    root.canonicalize()
        .ok()
        .filter(|root| root.is_dir())
        .ok_or("project_unreadable")
}

fn real_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_dir())
}

fn safe_id(id: &str) -> bool {
    id.strip_prefix("CG-B-")
        .or_else(|| id.strip_prefix("CG-"))
        .is_some_and(|suffix| {
            suffix.len() == 32
                && suffix
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        })
}

fn request_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |value| value.as_nanos());
    format!("view-{}-{nanos}", std::process::id())
}

fn output_error(
    operation: &str,
    json_format: bool,
    status: &str,
    code: u8,
    reason: &str,
) -> ExitCode {
    if json_format {
        println!(
            "{}",
            json!({
                "schema_version":"0.1.0","report_type":if operation == "status" { "workspace_status_preview" } else { "task_show_preview" },
                "operation":operation,"request_id":request_id(),"command_status":status,"exit_code":code,
                "reason":reason,"task":null,"next_actions":[],"authority":"local_unverified",
                "delivery_decision":"not_evaluated"
            })
        );
    } else {
        eprintln!("本地只读查询不可用：{reason}");
    }
    ExitCode::from(code)
}
