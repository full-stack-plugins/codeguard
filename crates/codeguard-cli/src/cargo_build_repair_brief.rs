//! 原生编译错误的修复简报；只给静态步骤，不执行诊断文本指令。

use serde_json::{Value, json};

/// 参数为已绑定发现、局部完整性、诊断原因及原工具argv；返回修复或调查指引，不授权关闭。
pub fn cargo_build_repair_brief(
    finding: &Value,
    complete: bool,
    reason: &str,
    recheck: &Value,
) -> Value {
    let permitted = complete && finding["identity_status"] == "unique_candidate";
    json!({
        "status":if permitted {"repair_guidance"} else {"investigation_required"},
        "evidence":{"finding_id":finding["finding_id"],"observation_reason":reason,"finding_fingerprint":finding["finding_fingerprint"],"source_sha256":finding["source_sha256"],"native_range_sha256":finding["native_range_sha256"],"rule_id":finding["rule_id"],"path":finding["path"],"line":finding["line"],"column":finding["column"],"byte_start":finding["byte_start"],"byte_end":finding["byte_end"]},
        "rule_basis":format!("原生rustc编译错误 {}；核对该错误码对应的类型、名称解析或语言约束，不通过关闭检查消除问题。",finding["rule_id"].as_str().unwrap_or("unknown")),
        "allowed_paths":if permitted {json!([finding["path"]])} else {json!([])},
        "steps":if permitted {json!(["按本轮字节范围核对实际源码和编译器错误码，识别类型或符号归属。","根据实现修复原目标；涉及其它文件时先扩展可核验修复范围，不猜测跨模块影响。","使用同一Cargo上下文复检；类型检查没有运行测试，还需按项目策略执行后续构建与测试。"]) } else {json!([format!("先诊断 {reason} 并重新运行原生构建；旧证据不能授权修改源码。")])},
        "recheck_argv":recheck,"working_directory":"selected_project_root",
        "tool_requirement":"使用本轮tool_sha256对应的显式Cargo；本地观察不证明可信工具或批准策略。",
        "attempt_history":[],"history_status":"not_integrated",
        "closure_conditions":["原工具复检并核对原目标、输入及构建范围","没有编译错误仅为局部缺失候选，不自动关闭任务","可信策略及完整交付检查另行核验"]
    })
}
