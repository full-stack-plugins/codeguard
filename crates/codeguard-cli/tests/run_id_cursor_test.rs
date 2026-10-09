//! Run ID 游标测试

use codeguard_cli::run_id_cursor::*;

fn create_cursor(run_id: &str, consumed: bool) -> RunIdCursor {
    RunIdCursor {
        workspace_id: "ws-1".into(),
        run_id: run_id.into(),
        report_digest: "abc123".into(),
        consumed,
    }
}

#[test]
fn is_consumed_check() {
    let cursor = create_cursor("run-1", true);
    assert!(RunIdCursorManager::is_consumed(&cursor));
    
    let cursor = create_cursor("run-2", false);
    assert!(!RunIdCursorManager::is_consumed(&cursor));
}

#[test]
fn mark_consumed() {
    let mut cursor = create_cursor("run-1", false);
    RunIdCursorManager::mark_consumed(&mut cursor);
    assert!(cursor.consumed);
}

#[test]
fn idempotent_sync_new_report() {
    let mut cursors = vec![];
    let new_reports = vec![create_cursor("run-1", false)];
    
    let result = RunIdCursorManager::idempotent_sync(&mut cursors, &new_reports);
    
    assert_eq!(result.consumed_count, 1);
    assert_eq!(result.new_findings, 1);
    assert!(result.idempotent);
}

#[test]
fn idempotent_sync_duplicate_report() {
    let mut cursors = vec![create_cursor("run-1", true)];
    let new_reports = vec![create_cursor("run-1", false)];
    
    let result = RunIdCursorManager::idempotent_sync(&mut cursors, &new_reports);
    
    assert_eq!(result.skipped_count, 1);
    assert_eq!(result.consumed_count, 0);
}

#[test]
fn idempotent_sync_multiple_reports() {
    let mut cursors = vec![];
    let new_reports = vec![
        create_cursor("run-1", false),
        create_cursor("run-2", false),
        create_cursor("run-1", false), // 重复
    ];
    
    let result = RunIdCursorManager::idempotent_sync(&mut cursors, &new_reports);
    
    assert_eq!(result.consumed_count, 2);
    assert_eq!(result.skipped_count, 1);
}

#[test]
fn no_cross_scope_closure() {
    let cursors = vec![];
    assert!(RunIdCursorManager::validate_no_cross_scope_closure(&cursors, "lint"));
}
