//! Manifest/锁/wrapper 静态观察测试

use codeguard_cli::manifest_lock_observer::*;

#[test]
fn observe_manifest() {
    let observation = StaticObserver::observe("package.json", "1.0.0", Some("1.0.0"));
    
    assert_eq!(observation.manifest_path, "package.json");
    assert_eq!(observation.declared_version, "1.0.0");
    assert_eq!(observation.resolved_version, Some("1.0.0".to_string()));
}

#[test]
fn no_script_execution() {
    let observation = StaticObserver::observe("package.json", "1.0.0", None);
    assert!(StaticObserver::validate_no_script_execution(&observation));
}

#[test]
fn validate_versions() {
    let observation = StaticObserver::observe("package.json", "1.0.0", Some("1.0.0"));
    assert!(StaticObserver::validate_versions(&observation));
}

#[test]
fn validate_versions_no_resolved() {
    let observation = StaticObserver::observe("package.json", "1.0.0", None);
    assert!(StaticObserver::validate_versions(&observation));
}
