//! Project schema 测试

use codeguard_cli::project_schema::*;

#[test]
fn generate_project_schema() {
    let schema = SchemaGenerator::generate_project(
        "product-1",
        "target-1",
        "1.0.0",
        Some("1.0.0"),
        &["/project".to_string()],
    );
    
    assert_eq!(schema.source_identity.product, "product-1");
    assert_eq!(schema.source_identity.target, "target-1");
    assert_eq!(schema.source_identity.resolved_version, "1.0.0");
    assert!(schema.source_identity.local_version.is_some());
}

#[test]
fn generate_module_graph() {
    let modules = vec!["mod-a".to_string(), "mod-b".to_string()];
    let edges = vec![("mod-a".to_string(), "mod-b".to_string())];
    
    let graph = SchemaGenerator::generate_module_graph(&modules, &edges);
    
    assert_eq!(graph.modules.len(), 2);
    assert_eq!(graph.edges.len(), 1);
}

#[test]
fn validate_no_guessing() {
    let schema = SchemaGenerator::generate_project(
        "product-1",
        "target-1",
        "1.0.0",
        None,
        &[],
    );
    
    assert!(SchemaGenerator::validate_no_guessing(&schema));
}
