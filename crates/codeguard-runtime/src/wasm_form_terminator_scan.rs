use crate::WasmFormTerminator;
use tree_sitter::{Node, Tree};

/// 观察直接form的句点/分号边界，不读取字符串或注释内容，不做函数名称/参数语义解析。
/// 参数为固定AST、form节点类型、最多128锚点和最多200000次访问；
/// 返回候选位置及截断标志，分号仅在后续直接同类form前保留为可能的合法续接。
pub fn scan_wasm_form_terminators(
    tree: &Tree,
    form_kind: &str,
    max_results: usize,
    max_visits: usize,
) -> Result<(Vec<WasmFormTerminator>, bool), String> {
    if !(1..=128).contains(&max_results)
        || !(1..=200_000).contains(&max_visits)
        || form_kind.is_empty()
        || form_kind.len() > 128
        || form_kind.chars().any(char::is_control)
    {
        return Err("form终止符扫描参数无效".into());
    }
    let root = tree.root_node();
    let mut cursor = root.walk();
    let mut visits = 0;
    let mut facts = Vec::new();
    let mut pending = None;
    for form in root.named_children(&mut cursor) {
        if visits == max_visits {
            return Ok((facts, true));
        }
        visits += 1;
        if form.kind() == "comment" {
            continue;
        }
        // 只在已访问的下一form类型确定后裁定前一分号；预算耗尽不能当EOF。
        if let Some(previous) = pending.take() {
            if form.kind() != form_kind {
                if facts.len() == max_results {
                    return Ok((facts, true));
                }
                facts.push(previous);
            }
        }
        if form.kind() != form_kind {
            continue;
        }
        let mut child_cursor = form.walk();
        let mut last = None;
        for child in form.children(&mut child_cursor) {
            if visits == max_visits {
                return Ok((facts, true));
            }
            visits += 1;
            if child.kind() != "comment" {
                last = Some(child);
            }
        }
        match last {
            Some(token) if token.kind() == "." => {}
            Some(token) if token.kind() == ";" => pending = Some(anchor(token, form_kind, false)),
            _ => {
                if facts.len() == max_results {
                    return Ok((facts, true));
                }
                facts.push(anchor(form, form_kind, true));
            }
        }
    }
    if let Some(previous) = pending {
        if facts.len() == max_results {
            return Ok((facts, true));
        }
        facts.push(previous);
    }
    Ok((facts, false))
}

fn anchor(node: Node<'_>, form_kind: &str, missing: bool) -> WasmFormTerminator {
    let start = if missing {
        node.end_position()
    } else {
        node.start_position()
    };
    let end = node.end_position();
    WasmFormTerminator {
        parent_syntax_kind: form_kind.into(),
        start_byte: if missing {
            node.end_byte()
        } else {
            node.start_byte()
        },
        end_byte: node.end_byte(),
        start_row: start.row,
        start_column_byte: start.column,
        end_row: end.row,
        end_column_byte: end.column,
    }
}
