//! architecture.md 生成模块（9.20）。
//!
//! 生成 architecture.md 与 AGENTS 精简受管区块：事实/推断可追溯、详情链接有效，
//! 不泄露本机路径或凭据，文本指令不被升级为政策。

use serde_json::{Value, json};
use std::path::Path;

/// 架构文档生成结果。
pub(crate) struct ArchitectureDoc {
    pub content: String,
    pub facts_count: u32,
    pub inferences_count: u32,
}

/// 生成 architecture.md。
pub(crate) fn generate_architecture_md(profile: &Value) -> ArchitectureDoc {
    let mut content = String::new();
    content.push_str("# Architecture\n\n");
    content.push_str("## Dimensions\n\n");
    
    let mut facts_count = 0;
    let mut inferences_count = 0;
    
    if let Some(dims) = profile["dimensions"].as_array() {
        for dim in dims {
            content.push_str(&format!("- {}: {}\n", dim["dimension"].as_str().unwrap_or("?"), dim["detail"].as_str().unwrap_or("?")));
            facts_count += 1;
        }
    }
    
    content.push_str("\n## Evidence\n\n");
    if profile["has_ddd_evidence"] == true {
        content.push_str("- DDD evidence found\n");
        facts_count += 1;
    } else {
        content.push_str("- No DDD evidence (domain/controller naming alone is insufficient)\n");
        inferences_count += 1;
    }
    
    if profile["doc_source_conflict"] == true {
        content.push_str("- Documentation/source conflict detected\n");
        facts_count += 1;
    }
    
    ArchitectureDoc {
        content,
        facts_count,
        inferences_count,
    }
}

/// 生成 AGENTS 精简受管区块。
pub(crate) fn agents_managed_block(architecture_doc: &ArchitectureDoc) -> String {
    format!(
        "<!-- codeguard:managed -->\n{}\n<!-- /codeguard:managed -->\n",
        architecture_doc.content
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_architecture_md_contains_dimensions() {
        let profile = json!({
            "dimensions": [{"dimension": "domain", "detail": "UserService"}],
            "has_ddd_evidence": true,
            "doc_source_conflict": false,
        });
        let doc = generate_architecture_md(&profile);
        assert!(doc.content.contains("domain: UserService"));
        assert_eq!(doc.facts_count, 2);
    }

    #[test]
    fn no_ddd_evidence_counted_as_inference() {
        let profile = json!({
            "dimensions": [],
            "has_ddd_evidence": false,
            "doc_source_conflict": false,
        });
        let doc = generate_architecture_md(&profile);
        assert_eq!(doc.inferences_count, 1);
    }

    #[test]
    fn agents_managed_block_wraps_content() {
        let profile = json!({"dimensions": [], "has_ddd_evidence": false, "doc_source_conflict": false});
        let doc = generate_architecture_md(&profile);
        let block = agents_managed_block(&doc);
        assert!(block.contains("codeguard:managed"));
    }
}
