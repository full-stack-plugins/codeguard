//! 占位策略报告的封闭形状与原生结构关联校验，不证明注释内容或执行权威。
use crate::clang_documentation_structure::valid_clang_documentation_structure;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// 校验占位报告与原生结构及可选冻结源码的关联；返回是否具有合法组件与唯一位置。
/// 参数为占位观察、原结构和可选源码；无源码仅作协议关联，调用者仍须核验原AST/工具/消费收据。
/// 本方法不以可编辑结构证明占位文本为真，不授予任务关闭或完整语义资格。
pub fn valid_clang_documentation_placeholders(
    report: &Value,
    structure: &Value,
    source: Option<&[u8]>,
) -> bool {
    if !valid_clang_documentation_structure(structure, source)
        || !exact(
            report,
            &[
                "schema_version",
                "observation_type",
                "rule",
                "policy_version",
                "authority",
                "qualification",
                "coverage_proven",
                "semantic_accuracy",
                "positions",
            ],
        )
        || report["schema_version"] != "0.1.0"
        || report["observation_type"] != "clang_documentation_placeholder_observation"
        || report["rule"] != "codeguard.documentation.placeholder_description"
        || report["policy_version"] != "1"
        || report["authority"] != "codeguard_structural_policy"
        || report["qualification"] != "not_granted"
        || report["coverage_proven"] != false
        || report["semantic_accuracy"] != "not_evaluated"
    {
        return false;
    }
    let Some(positions) = report["positions"]
        .as_array()
        .filter(|rows| rows.len() <= 516_000)
    else {
        return false;
    };
    let functions: BTreeMap<u64, &Value> = structure["functions"]
        .as_array()
        .expect("结构已校验")
        .iter()
        .map(|row| (row["offset_byte"].as_u64().expect("位置已校验"), row))
        .collect();
    let mut seen = BTreeSet::new();
    for position in positions {
        if !exact(
            position,
            &["offset_byte", "line", "column_byte", "component"],
        ) {
            return false;
        }
        let Some(offset) = position["offset_byte"].as_u64() else {
            return false;
        };
        let Some(component) = position["component"].as_str() else {
            return false;
        };
        if !seen.insert((offset, component)) {
            return false;
        }
        let Some(row) = functions.get(&offset) else {
            return false;
        };
        if row["structure_status"] != "observed_supported_subset"
            || row["comment_presence"] != "present"
            || position["line"] != row["line"]
            || position["column_byte"] != row["column_byte"]
        {
            return false;
        }
        let supported = match component {
            "purpose" => row["purpose_description"] == "nonempty",
            "return" => row["return_description"] == "nonempty",
            _ => component.strip_prefix("parameter:").is_some_and(|name| {
                row["parameters"]
                    .as_array()
                    .expect("参数已校验")
                    .iter()
                    .any(|parameter| {
                        parameter["name"] == name && parameter["description"] == "nonempty"
                    })
            }),
        };
        if !supported {
            return false;
        }
    }
    true
}

fn exact(value: &Value, keys: &[&str]) -> bool {
    value.as_object().is_some_and(|object| {
        object.len() == keys.len() && keys.iter().all(|key| object.contains_key(*key))
    })
}
