//! 单文件 P3C 复用项目观察和稳定修复工作台；规则归属与原生诊断不重复执行。

use crate::java_p3c_command::Args;
use crate::java_p3c_scan::{NativeContext, observe_project};
use crate::next_command::read_local_brief_for_checker;
use crate::work_sync::{save_local_report, sync_local_workspace};
use crate::workspace_refresh::read_workspace_baseline;
use codeguard_adapters::{inspect_maven_pom, inspect_maven_unreadable};
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

/// 定位显式或最近已有工作台；返回 None 保持既有未绑定单文件协议。
/// 参数为已解析请求；坏工作台由观察阶段反馈，不能越过它寻找父工作台。
pub(crate) fn locate(args: &Args) -> Result<Option<PathBuf>, &'static str> {
    if let Some(root) = &args.workspace {
        if !std::fs::symlink_metadata(root).is_ok_and(|m| m.file_type().is_dir()) {
            return Err("workspace_unavailable");
        }
        return root
            .canonicalize()
            .map(Some)
            .map_err(|_| "workspace_unavailable");
    }
    let Some(parent) = args.source.parent().filter(|p| !p.as_os_str().is_empty()) else {
        return std::env::current_dir()
            .map_err(|_| "workspace_unavailable")
            .and_then(|cwd| locate_parent(&cwd));
    };
    let Ok(parent) = parent.canonicalize() else {
        return Ok(None);
    };
    locate_parent(&parent)
}

fn locate_parent(parent: &Path) -> Result<Option<PathBuf>, &'static str> {
    Ok(parent
        .ancestors()
        .find(|root| std::fs::symlink_metadata(root.join(".codeguard")).is_ok())
        .map(Path::to_path_buf))
}

/// 保留无法绑定的明确原因；不执行原生工具、创建工作台或虚构任务。
pub(crate) fn unavailable(args: &Args, reason: &str) -> Value {
    let mut feedback = envelope(args);
    feedback["workbench"] = json!({"status":"backlog_update_failed","reason":reason});
    feedback
}

/// 读取最近 POM 并只检查一个文件，沿既有项目报告同步及 task verify 服务修复。
/// 参数包含统一截止时间和取消标志；返回局部反馈，不能签发项目 allow。
pub(crate) fn observe(
    args: &Args,
    root: &Path,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    let baseline = match read_workspace_baseline(root) {
        Ok(Some(baseline)) if baseline.workspace_id().is_some() => baseline,
        Ok(None) => return unavailable(args, "workspace_not_initialized"),
        _ => return unavailable(args, "workspace_invalid"),
    };
    if source_has_link(&args.source)
        || read_bounded_regular_file(&args.source, 16 * 1024 * 1024).is_err()
    {
        return unavailable(args, "source_unavailable");
    }
    let Ok(source) = args.source.canonicalize() else {
        return unavailable(args, "source_unavailable");
    };
    let Some(relative) = source.strip_prefix(root).ok().and_then(Path::to_str) else {
        return unavailable(args, "source_outside_workspace");
    };
    if source.extension().and_then(|value| value.to_str()) != Some("java") {
        return unavailable(args, "source_not_java_file");
    }
    let mut manifests = BTreeMap::new();
    let mut configurations = Vec::new();
    for directory in source.parent().into_iter().flat_map(Path::ancestors) {
        if !directory.starts_with(root) {
            break;
        }
        let pom = directory.join("pom.xml");
        if std::fs::symlink_metadata(&pom).is_err() {
            continue;
        }
        let build_root = directory
            .strip_prefix(root)
            .ok()
            .and_then(Path::to_str)
            .unwrap_or("");
        let build_root = if build_root.is_empty() {
            "."
        } else {
            build_root
        };
        let reference = pom
            .strip_prefix(root)
            .ok()
            .and_then(Path::to_str)
            .unwrap_or("pom.xml");
        configurations = match read_bounded_regular_file(&pom, 256 * 1024) {
            Ok(bytes) => {
                manifests.insert(
                    reference.to_owned(),
                    format!("{:x}", Sha256::digest(&bytes)),
                );
                inspect_maven_pom(&bytes, build_root, reference)
            }
            Err(_) => inspect_maven_unreadable(build_root, reference),
        };
        break;
    }
    let mut scan = observe_project(
        root,
        &BTreeSet::from([relative.to_owned()]),
        &configurations,
        &NativeContext {
            manifest_sha256: &manifests,
            maven_tool: args.maven_tool.as_deref(),
            java_home: args.java_home.as_deref(),
            maven_repo: args.maven_repo.as_deref(),
            repo_sha256: args.repo_sha256.as_deref(),
            deadline,
            cancelled,
        },
    );
    // 工作台身份在原生执行后再次核对；变更不能将当前结果归给另一工作区。
    if read_workspace_baseline(root)
        .ok()
        .flatten()
        .and_then(|b| b.workspace_id().map(str::to_owned))
        != baseline.workspace_id().map(str::to_owned)
    {
        let mut feedback = unavailable(args, "workspace_changed_during_scan");
        scan["backlog_status"] = json!("backlog_update_failed");
        scan["backlog_sync"] = Value::Null;
        scan["next"] = Value::Null;
        feedback["project_observation"] = scan;
        return feedback;
    }
    let workbench = persist_and_sync(root, &mut scan);
    let mut feedback = envelope(args);
    feedback["path"] = json!(relative);
    feedback["project_observation"] = scan.clone();
    feedback["workbench"] = workbench;
    feedback["next"] = scan["next"].clone();
    feedback
}

