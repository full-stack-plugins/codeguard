use tree_sitter::Tree;

/// 相邻 AST 关键词的字节锚点；仅为原始结构事实，不解释 SQL 或质量裁决。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmKeywordSequence {
    /// 根节点类型。
    pub root_syntax_kind: String,
    /// 原始字节起止位置。
    pub start_byte: usize,
    pub end_byte: usize,
    /// 零基行与字节列。
    pub start_row: usize,
    pub start_column_byte: usize,
    pub end_row: usize,
    pub end_column_byte: usize,
}

/// 在直接 AST 子节点中匹配固定关键词序列；注释可忽略，其它节点打断匹配。
/// 参数绑定树与冻结源码、关键词节点类型/序列及记录预算；返回事实和截断状态。
pub fn scan_wasm_keyword_sequence(
    tree: &Tree,
    source: &[u8],
    keyword_kind: &str,
    sequence: &[&str],
    max_records: usize,
) -> Result<(Vec<WasmKeywordSequence>, bool), String> {
    if !(1..=128).contains(&max_records)
        || sequence.is_empty()
        || sequence.len() > 8
        || keyword_kind.is_empty()
        || keyword_kind.len() > 128
        || sequence
            .iter()
            .any(|s| s.is_empty() || s.len() > 64 || !s.bytes().all(|b| b.is_ascii_alphabetic()))
        || std::str::from_utf8(source).is_err()
        || tree.root_node().end_byte() > source.len()
    {
        return Err("关键词事实参数或预算无效".into());
    }
    let root = tree.root_node();
    let mut cursor = root.walk();
    let mut matched = 0;
    let mut start = None;
    let mut rows = Vec::new();
    for (visited, node) in root.children(&mut cursor).enumerate() {
        if visited == 200_000 {
            return Ok((rows, true));
        }
        if node.kind() == "comment" {
            continue;
        }
        let word = node.utf8_text(source).map_err(|_| "关键词事实字节越界")?;
        if node.kind() == keyword_kind && word.eq_ignore_ascii_case(sequence[matched]) {
            if matched == 0 {
                start = Some(node);
            }
            matched += 1;
            if matched == sequence.len() {
                if rows.len() == max_records {
                    return Ok((rows, true));
                }
                let first = start.expect("首关键词与匹配状态同轮记录");
                rows.push(WasmKeywordSequence {
                    root_syntax_kind: root.kind().into(),
                    start_byte: first.start_byte(),
                    end_byte: node.end_byte(),
                    start_row: first.start_position().row,
                    start_column_byte: first.start_position().column,
                    end_row: node.end_position().row,
                    end_column_byte: node.end_position().column,
                });
                matched = 0;
                start = None;
            }
        } else {
            matched = 0;
            start = None;
            if node.kind() == keyword_kind && word.eq_ignore_ascii_case(sequence[0]) {
                matched = 1;
                start = Some(node);
            }
        }
    }
    Ok((rows, false))
}
