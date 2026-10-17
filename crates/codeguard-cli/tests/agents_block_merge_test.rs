//! AGENTS 区块合并测试

use codeguard_cli::agents_block_merge::*;

#[test]
fn merge_success() {
    let result = AgentsBlockMerger::merge("# Existing\n", "# New\n");
    assert!(result.success);
    assert!(result.conflict.is_none());
}

#[test]
fn merge_duplicate_marker() {
    let result = AgentsBlockMerger::merge("<!-- CODEGUARD -->\n<!-- CODEGUARD -->\n", "# New\n");
    assert!(!result.success);
    assert_eq!(result.conflict, Some("duplicate_marker".into()));
}

#[test]
fn validate_identity_protection() {
    assert!(AgentsBlockMerger::validate_identity_protection("# Existing\n", "# Existing\n# New\n"));
}

#[test]
fn check_concurrent_conflict() {
    assert!(AgentsBlockMerger::check_concurrent_conflict("# Original\n", "# Modified\n"));
}
