//! Init 事务测试

use codeguard_cli::init_transaction::*;

#[test]
fn dry_run_success() {
    let record = InitTransaction::dry_run(&["file1".to_string(), "file2".to_string()]);
    assert_eq!(record.status, TransactionStatus::Success);
    assert!(record.recoverable);
}

#[test]
fn apply_success() {
    let record = InitTransaction::apply(&["file1".to_string()], None);
    assert_eq!(record.status, TransactionStatus::Success);
}

#[test]
fn apply_partial_failure() {
    let record = InitTransaction::apply(&["file1".to_string(), "file2".to_string()], Some(1));
    assert_eq!(record.status, TransactionStatus::PartialFailure);
    assert!(record.recoverable);
}

#[test]
fn apply_failure() {
    let record = InitTransaction::apply(&["file1".to_string()], Some(0));
    assert_eq!(record.status, TransactionStatus::Failed);
}

#[test]
fn validate_no_implicit_actions() {
    assert!(InitTransaction::validate_no_implicit_actions());
}

#[test]
fn validate_no_false_success() {
    let record = InitTransaction::apply(&["file1".to_string(), "file2".to_string()], Some(1));
    assert!(InitTransaction::validate_no_false_success(&record));
}
