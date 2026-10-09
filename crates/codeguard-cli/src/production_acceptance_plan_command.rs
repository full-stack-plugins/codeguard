use codeguard_adapters::parse_production_acceptance_plan;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::process::ExitCode;

/// 查询仓库四核心验收计划；参数为命令选项，返回读取状态，不能授予交付资格。
pub fn run(args: &[String]) -> ExitCode {
    match report(args) {
        Ok((report, json_format)) => {
            if json_format {
                println!("{}", report);
            } else {
                println!(
                    "生产验收计划：{} 个语言，完整要求 {} 项；生产资格未授予。",
                    report["selected_language_count"], report["full_requirement_count"]
                );
                for row in report["plan"]["languages"]
                    .as_array()
                    .expect("validated rows")
                {
                    println!(
                        "{}：版本未验收；四核心均保留阻塞项。",
                        row["language"].as_str().expect("validated identity")
                    );
                    for (core, cell) in row["capabilities"].as_object().expect("validated cores") {
                        for path in cell["build_paths"].as_array().expect("validated paths") {
                            println!(
                                "  {core}/{}：{}；{}",
                                path["ecosystem"], path["implementation_status"], path["blockers"]
                            );
                        }
                    }
                }
            }
            ExitCode::SUCCESS
        }
        Err((code, message)) => {
            eprintln!("{message}");
            ExitCode::from(code)
        }
    }
}

fn report(args: &[String]) -> Result<(serde_json::Value, bool), (u8, String)> {
    let mut selection = None;
    let mut plan_flag = false;
    let mut format = None;
    let mut index = 0;
    while index < args.len() {
        let current = args[index].as_str();
        if current == "--acceptance-plan" && !plan_flag {
            plan_flag = true;
        } else if current == "--format" || current.starts_with("--format=") {
            let value = if current == "--format" {
                index += 1;
                args.get(index)
                    .map(String::as_str)
                    .ok_or((2, "缺输出格式".into()))?
            } else {
                &current[9..]
            };
            if format.is_some() || !matches!(value, "json" | "human") {
                return Err((2, "输出格式重复或无效".into()));
            }
            format = Some(value == "json");
        } else if current.starts_with('-') || selection.is_some() {
            return Err((2, format!("不支持的验收计划参数：{current}")));
        } else {
            selection = Some(current);
        }
        index += 1;
    }
    if !plan_flag {
        return Err((2, "缺 --acceptance-plan".into()));
    }
    let raw = include_bytes!("../../../rulepacks/production_acceptance_plan_v1.json");
    let mut plan = parse_production_acceptance_plan(raw).map_err(|error| (4, error))?;
    let rows = plan["languages"].as_array_mut().expect("validated rows");
    let full_count = rows.len();
    if let Some(language) = selection {
        rows.retain(|row| row["language"] == language);
        if rows.is_empty() {
            return Err((2, format!("未知语言：{language}")));
        }
    }
    let selected_count = rows.len();
    Ok((
        json!({
            "schema_version":"0.1.0", "report_type":"production_acceptance_plan_view",
            "operation":"capabilities", "selection":selection.unwrap_or("all"),
            "full_language_count":full_count,"full_requirement_count":full_count * 4,
            "selected_language_count":selected_count, "authority":"repository_plan_only",
            "qualification":"not_granted", "delivery_decision":"not_evaluated",
            "source_plan_sha256":format!("{:x}", Sha256::digest(raw)), "plan":plan
        }),
        format.unwrap_or(false),
    ))
}
