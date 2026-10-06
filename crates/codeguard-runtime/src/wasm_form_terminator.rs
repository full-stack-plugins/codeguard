/// 直接函数form中缺失终止符或末尾分号的AST位置事实；不是解析器恢复或原生违规。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmFormTerminator {
    /// 直接form节点类型。
    pub parent_syntax_kind: String,
    /// 起始源码字节偏移；缺失token为零宽锚点。
    pub start_byte: usize,
    /// 结束源码字节偏移。
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
