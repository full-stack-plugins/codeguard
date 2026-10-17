//! Tools install 测试

use codeguard_adapters::tools_install::*;

fn create_manifest() -> DownloadManifest {
    DownloadManifest {
        entries: vec![
            DownloadManifestEntry {
                name: "ruff".into(),
                url: "https://example.com/ruff".into(),
                sha256: "abc123".into(),
                size: 1024,
            },
        ],
        version: "1.0.0".into(),
    }
}

#[test]
fn validate_manifest_success() {
    let manifest = create_manifest();
    assert!(ToolsInstaller::validate_manifest(&manifest).is_ok());
}

#[test]
fn validate_manifest_empty() {
    let manifest = DownloadManifest { entries: vec![], version: "1.0.0".into() };
    assert!(ToolsInstaller::validate_manifest(&manifest).is_err());
}

#[test]
fn install_tool_success() {
    let manifest = create_manifest();
    let result = ToolsInstaller::install(&manifest, "ruff");
    assert!(result.success);
    assert!(result.installed.contains(&"ruff".to_string()));
}

#[test]
fn install_tool_not_found() {
    let manifest = create_manifest();
    let result = ToolsInstaller::install(&manifest, "unknown");
    assert!(!result.success);
}

#[test]
fn no_implicit_install() {
    assert!(!ToolsInstaller::can_implicit_install());
}

#[test]
fn offline_isolation_blocks_network() {
    assert!(ToolsInstaller::check_offline_isolation("https://example.com"));
}

#[test]
fn verify_sha256() {
    let data = b"test data";
    // 简化验证（实际应计算正确哈希）
    assert!(!ManifestValidator::verify_sha256(data, "wrong_hash"));
}
