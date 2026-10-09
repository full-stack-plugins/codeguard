use crate::WasmDuplicateBinding;
use std::collections::HashSet;
use tree_sitter::Tree;

/// 指定根作用域内的直接简单绑定重复事实；零事实不代表语言语法通过。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmSiblingBindingScan {
    /// 有界重复标识符位置；不输出原始标识符文本。
    pub duplicates: Vec<WasmDuplicateBinding>,
    /// 存在未完成的访问或未记录的重复事实。
    pub truncated: bool,
}

/// 扫描指定AST根的直接声明，仅比较name字段的指定简单节点字节。
/// 参数为同字节树/源码、根/声明/声明项/名称四种节点类型及记录/访问预算；
/// 返回事实或参数/源码边界错误。语言语义、解构、export、转义及嵌套范围由上层处理。
pub fn scan_wasm_sibling_bindings(
    tree: &Tree,
    source: &[u8],
    kinds: [&str; 4],
    max_records: usize,
    max_visits: usize,
) -> Result<WasmSiblingBindingScan, String> {
    let [root_kind, declaration_kind, declarator_kind, name_kind] = kinds;
    let root = tree.root_node();
    if !(1..=128).contains(&max_records)
        || !(1..=200_000).contains(&max_visits)
        || source.len() > 1024 * 1024
        || std::str::from_utf8(source).is_err()
        || root.kind() != root_kind
        || root.end_byte() > source.len()
        || kinds
            .iter()
            .any(|kind| kind.is_empty() || kind.len() > 128 || kind.chars().any(char::is_control))
    {
        return Err("直接绑定事实参数或源码边界无效".into());
    }
    let mut names = HashSet::new();
    let mut duplicates = Vec::new();
    let mut truncated = false;
    let mut visited = 1usize;
    let mut cursor = root.walk();
    'declarations: for declaration in root.named_children(&mut cursor) {
        visited += 1;
        if visited > max_visits {
            truncated = true;
            break;
        }
        if declaration.kind() != declaration_kind {
            continue;
        }
        let mut child_cursor = declaration.walk();
        for declarator in declaration.named_children(&mut child_cursor) {
            visited += 1;
            if visited > max_visits {
                truncated = true;
                break 'declarations;
            }
            if declarator.kind() != declarator_kind {
                continue;
            }
            let Some(name) = declarator.child_by_field_name("name") else {
                continue;
            };
            visited += 1;
            if visited > max_visits {
                truncated = true;
                break 'declarations;
            }
            if name.kind() != name_kind {
                continue;
            }
            let text = name.utf8_text(source).map_err(|_| "绑定名称源码边界无效")?;
            // 原始标识符字节相同才形成事实，不解析字符串或猜测转义后的等价名称。
            if names.insert(text) {
                continue;
            }
            if duplicates.len() == max_records {
                truncated = true;
                break 'declarations;
            }
            let start = name.start_position();
            let end = name.end_position();
            duplicates.push(WasmDuplicateBinding {
                parent_syntax_kind: root.kind().to_owned(),
                start_byte: name.start_byte(),
                end_byte: name.end_byte(),
                start_row: start.row,
                start_column_byte: start.column,
                end_row: end.row,
                end_column_byte: end.column,
            });
        }
    }
    Ok(WasmSiblingBindingScan {
        duplicates,
        truncated,
    })
}
