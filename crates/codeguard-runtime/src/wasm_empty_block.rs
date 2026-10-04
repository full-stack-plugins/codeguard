/// 原始语法树中的空 block 事实；不等同 ERROR/MISSING，也不直接代表任何语言违规。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmEmptyBlock {
    /// block 的直接父节点类型，供上层按固定语言规则解释。
    pub parent_syntax_kind: String,
    /// block 起始 UTF-8 字节偏移。
    pub start_byte: usize,
    /// block 结束 UTF-8 字节偏移。
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
