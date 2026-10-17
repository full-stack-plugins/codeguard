//! 关联轨迹测试

use codeguard_runtime::correlation_trace::*;

fn create_event(phase: &str) -> TraceEvent {
    TraceEvent {
        run_id: "run-1".into(),
        obligation_id: "ob-1".into(),
        task_id: "task-1".into(),
        attempt_id: "att-1".into(),
        phase: phase.into(),
        duration_ms: 100,
    }
}

#[test]
fn record_event() {
    let event = TraceCollector::record(create_event("scan"));
    assert_eq!(event.phase, "scan");
}

#[test]
fn validate_no_leak() {
    let event = create_event("scan");
    assert!(TraceCollector::validate_no_leak(&event));
}

#[test]
fn validate_no_default_telemetry() {
    assert!(TraceCollector::validate_no_default_telemetry());
}