/// 同步单文件或聚合 P3C 的同轮观察；只保存一次、不再次启动 Maven。
/// 参数为实际项目根和局部原生报告；返回同步状态及实际失败原因。
pub(crate) fn persist_and_sync(root: &Path, scan: &mut Value) -> Value {
    let (mut status, mut reason, summary) = match save_local_report(root, scan) {
        Ok(()) if scan["workspace_binding"] == "bound" => match sync_local_workspace(root) {
            Ok(summary) if summary.failed_reports == 0 => (
                "synced_partial",
                None,
                json!({
                    "new_findings":summary.new_findings,"new_blockers":summary.new_blockers,
                    "imported_reports":summary.imported_reports,"historical_findings":summary.historical_findings,
                    "failed_reports":summary.failed_reports
                }),
            ),
            Ok(_) => (
                "backlog_update_failed",
                Some("report_sync_incomplete"),
                Value::Null,
            ),
            Err(reason) => ("backlog_update_failed", Some(reason), Value::Null),
        },
        Ok(()) => (
            "not_initialized",
            Some("workspace_not_initialized"),
            Value::Null,
        ),
        Err(reason) => ("backlog_update_failed", Some(reason), Value::Null),
    };
    let next = if status == "synced_partial" {
        match read_local_brief_for_checker(root, "java.maven.p3c") {
            Ok(brief) => brief,
            Err(failure) => {
                status = "backlog_update_failed";
                reason = Some(failure);
                Value::Null
            }
        }
    } else {
        Value::Null
    };
    scan["backlog_status"] = json!(status);
    scan["backlog_sync"] = summary;
    scan["next"] = next;
    json!({"status":status,"reason":reason})
}

fn envelope(args: &Args) -> Value {
    json!({"schema_version":"0.1.0","report_type":"java_p3c_file_feedback",
        "operation":"lint","language":"java","checker_id":"java.maven.p3c",
        "command_status":"incomplete","exit_code":3,"scope":"single_file",
        "path":args.source.to_string_lossy(),"project_observation":null,
        "workbench":{"status":"backlog_update_failed","reason":"not_run"},"next":null,
        "coverage_proven":false,"delivery_decision":"not_evaluated"})
}

fn source_has_link(source: &Path) -> bool {
    source.ancestors().any(|path| {
        std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink())
    })
}

/// 渲染当前原生诊断和稳定任务指引，不把环境阻塞当源码规则违规。
pub(crate) fn print_feedback(report: &Value) {
    println!("Java P3C 单文件项目观察：incomplete");
    let scan = &report["project_observation"];
    for file in scan["files"].as_array().into_iter().flatten() {
        println!("配置：{}；观察：{}", file["configuration"], file["reason"]);
        if file["observation"].is_object() {
            println!(
                "原生执行：{}；原因：{}",
                file["observation"]["local_status"]
                    .as_str()
                    .unwrap_or("incomplete"),
                file["observation"]["reason"].as_str().unwrap_or("unknown")
            );
        }
        for finding in file["observation"]["findings"]
            .as_array()
            .into_iter()
            .flatten()
        {
            println!(
                "{}:{} {}",
                file["path"].as_str().unwrap_or("<unknown>"),
                finding["line"],
                finding["rule_id"]
            );
        }
    }
    println!("修复工作台：{}", report["workbench"]);
    if !report["next"].is_null() {
        println!("下一步：{}", report["next"]);
    }
    println!("局部原生观察不证明项目规则覆盖，不能签发质量通过。");
}
