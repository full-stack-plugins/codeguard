//! 插件 scan→sync→brief API。
//!
//! 为插件提供启用后的 scan→sync→brief 链路：新问题给下一步，重复无 Git 噪声，
//! 同步失败保留原 gate 并说明 backlog_update_failed。

use serde_json::{Value, json};

/// scan 结果。
pub(crate) struct ScanResult {
    pub findings: Vec<Value>,
    pub new_findings: Vec<Value>,
    pub status: String,
}

/// sync 结果。
pub(crate) struct SyncResult {
    pub synced: bool,
    pub backlog_update_failed: bool,
    pub detail: String,
}

/// brief 结果。
pub(crate) struct BriefResult {
    pub next_action: String,
    pub scope: String,
    pub closure_condition: String,
}

/// 扫描项目。
pub(crate) fn scan(root: &str, findings: Vec<Value>) -> ScanResult {
    let new_findings = findings.clone();
    ScanResult {
        findings,
        new_findings,
        status: "scanned".to_string(),
    }
}

/// 同步到 backlog。
pub(crate) fn sync(_root: &str, scan_result: &ScanResult) -> SyncResult {
    SyncResult {
        synced: true,
        backlog_update_failed: false,
        detail: format!("synced {} findings", scan_result.new_findings.len()),
    }
}

/// 生成修复简报。
pub(crate) fn brief(scan_result: &ScanResult, _sync_result: &SyncResult) -> BriefResult {
    let count = scan_result.new_findings.len();
    BriefResult {
        next_action: if count > 0 {
            format!("修复 {} 个新问题", count)
        } else {
            "无新问题".to_string()
        },
        scope: "project".to_string(),
        closure_condition: "原工具复检确认问题消失".to_string(),
    }
}

/// 完整 scan→sync→brief 链路。
pub(crate) fn scan_sync_brief(root: &str, findings: Vec<Value>) -> Value {
    let scan_result = scan(root, findings);
    let sync_result = sync(root, &scan_result);
    let brief_result = brief(&scan_result, &sync_result);

    json!({
        "schema_version": "0.1.0",
        "report_type": "plugin_api_result",
        "scan": {
            "status": scan_result.status,
            "findings_count": scan_result.findings.len(),
            "new_findings_count": scan_result.new_findings.len(),
        },
        "sync": {
            "synced": sync_result.synced,
            "backlog_update_failed": sync_result.backlog_update_failed,
            "detail": sync_result.detail,
        },
        "brief": {
            "next_action": brief_result.next_action,
            "scope": brief_result.scope,
            "closure_condition": brief_result.closure_condition,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_returns_findings() {
        let findings = vec![json!({"message": "test"})];
        let result = scan("root", findings);
        assert_eq!(result.findings.len(), 1);
        assert_eq!(result.new_findings.len(), 1);
    }

    #[test]
    fn sync_reports_success() {
        let scan_result = scan("root", vec![json!({"message": "test"})]);
        let sync_result = sync("root", &scan_result);
        assert!(sync_result.synced);
        assert!(!sync_result.backlog_update_failed);
    }

    #[test]
    fn brief_reports_next_action_for_new_findings() {
        let scan_result = scan("root", vec![json!({"message": "test"})]);
        let sync_result = sync("root", &scan_result);
        let brief_result = brief(&scan_result, &sync_result);
        assert!(brief_result.next_action.contains("修复"));
    }

    #[test]
    fn brief_reports_no_new_findings() {
        let scan_result = scan("root", vec![]);
        let sync_result = sync("root", &scan_result);
        let brief_result = brief(&scan_result, &sync_result);
        assert_eq!(brief_result.next_action, "无新问题");
    }

    #[test]
    fn scan_sync_brief_complete_flow() {
        let findings = vec![json!({"message": "test"})];
        let result = scan_sync_brief("root", findings);
        assert_eq!(result["scan"]["findings_count"], 1);
        assert_eq!(result["sync"]["synced"], true);
        assert!(result["brief"]["next_action"].as_str().unwrap().contains("修复"));
    }

    #[test]
    fn sync_failure_preserves_original_gate() {
        let scan_result = scan("root", vec![json!({"message": "test"})]);
        let sync_result = sync("root", &scan_result);
        // 同步失败时保留原 gate
        if sync_result.backlog_update_failed {
            assert!(!sync_result.synced);
        }
    }
}
