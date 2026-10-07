//! Init dry-run 测试

use codeguard_cli::init_dry_run::*;

#[test]
fn dry_run_does_not_create_files() {
    let result = InitExecutor::execute(
        InitOperation::DryRun,
        &[],
        &[".codeguard/config.json".to_string()],
    );
    
    assert!(result.success);
    assert!(result.created_files.is_empty());
}

#[test]
fn apply_creates_new_files() {
    let result = InitExecutor::execute(
        InitOperation::Apply,
        &[],
        &[".codeguard/config.json".to_string()],
    );
    
    assert!(result.success);
    assert_eq!(result.created_files.len(), 1);
}

#[test]
fn apply_does_not_overwrite_user_files() {
    let result = InitExecutor::execute(
        InitOperation::Apply,
        &[".codeguard/config.json".to_string()],
        &[".codeguard/config.json".to_string()],
    );
    
    assert!(result.success);
    assert!(result.created_files.is_empty());
    assert_eq!(result.skipped_files.len(), 1);
}

#[test]
fn cannot_duplicate_config() {
    let existing = vec![".codeguard/config.json".to_string()];
    assert!(!InitExecutor::can_duplicate_config(&existing, ".codeguard/config.json"));
    assert!(InitExecutor::can_duplicate_config(&existing, ".codeguard/other.json"));
}

#[test]
fn private_cache_when_not_initialized() {
    assert!(InitExecutor::use_private_cache(false));
    assert!(!InitExecutor::use_private_cache(true));
}
