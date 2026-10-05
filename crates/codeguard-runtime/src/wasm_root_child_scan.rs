use tree_sitter::Tree;

/// 根节点直接命名子节点的有界存在事实；不解释语言规则或赋予源码违规权威。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmRootChildScan {
    /// 固定grammar返回的根节点类型；相同根类型不代表相同语言。
    pub root_syntax_kind: String,
    /// 调用方明确指定的子节点类型。
    pub child_syntax_kind: String,
    /// 已遍历的直接命名子节点中是否存在该类型。
    pub present: bool,
    /// 存在未遍历的直接命名子节点；为true时，present=false不能证明缺失。
    pub truncated: bool,
}

/// 有界检查根节点的直接命名子节点，不搜索注释文本或嵌套节点。
/// 参数为固定grammar语法树、非空节点类型及1至200000个子节点预算；
/// 返回AST事实及截断状态，片段/完整文件和语言规则必须由上层单独核对。
pub fn scan_wasm_root_child(
    tree: &Tree,
    child_syntax_kind: &str,
    max_children: usize,
) -> Result<WasmRootChildScan, String> {
    if !(1..=200_000).contains(&max_children)
        || child_syntax_kind.is_empty()
        || child_syntax_kind.len() > 128
        || child_syntax_kind.chars().any(char::is_control)
    {
        return Err("根节点事实参数或预算无效".into());
    }
    let root = tree.root_node();
    let mut cursor = root.walk();
    let mut present = false;
    let mut truncated = false;
    // 游标保持线性遍历；不能反复按索引查找而造成大文件二次复杂度。
    for (visited, child) in root.named_children(&mut cursor).enumerate() {
        if visited == max_children {
            truncated = true;
            break;
        }
        present |= child.kind() == child_syntax_kind;
    }
    Ok(WasmRootChildScan {
        root_syntax_kind: root.kind().to_owned(),
        child_syntax_kind: child_syntax_kind.to_owned(),
        present,
        truncated,
    })
}
