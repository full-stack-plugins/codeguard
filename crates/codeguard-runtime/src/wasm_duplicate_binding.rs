/// 同一直接作用域下重复简单绑定的AST位置事实；不含源码名称或语言裁决。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmDuplicateBinding {
    /// 被调用方明确选中的直接父作用域节点类型。
    pub parent_syntax_kind: String,
    /// 重复标识符的原始字节起点。
    pub start_byte: usize,
    /// 重复标识符的原始字节终点。
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
