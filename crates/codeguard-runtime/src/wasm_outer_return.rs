/// 函数边界之外的return节点字节事实；不自行认定模块模式或源码违规。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmOuterReturn {
    /// 当前AST节点类型。
    pub syntax_kind: String,
    /// 原始字节起点。
    pub start_byte: usize,
    /// 原始字节终点。
    pub end_byte: usize,
    /// 零基起始行。
    pub start_row: usize,
    /// 零基起始字节列。
    pub start_column_byte: usize,
    /// 零基结束行。
    pub end_row: usize,
    /// 零基结束字节列。
    pub end_column_byte: usize,
}
