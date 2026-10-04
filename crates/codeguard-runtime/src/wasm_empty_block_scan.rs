use crate::WasmEmptyBlock;
use tree_sitter::Tree;

/// 有界的空 block 事实扫描结果；不包含质量裁决或合成的解析器恢复节点。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmEmptyBlockScan {
    /// 没有非 comment 命名子节点的 block；其它语言可合法使用空块。
    pub blocks: Vec<WasmEmptyBlock>,
    /// 节点遍历或记录预算耗尽；消费者不得据零事实宣称检查完整。
    pub truncated: bool,
}

/// 扫描整个语法树中的空 block，不以 has_error 裁剪分支。
/// 参数为语法树和1至1024条记录预算；返回原始字节位置及截断状态。
/// 只观察精确名为 block 的节点；语言专用规则和确认权威由上层承担。
pub fn scan_wasm_empty_blocks(
    tree: &Tree,
    max_records: usize,
) -> Result<WasmEmptyBlockScan, String> {
    if !(1..=1024).contains(&max_records) {
        return Err("空语句块事实预算无效".into());
    }
    let mut stack = vec![tree.root_node()];
    let mut blocks = Vec::new();
    let mut visited = 0usize;
    let mut truncated = false;
    while let Some(node) = stack.pop() {
        visited += 1;
        if visited > 200_000 {
            truncated = true;
            break;
        }
        if node.kind() == "block" {
            let has_statement = (0..node.named_child_count()).any(|index| {
                node.named_child(index)
                    .is_some_and(|child| child.kind() != "comment")
            });
            if !has_statement {
                if let Some(parent) = node.parent() {
                    if blocks.len() == max_records {
                        truncated = true;
                        break;
                    }
                    let start = node.start_position();
                    let end = node.end_position();
                    blocks.push(WasmEmptyBlock {
                        parent_syntax_kind: parent.kind().to_owned(),
                        start_byte: node.start_byte(),
                        end_byte: node.end_byte(),
                        start_row: start.row,
                        start_column_byte: start.column,
                        end_row: end.row,
                        end_column_byte: end.column,
                    });
                }
            }
        }
        for index in (0..node.child_count()).rev() {
            if let Some(child) = node.child(index) {
                if stack.len() == 200_000 {
                    truncated = true;
                    break;
                }
                stack.push(child);
            }
        }
        if truncated {
            break;
        }
    }
    Ok(WasmEmptyBlockScan { blocks, truncated })
}
