//! 纠错事件的可读附件；附件是历史投影，不是批准或当前任务状态。

use serde_json::Value;

/// 从已核对的提案生成可读附件；用户输入只按文本展示，不作为指令执行。
pub(crate) fn render(report: &Value, event_ref: &str, brief: &Value) -> String {
    let proposal = &report["proposal"];
    let id = text(&report["finding_id"]);
    let old = text(&proposal["old_decision_id"]);
    let reason = match proposal["correction_reason"].as_str() {
        Some("erroneous_approval") => "原批准可能有误",
        Some("scope_too_broad") => "原白名单范围过宽",
        Some("evidence_invalid") => "原证据已失效",
        Some("root_cause_fixed") => "根因已修复，申请撤销旧条目",
        _ => "纠错原因待核查",
    };
    let replacement = &proposal["replacement_candidate"];
    let replacement_text = if replacement.is_null() {
        "仅申请撤销旧条目，不创建替代白名单。".to_owned()
    } else {
        format!(
            "替代草稿 {} 引用旧条目 {}，摘要 {}；仍缺独立批准与完整规则身份。",
            text(&replacement["id"]),
            old,
            text(&replacement["candidate_sha256"])
        )
    };
    let missing = report["missing_evidence"]
        .as_array()
        .map(|items| items.iter().map(text).collect::<Vec<_>>().join("、"))
        .unwrap_or_else(|| "未知，须重新核查".into());
    format!(
        "# {id} 白名单纠错附件\n\n状态：待独立评审。此附件是创建时的历史投影；当前状态须运行 codeguard next。修改或勾选附件不改变门禁，也不关闭问题。\n\n\
         ## 问题证据\n\n稳定问题：{id}。原决策：{old}。原因：{reason}。\n\n复检运行：{}；报告 SHA-256：{}；观察：{}。\n\n事件引用：{}。原问题记录：.codeguard/findings/{id}/finding.json。\n\n\
         ## 规则依据\n\n原生检查器：{}；原生规则：{}；目标：{}。\n\n精确误报条目必须绑定原生规则、目标内容及工具/规则身份，并获独立批准；缺证据、撤销或冲突不得放行。原生复检未完成不能由白名单补足。\n\n\
         ## 允许修改的范围\n\n仅调查本稳定问题和旧条目 {old}；不得扩大到其它文件或规则，不得修改批准快照、删除原始发现或借此关闭源码任务。{replacement_text}\n\n\
         ## 修复步骤\n\n1. 核对原问题、原决策与本轮原工具复检。\n2. 收集缺失证据：{missing}。\n3. 提交撤销或精确替代提案供独立评审；受保护发布必须原子处理撤销与新增。\n4. 原工具重新检查；由当前策略判断剩余阻断或例外。\n\n\
         ## 复检命令\n\n在项目根执行：\n\n```bash\ncodeguard task verify {id} . --ruff-tool <原生Ruff绝对路径> --format json\ncodeguard next . --format json\n```\n\n当前入口只支持已复核的 Ruff 本地纠错收据；零诊断仍不证明完整策略或正式关闭。\n\n\
         ## 历史尝试\n\n本次事件：{}。此前的尝试与复检在同一 finding/events 中；附件不覆盖历史。重新复检或源码变化后，请由 next 重新核对本提案是否仍适用。\n\n\
         ## 关闭条件\n\n原问题保持待处理。独立批准、可信发布及原工具完整复检均完成后，才能按正式策略处置；已批准误报须显示带例外结果，不能伪装为代码修复。当前本地提案 authority=unverified、gate_effect=none。\n",
        text(&proposal["verification_ref"]["run_id"]),
        text(&proposal["verification_ref"]["report_sha256"]),
        text(&proposal["verification_ref"]["observation"]),
        escape(event_ref),
        text(&brief["checker_id"]),
        text(&brief["native_rule_id"]),
        text(&brief["scope"]),
        escape(event_ref)
    )
}

fn text(value: &Value) -> String {
    escape(value.as_str().unwrap_or("未提供"))
}

fn escape(value: &str) -> String {
    // 把可写候选中的 Markdown/HTML 标点实体化，防止记录文本变成链接或图片。
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '/' | '.' | ' ') {
                ch.to_string()
            } else {
                format!("&#{};", u32::from(ch))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::render;
    use serde_json::json;

    #[test]
    fn candidate_text_cannot_create_markdown_or_html_actions() {
        let report = json!({"finding_id":"CG-123", "proposal":{"old_decision_id":"![open](https://example.com)<script>`", "replacement_candidate":null}});
        let rendered = render(&report, "codeguard/event.json", &json!({}));
        assert!(!rendered.contains("![open]"));
        assert!(!rendered.contains("<script>"));
        assert!(rendered.contains("&#33;&#91;open&#93;"));
        assert!(rendered.contains("待独立评审"));
    }
}
