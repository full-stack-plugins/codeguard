//! Rust 能力矩阵文档生成器；默认只核对，显式 --write 才更新。

use codeguard_adapters::{capability_row, legacy_registry, validate_capability_inventory};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::process::ExitCode;

fn render(document: &Value) -> Result<String, String> {
    validate_capability_inventory(document)?;
    let rows = document["languages"]
        .as_array()
        .ok_or("能力清单缺语言数组")?;
    let mut lines = vec![
        "# Codeguard 能力矩阵（生成）".to_string(),
        String::new(),
        "本表来自 `codeguard capabilities --format json`。旧状态只作迁移对照；新能力必须有对应平台、类别和验收证据。`gap` 不得被解释为质量通过。".to_string(),
        String::new(),
        "| 语言 | 旧状态 | 旧 lint | 旧 formatter | 六类别 × 五平台新能力 |".to_string(),
        "|---|---|---|---|---|".to_string(),
    ];
    for row in rows {
        let mut counts = [0usize; 3];
        for categories in row["platforms"].as_object().ok_or("缺平台矩阵")?.values() {
            for cell in categories.as_object().ok_or("缺类别矩阵")?.values() {
                match cell["status"].as_str() {
                    Some("implemented") => counts[0] += 1,
                    Some("gap") => counts[1] += 1,
                    Some("not_applicable") => counts[2] += 1,
                    _ => return Err("非法能力状态".into()),
                }
            }
        }
        let summary = if counts == [0, 30, 0] {
            "30 gap".to_string()
        } else {
            format!(
                "{} implemented / {} gap / {} not_applicable",
                counts[0], counts[1], counts[2]
            )
        };
        lines.push(format!(
            "| {} | {} | {} | {} | {summary} |",
            row["language"].as_str().ok_or("缺语言 ID")?,
            row["legacy_status"].as_str().ok_or("缺旧状态")?,
            if row["legacy_lint_declared"] == true {
                "yes"
            } else {
                "no"
            },
            if row["legacy_formatter_declared"] == true {
                "yes"
            } else {
                "no"
            },
        ));
    }
    lines.push(String::new());
    lines.push(
        "当前所有 57 种语言均未取得新原生适配器验收；平台目标只是候选登记，非 stable 承诺。"
            .to_string(),
    );
    lines.push(String::new());
    Ok(lines.join("\n"))
}

fn main() -> ExitCode {
    let write = match std::env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => false,
        [option] if option == "--check" => false,
        [option] if option == "--write" => true,
        _ => {
            eprintln!("用法: cargo run --example gen_capability_docs -- [--check|--write]");
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
    let rows: Vec<_> = registry.languages.iter().map(capability_row).collect();
    let document = json!({"schema_version":"0.2.0","report_type":"capability_inventory","release_version":env!("CARGO_PKG_VERSION"),"languages":rows});
    let generated = match render(&document) {
        Ok(generated) => generated,
        Err(error) => {
            eprintln!("能力矩阵校验失败：{error}");
            return ExitCode::from(4);
        }
    };
    let output = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/CAPABILITIES.md");
    if write {
        if let Err(error) = fs::write(&output, generated) {
            eprintln!("能力文档写入失败：{error}");
            return ExitCode::from(3);
        }
    } else if fs::read_to_string(&output).ok().as_deref() != Some(generated.as_str()) {
        eprintln!("能力文档与 Rust 发行清单不一致：{}", output.display());
        return ExitCode::from(3);
    }
    ExitCode::SUCCESS
}
