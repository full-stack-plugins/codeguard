//! 初始化观察的公开摘要；仅结构化事实与未知状态，不产生质量结论。

use crate::discovery::DiscoveryReport;
use serde_json::{Value, json};

/// 为 dry-run/apply 生成同语义摘要；不读新文件、不探测本机工具或猜测架构。
pub(crate) fn render(discovery: &DiscoveryReport) -> Value {
    let languages:Vec<_>=discovery.languages.iter().map(|(id,evidence)|json!({"id":id,"source_file_count":evidence.source_files.len(),"manifest_count":evidence.manifests.len()})).collect();
    let roots: Vec<_> = discovery
        .build_roots
        .iter()
        .map(|(path, manifests)| json!({"path":path,"manifest_refs":manifests}))
        .collect();
    let checkers:Vec<_>=discovery.checker_configurations.iter().map(|entry| json!({
        "build_root":entry.build_root,"checker_id":entry.checker_id,"category":entry.category,
        "configuration":entry.configuration,"configuration_ref":entry.configuration_ref,
        "reason":entry.reason,"next_action":entry.next_action,"execution":"not_run",
        "required_by_policy":null,"gate_effect":"none"
    })).collect();
    json!({"languages":languages,"build_roots":roots,"language_targets":crate::language_target_profile::render(discovery),"installed_versions":"not_probed","build_model":"not_resolved","architecture":"unknown","unknown_conditions":discovery.unknown_conditions,"checker_inventory":"partial","checkers":checkers})
}

/// 展示同一结构化摘要；不可信路径/字段以 JSON 字符串转义，不能伪造终端行。
pub(crate) fn print_human(summary: &Value) {
    println!("项目观察：静态声明；本机版本：未探测；构建模型：未解析；架构：unknown");
    if let Some(languages) = summary["languages"].as_array() {
        if languages.is_empty() {
            println!("语言：未确认");
        }
        for language in languages {
            println!(
                "语言 {}：源码文件 {}，清单 {}",
                language["id"], language["source_file_count"], language["manifest_count"]
            );
        }
    }
    if let Some(roots) = summary["build_roots"].as_array() {
        for root in roots {
            println!("构建根 {}：清单 {}", root["path"], root["manifest_refs"]);
        }
    }
    if let Some(targets) = summary["language_targets"].as_array() {
        if targets.is_empty() {
            println!("语言目标：未确认");
        }
        for target in targets {
            println!(
                "目标声明 {} {} = {}；来源 {}；状态 declared_only",
                target["language_id"],
                target["target_kind"],
                target["value"],
                target["manifest_ref"]
            );
        }
    }
    println!("检查器清单：partial；检查义务：未绑定");
    if let Some(checkers) = summary["checkers"].as_array() {
        if checkers.is_empty() {
            println!("检查配置：尚无可展示的原生配置观察");
        }
        for checker in checkers {
            println!(
                "检查配置 {}（构建根 {}）：{}；执行：not_run；必需性：未绑定；来源 {}",
                checker["checker_id"],
                checker["build_root"],
                checker["configuration"],
                checker["configuration_ref"]
            );
            println!(
                "配置依据：{}；下一步：{}",
                checker["reason"], checker["next_action"]
            );
        }
    }
    if let Some(reasons) = summary["unknown_conditions"].as_array() {
        for reason in reasons {
            println!("待确认：{reason}");
        }
    }
}
