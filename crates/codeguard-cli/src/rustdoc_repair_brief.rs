//! 原生文档观察的逐问题修复指引；历史任务与关闭授权由持久工作流承担。

use serde_json::{Value, json};

/// 为已核对的本轮发现生成修复简报。
/// 参数为发现、本轮局部完成状态、原因与原工具 argv；返回七项修复信息，不生成批准或关闭记录。
pub fn rustdoc_repair_brief(
    finding: &Value,
    local_complete: bool,
    reason: &str,
    recheck_argv: &Value,
) -> Value {
    let actionable = local_complete && finding["identity_status"] == "unique_candidate";
    let (basis, steps) = if !actionable {
        (
            "本轮输入或发现身份尚未核对完整，历史定位不能授权修改当前源码。",
            vec![
                "先解决本轮未完成原因或身份歧义，再用同一原工具重新检查；取得新证据后选择修复对象。",
            ],
        )
    } else if finding["rule_id"] == "missing_docs" {
        (
            "原生 missing_docs 检查公开接口的文档缺失；先核对目标是库、模块还是具体公开项。",
            vec![
                "按原生范围定位公开项；库和模块使用内部文档 //!，公开项使用 ///。",
                "根据实际实现补充用途、参数、返回、错误或约束；不编造行为，不通过隐藏公开接口或关闭规则消除告警。",
                "使用同一 Cargo 工具和相同目标复检；原生抑制或配置改变需另行核查，不能当作修复完成。",
            ],
        )
    } else {
        (
            "原生 rustdoc::broken_intra_doc_links 检查文档内链接的可解析性；须核对实际符号及其所在作用域。",
            vec![
                "在原生范围核对链接目标、命名空间和作用域，确认目标真实存在。",
                "修正链接路径或符号限定；仅为文字或代码示例时使用合适的代码格式，不创建虚构接口。",
                "使用同一 Cargo 工具和相同目标复检；规则关闭或链接检查被抑制不能当作修复完成。",
            ],
        )
    };
    json!({
        "status":if actionable {"repair_guidance"} else {"investigation_required"},
        "evidence":{
            "finding_id":finding["finding_id"],"finding_fingerprint":finding["finding_fingerprint"],
            "path":finding["path"],"source_sha256":finding["source_sha256"],
            "byte_start":finding["byte_start"],"byte_end":finding["byte_end"],
            "rule_id":finding["rule_id"],"observation_reason":reason
        },
        "rule_basis":basis,
        "allowed_paths":if actionable {vec![finding["path"].clone()]} else {Vec::<Value>::new()},
        "steps":steps,"recheck_argv":recheck_argv,"working_directory":"selected_project_root",
        "tool_requirement":"使用本轮 tool_sha256 对应的显式 Cargo 工具；argv 不代表可信工具选择或自动执行授权。",
        "attempt_history":[],"history_status":"not_integrated",
        "closure_conditions":[
            "原工具在输入稳定且原规则有效时复检，问题确实消失；改任务文字或新增抑制不算修复。",
            "由持久工作流记录复检与尝试证据，再核验完整项目策略；本简报不能关闭任务或批准白名单。"
        ]
    })
}
