//! 受控 fix 事件测试

use codeguard_cli::controlled_fix_events::*;

fn create_event(id: &str, event_type: FixEventType) -> FixEvent {
    FixEvent {
        id: id.into(),
        event_type,
        fake_fix: false,
    }
}

#[test]
fn record_noop_event() {
    let event = FixExecutor::record_event(create_event("e1", FixEventType::Noop));
    assert_eq!(event.event_type, FixEventType::Noop);
    assert!(!event.fake_fix);
}

#[test]
fn record_partial_failure() {
    let event = FixExecutor::record_event(create_event("e2", FixEventType::PartialFailure));
    assert_eq!(event.event_type, FixEventType::PartialFailure);
}

#[test]
fn record_concurrent_edit() {
    let event = FixExecutor::record_event(create_event("e3", FixEventType::ConcurrentEdit));
    assert_eq!(event.event_type, FixEventType::ConcurrentEdit);
}

#[test]
fn validate_no_fake_fix() {
    let events = vec![
        create_event("e1", FixEventType::Noop),
        create_event("e2", FixEventType::Success),
    ];
    
    assert!(FixExecutor::validate_events(&events));
}

#[test]
fn validate_rejects_fake_fix() {
    let events = vec![
        FixEvent {
            id: "e1".into(),
            event_type: FixEventType::Success,
            fake_fix: true,
        },
    ];
    
    assert!(!FixExecutor::validate_events(&events));
}
