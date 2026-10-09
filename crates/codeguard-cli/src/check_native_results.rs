//! `check` 报告的 `native_results` 槽位装配规则。
//!
//! 槽位是否存在由**实际结果**决定，而不是由 `schema_version` 决定。`schema_version` 只反映
//! 版本选择分支的优先级：`gradle_model_unresolved` 命中 `0.61.0` 时，早于任何 Kotlin 判断，
//! 于是「本项目根本没有 Kotlin 源码」也会留下一个 `kotlin_lint: null` 的假槽位。消费侧据此
//! 无法区分「没有该语言源码」与「该语言结果为 null」，会误判覆盖范围。
//!
//! 存在性规则：
//!
//! - **对象**：一律保留。即使内容是 `incomplete` / `not_evaluated`，它表达的是环境阻塞或
//!   覆盖缺口，属于「跑了并有结论」，不能因为不完整就抹掉。
//! - **数组**：一律保留，哪怕为空。空数组表示扫描完成且无发现，与「未扫描」语义不同。
//! - **null**：不保留。没有结果就没有槽位。
//!
//! 键的顺序沿用传入顺序，使生成的报告在逐行 diff 中保持稳定。

use serde_json::{Map, Value};

/// 按存在性规则装配 `native_results`。
pub(crate) fn assemble(slots: Vec<(&str, Value)>) -> Value {
    let mut out = Map::new();
    for (key, value) in slots {
        if is_reportable(&value) {
            out.insert(key.to_owned(), value);
        }
    }
    Value::Object(out)
}

/// 该结果是否应作为槽位出现在报告中。
fn is_reportable(value: &Value) -> bool {
    matches!(value, Value::Object(_) | Value::Array(_))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 没有结果时不能留下假槽位：否则「本项目没有该语言源码」与「结果为 null」不可区分。
    #[test]
    fn null_results_do_not_create_slots() {
        let out = assemble(vec![
            ("kotlin_lint", Value::Null),
            ("swift_lint", Value::Null),
        ]);
        assert_eq!(out, json!({}));
    }

    /// 不完整结论仍要保留：它表达环境阻塞或覆盖缺口，不是「没跑」。
    #[test]
    fn incomplete_objects_are_kept() {
        let out = assemble(vec![(
            "kotlin_lint",
            json!({"status":"incomplete","reason":"kotlin_tool_not_found"}),
        )]);
        assert_eq!(out["kotlin_lint"]["reason"], json!("kotlin_tool_not_found"));
    }

    /// 空数组表示扫描完成且无发现，与「未扫描」不同，必须保留。
    #[test]
    fn empty_arrays_are_kept() {
        let out = assemble(vec![("npm_cve", json!([])), ("python_cve", json!([]))]);
        assert!(out.get("npm_cve").is_some());
        assert!(out.get("python_cve").is_some());
    }

    /// 混合输入下只丢弃 null，其余槽位全部保留。
    ///
    /// 这里断言集合而非顺序：`serde_json::Map` 在未启用 `preserve_order` 时是 `BTreeMap`，
    /// 序列化输出按字母序而非插入序。因此「传入顺序稳定」不能作为可依赖的契约来断言，
    /// 否则测试会锁死一个 serde_json 特性开关，而不是本模块的行为。
    #[test]
    fn mixed_slots_drop_only_nulls() {
        let out = assemble(vec![
            ("node_lint", Value::Null),
            ("npm_cve", json!([])),
            ("rust_lint", json!({"status":"incomplete"})),
            ("kotlin_lint", Value::Null),
            ("java_p3c", json!({"status":"not_configured"})),
        ]);
        let mut keys: Vec<&str> = out
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["java_p3c", "npm_cve", "rust_lint"]);
    }

    /// 报告缺失的槽位按 serde 语义读回为 null，与显式 null 对读取方等价——
    /// 这正是消费侧必须改用存在性判断、而不是比较值的原因。
    #[test]
    fn absent_slot_reads_as_null() {
        let out = assemble(vec![("kotlin_lint", Value::Null)]);
        assert_eq!(out["kotlin_lint"], Value::Null);
        assert!(out.get("kotlin_lint").is_none());
    }
}
