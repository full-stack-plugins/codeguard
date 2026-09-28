//! 发行能力矩阵的完整性校验；旧 stable 状态不得提升新能力。

use crate::legacy_registry;
use codeguard_core::{CANDIDATE_PLATFORMS, CHECK_CATEGORIES};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// 验证版本化能力清单的语言、平台、类别、证据和工具类型。
pub fn validate_capability_inventory(document: &Value) -> Result<(), String> {
    if document["schema_version"] != "0.2.0" || document["report_type"] != "capability_inventory" {
        return Err("能力清单协议版本或类型不匹配".into());
    }
    if document["release_version"] != env!("CARGO_PKG_VERSION") {
        return Err("能力清单发行版本不匹配".into());
    }
    let rows = document["languages"]
        .as_array()
        .ok_or("能力清单缺语言数组")?;
    let legacy = legacy_registry()?;
    if rows.len() != legacy.languages.len() {
        return Err("能力清单语言数不完整".into());
    }
    let expected: BTreeMap<&str, &str> = legacy
        .languages
        .iter()
        .map(|language| (language.id.as_str(), language.status.as_str()))
        .collect();
    let required_platforms: BTreeSet<&str> = CANDIDATE_PLATFORMS.into_iter().collect();
    let required_categories: BTreeSet<&str> = CHECK_CATEGORIES.into_iter().collect();
    let mut seen = BTreeSet::new();
    for row in rows {
        let language = row["language"].as_str().ok_or("语言 ID 缺失")?;
        if !seen.insert(language) {
            return Err(format!("重复语言 ID：{language}"));
        }
        let status = row["legacy_status"].as_str().ok_or("旧状态缺失")?;
        if expected.get(language) != Some(&status) {
            return Err(format!("{language}: 旧注册身份不匹配"));
        }
        if matches!(language, "julia" | "pascal")
            && (row["legacy_lint_declared"] != false || row["legacy_formatter_declared"] != true)
        {
            return Err(format!("{language}: formatter-only 身份漂移"));
        }
        let platforms = row["platforms"]
            .as_object()
            .ok_or_else(|| format!("{language}: 缺平台维度"))?;
        if platforms
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            != required_platforms
        {
            return Err(format!("{language}: 平台维度不完整"));
        }
        for (platform, categories) in platforms {
            let categories = categories
                .as_object()
                .ok_or_else(|| format!("{language}/{platform}: 缺类别维度"))?;
            if categories
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
                != required_categories
            {
                return Err(format!("{language}/{platform}: 类别维度不完整"));
            }
            for (category, cell) in categories {
                let status = cell["status"]
                    .as_str()
                    .ok_or_else(|| format!("{language}/{platform}/{category}: 缺能力状态"))?;
                if !matches!(status, "implemented" | "gap" | "not_applicable") {
                    return Err(format!("{language}/{platform}/{category}: 非法状态"));
                }
                if status != "gap" && cell["evidence_ref"].as_str().is_none_or(str::is_empty) {
                    return Err(format!("{language}/{platform}/{category}: 缺能力依据"));
                }
                if status == "implemented" {
                    let kind = cell["tool_kind"]
                        .as_str()
                        .ok_or_else(|| format!("{language}/{platform}/{category}: 缺工具类型"))?;
                    if category == "lint" && kind != "diagnostic" {
                        return Err(format!(
                            "{language}/{platform}/{category}: formatter 不得冒充 lint"
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}
