//! 只读检查计划预览；缺可信策略时不伪造正式 CheckPlan 或执行任务。

use crate::discovery::discover;
use codeguard_adapters::{CheckerConfiguration, LegacyRegistry, legacy_registry};
use codeguard_core::CHECK_CATEGORIES;
use codeguard_runtime::NativeObservation;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::ExitCode;

struct Args {
    operation: String,
    language: String,
    root: PathBuf,
    json: bool,
}

/// 预览项目检查范围；目前没有可信策略来源，始终返回未完成而非质量认证。
pub fn run(args: &[String]) -> ExitCode {
    let args = match parse_args(args) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let registry = match legacy_registry() {
        Ok(registry) => registry,
        Err(error) => {
            eprintln!("内置语言清单损坏：{error}");
            return ExitCode::from(4);
        }
    };
    if args.language != "all"
        && !registry
            .languages
            .iter()
            .any(|item| item.id == args.language)
    {
        eprintln!("未知语言 ID：{}", args.language);
        return ExitCode::from(2);
    }
    let report = preview(&args, &registry);
    if args.json {
        println!("{report}");
    } else {
        println!("计划预览：{} / {}", args.operation, args.language);
        println!("项目：{}", report["root"].as_str().unwrap_or("<unknown>"));
        println!(
            "观察到的检查器：{}",
            report["observed_checkers"].as_array().map_or(0, Vec::len)
        );
        println!(
            "待确认执行候选：{}",
            report["candidate_tasks"].as_array().map_or(0, Vec::len)
        );
        for condition in report["unresolved_conditions"]
            .as_array()
            .into_iter()
            .flatten()
        {
            if let Some(condition) = condition.as_str() {
                println!("待解析：{condition}");
            }
        }
        println!("检查尚未执行；可信策略和工具锁未确认，不能签发质量结论。");
    }
    ExitCode::from(3)
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let Some(operation) = args.first() else {
        return Err("plan 缺少检查类别或 check".into());
    };
    if operation != "check" && !CHECK_CATEGORIES.contains(&operation.as_str()) {
        return Err(format!("未知检查类别：{operation}"));
    }
    let Some(language) = args.get(1) else {
        return Err("plan 缺少语言 ID 或 all".into());
    };
    if language.starts_with('-') {
        return Err("plan 缺少语言 ID 或 all".into());
    }
    let mut root = None;
    let mut json = false;
    let mut index = 2;
    while index < args.len() {
        let current = &args[index];
        let format = if current == "--format" {
            index += 1;
            Some(args.get(index).ok_or("--format 缺少值")?.as_str())
        } else {
            current.strip_prefix("--format=")
        };
        if let Some(format) = format {
            if !matches!(format, "human" | "json") {
                return Err(format!("不支持的格式：{format}"));
            }
            json = format == "json";
        } else if current.starts_with('-') || root.is_some() {
            return Err(format!("不支持的参数：{current}"));
        } else {
            root = Some(PathBuf::from(current));
        }
        index += 1;
    }
    Ok(Args {
        operation: operation.clone(),
        language: language.clone(),
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        json,
    })
}

fn preview(args: &Args, registry: &LegacyRegistry) -> Value {
    let discovery = discover(&args.root, registry, &NativeObservation);
    let categories: Vec<&str> = if args.operation == "check" {
        CHECK_CATEGORIES.to_vec()
    } else {
        vec![&args.operation]
    };
    let languages: Vec<String> = if args.language == "all" {
        discovery.languages.keys().cloned().collect()
    } else {
        vec![args.language.clone()]
    };
    let selected: Vec<&CheckerConfiguration> = discovery
        .checker_configurations
        .iter()
        .filter(|checker| {
            categories.contains(&checker.category.as_str())
                && languages.iter().any(|language| {
                    checker.checker_id.split('.').next() == Some(language.as_str())
                        || (checker.checker_id.starts_with("node.") && language == "typescript")
                })
        })
        .collect();
    let observed_checkers: Vec<Value> = selected
        .iter()
        .map(|checker| {
            json!({
                "build_root":checker.build_root,
                "checker_id":checker.checker_id,
                "category":checker.category,
                "configuration":checker.configuration,
                "configuration_ref":checker.configuration_ref,
                "reason":checker.reason,
                "next_action":checker.next_action,
            })
        })
        .collect();
    let candidate_tasks: Vec<Value> = selected
        .iter()
        .filter(|checker| checker.configuration == "configured")
        .map(|checker| {
            json!({
                "checker_id":checker.checker_id,
                "category":checker.category,
                "build_root":checker.build_root,
                "state":"pending_policy_and_tool_lock",
                "command":null,
            })
        })
        .collect();
    let mut unresolved: BTreeSet<String> = [
        "trusted_policy_unavailable".into(),
        "tool_lock_unverified".into(),
        "content_identity_unverified".into(),
        "required_obligations_unresolved".into(),
    ]
    .into_iter()
    .collect();
    if languages.is_empty() {
        unresolved.insert("no_language_evidence".into());
    }
    if !discovery.observation_complete {
        unresolved.insert("project_observation_incomplete".into());
    }
    unresolved.extend(discovery.unknown_conditions.iter().cloned());
    for checker in &selected {
        if matches!(checker.configuration.as_str(), "unknown" | "invalid") {
            unresolved.insert(format!(
                "checker_configuration_unresolved:{}",
                checker.checker_id
            ));
        }
    }
    json!({
        "schema_version":"0.1.0",
        "report_type":"plan_preview",
        "planning_status":"incomplete",
        "quality_decision":"not_evaluated",
        "root":discovery.root,
        "selection":{"operation":args.operation,"language":args.language},
        "selected_categories":categories,
        "observed_languages":languages,
        "observation_complete":discovery.observation_complete,
        "blocked_paths":discovery.blocked_paths,
        "observed_checkers":observed_checkers,
        "candidate_tasks":candidate_tasks,
        "policy_identity":null,
        "obligations":[],
        "unresolved_conditions":unresolved,
        "next_action":"确认可信策略、工具锁与本轮内容身份后重新生成正式计划；再运行原生检查",
    })
}
