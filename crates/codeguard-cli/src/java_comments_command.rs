//! Java 注释统一入口，复用原生探针并保留项目配置与局部检查边界。

use crate::check_budget::{
    budget_record, parse_check_timeout, resolve_project_default, select_check_timeout,
};
use crate::discovery::discover;
use crate::java_javadoc_command::{Args, observe};
use crate::java_javadoc_scan::{NativeContext, observe_project};
use codeguard_adapters::legacy_registry;
use codeguard_runtime::NativeObservation;
use serde_json::json;
use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

/// 执行 Java 注释检查；输入为入口剩余参数，返回参数错误、取消或局部未完成状态。
pub fn run(args: &[String]) -> ExitCode {
    let mut target = PathBuf::from(".");
    let mut options = std::collections::BTreeMap::new();
    let mut seen = HashSet::new();
    let mut positional = false;
    let mut index = 0;
    while index < args.len() {
        let argument = &args[index];
        if argument.starts_with('-') {
            let (key, inline) = argument
                .split_once('=')
                .map_or((argument.as_str(), None), |(key, value)| (key, Some(value)));
            if !matches!(
                key,
                "--java-home"
                    | "--maven-tool"
                    | "--maven-repo"
                    | "--repo-sha256"
                    | "--timeout"
                    | "--format"
                    | "--workspace"
            ) || !seen.insert(key.to_owned())
            {
                eprintln!("未知或重复参数：{key}");
                return ExitCode::from(2);
            }
            let value = if let Some(value) = inline {
                value.to_owned()
            } else {
                index += 1;
                let Some(value) = args.get(index).filter(|value| !value.starts_with('-')) else {
                    eprintln!("{key} 缺少参数值");
                    return ExitCode::from(2);
                };
                value.clone()
            };
            if value.is_empty() {
                eprintln!("{key} 不能为空");
                return ExitCode::from(2);
            }
            options.insert(key.to_owned(), value);
        } else if positional {
            eprintln!("只能指定一个检查路径");
            return ExitCode::from(2);
        } else {
            target = PathBuf::from(argument);
            positional = true;
        }
        index += 1;
    }
    let format = options.get("--format").map_or("human", String::as_str);
    if !matches!(format, "human" | "json") {
        eprintln!("--format 仅支持 human/json");
        return ExitCode::from(2);
    }
    let workspace = match options.get("--workspace") {
        Some(value) => match PathBuf::from(value)
            .canonicalize()
            .ok()
            .filter(|p| p.is_dir())
        {
            Some(root)
                if target
                    .canonicalize()
                    .is_ok_and(|p| p.starts_with(&root) && (!p.is_dir() || p == root)) =>
            {
                Some(root)
            }
            _ => {
                eprintln!("--workspace需要可读目录，文件须位于其中，项目目标须为同一根");
                return ExitCode::from(2);
            }
        },
        None => None,
    };
    let timeout = options
        .get("--timeout")
        .map(|value| parse_check_timeout(value))
        .transpose()
        .and_then(select_check_timeout)
        .and_then(|(value, source)| {
            resolve_project_default(
                if let Some(root) = workspace.as_deref() {
                    root
                } else if target.is_file() {
                    target.parent().unwrap_or(&target)
                } else {
                    &target
                },
                value,
                source,
            )
        });
    let (timeout, source) = match timeout {
        Ok(value) => value,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    };
    let java_home = options.get("--java-home").map(PathBuf::from);
    let maven_tool = options.get("--maven-tool").map(PathBuf::from);
    let maven_repo = options.get("--maven-repo").map(PathBuf::from);
    if [java_home.as_ref(), maven_tool.as_ref(), maven_repo.as_ref()]
        .into_iter()
        .flatten()
        .any(|path| !path.is_absolute())
    {
        eprintln!("原生工具与仓库路径必须为绝对路径");
        return ExitCode::from(2);
    }
    let deadline = Instant::now() + Duration::from_millis(timeout);
    let cancelled = AtomicBool::new(false);
    let mut report = json!({"schema_version":"0.1.0", "report_type":"java_comments_feedback", "operation":"comments", "language":"java", "category":"comments", "target_kind":"unavailable", "command_status":"incomplete", "exit_code":3, "reason":"target_unavailable", "execution_budget":budget_record(timeout, source), "native_observation":null, "discovery":null, "coverage_proven":false, "authority":"local_unverified", "delivery_decision":"not_evaluated", "workbench_status":"not_integrated", "next_actions":["依据原生诊断修复注释或配置；使用相同原生上下文重新执行 comments java", "Javadoc 持久任务适配尚未接通，不能据此关闭任务"]});
    if let Ok(path) = target.canonicalize() {
        if path.is_file() {
            if maven_tool.is_some() || maven_repo.is_some() || options.contains_key("--repo-sha256")
            {
                eprintln!("Maven 上下文仅用于项目目录检查");
                return ExitCode::from(2);
            }
            report["target_kind"] = json!("file");
            report["native_observation"] = observe(
                &Args {
                    source: path,
                    java_home,
                    json: true,
                },
                deadline,
                &cancelled,
            );
            report["reason"] = report["native_observation"]["reason"].clone();
        } else if path.is_dir() {
            report["target_kind"] = json!("project");
            if let Ok(registry) = legacy_registry() {
                let discovery = discover(&path, &registry, &NativeObservation);
                let sources = discovery
                    .languages
                    .get("java")
                    .map(|language| language.source_files.clone())
                    .unwrap_or_else(BTreeSet::new);
                report["native_observation"] = observe_project(
                    &path,
                    &sources,
                    &discovery.checker_configurations,
                    &NativeContext {
                        manifest_sha256: &discovery.manifest_sha256,
                        java_home: java_home.as_deref(),
                        maven_tool: maven_tool.as_deref(),
                        maven_repo: maven_repo.as_deref(),
                        repo_sha256: options.get("--repo-sha256").map(String::as_str),
                        deadline,
                        cancelled: &cancelled,
                    },
                );
                report["discovery"] = discovery.to_json();
                report["reason"] = json!("project_comments_observed_with_unverified_coverage");
            } else {
                report["reason"] = json!("registry_unavailable");
            }
        }
    }
    let root = workspace.or_else(|| {
        target
            .canonicalize()
            .ok()
            .filter(|p| p.is_dir() && p.join(".codeguard/workspace.json").exists())
    });
    if let Some(root) = root {
        report["schema_version"] = json!("0.4.0");
        report["workbench"] = crate::javadoc_workbench::connect(&root, &report);
        report["workbench_status"] = report["workbench"]["status"].clone();
        report["next_actions"] = json!([
            "按本次显式文件或项目配置模式核对原生诊断与工作台简报",
            "按原任务运行task verify保存原工具复检；可信关闭尚未接通，局部零诊断不能关闭任务"
        ]);
    }
    let exit = if codeguard_runtime::sigint_cancellation_requested()
        || report["reason"] == "request_cancelled"
    {
        130
    } else {
        3
    };
    if exit == 130 {
        report["command_status"] = json!("cancelled");
        report["reason"] = json!("request_cancelled");
        report["exit_code"] = json!(130);
    }
    if format == "json" {
        println!("{report}");
    } else {
        println!(
            "Java 注释检查：{}；范围 {}；原因 {}",
            report["command_status"], report["target_kind"], report["reason"]
        );
        println!("原生观察：{}", report["native_observation"]);
        if report["workbench"].is_object() {
            println!(
                "工作台：{}；新增问题 {}，准备任务 {}",
                report["workbench"]["status"],
                report["workbench"]["new_findings"],
                report["workbench"]["new_blockers"]
            );
            let brief = &report["workbench"]["next"]["repair_brief"];
            if brief.is_object() {
                println!(
                    "任务 {}；模式 {}；下一步 {}；复检参数 {}",
                    brief["task_id"],
                    brief["observation_scope"],
                    brief["step"],
                    brief["recheck_argv"]
                );
            }
        }

        for action in report["next_actions"].as_array().into_iter().flatten() {
            println!("下一步：{action}");
        }
    }
    ExitCode::from(exit)
}
