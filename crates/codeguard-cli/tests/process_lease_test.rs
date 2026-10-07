//! 跨进程租约测试

use codeguard_cli::process_lease::*;

#[test]
fn claim_lease_success() {
    let manager = LeaseManager::new();
    assert!(manager.claim("ws-1", "agent-1", 100).unwrap());
}

#[test]
fn claim_lease_conflict() {
    let manager = LeaseManager::new();
    manager.claim("ws-1", "agent-1", 100).unwrap();
    
    // 同工作区只有一个有效领取者
    assert!(!manager.claim("ws-1", "agent-2", 100).unwrap());
}

#[test]
fn heartbeat_success() {
    let manager = LeaseManager::new();
    manager.claim("ws-1", "agent-1", 100).unwrap();
    
    assert!(manager.heartbeat("ws-1", "agent-1", 200).unwrap());
}

#[test]
fn release_lease() {
    let manager = LeaseManager::new();
    manager.claim("ws-1", "agent-1", 100).unwrap();
    
    assert!(manager.release("ws-1", "agent-1").unwrap());
    
    // 释放后可再次领取
    assert!(manager.claim("ws-1", "agent-2", 100).unwrap());
}

#[test]
fn expired_lease_recoverable() {
    let manager = LeaseManager::new();
    manager.claim("ws-1", "agent-1", 100).unwrap();
    
    // 过期检查
    let expired = manager.check_expired(200);
    assert_eq!(expired.len(), 1);
    
    // 过期后可再次领取
    assert!(manager.claim("ws-1", "agent-2", 300).unwrap());
}
