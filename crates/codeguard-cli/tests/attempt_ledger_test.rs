//! 尝试记录测试

use codeguard_cli::attempt_ledger::*;

#[test]
fn start_attempt() {
    let ledger = AttemptManager::start("token-1", 1, "action-1", "attempt-1");
    assert_eq!(ledger.token, "token-1");
    assert!(!ledger.finished);
}

#[test]
fn finish_idempotent() {
    let mut ledger = AttemptManager::start("token-1", 1, "action-1", "attempt-1");
    
    assert!(AttemptManager::finish(&mut ledger));
    assert!(!AttemptManager::finish(&mut ledger)); // 重复 finish 不重复预算
}

#[test]
fn validate_no_old_owner_write() {
    assert!(AttemptManager::validate_no_old_owner_write("old-token", "new-token"));
    assert!(!AttemptManager::validate_no_old_owner_write("token-1", "token-1"));
}
