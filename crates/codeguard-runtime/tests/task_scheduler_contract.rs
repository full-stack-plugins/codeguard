use codeguard_core::{TaskGraph, TaskNode};
use codeguard_runtime::{TaskExecution, TaskOutcome, run_task_graph};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

fn node(id: &str, dependencies: &[&str], resources: &[&str]) -> TaskNode {
    TaskNode {
        id: id.into(),
        dependencies: dependencies.iter().map(|value| (*value).into()).collect(),
        resources: resources.iter().map(|value| (*value).into()).collect(),
    }
}

#[test]
fn invalid_graph_or_parallelism_cannot_start_work() {
    assert!(TaskGraph::new(vec![node("a", &["missing"], &[])]).is_err());
    assert!(TaskGraph::new(vec![node("a", &["b"], &[]), node("b", &["a"], &[])]).is_err());
    assert!(TaskGraph::new(vec![node("a", &[], &[]), node("a", &[], &[])]).is_err());
    let graph = TaskGraph::new(vec![node("a", &[], &[])]).unwrap();
    let starts = AtomicUsize::new(0);
    assert!(
        run_task_graph(
            &graph,
            0,
            Instant::now() + Duration::from_secs(2),
            &AtomicBool::new(false),
            |_, _, _| {
                starts.fetch_add(1, Ordering::SeqCst);
                TaskExecution::Succeeded
            }
        )
        .is_err()
    );
    assert_eq!(starts.load(Ordering::SeqCst), 0);
}

#[test]
fn failed_dependency_is_skipped_while_independent_work_continues() {
    let graph = TaskGraph::new(vec![
        node("a", &[], &["build"]),
        node("b", &["a"], &["build"]),
        node("c", &[], &["other"]),
    ])
    .unwrap();
    let starts = AtomicUsize::new(0);
    let outcomes = run_task_graph(
        &graph,
        2,
        Instant::now() + Duration::from_secs(2),
        &AtomicBool::new(false),
        |task, _, _| {
            starts.fetch_add(1, Ordering::SeqCst);
            if task.id == "a" {
                TaskExecution::Failed
            } else {
                TaskExecution::Succeeded
            }
        },
    )
    .unwrap();
    assert_eq!(outcomes["a"], TaskOutcome::Failed);
    assert_eq!(outcomes["b"], TaskOutcome::DependencyFailed);
    assert_eq!(outcomes["c"], TaskOutcome::Succeeded);
    assert_eq!(starts.load(Ordering::SeqCst), 2);
}

#[test]
fn shared_build_resource_never_overlaps_but_independent_tasks_can() {
    let graph = TaskGraph::new(vec![
        node("a", &[], &["build/root"]),
        node("b", &[], &["build/root"]),
        node("c", &[], &["build/other"]),
    ])
    .unwrap();
    let active = AtomicUsize::new(0);
    let build_active = AtomicUsize::new(0);
    let peak = AtomicUsize::new(0);
    let overlap = Barrier::new(2);
    let outcomes = run_task_graph(
        &graph,
        3,
        Instant::now() + Duration::from_secs(3),
        &AtomicBool::new(false),
        |task, _, _| {
            let count = active.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(count, Ordering::SeqCst);
            if task.resources == ["build/root"] {
                assert_eq!(build_active.fetch_add(1, Ordering::SeqCst), 0);
            }
            if task.id != "b" {
                overlap.wait();
            }
            thread::sleep(Duration::from_millis(70));
            if task.resources == ["build/root"] {
                build_active.fetch_sub(1, Ordering::SeqCst);
            }
            active.fetch_sub(1, Ordering::SeqCst);
            TaskExecution::Succeeded
        },
    )
    .unwrap();
    assert!(
        outcomes
            .values()
            .all(|state| *state == TaskOutcome::Succeeded)
    );
    assert_eq!(peak.load(Ordering::SeqCst), 2);
}

#[test]
fn transitive_failed_dependencies_are_all_visible_even_in_reverse_id_order() {
    let graph = TaskGraph::new(vec![
        node("z", &[], &[]),
        node("y", &["z"], &[]),
        node("x", &["y"], &[]),
    ])
    .unwrap();
    let outcomes = run_task_graph(
        &graph,
        2,
        Instant::now() + Duration::from_secs(2),
        &AtomicBool::new(false),
        |task, _, _| {
            assert_eq!(task.id, "z");
            TaskExecution::Failed
        },
    )
    .unwrap();
    assert_eq!(outcomes["z"], TaskOutcome::Failed);
    assert_eq!(outcomes["y"], TaskOutcome::DependencyFailed);
    assert_eq!(outcomes["x"], TaskOutcome::DependencyFailed);
}

#[test]
fn deadline_before_start_and_worker_panic_never_count_as_success() {
    let graph = TaskGraph::new(vec![node("a", &[], &[]), node("b", &["a"], &[])]).unwrap();
    let starts = AtomicUsize::new(0);
    let expired = run_task_graph(
        &graph,
        1,
        Instant::now() - Duration::from_millis(1),
        &AtomicBool::new(false),
        |_, _, _| {
            starts.fetch_add(1, Ordering::SeqCst);
            TaskExecution::Succeeded
        },
    )
    .unwrap();
    assert_eq!(starts.load(Ordering::SeqCst), 0);
    assert_eq!(expired["a"], TaskOutcome::DeadlineBeforeStart);
    assert_eq!(expired["b"], TaskOutcome::DeadlineBeforeStart);

    let failed = run_task_graph(
        &graph,
        1,
        Instant::now() + Duration::from_secs(2),
        &AtomicBool::new(false),
        |_, _, _| panic!("simulated worker failure"),
    )
    .unwrap();
    assert_eq!(failed["a"], TaskOutcome::InternalFailure);
    assert_eq!(failed["b"], TaskOutcome::DependencyFailed);
}

#[test]
fn cancellation_marks_queued_tasks_without_starting_them() {
    let graph = TaskGraph::new(vec![node("a", &[], &[]), node("b", &[], &[])]).unwrap();
    let cancelled = Arc::new(AtomicBool::new(false));
    let starts = AtomicUsize::new(0);
    let signal = Arc::clone(&cancelled);
    let trigger = thread::spawn(move || {
        thread::sleep(Duration::from_millis(30));
        signal.store(true, Ordering::SeqCst);
    });
    let outcomes = run_task_graph(
        &graph,
        1,
        Instant::now() + Duration::from_secs(2),
        &cancelled,
        |_, _, _| {
            starts.fetch_add(1, Ordering::SeqCst);
            thread::sleep(Duration::from_millis(80));
            TaskExecution::Cancelled
        },
    )
    .unwrap();
    trigger.join().unwrap();
    assert_eq!(starts.load(Ordering::SeqCst), 1);
    assert_eq!(outcomes["a"], TaskOutcome::Cancelled);
    assert_eq!(outcomes["b"], TaskOutcome::CancelledBeforeStart);
}
