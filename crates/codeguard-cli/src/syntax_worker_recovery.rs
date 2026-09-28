use serde::{Deserialize, Serialize};

/// 跨工作进程传输的语法恢复字节锚点；不是已确认源码违规。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SyntaxWorkerRecovery {
    /// 恢复节点类型，仅可为 ERROR 或 MISSING。
    pub kind: String,
    /// 同一棵树中的结构祖先组。
    pub group_id: usize,
    /// Grammar 原始节点种类。
    pub syntax_kind: String,
    /// 原始源码字节起点。
    pub start_byte: usize,
    /// 原始源码字节终点。
    pub end_byte: usize,
    /// 零起始起点行。
    pub start_row: usize,
    /// 零起始起点字节列。
    pub start_column_byte: usize,
    /// 零起始终点行。
    pub end_row: usize,
    /// 零起始终点字节列。
    pub end_column_byte: usize,
}
