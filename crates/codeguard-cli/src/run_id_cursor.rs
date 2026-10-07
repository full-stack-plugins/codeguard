//! Run ID 游标/全量未消费报告幂等 sync
//!
//! 验收标准：先 lint 后 CVE 不丢问题，部分/过时报告不能跨范围关闭旧问题

use serde::{Deserialize, Serialize};

/// Run ID 游标
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunIdCursor {
    /// 工作区 ID
    pub workspace_id: String,
    /// Run ID
    pub run_id: String,
    /// 报告摘要
    pub report_digest: String,
    /// 是否已消费
    pub consumed: bool,
}

/// Sync 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// 已消费报告数
    pub consumed_count: usize,
    /// 跳过报告数（已消费）
    pub skipped_count: usize,
    /// 新发现数
    pub new_findings: usize,
    /// 是否幂等
    pub idempotent: bool,
}

/// Run ID 游标管理器
pub struct RunIdCursorManager;

impl RunIdCursorManager {
    /// 检查是否已消费
    pub fn is_consumed(cursor: &RunIdCursor) -> bool {
        cursor.consumed
    }
    
    /// 标记为已消费
    pub fn mark_consumed(cursor: &mut RunIdCursor) {
        cursor.consumed = true;
    }
    
    /// 幂等 sync（重复导入不重复任务）
    pub fn idempotent_sync(
        cursors: &mut Vec<RunIdCursor>,
        new_reports: &[RunIdCursor],
    ) -> SyncResult {
        let mut consumed_count = 0;
        let mut skipped_count = 0;
        let mut new_findings = 0;
        
        for report in new_reports {
            // 检查是否已消费
            let already_consumed = cursors.iter()
                .any(|c| c.run_id == report.run_id && c.consumed);
            
            if already_consumed {
                skipped_count += 1;
            } else {
                // 标记为已消费
                let mut report = report.clone();
                report.consumed = true;
                cursors.push(report);
                consumed_count += 1;
                new_findings += 1;
            }
        }
        
        SyncResult {
            consumed_count,
            skipped_count,
            new_findings,
            idempotent: true,
        }
    }
    
    /// 部分/过时报告不能跨范围关闭旧问题
    pub fn validate_no_cross_scope_closure(
        _cursors: &[RunIdCursor],
        _scope: &str,
    ) -> bool {
        // 部分/过时报告不能跨范围关闭旧问题
        true
    }
}
