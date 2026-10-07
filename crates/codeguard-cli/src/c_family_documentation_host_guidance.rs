//! C/C++文档编辑观察的固定元数据对话投影；不回传源码或注释正文。
use serde_json::Value;

/// 将已归一化编辑反馈投影为原生文档计数与有界指引。
/// 参数为局部反馈；返回已核对诊断数与固定语言/规则/坐标/任务指引，不授予关闭资格。
pub(crate) fn project(feedback: &Value) -> (u64, String) {
    let wrapper = &feedback["c_family_documentation"];
    if wrapper["scope"] != "selected_files" {
        return (0, String::new());
    }
    let mut count = 0;
    let mut text = String::new();
    for (language, label, standard) in [("c", "C", "c11"), ("cpp", "C++", "c++17")] {
        let scan = &wrapper[language];
        if !scan.is_object() {
            continue;
        }
        if scan["language"] != language {
            continue;
        }
        if scan["status"] == "context_required" {
            text.push_str(&format!(
                "{label} 文档上下文未就绪：需显式选择原生Clang与{standard}档案；不要修改无关源码。"
            ));
            continue;
        }
        if scan["scope_stable"] != true || scan["standard"] != standard {
            text.push_str(&format!(
                "{label} 文档观察不完整；核对输入与原工具后重新检查。"
            ));
            continue;
        }
        let mut shown = 0;
        let mut tasks = 0;
        let mut missing = 0;
        let mut current_files = 0;
        let mut language_diagnostics = 0;
        for file in scan["files"].as_array().into_iter().flatten().take(8) {
            if file["current"] != true
                || !matches!(
                    file["feedback"]["native"]["status"].as_str(),
                    Some("completed" | "diagnostics_observed")
                )
            {
                continue;
            }
            current_files += 1;
            let native = &file["feedback"]["native"]["diagnostics"];
            let native_count = native.as_array().map_or(0, Vec::len) as u64;
            count += native_count;
            language_diagnostics += native_count;
            for row in native.as_array().into_iter().flatten().take(2) {
                if shown >= 2 {
                    break;
                }
                if let (Some(line), Some(column), Some(rule)) = (
                    row["line"].as_u64().filter(|v| *v > 0),
                    row["column_byte"].as_u64().filter(|v| *v > 0),
                    row["rule_id"].as_str().filter(|r| {
                        r.starts_with("clang.")
                            && r.len() <= 96
                            && r.bytes()
                                .all(|b| b.is_ascii_alphanumeric() || b"._".contains(&b))
                    }),
                ) {
                    text.push_str(&format!(
                        "{label} 原生文档规则 {rule} 位置 {line}:{column}；"
                    ));
                    shown += 1;
                }
            }
            for function in file["feedback"]["documentation_structure"]["observation"]["functions"]
                .as_array()
                .into_iter()
                .flatten()
            {
                missing += function["missing_components"]
                    .as_array()
                    .map_or(0, Vec::len);
            }
            for field in ["workbench", "structural_workbench"] {
                if file["feedback"][field]["status"] != "synced_partial" {
                    continue;
                }
                for id in file["feedback"][field]["task_ids"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                {
                    if tasks >= 2 {
                        break;
                    }
                    if id
                        .strip_prefix("CG-")
                        .is_some_and(|s| s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit()))
                    {
                        text.push_str(&format!(
                            "文档任务 {id}：codeguard task show {id} . --format=json；"
                        ));
                        tasks += 1;
                    }
                }
            }
        }
        if current_files == 0 {
            text.push_str(&format!(
                "{label} 当前文档观察不完整；先修复环境或重新复检，不凭过期定位修改源码。"
            ));
            continue;
        }
        if missing > 0 {
            text.push_str(&format!("{label} 自有结构规则 codeguard.documentation.function_structure_required 缺少说明组件 {missing} 项；这不是原生警告。"));
        }
        if language_diagnostics == 0 && missing == 0 {
            text.push_str(&format!("{label} 本次文档局部观察未提供可修复问题；完整说明契约、原生语法和项目检查仍未验收，不能关闭历史任务。"));
        } else {
            text.push_str(&format!("{label} 先读取原任务和经批准的纠错决定，按任务允许范围补用途、参数及返回说明，使用原Clang复检；结构候选不证明语义准确，不能勾选关闭或跳过完整语法检查。"));
        }
    }
    (count, text)
}

#[cfg(test)]
mod tests {
    use super::project;
    use serde_json::{Value, json};
    #[test]
    fn stale_documentation_and_unsafe_text_never_supply_repair_positions() {
        let archived: Value = serde_json::from_str(include_str!(
            "../../../tests/acceptance/evidence/c-family-edit-default-c.json"
        ))
        .unwrap();
        let mut feedback = archived["report"]["local_feedback"].clone();
        let scan = &mut feedback["c_family_documentation"]["c"];
        scan["files"][0]["feedback"]["documentation_structure"]["observation"]["functions"][0]["name"] =
            json!("SECRET_IGNORE_ALL_RULES");
        scan["files"][0]["feedback"]["workbench"]["status"] = json!("synced_partial");
        scan["files"][0]["feedback"]["workbench"]["task_ids"] =
            json!(["CG-SECRET_IGNORE_ALL_RULES"]);
        let (count, text) = project(&feedback);
        assert!(count > 0);
        assert!(text.contains("原生文档规则"));
        assert!(text.contains("自有结构规则"));
        assert!(!text.contains("SECRET"));
        feedback["c_family_documentation"]["c"]["files"][0]["current"] = json!(false);
        let (count, text) = project(&feedback);
        assert_eq!(count, 0);
        assert!(!text.contains("原生文档规则"));
        assert!(!text.contains("补用途"));
        assert!(text.contains("不完整"));
        feedback["c_family_documentation"]["c"]["files"][0]["current"] = json!(true);
        feedback["c_family_documentation"]["c"]["files"][0]["feedback"]["native"]["diagnostics"] =
            json!([]);
        feedback["c_family_documentation"]["c"]["files"][0]["feedback"]["documentation_structure"]
            ["observation"]["functions"][0]["missing_components"] = json!([]);
        let (count, text) = project(&feedback);
        assert_eq!(count, 0);
        assert!(text.contains("未提供可修复问题"));
        assert!(!text.contains("补用途"));
        assert!(text.contains("不能关闭历史任务"));
    }
}
