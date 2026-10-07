//! 质量配置映射模块（9.24）。
//!
//! 生成质量配置映射、测试/环境前置清单及准备任务：缺工具或未知架构不虚构代码违规，
//! 不自动增加排除或存量豁免。

use serde_json::{Value, json};

/// 配置映射项。
pub(crate) struct ConfigMapping {
    pub checker_id: String,
    pub category: String,
    pub status: String,
    pub next_action: String,
}

/// 生成质量配置映射。
pub(crate) fn generate_config_mapping(checkers: Vec<ConfigMapping>) -> Value {
    let items: Vec<Value> = checkers
        .iter()
        .map(|c| {
            json!({
                "checker_id": c.checker_id,
                "category": c.category,
                "status": c.status,
                "next_action": c.next_action,
            })
        })
        .collect();

    json!({
        "schema_version": "0.1.0",
        "report_type": "quality_config_mapping",
        "checkers": items,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_mapping_contains_checkers() {
        let checkers = vec![ConfigMapping {
            checker_id: "phpcs".to_string(),
            category: "lint".to_string(),
            status: "configured".to_string(),
            next_action: "run phpcs".to_string(),
        }];
        let mapping = generate_config_mapping(checkers);
        assert_eq!(mapping["checkers"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn missing_tool_not_fabricated_as_violation() {
        let checkers = vec![ConfigMapping {
            checker_id: "solc".to_string(),
            category: "lint".to_string(),
            status: "missing".to_string(),
            next_action: "install solc".to_string(),
        }];
        let mapping = generate_config_mapping(checkers);
        assert_eq!(mapping["checkers"][0]["status"], "missing");
    }
}
