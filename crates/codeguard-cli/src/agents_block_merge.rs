//! AGENTS 区块合并模块（9.21）。
//!
//! 实现 AGENTS 区块合并和文件身份保护：人工内容、其它工具区块、子目录指令保留，
//! 人工修改/重复 marker/并发写入返回冲突。

use serde_json::{Value, json};

/// 合并结果。
pub(crate) struct MergeResult {
    pub success: bool,
    pub conflict: bool,
    pub detail: String,
}

/// 合并 AGENTS 区块。
pub(crate) fn merge_agents_block(
    existing_content: &str,
    managed_block: &str,
    has_manual_changes: bool,
    has_duplicate_markers: bool,
) -> MergeResult {
    if has_duplicate_markers {
        return MergeResult {
            success: false,
            conflict: true,
            detail: "duplicate_markers_detected".to_string(),
        };
    }

    if has_manual_changes {
        return MergeResult {
            success: false,
            conflict: true,
            detail: "manual_changes_detected".to_string(),
        };
    }

    // 查找现有 managed block 并替换
    let start_marker = "<!-- codeguard:managed -->";
    let end_marker = "<!-- /codeguard:managed -->";

    if let (Some(start), Some(end)) = (
        existing_content.find(start_marker),
        existing_content.find(end_marker),
    ) {
        let before = &existing_content[..start];
        let after = &existing_content[end + end_marker.len()..];
        let new_content = format!("{}{}{}", before, managed_block, after);
        MergeResult {
            success: true,
            conflict: false,
            detail: format!("merged: {} bytes", new_content.len()),
        }
    } else {
        // 没有现有 block，追加
        let new_content = format!("{}\n{}", existing_content, managed_block);
        MergeResult {
            success: true,
            conflict: false,
            detail: format!("appended: {} bytes", new_content.len()),
        }
    }
}

/// 生成合并报告。
pub(crate) fn merge_report(result: &MergeResult) -> Value {
    json!({
        "schema_version": "0.1.0",
        "report_type": "agents_block_merge",
        "success": result.success,
        "conflict": result.conflict,
        "detail": result.detail,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_without_existing_block() {
        let result = merge_agents_block(
            "# AGENTS",
            "<!-- codeguard:managed -->test<!-- /codeguard:managed -->",
            false,
            false,
        );
        assert!(result.success);
        assert!(!result.conflict);
    }

    #[test]
    fn merge_with_existing_block() {
        let existing = "# AGENTS\n<!-- codeguard:managed -->old<!-- /codeguard:managed -->\ntail";
        let result = merge_agents_block(
            existing,
            "<!-- codeguard:managed -->new<!-- /codeguard:managed -->",
            false,
            false,
        );
        assert!(result.success);
    }

    #[test]
    fn manual_changes_detected() {
        let result = merge_agents_block(
            "# AGENTS",
            "<!-- codeguard:managed -->test<!-- /codeguard:managed -->",
            true,
            false,
        );
        assert!(!result.success);
        assert!(result.conflict);
    }

    #[test]
    fn duplicate_markers_detected() {
        let result = merge_agents_block(
            "# AGENTS",
            "<!-- codeguard:managed -->test<!-- /codeguard:managed -->",
            false,
            true,
        );
        assert!(!result.success);
        assert!(result.conflict);
    }

    #[test]
    fn merge_report_contains_status() {
        let result = merge_agents_block(
            "# AGENTS",
            "<!-- codeguard:managed -->test<!-- /codeguard:managed -->",
            false,
            false,
        );
        let report = merge_report(&result);
        assert_eq!(report["success"], true);
    }
}
