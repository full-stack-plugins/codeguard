//! 工具解析测试

use codeguard_adapters::tool_resolution::*;

#[test]
fn resolve_tool_success() {
    let result = ToolResolver::resolve("ruff", ToolSource::System, "0.16.8", "abc123");
    assert!(result.success);
    assert!(result.identity.is_some());
}

#[test]
fn verify_lock_match() {
    let identity = ToolIdentity {
        name: "ruff".into(),
        version: "0.16.8".into(),
        path: "/usr/bin/ruff".into(),
        sha256: "abc123".into(),
        source: ToolSource::System,
    };
    let lock = ToolLockEntry {
        name: "ruff".into(),
        version: "0.16.8".into(),
        sha256: "abc123".into(),
    };
    
    assert!(ToolResolver::verify_lock(&identity, &lock));
}

#[test]
fn verify_lock_mismatch() {
    let identity = ToolIdentity {
        name: "ruff".into(),
        version: "0.16.8".into(),
        path: "/usr/bin/ruff".into(),
        sha256: "abc123".into(),
        source: ToolSource::System,
    };
    let lock = ToolLockEntry {
        name: "ruff".into(),
        version: "0.16.9".into(),
        sha256: "abc123".into(),
    };
    
    assert!(!ToolResolver::verify_lock(&identity, &lock));
}

#[test]
fn missing_tool_recovery_steps() {
    let steps = ToolResolver::missing_tool_recovery("ruff");
    assert!(!steps.is_empty());
    assert!(steps[0].contains("ruff"));
}

#[test]
fn doctor_check_success() {
    let result = ToolResolver::doctor_check("ruff", "0.16.8", "abc123");
    assert!(result.success);
}

#[test]
fn doctor_check_missing_tool() {
    let result = ToolResolver::doctor_check("ruff", "", "");
    assert!(!result.success);
    assert!(result.recovery_steps.is_some());
}

#[test]
fn tool_source_wrapper() {
    let result = ToolResolver::resolve("ruff", ToolSource::Wrapper, "0.16.8", "abc123");
    assert!(result.success);
    assert_eq!(result.identity.unwrap().source, ToolSource::Wrapper);
}

#[test]
fn tool_source_managed_cache() {
    let result = ToolResolver::resolve("ruff", ToolSource::ManagedCache, "0.16.8", "abc123");
    assert!(result.success);
    assert_eq!(result.identity.unwrap().source, ToolSource::ManagedCache);
}
