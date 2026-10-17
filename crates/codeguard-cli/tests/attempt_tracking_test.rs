//! 尝试跟踪测试

use codeguard_cli::attempt_tracking::*;

fn create_record(id: &str, action: &str, result: AttemptResult) -> AttemptRecord {
    AttemptRecord {
        id: id.into(),
        action: action.into(),
        patch_identity: "abc123".into(),
        result,
        counted: true,
    }
}

#[test]
fn record_attempt() {
    let tracker = AttemptTracker::new(3);
    tracker.record(create_record("a1", "fix", AttemptResult::Success));
    
    assert_eq!(tracker.history().len(), 1);
}

#[test]
fn no_progress_counting() {
    let tracker = AttemptTracker::new(3);
    
    tracker.record(create_record("a1", "fix", AttemptResult::NoChange));
    tracker.count_no_progress();
    
    assert!(!tracker.is_exhausted("fix"));
}

#[test]
fn exhausted_after_max() {
    let tracker = AttemptTracker::new(2);
    
    tracker.count_no_progress();
    tracker.count_no_progress();
    
    assert!(tracker.is_exhausted("fix"));
}

#[test]
fn failure_counts() {
    let tracker = AttemptTracker::new(3);
    tracker.record(create_record("a1", "fix", AttemptResult::Failure));
    
    assert_eq!(tracker.history().len(), 1);
}

#[test]
fn abandoned_counts() {
    let tracker = AttemptTracker::new(3);
    tracker.record(create_record("a1", "fix", AttemptResult::Abandoned));
    
    assert_eq!(tracker.history().len(), 1);
}
