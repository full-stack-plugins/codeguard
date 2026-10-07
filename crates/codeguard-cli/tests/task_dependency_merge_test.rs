//! 任务依赖归并测试

use codeguard_cli::task_dependency_merge::*;

fn create_blocker(id: &str, reason: &str, modules: Vec<&str>) -> Blocker {
    Blocker {
        id: id.into(),
        reason: reason.into(),
        affected_modules: modules.into_iter().map(String::from).collect(),
    }
}

#[test]
fn merge_same_reason() {
    let blockers = vec![
        create_blocker("b1", "missing_jdk", vec!["module_a"]),
        create_blocker("b2", "missing_jdk", vec!["module_b"]),
    ];
    
    let result = BlockerMerger::merge(&blockers);
    
    assert_eq!(result.prerequisite_tasks.len(), 1);
    assert_eq!(result.prerequisite_tasks[0].affected_modules.len(), 2);
}

#[test]
fn merge_different_reasons() {
    let blockers = vec![
        create_blocker("b1", "missing_jdk", vec!["module_a"]),
        create_blocker("b2", "missing_node", vec!["module_b"]),
    ];
    
    let result = BlockerMerger::merge(&blockers);
    
    assert_eq!(result.prerequisite_tasks.len(), 2);
}

#[test]
fn validate_obligations_visible() {
    let blockers = vec![
        create_blocker("b1", "missing_jdk", vec!["module_a"]),
        create_blocker("b2", "missing_jdk", vec!["module_b"]),
    ];
    
    let result = BlockerMerger::merge(&blockers);
    assert!(BlockerMerger::validate_obligations_visible(&result, 2));
}

#[test]
fn multiple_modules_shared_prerequisite() {
    let blockers = vec![
        create_blocker("b1", "missing_jdk", vec!["module_a"]),
        create_blocker("b2", "missing_jdk", vec!["module_b"]),
        create_blocker("b3", "missing_jdk", vec!["module_c"]),
    ];
    
    let result = BlockerMerger::merge(&blockers);
    
    // 三个模块共缺 JDK 形成一个前置任务
    assert_eq!(result.prerequisite_tasks.len(), 1);
    assert_eq!(result.prerequisite_tasks[0].affected_modules.len(), 3);
}
