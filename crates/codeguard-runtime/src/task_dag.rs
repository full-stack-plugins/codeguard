//! 任务 DAG：资源锁及依赖失败传播
//!
//! 验收标准：独立任务继续、共享 build 目录互斥、失败依赖未完成可见

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// 等待
    Pending,
    /// 运行中
    Running,
    /// 完成
    Completed,
    /// 失败
    Failed,
    /// 因依赖失败而跳过
    Skipped,
}

/// 任务节点
#[derive(Debug, Clone)]
pub struct TaskNode {
    /// 任务 ID
    pub id: String,
    /// 依赖的任务 ID
    pub dependencies: Vec<String>,
    /// 所需资源锁
    pub resource_locks: Vec<String>,
    /// 状态
    pub status: TaskStatus,
}

/// 任务 DAG
pub struct TaskDag {
    /// 任务节点
    tasks: HashMap<String, TaskNode>,
    /// 资源锁
    resource_locks: Mutex<HashMap<String, bool>>,
}

impl TaskDag {
    /// 创建空 DAG
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            resource_locks: Mutex::new(HashMap::new()),
        }
    }
    
    /// 添加任务
    pub fn add_task(&mut self, node: TaskNode) {
        self.tasks.insert(node.id.clone(), node);
    }
    
    /// 获取可执行任务（依赖已完成）
    pub fn ready_tasks(&self) -> Vec<String> {
        self.tasks.values()
            .filter(|t| t.status == TaskStatus::Pending)
            .filter(|t| {
                t.dependencies.iter().all(|dep| {
                    self.tasks.get(dep).map(|d| d.status == TaskStatus::Completed).unwrap_or(false)
                })
            })
            .map(|t| t.id.clone())
            .collect()
    }
    
    /// 获取失败任务的依赖链
    pub fn failed_dependency_chain(&self, task_id: &str) -> Vec<String> {
        let mut chain = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(task_id.to_string());
        
        while let Some(id) = queue.pop_front() {
            if let Some(task) = self.tasks.get(&id) {
                if task.status == TaskStatus::Failed || task.status == TaskStatus::Skipped {
                    chain.push(id.clone());
                    for dep in &task.dependencies {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }
        
        chain
    }
    
    /// 标记任务完成
    pub fn complete_task(&mut self, task_id: &str) -> Result<(), String> {
        self.tasks.get_mut(task_id)
            .ok_or("task_not_found")?
            .status = TaskStatus::Completed;
        self.release_locks(task_id);
        Ok(())
    }
    
    /// 标记任务失败并传播
    pub fn fail_task(&mut self, task_id: &str) -> Result<Vec<String>, String> {
        self.tasks.get_mut(task_id)
            .ok_or("task_not_found")?
            .status = TaskStatus::Failed;
        self.release_locks(task_id);
        
        // 传播到依赖此任务的任务
        let mut skipped = Vec::new();
        for (id, task) in self.tasks.iter_mut() {
            if task.status == TaskStatus::Pending && task.dependencies.contains(&task_id.to_string()) {
                task.status = TaskStatus::Skipped;
                skipped.push(id.clone());
            }
        }
        
        Ok(skipped)
    }
    
    /// 尝试获取资源锁
    pub fn acquire_locks(&self, task_id: &str) -> Result<bool, String> {
        let task = self.tasks.get(task_id).ok_or("task_not_found")?;
        let mut locks = self.resource_locks.lock().map_err(|_| "lock_error")?;
        
        // 检查所有锁是否可用
        for lock in &task.resource_locks {
            if locks.get(lock).copied().unwrap_or(false) {
                return Ok(false); // 锁被占用
            }
        }
        
        // 获取所有锁
        for lock in &task.resource_locks {
            locks.insert(lock.clone(), true);
        }
        
        Ok(true)
    }
    
    /// 释放资源锁
    fn release_locks(&self, task_id: &str) {
        if let Some(task) = self.tasks.get(task_id) {
            if let Ok(mut locks) = self.resource_locks.lock() {
                for lock in &task.resource_locks {
                    locks.insert(lock.clone(), false);
                }
            }
        }
    }
    
    /// 独立任务继续（不受其他任务失败影响）
    pub fn independent_tasks(&self) -> Vec<String> {
        self.tasks.values()
            .filter(|t| t.dependencies.is_empty() && t.status == TaskStatus::Pending)
            .map(|t| t.id.clone())
            .collect()
    }
    
    /// 获取任务状态
    pub fn task_status(&self, task_id: &str) -> Option<TaskStatus> {
        self.tasks.get(task_id).map(|t| t.status)
    }
}

impl Default for TaskDag {
    fn default() -> Self {
        Self::new()
    }
}
