use std::collections::BTreeSet;
use tree_sitter::{Node, Tree};

use crate::WasmRecovery;

/// 有界的原始恢复节点观察；截断时不得将已检文件宣称为完整。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmRecoveryScan {
    /// 保留的 ERROR/MISSING 原始恢复节点。
    pub recoveries: Vec<WasmRecovery>,
    /// 诊断数量或遍历预算耗尽；消费者须标记初检不完整。
    pub truncated: bool,
}

/// 遍历语法树中有错误的分支，提取 ERROR 与 MISSING 并去除精确重复节点。
/// 参数为语法树和最大诊断数量；返回原始恢复节点与截断状态。
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
        for index in (0..node.child_count()).rev() {
            let Some(child) = node.child(index) else {
                continue;
            };
            if child.has_error() || child.is_error() || child.is_missing() {
                stack.push((child, child_error_group));
                if stack.len() > 200_000 {
                    truncated = true;
                    break;
                }
            }
        }
        if truncated {
            break;
        }
    }
    Ok(WasmRecoveryScan {
        recoveries,
        truncated,
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
