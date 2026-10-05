use std::collections::BTreeSet;
use tree_sitter::{Node, Tree};

use crate::WasmRecovery;

/// 有界的原始恢复节点观察；截断时不得将已检文件宣称为完整。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmRecoveryScan {
    /// 保留的 ERROR/MISSING 原始恢复节点。
    pub recoveries: Vec<WasmRecovery>,
    /// 诊断数量、遍历预算耗尽，或语法树错误无法定位；消费者须标记初检不完整。
    pub truncated: bool,
}

/// 遍历语法树中有错误的分支，提取 ERROR 与 MISSING 并去除精确重复节点。
/// 参数为语法树和最大诊断数量；返回原始恢复节点与截断状态。
/// 节点取出与每次子节点检查均计入二十万次访问预算，包括正常兄弟节点。
pub fn scan_wasm_recoveries(tree: &Tree, max_records: usize) -> Result<WasmRecoveryScan, String> {
    if !(1..=1024).contains(&max_records) {
        return Err("语法恢复节点预算无效".into());
    }
    let mut stack = vec![(tree.root_node(), None)];
    let mut next_group_id = 1usize;
    let mut visited = 0usize;
    let mut recoveries = Vec::new();
    let mut seen = BTreeSet::new();
    let mut truncated = false;
    let mut unlocated_error = false;
    while let Some((node, ancestor_error_group)) = stack.pop() {
        visited += 1;
        if visited > 200_000 {
            truncated = true;
            break;
        }
        if !node.has_error() && !node.is_error() && !node.is_missing() {
            continue;
        }
        let kind = if node.is_error() {
            Some("ERROR")
        } else if node.is_missing() {
            Some("MISSING")
        } else {
            None
        };
        let group_id = kind.map(|_| {
            ancestor_error_group.unwrap_or_else(|| {
                let id = next_group_id;
                next_group_id += 1;
                id
            })
        });
        if let (Some(kind), Some(group_id)) = (kind, group_id) {
            let key = (
                group_id,
                kind,
                node.kind(),
                node.start_byte(),
                node.end_byte(),
            );
            if seen.insert(key) {
                if recoveries.len() == max_records {
                    truncated = true;
                    break;
                }
                recoveries.push(recovery(kind, group_id, node));
            }
        }
        let child_error_group = if node.is_error() {
            group_id
        } else {
            ancestor_error_group
        };
        let mut visible_error_child = false;
        // 逆向游标维持原有入栈顺序，宽节点只顺序遍历一次。
        // 正常兄弟也参与错误定位工作，不能绕过访问预算。
        let mut cursor = node.walk();
        if cursor.goto_last_child() {
            loop {
                visited += 1;
                if visited > 200_000 {
                    truncated = true;
                    break;
                }
                let child = cursor.node();
                if child.has_error() || child.is_error() || child.is_missing() {
                    visible_error_child = true;
                    stack.push((child, child_error_group));
                    if stack.len() > 200_000 {
                        truncated = true;
                        break;
                    }
                }
                if !cursor.goto_previous_sibling() {
                    break;
                }
            }
        }
        // 部分 grammar 的 MISSING token 只体现在 has_error 与 S-expression，
        // 不会成为可遍历子节点。此时不能把零恢复节点解释成语法有效。
        if node.has_error() && !node.is_error() && !node.is_missing() && !visible_error_child {
            unlocated_error = true;
        }
        if truncated {
            break;
        }
    }
    Ok(WasmRecoveryScan {
        recoveries,
        truncated: truncated || unlocated_error,
    })
}

fn recovery(kind: &'static str, group_id: usize, node: Node<'_>) -> WasmRecovery {
    let start = node.start_position();
    let end = node.end_position();
    WasmRecovery {
        kind,
        group_id,
        syntax_kind: node.kind().to_owned(),
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        start_row: start.row,
        start_column_byte: start.column,
        end_row: end.row,
        end_column_byte: end.column,
    }
}
