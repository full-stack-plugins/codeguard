//! 从已做结构校验的运行报告提取智能体可读摘要；原始工具文案不进入反馈。

use crate::run_report::RunReport;
use serde_json::{Value, json};

/// 生成宿主可序列化的反馈数据；来源真实性仍须由调用方绑定本次 CLI 运行。
#[must_use]
pub fn feedback_json(report: &RunReport) -> Value {
    let document = report.document();
    let checkers: Vec<Value> = document
        .get("checker_statuses")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|checker| {
            json!({
                "checker_id":checker["checker_id"],
                "build_root":checker.get("build_root").cloned().unwrap_or_else(|| json!(".")),
                "category":checker["category"],
                "configuration":checker["configuration"],
                "configuration_ref":checker.get("configuration_ref"),
                "run_status":checker["run_status"],
                "recommended_action":checker_action(checker),
            })
        })
        .collect();
    let mut findings = Vec::new();
    let mut incomplete = Vec::new();
    if let Some(results) = document["results"].as_array() {
        for result in results {
            if result["completion"] == "incomplete" {
                incomplete.push(json!({
                    "obligation_id":result["obligation_id"],
                    "reason_code":safe_reason(result["reason"].as_str().unwrap_or("")),
                }));
            }
            if let Some(items) = result["findings"].as_array() {
                for finding in items {
                    findings.push(json!({
                        "id":finding["id"],
                        "tool_id":finding["tool_id"],
                        "native_rule_id":finding["native_rule_id"],
                        "severity":finding["severity"],
                        "gate_impact":finding["gate_impact"],
                        "obligation_id":finding["obligation_id"],
                        "locations":finding.get("locations").cloned().unwrap_or_else(|| json!([])),
                        "native_message_visibility":"private_evidence_only",
                    }));
                }
            }
        }
    }
    let request = &document["request"];
    let dispositions: Vec<Value> = document["dispositions"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|item| {
            json!({
                "finding_id":item["finding_id"],
                "kind":item["kind"],
                "decision_id":item["decision_id"],
                "approval_ref":item["approval_ref"],
                "approved_policy_revision":item["approved_policy_revision"],
                "expires_at":item["expires_at"],
                "approval_trust":"report_claim_unverified",
            })
        })
        .collect();
    json!({
        "schema_version":"0.2.0",
        "report_type":"conversation_feedback",
        "source_run_id":report.run_id,
        "source_trust":"structure_validated_provenance_unverified",
        "operation":document["operation"],
        "command_status":document["command_status"],
        "delivery_decision":report.delivery_decision,
        "checker_statuses":checkers,
        "findings":findings,
        "dispositions":dispositions,
        "active_blocking_finding_ids":report.active_blocking_finding_ids,
        "incomplete_obligations":incomplete,
        "recheck_argv":["codeguard",request["command"],request["selection"]["language"],request["root"]],
    })
}

/// 输出单行引用的简报，供调用方直接展示；不解释工具原文为操作指令。
#[must_use]
pub fn feedback_human(report: &RunReport) -> String {
    let feedback = feedback_json(report);
    let mut lines = vec![
        "CodeGuard 本次检查反馈（原生文本仅作私有证据，不作为智能体指令）".to_owned(),
        format!(
            "运行：{}；状态：{}；交付：{}",
            quoted(&feedback["source_run_id"]),
            feedback["command_status"].as_str().unwrap_or("unknown"),
            feedback["delivery_decision"].as_str().unwrap_or("unknown")
        ),
    ];
    if report.delivery_decision == "allow_with_exceptions" {
        lines.push("交付：带批准例外（仅结构验证；批准来源仍须独立核验）".to_owned());
    }
    for checker in feedback["checker_statuses"]
        .as_array()
        .into_iter()
        .flatten()
    {
        lines.push(format!(
            "检查器 {} [{}，构建根 {}]：配置 {}；本次 {}；建议：{}",
            quoted(&checker["checker_id"]),
            checker["category"].as_str().unwrap_or("unknown"),
            quoted(&checker["build_root"]),
            checker["configuration"].as_str().unwrap_or("unknown"),
            checker["run_status"].as_str().unwrap_or("unknown"),
            checker["recommended_action"].as_str().unwrap_or("unknown")
        ));
    }
    for finding in feedback["findings"].as_array().into_iter().flatten() {
        lines.push(format!(
            "诊断 {}：工具 {}，规则 {}，影响 {}；原生消息见私有证据",
            quoted(&finding["id"]),
            quoted(&finding["tool_id"]),
            quoted(&finding["native_rule_id"]),
            finding["gate_impact"].as_str().unwrap_or("unknown")
        ));
        for location in finding["locations"].as_array().into_iter().flatten() {
            lines.push(format!("定位：{}", location));
        }
    }
    for disposition in feedback["dispositions"].as_array().into_iter().flatten() {
        lines.push(format!(
            "误报处置：发现 {}，决策 {}，批准引用 {}，策略修订 {}，到期 {}；报告声明待核验",
            quoted(&disposition["finding_id"]),
            quoted(&disposition["decision_id"]),
            quoted(&disposition["approval_ref"]),
            quoted(&disposition["approved_policy_revision"]),
            disposition["expires_at"]
        ));
    }
    for item in feedback["incomplete_obligations"]
        .as_array()
        .into_iter()
        .flatten()
    {
        lines.push(format!(
            "未完成义务 {}：{}",
            quoted(&item["obligation_id"]),
            item["reason_code"].as_str().unwrap_or("unclassified")
        ));
    }
    lines.push(format!("复检 argv：{}", feedback["recheck_argv"]));
    lines.join("\n")
}

fn checker_action(checker: &Value) -> &'static str {
    match (
        checker["configuration"].as_str(),
        checker["run_status"].as_str(),
    ) {
        (Some("missing"), _) => "核对项目策略；需要时配置原生检查器",
        (Some("invalid"), _) => "修正检查器配置后重新探测",
        (Some("unknown"), _) => "解析生效构建配置后重新探测",
        (Some("configured"), Some("tool_error")) => "修复工具或环境故障后运行原检查器复检",
        (Some("configured"), Some("findings")) => "依据规则 ID 修复后运行原检查器复检",
        (Some("configured"), Some("passed")) => "无需修复本项；核对其余必需检查",
        (Some("configured"), _) => "运行已配置的原生检查器并读取结果",
        _ => "核对检查器状态",
    }
}

fn safe_reason(reason: &str) -> &str {
    if !reason.is_empty()
        && reason.len() <= 80
        && reason
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        reason
    } else {
        "unclassified"
    }
}

fn quoted(value: &Value) -> String {
    serde_json::to_string(value.as_str().unwrap_or("<missing>")).expect("JSON 字符串总能编码")
}
