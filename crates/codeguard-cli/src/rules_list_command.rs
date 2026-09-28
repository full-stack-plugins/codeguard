//! 规则目录的只读静态视图；配置声明、候选映射和实际生效规则分别展示。

use crate::discovery::discover;
use codeguard_adapters::{bundled_ruff_rulepack, legacy_registry};
use codeguard_core::CHECK_CATEGORIES;
use codeguard_runtime::NativeObservation;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::ExitCode;

struct Arguments {
    language: String,
    root: PathBuf,
    json: bool,
}

/// 列出指定语言或所有登记语言的候选规则及配置；返回值不授予策略或门禁权威。
pub fn run(args: &[String]) -> ExitCode {
    let args = match parse_args(args) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let registry = match legacy_registry() {
        Ok(value) => value,
        Err(_) => {
            eprintln!("内置语言登记损坏；恢复二进制后重试。");
            return ExitCode::from(4);
        }
    };
    if args.language != "all" && !registry.languages.iter().any(|row| row.id == args.language) {
        eprintln!("未知语言 ID；使用 capabilities 查看登记语言。");
        return ExitCode::from(2);
    }
    let pack = match bundled_ruff_rulepack() {
        Ok(value) => value,
        Err(_) => {
            eprintln!("内置规则映射损坏；恢复二进制后重试。");
            return ExitCode::from(4);
        }
    };
    let discovery = discover(&args.root, &registry, &NativeObservation);
    let mut languages = Vec::new();
    for language in &registry.languages {
        if args.language != "all" && args.language != language.id {
            continue;
        }
        // 旧语言登记将 JS/TS 同属 typescript；原生 Node 检查器使用明确的生态映射。
        let checker_prefix = if language.id == "typescript" {
            "node.".to_owned()
        } else {
            format!("{}.", language.id)
        };
        let configurations: Vec<_> = discovery
            .checker_configurations
            .iter()
            .filter(|row| row.checker_id.starts_with(&checker_prefix))
            .collect();
        let rules: Vec<Value> = if language.id == "python" {
            pack.mappings.iter().map(|mapping| json!({
                "checker_id":"python.ruff",
                "native_rule_id":mapping.native_rule_id,
                "codeguard_rule_id":mapping.codeguard_rule_id,
                "category":mapping.category,
                "source_ref":mapping.source_ref,
                "severity":null,
                "enabled":"unverified",
                "execution":"not_run",
                "approval":"unverified",
                "applicability":"requires_native_effective_configuration",
                "suppression":"not_evaluated",
                "approved_exception":null,
                "next_action":"先核对原生逐文件有效配置，再执行原工具检查；映射不是启用证明",
                "rulepack":{
                    "id":pack.id,"version":pack.version,"sha256":pack.sha256,
                    "status":pack.status,"scope":pack.scope,"origin_url":pack.origin_url,
                    "license":pack.license,"license_ref":pack.license_ref,
                    "compatible_tool_versions":pack.compatible_tool_versions
                }
            })).collect()
        } else {
            Vec::new()
        };
        let gaps: Vec<Value> = CHECK_CATEGORIES.iter().map(|category| json!({
            "category":category,
            "reason":if language.id == "python" && *category == "lint" {
                "candidate_mapping_is_not_complete_effective_catalog"
            } else { "native_rule_catalog_not_bound" },
            "next_action":"核对该类别的原生规则目录、项目生效配置与批准策略；不能把空目录解释为检查通过"
        })).collect();
        languages.push(json!({
            "language":language.id,
            "project_detected":discovery.languages.contains_key(&language.id),
            "catalog_status":"incomplete",
            "checker_configurations":configurations,
            "rules":rules,"catalog_gaps":gaps
        }));
    }
    let report = json!({
        "schema_version":"0.1.0","report_type":"rules_inventory","operation":"list",
        "inspection_status":"incomplete","command_status":"incomplete","exit_code":3,
        "authority":"unverified","gate_effect":"none","delivery_decision":"not_evaluated",
        "observation_complete":discovery.observation_complete,
        "scope":"static_project_discovery_and_bundled_candidate_mappings",
        "approved_policy":"unbound","tool_lock":"not_evaluated",
        "languages":languages,
        "next_actions":["使用 config explain 核对配置与可信策略来源","使用 plan 查看检查义务；rules list 不执行原生检查"]
    });
    if args.json {
        println!("{report}");
    } else {
        println!("规则目录：未完成；本次只读发现，不执行检查或批准白名单。");
        if !discovery.observation_complete {
            println!("项目发现不完整：恢复目标目录、权限或物理路径后重试。");
        }
        for language in report["languages"].as_array().into_iter().flatten() {
            println!(
                "语言：{}；目录：未完成",
                language["language"].as_str().unwrap_or("unknown")
            );
            for checker in language["checker_configurations"]
                .as_array()
                .into_iter()
                .flatten()
            {
                println!(
                    "  检查器 {}：配置 {}；原因 {}",
                    checker["checker_id"], checker["configuration"], checker["reason"]
                );
            }
            for rule in language["rules"].as_array().into_iter().flatten() {
                println!(
                    "  候选规则 {}：启用未核验、执行未运行、批准未核验；依据 {}",
                    rule["native_rule_id"], rule["source_ref"]
                );
            }
            println!("  六类规则目录/生效配置仍须核对；使用 config explain 和 plan 获取准备动作。");
        }
    }
    ExitCode::from(3)
}

fn parse_args(args: &[String]) -> Result<Arguments, &'static str> {
    let Some(language) = args.first() else {
        return Err("rules list 缺少语言；使用 <language|all> [path] [--format human|json]");
    };
    if language.is_empty() || language.starts_with('-') {
        return Err("rules list 语言参数非法");
    }
    let mut root = None;
    let mut format = None;
    let mut index = 1;
    while index < args.len() {
        let current = &args[index];
        let value = if current == "--format" {
            index += 1;
            Some(args.get(index).ok_or("--format 缺少值")?.as_str())
        } else {
            current.strip_prefix("--format=")
        };
        if let Some(value) = value {
            if format.is_some() || !matches!(value, "human" | "json") {
                return Err("--format 重复或不支持；仅允许 human/json");
            }
            format = Some(value);
        } else if current.is_empty() || current.starts_with('-') || root.is_some() {
            return Err("rules list 参数非法；不支持修改或批准操作");
        } else {
            root = Some(PathBuf::from(current));
        }
        index += 1;
    }
    Ok(Arguments {
        language: language.clone(),
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        json: format == Some("json"),
    })
}
