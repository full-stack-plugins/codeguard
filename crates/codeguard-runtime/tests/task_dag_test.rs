//! 任务 DAG 测试

use codeguard_runtime::task_dag::*;

fn create_task(id: &str, deps: Vec<&str>, locks: Vec<&str>) -> TaskNode {
    TaskNode {
        id: id.into(),
        dependencies: deps.into_iter().map(String::from).collect(),
        resource_locks: locks.into_iter().map(String::from).collect(),
        status: TaskStatus::Pending,
    }
}

#[test]
fn dag_add_and_ready_tasks() {
    let mut dag = TaskDag::new();
    dag.add_task(create_task("a", vec![], vec![]));
    dag.add_task(create_task("b", vec!["a"], vec![]));
    
    let ready = dag.ready_tasks();
    assert_eq!(ready, vec!["a".to_string()]);
}

#[test]
fn dag_complete_task_unlocks_dependents() {
    let mut dag = TaskDag::new();
    dag.add_task(create_task("a", vec![], vec![]));
    dag.add_task(create_task("b", vec!["a"], vec![]));
    
    dag.complete_task("a").unwrap();
    
    let ready = dag.ready_tasks();
    assert_eq!(ready, vec!["b".to_string()]);
}

#[test]
fn dag_fail_task_propagates() {
    let mut dag = TaskDag::new();
    dag.add_task(create_task("a", vec![], vec![]));
    dag.add_task(create_task("b", vec!["a"], vec![]));
    dag.add_task(create_task("c", vec!["b"], vec![]));
    
    let skipped = dag.fail_task("a").unwrap();
    assert!(skipped.contains(&"b".to_string()));
    
    // b 被跳过，c 的依赖未完成
    assert_eq!(dag.task_status("b"), Some(TaskStatus::Skipped));
}

#[test]
fn dag_resource_lock_mutex() {
    let mut dag = TaskDag::new();
    dag.add_task(create_task("a", vec![], vec!["build_dir"]));
    dag.add_task(create_task("b", vec![], vec!["build_dir"]));
    
    // a 获取锁
    assert!(dag.acquire_locks("a").unwrap());
    
    // b 无法获取锁（共享 build 目录互斥）
    assert!(!dag.acquire_locks("b").unwrap());
    
    // a 完成后释放锁
    dag.complete_task("a").unwrap();
    assert!(dag.acquire_locks("b").unwrap());
}

#[test]
fn dag_independent_tasks_continue() {
    let mut dag = TaskDag::new();
    dag.add_task(create_task("a", vec![], vec![]));
    dag.add_task(create_task("b", vec![], vec![]));
    dag.add_task(create_task("c", vec!["a"], vec![]));
    
    // a 失败不影响 b
    dag.fail_task("a").unwrap();
    
    let independent = dag.independent_tasks();
    assert!(independent.contains(&"b".to_string()));
}

#[test]
fn dag_failed_dependency_chain() {
    let mut dag = TaskDag::new();
    dag.add_task(create_task("a", vec![], vec![]));
    dag.add_task(create_task("b", vec!["a"], vec![]));
    dag.add_task(create_task("c", vec!["b"], vec![]));
    
    dag.fail_task("a").unwrap();
    
    let chain = dag.failed_dependency_chain("b");
    assert!(chain.contains(&"a".to_string()));
}

#[test]
fn dag_incomplete_dependency_visible() {
    let mut dag = TaskDag::new();
    dag.add_task(create_task("a", vec![], vec![]));
    dag.add_task(create_task("b", vec!["a"], vec![]));
    
    // a 未完成，b 不应就绪
    assert!(!dag.ready_tasks().contains(&"b".to_string()));
    
    // a 失败后 b 被跳过，状态可见
    dag.fail_task("a").unwrap();
    assert_eq!(dag.task_status("b"), Some(TaskStatus::Skipped));
}
