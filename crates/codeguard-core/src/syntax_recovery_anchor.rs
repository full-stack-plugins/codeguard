/// Tree-sitter 语法恢复节点在原始源码中的锚点，不代表已确认的源码违规。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxRecoveryAnchor {
    /// 恢复种类，固定为 ERROR 或 MISSING。
    pub kind: &'static str,
    /// Grammar 给出的节点种类，不推断缺少的具体源码文本。
    pub syntax_kind: String,
    /// 原始源码字节起点。
    pub start_byte: usize,
    /// 原始源码字节终点（不含）。
    pub end_byte: usize,
    /// 零起始源码行。
    pub start_row: usize,
    /// 零起始行内字节列；不能直接当作字符列显示。
    pub start_column_byte: usize,
    /// 零起始源码结束行。
    pub end_row: usize,
    /// 零起始结束行的字节列。
    pub end_column_byte: usize,
}
