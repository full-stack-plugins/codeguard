//! Architecture.md 生成测试

use codeguard_cli::architecture_md::*;

#[test]
fn generate_architecture_doc() {
    let doc = ArchitectureDocGenerator::generate(
        &["DDD".to_string()],
        &["mod-a".to_string()],
    );
    
    assert_eq!(doc.dimensions.len(), 1);
    assert_eq!(doc.modules.len(), 1);
}

#[test]
fn validate_consistency() {
    let doc = ArchitectureDocGenerator::generate(&[], &[]);
    assert!(ArchitectureDocGenerator::validate_consistency(&doc));
}

#[test]
fn validate_inference_no_blocking() {
    assert!(ArchitectureDocGenerator::validate_inference_no_blocking());
}
