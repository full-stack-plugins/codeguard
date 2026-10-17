//! 构建模型解析测试

use codeguard_adapters::build_model_resolution::*;

fn create_root(path: &str, state: BuildModelState) -> BuildRoot {
    BuildRoot {
        path: path.into(),
        build_system: "maven".into(),
        state,
        pending_conditions: vec![],
    }
}

#[test]
fn resolve_resolved_roots() {
    let roots = vec![
        create_root("/project", BuildModelState::Resolved),
        create_root("/project2", BuildModelState::Pending),
    ];
    
    let result = BuildModelResolver::resolve(&roots);
    
    assert_eq!(result.resolved.len(), 1);
    assert_eq!(result.pending.len(), 1);
    assert_eq!(result.obligations.len(), 6); // 6 categories
}

#[test]
fn resolve_pending_roots() {
    let roots = vec![
        create_root("/project", BuildModelState::Pending),
    ];
    
    let result = BuildModelResolver::resolve(&roots);
    assert_eq!(result.pending.len(), 1);
    assert!(result.obligations.is_empty());
}

#[test]
fn only_pending_conditions() {
    let mut root = create_root("/project", BuildModelState::Pending);
    root.pending_conditions = vec!["parent_pom_unresolved".into()];
    
    assert!(BuildModelResolver::only_pending_conditions(&[root]));
}

#[test]
fn validate_execution_evidence() {
    let obligations = vec![
        ScanObligation {
            id: "1".into(),
            build_root: "/project".into(),
            category: "lint".into(),
            resolved: true,
        },
    ];
    
    assert!(BuildModelResolver::validate_execution_evidence(&obligations));
}

#[test]
fn failed_root_no_obligations() {
    let roots = vec![
        create_root("/project", BuildModelState::Failed),
    ];
    
    let result = BuildModelResolver::resolve(&roots);
    assert!(result.obligations.is_empty());
}
