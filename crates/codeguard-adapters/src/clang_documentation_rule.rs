//! 固定Clang21文档规则的精确分类与修复契约，不依据前缀或自由诊断文本补造规则。
use serde_json::{Value, json};

/// 返回已适配原生规则的文档修复说明；参数为完整原生规则ID，未知规则返回None。
pub fn clang_documentation_guidance(rule_id: &str) -> Option<Value> {
    let (summary, steps) = match rule_id {
        "clang.warn_doc_block_command_empty_paragraph" => (
            "文档命令缺少非空说明段落",
            [
                "查看原生定位处的实际文档命令，补充其参数、返回或其他命令所需的准确说明。",
                "说明必须依据真实声明和行为契约，不能只保留标签、填写占位词或改动接口来迎合注释。",
            ],
        ),
        "clang.warn_doc_param_not_found" => (
            "文档参数名与实际函数声明不一致",
            [
                "核对原生定位处的参数名及其关联函数的实际参数声明。",
                "修正文档中的参数名并保留准确用途说明；不要为了使注释通过而改动函数签名。",
            ],
        ),
        "clang.warn_doc_returns_attached_to_a_void_function" => (
            "返回说明附着在void函数上",
            [
                "核对实际返回类型与该文档注释的关联对象。",
                "删除不适用的返回值标签，并按实际行为补充副作用或错误处理契约；不要改变返回类型绕过原规则。",
            ],
        ),
        _ => return None,
    };
    Some(
        json!({"rule_summary":summary,"repair_steps":steps,"allowed_changes":"documentation_comment_only","closing_condition":"original_native_recheck_and_approved_complete_coverage_required"}),
    )
}
