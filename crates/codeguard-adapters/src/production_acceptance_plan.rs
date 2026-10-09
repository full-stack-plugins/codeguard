use crate::{legacy_registry, parse_unique_json};
use codeguard_core::CANDIDATE_PLATFORMS;
use serde_json::Value;
use std::collections::BTreeSet;

/// 读取四核心验收计划；参数为有界原始 JSON，返回完整未授予资格的仓库映射。
pub fn parse_production_acceptance_plan(raw: &[u8]) -> Result<Value, String> {
    if raw.len() > 2 * 1024 * 1024 {
        return Err("验收计划超出读取预算".into());
    }
    let doc = parse_unique_json(raw).map_err(str::to_owned)?;
    exact_keys(
        &doc,
        &[
            "schema_version",
            "document_type",
            "authority",
            "qualification",
            "platform_scope",
            "platform_targets",
            "source_hashes",
            "languages",
        ],
    )?;
    if doc["schema_version"] != "0.1.0"
        || doc["document_type"] != "production_acceptance_plan"
        || doc["authority"] != "repository_plan_only"
        || !matches!(
            doc["qualification"].as_str(),
            Some("not_granted") | Some("v1_granted")
        )
        || doc["platform_scope"] != "candidate_targets_not_advertised_support"
    {
        return Err("验收计划协议或资格越权".into());
    }
    let platforms = strings(&doc["platform_targets"])?;
    if platforms != CANDIDATE_PLATFORMS.into_iter().collect() {
        return Err("验收平台维度不完整".into());
    }
    let source_hashes = doc["source_hashes"].as_object().ok_or("缺来源摘要映射")?;
    if source_hashes.len() > 512 {
        return Err("来源引用过多".into());
    }
    for (path, digest) in source_hashes {
        if !safe_reference(path) || !digest.as_str().is_some_and(valid_digest) {
            return Err("来源引用或摘要无效".into());
        }
    }
    let registry = legacy_registry()?;
    let legacy = parse_unique_json(include_bytes!("../../../rulepacks/legacy_languages.json"))
        .map_err(str::to_owned)?;
    let manifest = parse_unique_json(include_bytes!("../../../grammars/manifest.json"))
        .map_err(str::to_owned)?;
    let grammar_ids: BTreeSet<&str> = manifest["assets"]
        .as_array()
        .ok_or("语法清单无效")?
        .iter()
        .filter_map(|asset| asset["language"].as_str())
        .collect();
    let expected: BTreeSet<&str> = registry
        .languages
        .iter()
        .map(|language| language.id.as_str())
        .collect();
    let rows = doc["languages"].as_array().ok_or("缺语言映射")?;
    if rows.len() != expected.len() {
        return Err("验收语言数量不完整".into());
    }
    let mut seen = BTreeSet::new();
    let mut seen_grammars = BTreeSet::new();
    let mut refs = BTreeSet::from(["rulepacks/legacy_languages.json", "grammars/manifest.json"]);
    for row in rows {
        exact_keys(
            row,
            &[
                "language",
                "legacy_status",
                "version_scope",
                "build_targets",
                "grammar_candidates",
                "legacy_lint_candidate",
                "capabilities",
            ],
        )?;
        let id = row["language"].as_str().ok_or("缺语言身份")?;
        let original = legacy["languages"]
            .as_array()
            .ok_or("旧语言清单无效")?
            .iter()
            .find(|entry| entry["id"] == id)
            .ok_or("未知验收语言")?;
        if !seen.insert(id)
            || row["legacy_status"] != original["status"]
            || row["legacy_lint_candidate"] != original["lint"]
        {
            return Err("验收语言重复或旧身份漂移".into());
        }
        exact_keys(
            &row["version_scope"],
            &[
                "qualification",
                "policy",
                "manifest_markers",
                "dialect_requirement",
                "blockers",
            ],
        )?;
        if !matches!(
            row["version_scope"]["qualification"].as_str(),
            Some("unqualified") | Some("v1_qualified")
        ) || row["version_scope"]["policy"]
                != "project_declared_version_and_dialect_must_be_verified"
            || row["version_scope"]["manifest_markers"] != original["markers"]
            || !bounded_text(&row["version_scope"]["dialect_requirement"])
            || strings(&row["version_scope"]["blockers"])?.is_empty()
        {
            return Err("版本方言范围未保留未验收边界".into());
        }
        let builds = strings(&row["build_targets"])?;
        if builds.is_empty() || builds.iter().any(|build| !safe_id(build)) {
            return Err("构建生态维度无效".into());
        }
        if id == "java" && builds != BTreeSet::from(["maven", "gradle"]) {
            return Err("Java Maven/Gradle路径不完整".into());
        }
        let candidates = strings(&row["grammar_candidates"])?;
        let expected_candidates: BTreeSet<&str> = match id {
            "typescript" => BTreeSet::from(["typescript", "javascript", "tsx"]),
            "cfml" => BTreeSet::from(["cfml", "cfquery", "cfscript"]),
            _ if grammar_ids.contains(id) => BTreeSet::from([id]),
            _ => BTreeSet::new(),
        };
        if candidates != expected_candidates {
            return Err("语法候选归属不完整或发生漂移".into());
        }
        seen_grammars.extend(candidates);
        exact_keys(
            &row["capabilities"],
            &["syntax", "documentation", "conventions", "vulnerabilities"],
        )?;
        for (cap, cell) in row["capabilities"].as_object().ok_or("缺核心能力")? {
            exact_keys(cell, &["qualification", "task_refs", "build_paths"])?;
            if !matches!(
                cell["qualification"].as_str(),
                Some("blocked") | Some("v1_qualified")
            ) || strings(&cell["task_refs"])?.is_empty()
            {
                return Err("核心能力越权或缺任务归属".into());
            }
            if strings(&cell["task_refs"])?
                .iter()
                .any(|task| !task.bytes().all(|b| b.is_ascii_digit() || b == b'.'))
            {
                return Err("任务编号无效".into());
            }
            let paths = cell["build_paths"].as_array().ok_or("缺原生构建路径")?;
            let mut path_ids = BTreeSet::new();
            for path in paths {
                exact_keys(
                    path,
                    &[
                        "ecosystem",
                        "implementation_status",
                        "adapter_refs",
                        "evidence_refs",
                        "blockers",
                    ],
                )?;
                let ecosystem = path["ecosystem"].as_str().ok_or("构建路径身份无效")?;
                if !path_ids.insert(ecosystem) || !builds.contains(ecosystem) {
                    return Err("构建路径重复或错配".into());
                }
                let state = path["implementation_status"].as_str().ok_or("缺实现状态")?;
                if !matches!(
                    state,
                    "partial" | "configuration_only" | "wasm_candidate_only" | "not_integrated"
                ) {
                    return Err("计划状态不能授予生产资格".into());
                }
                if state == "wasm_candidate_only"
                    && (cap != "syntax" || expected_candidates.is_empty())
                {
                    return Err("语法资产不能代替其它能力".into());
                }
                let adapters = strings(&path["adapter_refs"])?;
                let evidence = strings(&path["evidence_refs"])?;
                if matches!(state, "partial" | "configuration_only")
                    && (adapters.is_empty() || evidence.is_empty())
                {
                    return Err("局部实现缺来源或证据映射".into());
                }
                if matches!(state, "not_integrated" | "wasm_candidate_only")
                    && (!adapters.is_empty() || !evidence.is_empty())
                {
                    return Err("未整合状态与实现引用矛盾".into());
                }
                if strings(&path["blockers"])?.is_empty() {
                    return Err("缺未完成条件".into());
                }
                for reference in adapters.into_iter().chain(evidence) {
                    if !safe_reference(reference) {
                        return Err("引用不能越界或指向命令".into());
                    }
                    refs.insert(reference);
                }
            }
            if path_ids != builds {
                return Err("每项核心能力必须保留全部构建路径".into());
            }
        }
    }
    if seen != expected || seen_grammars != grammar_ids {
        return Err("规范语言或32份语法归属不完整".into());
    }
    if refs != source_hashes.keys().map(String::as_str).collect() {
        return Err("引用与来源摘要映射不一致".into());
    }
    Ok(doc)
}

