//! Finding 身份测试

use codeguard_cli::finding_identity::*;

#[test]
fn generate_stable_identity() {
    let id1 = IdentityGenerator::generate("main.rs", "unused_var", "let x = 1;");
    let id2 = IdentityGenerator::generate("main.rs", "unused_var", "let x = 1;");
    
    assert_eq!(id1, id2);
}

#[test]
fn line_change_no_duplicate() {
    assert!(IdentityGenerator::same_finding_despite_line_change(
        "main.rs", "unused_var", "let x = 1;", 10, 20
    ));
}

#[test]
fn match_identity_rename() {
    let id1 = IdentityGenerator::generate("main.rs", "unused_var", "let x = 1;");
    let id2 = IdentityGenerator::generate("renamed.rs", "unused_var", "let x = 1;");
    
    // 重命名后仍匹配（规则和内容指纹相同）
    assert!(IdentityGenerator::match_identity(&id1, &id2));
}

#[test]
fn ten_scans_one_problem() {
    let mut registry = IdentityRegistry::new();
    
    for _ in 0..10 {
        let id = IdentityGenerator::generate("main.rs", "unused_var", "let x = 1;");
        registry.register(id);
    }
    
    assert_eq!(registry.unique_count(), 1);
}

#[test]
fn different_content_different_identity() {
    let id1 = IdentityGenerator::generate("main.rs", "unused_var", "let x = 1;");
    let id2 = IdentityGenerator::generate("main.rs", "unused_var", "let y = 2;");
    
    assert!(!IdentityGenerator::match_identity(&id1, &id2));
}

#[test]
fn uncertain_match_not_closed() {
    let id1 = IdentityGenerator::generate("main.rs", "unused_var", "let x = 1;");
    let id2 = IdentityGenerator::generate("main.rs", "unused_var", "let z = 3;");
    
    // 不确定匹配不误关闭
    assert!(!IdentityGenerator::match_identity(&id1, &id2));
}
