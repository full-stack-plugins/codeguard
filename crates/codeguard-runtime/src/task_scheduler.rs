//! 已校验检查任务图的本地并发调度；回调须沿用同一截止时间与取消标记。

use std::collections::{BTreeMap, BTreeSet};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use codeguard_core::{TaskGraph, TaskNode};

/// 单个执行回调返回的真实状态；不得把失败映射为无发现。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskExecution {
    /// 任务执行完成，结果仍须由适配器解释。
    Succeeded,
    /// 任务运行失败。
    Failed,
    /// 任务响应取消。
    Cancelled,
    /// 任务在共享截止时间内未完成。
    TimedOut,
}

/// 调度后每个任务的状态；跳过任务均为未完成而非成功。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskOutcome {
    /// 执行回调成功。
    Succeeded,
    /// 执行回调失败。
    Failed,
    /// 运行中取消。
    Cancelled,
    /// 运行中超时。
    TimedOut,
    /// 依赖失败、取消或超时，任务未启动。
    DependencyFailed,
    /// 排队期间取消，任务未启动。
    CancelledBeforeStart,
    /// 排队期间达到总截止时间，任务未启动。
    DeadlineBeforeStart,
    /// 执行回调 panic，任务未能形成可信结果。
    InternalFailure,
}

/// 调度器请求错误；非法并行度在任何任务启动前拒绝。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedulerError {
    /// 并发上限必须大于零。
    InvalidParallelism,
    /// 已校验 DAG 发生不应出现的无法推进状态。
    Stalled,
}

/// 并行执行已校验 DAG；失败依赖不可启动，资源 ID 相同的任务不可重叠。
///
/// `execute` 必须把传入的共同 `deadline` 和 `cancelled` 传给受控原生执行；
/// 此调度器能停止排队任务，不能抢占忽略取消/截止时间的任意 Rust 回调。
pub fn run_task_graph<F>(
    graph: &TaskGraph,
    max_jobs: usize,
    deadline: Instant,
    cancelled: &AtomicBool,
    execute: F,
) -> Result<BTreeMap<String, TaskOutcome>, SchedulerError>
where
    F: Fn(&TaskNode, Instant, &AtomicBool) -> TaskExecution + Sync,
{
    if max_jobs == 0 {
        return Err(SchedulerError::InvalidParallelism);
    }
    thread::scope(|scope| {
        let (sender, receiver) = mpsc::channel();
        let mut outcomes = BTreeMap::new();
        let mut running = BTreeMap::<String, Vec<String>>::new();
        let mut active_resources = BTreeSet::new();
        while outcomes.len() < graph.len() {
            let completed_before_iteration = outcomes.len();
            let cancellation =
                cancelled.load(Ordering::Relaxed) || crate::sigint_cancellation_requested();
            let timed_out = Instant::now() >= deadline;
            if cancellation || timed_out {
                for task in graph.tasks() {
                    if !outcomes.contains_key(&task.id) && !running.contains_key(&task.id) {
                        outcomes.insert(
                            task.id.clone(),
                            if cancellation {
                                TaskOutcome::CancelledBeforeStart
                            } else {
                                TaskOutcome::DeadlineBeforeStart
                            },
                        );
                    }
                }
            } else {
                for task in graph.tasks() {
                    if !outcomes.contains_key(&task.id)
                        && !running.contains_key(&task.id)
                        && task.dependencies.iter().any(|dependency| {
                            outcomes
                                .get(dependency)
                                .is_some_and(|state| *state != TaskOutcome::Succeeded)
                        })
                    {
                        outcomes.insert(task.id.clone(), TaskOutcome::DependencyFailed);
                    }
                }
                for task in graph.tasks() {
                    if running.len() >= max_jobs {
                        break;
                    }
                    if outcomes.contains_key(&task.id)
                        || running.contains_key(&task.id)
                        || !task.dependencies.iter().all(|dependency| {
                            outcomes.get(dependency) == Some(&TaskOutcome::Succeeded)
                        })
                        || task
                            .resources
                            .iter()
                            .any(|resource| active_resources.contains(resource))
                    {
                        continue;
                    }
                    for resource in &task.resources {
                        active_resources.insert(resource.clone());
                    }
                    running.insert(task.id.clone(), task.resources.clone());
                    let sender = sender.clone();
                    let execute = &execute;
                    scope.spawn(move || {
                        let result =
                            catch_unwind(AssertUnwindSafe(|| execute(task, deadline, cancelled)))
                                .ok();
                        let _ = sender.send((task.id.clone(), result, Instant::now()));
                    });
                }
            }
            if outcomes.len() == graph.len() {
                break;
            }
            if running.is_empty() {
                if outcomes.len() > completed_before_iteration {
                    continue;
                }
                return Err(SchedulerError::Stalled);
            }
            match receiver.recv_timeout(Duration::from_millis(10)) {
                Ok((id, result, completed_at)) => {
                    for resource in running.remove(&id).expect("运行中任务已登记") {
                        active_resources.remove(&resource);
                    }
                    let outcome = match result {
                        Some(TaskExecution::Succeeded) if completed_at >= deadline => {
                            TaskOutcome::TimedOut
                        }
                        Some(TaskExecution::Succeeded) => TaskOutcome::Succeeded,
                        Some(TaskExecution::Failed) => TaskOutcome::Failed,
                        Some(TaskExecution::Cancelled) => TaskOutcome::Cancelled,
                        Some(TaskExecution::TimedOut) => TaskOutcome::TimedOut,
                        None => TaskOutcome::InternalFailure,
                    };
                    outcomes.insert(id, outcome);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(SchedulerError::Stalled);
                }
            }
        }
        Ok(outcomes)
    })
}