fn exact_keys(value: &Value, keys: &[&str]) -> Result<(), String> {
    let map = value.as_object().ok_or("计划对象形状无效")?;
    if map.keys().map(String::as_str).collect::<BTreeSet<_>>() != keys.iter().copied().collect() {
        return Err("计划对象含未知字段或缺字段".into());
    }
    Ok(())
}

fn strings(value: &Value) -> Result<BTreeSet<&str>, String> {
    let values = value
        .as_array()
        .filter(|values| values.len() <= 128)
        .ok_or("计划数组无效或过多")?;
    let mut result = BTreeSet::new();
    for value in values {
        let text = value
            .as_str()
            .filter(|_| bounded_text(value))
            .ok_or("计划文本无效")?;
        if !result.insert(text) {
            return Err("计划数组身份重复".into());
        }
    }
    Ok(result)
}

fn bounded_text(value: &Value) -> bool {
    value.as_str().is_some_and(|text| {
        !text.is_empty() && text.len() <= 2048 && !text.chars().any(char::is_control)
    })
}

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}

fn safe_reference(path: &str) -> bool {
    path.len() <= 256
        && !path.starts_with('/')
        && path.split('/').all(|part| !matches!(part, "" | "." | ".."))
        && path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'_' | b'-' | b'.'))
}

fn valid_digest(digest: &str) -> bool {
    digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
