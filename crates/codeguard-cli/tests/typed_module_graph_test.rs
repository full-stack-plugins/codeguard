//! 分类型模块图测试

use codeguard_cli::typed_module_graph::*;

#[test]
fn generate_complete_graph() {
    let modules = vec!["mod-a".to_string(), "mod-b".to_string()];
    let edges = vec![
        ("mod-a".to_string(), "mod-b".to_string(), DependencyType::Build),
    ];
    
    let graph = TypedModuleGraphGenerator::generate(&modules, &edges);
    
    assert!(graph.complete);
    assert_eq!(graph.edges.len(), 1);
    assert!(graph.edges[0].resolved);
}

#[test]
fn generate_incomplete_graph() {
    let modules = vec!["mod-a".to_string(), "mod-b".to_string()];
    let edges = vec![
        ("mod-a".to_string(), "mod-b".to_string(), DependencyType::DynamicUnknown),
    ];
    
    let graph = TypedModuleGraphGenerator::generate(&modules, &edges);
    
    assert!(!graph.complete);
    assert!(!graph.edges[0].resolved);
}

#[test]
fn check_improper_aggregation() {
    let modules = vec!["mod-a".to_string()];
    let edges = vec![
        ("mod-a".to_string(), "mod-b".to_string(), DependencyType::DynamicUnknown),
    ];
    
    let graph = TypedModuleGraphGenerator::generate(&modules, &edges);
    assert!(!TypedModuleGraphGenerator::check_improper_aggregation(&graph));
}

#[test]
fn validate_no_stale_codegraph() {
    let modules = vec!["mod-a".to_string()];
    let edges = vec![
        ("mod-a".to_string(), "mod-b".to_string(), DependencyType::Build),
    ];
    
    let graph = TypedModuleGraphGenerator::generate(&modules, &edges);
    assert!(TypedModuleGraphGenerator::validate_no_stale_codegraph(&graph));
}
