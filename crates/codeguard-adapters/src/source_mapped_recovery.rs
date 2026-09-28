/// 已与原始源码字节核对的语法恢复锚点；仍只是疑似语法观察。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceMappedRecovery {
    /// ERROR 或 MISSING。
    pub kind: &'static str,
    /// 原始结构恢复组 ID；不作为跨扫描稳定任务身份。
    pub group_id: usize,
    /// 原始 grammar 节点名。
    pub syntax_kind: String,
    /// 原始源码字节起点。
    pub start_byte: usize,
    /// 原始源码字节终点（不含）。
    pub end_byte: usize,
    /// 一起始源码行。
    pub start_line: usize,
    /// 一起始 Unicode 标量列，不是终端宽度或 UTF-16 列。
    pub start_column_scalar: usize,
    /// 一起始源码结束行。
    pub end_line: usize,
    /// 一起始 Unicode 标量结束列。
    pub end_column_scalar: usize,
}
