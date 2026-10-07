//! 进程组取消测试

use codeguard_runtime::process_cancellation::*;

#[test]
fn cancel_state_initial() {
    let state = CancelState::new();
    assert!(!state.is_cancelled());
    assert_eq!(state.reason(), None);
}

#[test]
fn cancel_state_cancel() {
    let state = CancelState::new();
    state.cancel(CancelReason::Timeout);
    assert!(state.is_cancelled());
    assert_eq!(state.reason(), Some(CancelReason::Timeout));
}

#[test]
fn cancel_state_ctrl_c() {
    let state = CancelState::new();
    state.cancel(CancelReason::CtrlC);
    assert!(state.is_cancelled());
    assert_eq!(state.reason(), Some(CancelReason::CtrlC));
}

#[test]
fn process_group_register_and_cancel() {
    let manager = ProcessGroupManager::new();
    manager.register_pgid(12345);
    manager.cancel_all(CancelReason::Manual);
    
    assert!(manager.cancel_state().is_cancelled());
    assert_eq!(manager.cancel_state().reason(), Some(CancelReason::Manual));
}

#[test]
fn process_group_timeout_cancel() {
    let manager = ProcessGroupManager::new();
    manager.register_pgid(12345);
    manager.cancel_on_timeout(10);
    
    std::thread::sleep(std::time::Duration::from_millis(50));
    assert!(manager.cancel_state().is_cancelled());
    assert_eq!(manager.cancel_state().reason(), Some(CancelReason::Timeout));
}

#[test]
fn process_group_reap() {
    let manager = ProcessGroupManager::new();
    manager.register_pgid(12345);
    manager.reap(); // 不应 panic
}

#[test]
fn cancellation_queue_enqueue_and_process() {
    let queue = CancellationQueue::new();
    queue.enqueue(12345);
    queue.enqueue(67890);
    
    let cancelled = queue.process();
    assert_eq!(cancelled.len(), 2);
    assert!(cancelled.contains(&12345));
    assert!(cancelled.contains(&67890));
}

#[test]
fn cancellation_queue_empty() {
    let queue = CancellationQueue::new();
    let cancelled = queue.process();
    assert!(cancelled.is_empty());
}

#[test]
fn queued_cancel_reason() {
    let state = CancelState::new();
    state.cancel(CancelReason::Queued);
    assert_eq!(state.reason(), Some(CancelReason::Queued));
}
