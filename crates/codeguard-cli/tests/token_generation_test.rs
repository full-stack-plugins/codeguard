//! Token/generation 测试

use codeguard_cli::token_generation::*;

#[test]
fn finish_idempotent() {
    let result = TokenGenerationManager::finish("token-1", 1, 1);
    assert!(result.idempotent);
    assert!(!result.new_event);
}

#[test]
fn finish_new_event() {
    let result = TokenGenerationManager::finish("token-1", 2, 1);
    assert!(!result.idempotent);
    assert!(result.new_event);
}

#[test]
fn validate_idempotent() {
    let result = TokenGenerationManager::finish("token-1", 1, 1);
    assert!(TokenGenerationManager::validate_idempotent(&result));
}

#[test]
fn validate_generation_change() {
    assert!(TokenGenerationManager::validate_generation_change(2, 1));
    assert!(!TokenGenerationManager::validate_generation_change(1, 1));
}
