use crate::WasmOuterReturn;
use tree_sitter::Tree;

/// 有界AST函数外return事实集合；是否适用模块规则由调用者证明。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmOuterReturnScan {
    /// 已观察到的原始节点位置，不输出源码文本。
    pub returns: Vec<WasmOuterReturn>,
    /// 记录或访问预算耗尽，不得解释为完整清洁结果。
    pub truncated: bool,
}

/// 在固定根节点内线性遍历，跳过调用者列出的完整函数子树。
/// 参数为同字节树/源码、根/return类型、函数边界及记录/访问预算；
/// 返回仅AST事实或参数错误，不决定module/CommonJS/未知模式的语言语义。
pub fn scan_wasm_outer_returns(
    tree: &Tree,
    source: &[u8],
    kinds: [&str; 2],
    function_kinds: &[&str],
    max_records: usize,
    max_visits: usize,
) -> Result<WasmOuterReturnScan, String> {
    let root = tree.root_node();
    let [root_kind, return_kind] = kinds;
    let valid_kind =
        |kind: &&str| !kind.is_empty() && kind.len() <= 128 && !kind.chars().any(char::is_control);
    if !(1..=128).contains(&max_records)
        || !(1..=200_000).contains(&max_visits)
        || !(1..=32).contains(&function_kinds.len())
        || !kinds.iter().all(valid_kind)
        || !function_kinds.iter().all(valid_kind)
        || function_kinds.contains(&root_kind)
        || function_kinds.contains(&return_kind)
        || root_kind == return_kind
        || source.len() > 1024 * 1024
        || std::str::from_utf8(source).is_err()
        || root.kind() != root_kind
        || root.end_byte() > source.len()
    {
        return Err("函数外return事实参数或源码边界无效".into());
    }
    let mut cursor = root.walk();
    let mut returns = Vec::new();
    let mut visited = 0usize;
    let mut truncated = false;
    loop {
        // 每个游标节点只处理一次；函数子树整体跳过，不做递归或反复祖先查找。
        if visited == max_visits {
            truncated = true;
            break;
        }
        visited += 1;
        let node = cursor.node();
        if node.kind() == return_kind {
            if returns.len() == max_records {
                truncated = true;
                break;
            }
            let start = node.start_position();
            let end = node.end_position();
            returns.push(WasmOuterReturn {
                syntax_kind: node.kind().to_owned(),
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                start_row: start.row,
                start_column_byte: start.column,
                end_row: end.row,
                end_column_byte: end.column,
            });
        }
        if !function_kinds.contains(&node.kind()) && cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return Ok(WasmOuterReturnScan { returns, truncated });
            }
        }
    }
    Ok(WasmOuterReturnScan { returns, truncated })
}
