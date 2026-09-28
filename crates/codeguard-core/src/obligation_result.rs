//! 单项检查义务的执行与发现结果。

use crate::{Completion, Finding};
use serde::{Deserialize, Serialize};

/// 一个义务的有效发现和执行完整性。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObligationResult {
    /// 稳定义务标识。
    pub id: String,
    /// 此义务的执行完整性。
    pub completion: Completion,
    /// 执行不完整或不适用时的具体依据。
    pub reason: Option<String>,
    /// 已由工具契约确认的发现；即使后续崩溃也保留。
    pub findings: Vec<Finding>,
}
