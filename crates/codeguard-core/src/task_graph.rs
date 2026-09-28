//! 纯领域任务图校验；不运行工具或获取锁。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::TaskNode;

/// 非法任务图原因；任何错误都必须在执行前发现。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskGraphError {
    /// 空或含控制字符的任务 ID。
    InvalidTaskId,
    /// 同一任务 ID 重复定义。
    DuplicateTaskId(String),
    /// 依赖不存在、自依赖或重复。
    InvalidDependency(String),
    /// 互斥资源名为空或重复。
    InvalidResource(String),
    /// 任务之间存在依赖环。
    DependencyCycle,
}

/// 已校验的任务 DAG；按任务 ID 固定遍历顺序。
#[derive(Clone, Debug)]
pub struct TaskGraph {
    tasks: BTreeMap<String, TaskNode>,
}

impl TaskGraph {
    /// 校验任务 ID、依赖、资源和无环性；失败时不产生可运行图。
    pub fn new(nodes: Vec<TaskNode>) -> Result<Self, TaskGraphError> {
        let mut tasks = BTreeMap::new();
        for node in nodes {
            if node.id.trim().is_empty() || node.id.chars().any(char::is_control) {
                return Err(TaskGraphError::InvalidTaskId);
            }
            let id = node.id.clone();
            if tasks.insert(id.clone(), node).is_some() {
                return Err(TaskGraphError::DuplicateTaskId(id));
            }
        }
        let mut remaining = BTreeMap::new();
        let mut children = BTreeMap::<String, Vec<String>>::new();
        for task in tasks.values() {
            let mut dependencies = BTreeSet::new();
            for dependency in &task.dependencies {
                if dependency == &task.id
                    || !tasks.contains_key(dependency)
                    || !dependencies.insert(dependency)
                {
                    return Err(TaskGraphError::InvalidDependency(task.id.clone()));
                }
                children
                    .entry(dependency.clone())
                    .or_default()
                    .push(task.id.clone());
            }
            let mut resources = BTreeSet::new();
            if task
                .resources
                .iter()
                .any(|resource| resource.trim().is_empty() || !resources.insert(resource))
            {
                return Err(TaskGraphError::InvalidResource(task.id.clone()));
            }
            remaining.insert(task.id.clone(), dependencies.len());
        }
        let mut ready: VecDeque<_> = remaining
            .iter()
            .filter(|(_, count)| **count == 0)
            .map(|(id, _)| id.clone())
            .collect();
        let mut visited = 0;
        while let Some(id) = ready.pop_front() {
            visited += 1;
            for child in children.get(&id).into_iter().flatten() {
                let count = remaining.get_mut(child).expect("已校验的依赖子任务");
                *count -= 1;
                if *count == 0 {
                    ready.push_back(child.clone());
                }
            }
        }
        if visited != tasks.len() {
            return Err(TaskGraphError::DependencyCycle);
        }
        Ok(Self { tasks })
    }

    /// 按固定 ID 顺序遍历已校验节点。
    pub fn tasks(&self) -> impl Iterator<Item = &TaskNode> {
        self.tasks.values()
    }

    /// 返回图中的任务数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// 判断任务图是否为空；空图不代表质量检查已完成。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}
