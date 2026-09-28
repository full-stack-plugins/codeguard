//! 将结构有效的运行报告投影为保守的 SARIF 2.1.0 反馈；来源可信性须另行核验。

use crate::run_report::RunReport;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// 生成公开 SARIF 反馈。参数为结构有效的运行报告，返回不含原生消息与源码路径的 SARIF 文档。
#[must_use]
pub fn feedback_sarif(report: &RunReport) -> Value {
    let document = report.document();
    let status = document["command_status"]
        .as_str()
        .unwrap_or("internal_error");
    let gate = &document["delivery_gate"];
    let dispositions: BTreeMap<&str, &Value> = document["dispositions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["finding_id"].as_str().map(|id| (id, item)))
        .collect();
    let results: Vec<Value> = document["results"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|obligation| obligation["findings"].as_array().into_iter().flatten())
        .map(|finding| sarif_result(finding, &dispositions))
        .collect();
    let execution_successful = status == "complete";
    let notifications = if execution_successful {
        vec![]
    } else {
        vec![json!({
            "level":"error",
            "message":{"text":"CodeGuard analysis is incomplete; inspect the structured report and private evidence."}
        })]
    };
    json!({
        "$schema":"https://docs.oasis-open.org/sarif/sarif/v2.1.0/os/schemas/sarif-schema-2.1.0.json",
        "version":"2.1.0",
        "runs":[{
            "tool":{"driver":{"name":"CodeGuard","version":env!("CARGO_PKG_VERSION")}},
            "invocations":[{
                "executionSuccessful":execution_successful,
                "toolExecutionNotifications":notifications
            }],
            "results":results,
            "properties":{
                "codeguardDeliveryDecision":report.delivery_decision,
                "codeguardCommandStatus":status,
                "codeguardExitCode":report.exit_code,
                "codeguardSourceTrust":"structure_validated_provenance_unverified",
                "incompleteObligationCount":count(gate, "incomplete_obligation_ids"),
                "coverageMismatchCount":count(gate, "coverage_mismatch_ids"),
                "invalidLedgerCount":count(gate, "invalid_ledger_ids"),
                "rawFindingCount":report.finding_ids.len(),
                "whitelistedFindingCount":report.whitelisted_finding_ids.len(),
                "activeBlockingFindingCount":report.active_blocking_finding_ids.len(),
                "locationsVisibility":"private_evidence_only"
            }
        }]
    })
}

fn sarif_result(finding: &Value, dispositions: &BTreeMap<&str, &Value>) -> Value {
    let finding_id = finding["id"].as_str().unwrap_or_default();
    let tool_id = finding["tool_id"].as_str().unwrap_or_default();
    let native_rule_id = finding["native_rule_id"].as_str().unwrap_or_default();
    let impact = finding["gate_impact"].as_str().unwrap_or("undetermined");
    let disposition = dispositions.get(finding_id).copied();
    let mut rule_hasher = Sha256::new();
    rule_hasher.update(tool_id.as_bytes());
    rule_hasher.update([0]);
    rule_hasher.update(native_rule_id.as_bytes());
    let rule_digest = format!("{:x}", rule_hasher.finalize());
    let mut properties = json!({
        "codeguardGateImpact":impact,
        "codeguardDisposition":match (disposition.is_some(), impact) {
            (true, _) => "whitelisted_false_positive",
            (false, "blocking") => "active",
            (false, "non_blocking") => "not_applicable",
            _ => "undetermined"
        },
        "privateLocationCount":finding["locations"].as_array().map_or(0, Vec::len),
        "nativeMessageVisibility":"private_evidence_only"
    });
    if let Some(decision) = disposition {
        properties["decisionId"] = decision["decision_id"].clone();
        properties["approvalRef"] = decision["approval_ref"].clone();
        properties["approvedPolicyRevision"] = decision["approved_policy_revision"].clone();
        properties["expiresAt"] = decision["expires_at"].clone();
        properties["codeguardApprovalTrust"] = json!("report_claim_unverified");
    }
    json!({
        "ruleId":format!("CG-NATIVE-{}", &rule_digest[..16]),
        "level":match impact {
            "blocking" => "error",
            "undetermined" => "warning",
            _ => "note"
        },
        "message":{"text":"CodeGuard recorded a native finding; inspect private evidence for the original diagnostic."},
        "partialFingerprints":{"codeguardFindingId":digest(finding_id)},
        "properties":properties
    })
}

fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn count(object: &Value, key: &str) -> usize {
    object[key].as_array().map_or(0, Vec::len)
}
