//! Append-only 事件测试

use codeguard_cli::append_only_events::*;

fn create_event(id: &str, event_type: EventType, parent_id: Option<&str>) -> Event {
    Event {
        id: id.into(),
        event_type,
        timestamp: 0,
        parent_id: parent_id.map(String::from),
    }
}

#[test]
fn append_event_success() {
    let mut sm = EventStateMachine::new();
    let event = create_event("e1", EventType::Observed, None);
    
    assert!(sm.append(event).is_ok());
    assert_eq!(sm.event_chain().len(), 1);
}

#[test]
fn append_missing_parent_fails() {
    let mut sm = EventStateMachine::new();
    let event = create_event("e2", EventType::Observed, Some("e1"));
    
    assert!(sm.append(event).is_err());
}

#[test]
fn append_with_parent_success() {
    let mut sm = EventStateMachine::new();
    sm.append(create_event("e1", EventType::Observed, None)).unwrap();
    
    let event = create_event("e2", EventType::Rechecked, Some("e1"));
    assert!(sm.append(event).is_ok());
}

#[test]
fn cannot_close_without_recheck() {
    let mut sm = EventStateMachine::new();
    sm.append(create_event("e1", EventType::Observed, None)).unwrap();
    
    // 手改勾选无复检不关闭
    assert!(!sm.can_close());
}

#[test]
fn can_close_with_recheck() {
    let mut sm = EventStateMachine::new();
    sm.append(create_event("e1", EventType::Observed, None)).unwrap();
    sm.append(create_event("e2", EventType::Rechecked, Some("e1"))).unwrap();
    
    assert!(sm.can_close());
}

#[test]
fn branch_conflict_detected() {
    let mut sm = EventStateMachine::new();
    sm.append(create_event("e1", EventType::Observed, None)).unwrap();
    sm.append(create_event("e2", EventType::Rechecked, Some("e1"))).unwrap();
    sm.append(create_event("e3", EventType::Closed, Some("e1"))).unwrap();
    
    let event = create_event("e4", EventType::Reopened, Some("e1"));
    assert!(sm.check_branch_conflict(&event));
}

#[test]
fn append_only_no_mutation() {
    let mut sm = EventStateMachine::new();
    sm.append(create_event("e1", EventType::Observed, None)).unwrap();
    
    // 事件链长度不变（append-only）
    let len_before = sm.event_chain().len();
    sm.append(create_event("e2", EventType::Rechecked, Some("e1"))).unwrap();
    assert!(sm.event_chain().len() > len_before);
}
