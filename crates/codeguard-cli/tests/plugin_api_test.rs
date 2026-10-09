//! Plugin API 测试

use codeguard_cli::plugin_api::*;

#[test]
fn scan_success() {
    let result = PluginApi::scan(5);
    assert!(result.success);
    assert_eq!(result.findings, 5);
}

#[test]
fn sync_success() {
    let result = PluginApi::sync(true);
    assert!(result.success);
    assert!(result.reason.is_none());
}

#[test]
fn sync_failure() {
    let result = PluginApi::sync(false);
    assert!(!result.success);
    assert_eq!(result.reason, Some("backlog_update_failed".into()));
}

#[test]
fn brief_with_findings() {
    let brief = PluginApi::brief(3);
    assert_eq!(brief.new_findings, 3);
    assert!(!brief.next_steps.is_empty());
    assert!(!brief.git_noise);
}

#[test]
fn brief_no_findings() {
    let brief = PluginApi::brief(0);
    assert_eq!(brief.new_findings, 0);
    assert!(brief.next_steps.is_empty());
}
