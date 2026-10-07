//! Tools 库存测试

use codeguard_adapters::tools_inventory::*;

fn create_entry(name: &str, installed: bool, launchable: bool) -> ToolInventoryEntry {
    ToolInventoryEntry {
        name: name.into(),
        version: "1.0.0".into(),
        artifact_identity: "abc123".into(),
        installed,
        launchable,
    }
}

#[test]
fn list_tools() {
    let entries = vec![
        create_entry("ruff", true, true),
        create_entry("eslint", false, false),
    ];
    
    let names = ToolsInventory::list(&entries);
    assert_eq!(names, vec!["ruff".to_string(), "eslint".to_string()]);
}

#[test]
fn verify_identity_success() {
    let entry = create_entry("ruff", true, true);
    assert!(ToolsInventory::verify_identity(&entry));
}

#[test]
fn verify_identity_empty_fails() {
    let entry = ToolInventoryEntry {
        name: "ruff".into(),
        version: "1.0.0".into(),
        artifact_identity: "".into(),
        installed: true,
        launchable: true,
    };
    assert!(!ToolsInventory::verify_identity(&entry));
}

#[test]
fn identity_pass_not_launchable() {
    // 身份通过不等于可启动
    let entry = create_entry("ruff", false, true);
    assert!(ToolsInventory::verify_identity(&entry));
    assert!(!ToolsInventory::is_launchable(&entry));
}

#[test]
fn create_install_preview() {
    let preview = ToolsInventory::create_preview("ruff");
    assert!(preview.preview_only);
    assert!(preview.requires_explicit_install);
}

#[test]
fn create_preparation_evidence() {
    let evidence = ToolsInventory::create_preparation_evidence("run-1", "ruff", true, true);
    assert_eq!(evidence.run_id, "run-1");
    assert!(evidence.identity_verified);
    assert!(evidence.launchable);
    assert!(evidence.idempotent_sync);
}

#[test]
fn partial_install_recovery() {
    assert!(ToolsInventory::validate_partial_install_recovery(
        &["ruff".to_string()],
        &["eslint".to_string()],
    ));
    
    assert!(!ToolsInventory::validate_partial_install_recovery(
        &["ruff".to_string()],
        &[],
    ));
}
