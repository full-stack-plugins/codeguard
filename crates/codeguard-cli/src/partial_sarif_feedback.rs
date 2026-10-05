//! 将局部 check 反馈投影为 SARIF；局部观察不能宣称完整检查成功。

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// 将本轮 check_feedback/check_aborted 转成保守 SARIF；参数报告仍须由调用者保证来自本次运行。
#[must_use]
pub fn partial_check_sarif(report: &Value) -> Value {
    let mut results = Vec::new();
    if let Some(checkers) = report["native_results"].as_object() {
        for (checker, value) in checkers {
            if checker == "zig_lint" {
                // 原生语法探针使用 diagnostics；仅投影当前输入的定位，不遗失真实发现或输出旧坐标。
                for file in value["files"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|f| f["current"] == true)
                {
                    for diagnostic in file["native"]["diagnostics"]
                        .as_array()
                        .into_iter()
                        .flatten()
                    {
                        results.push(sarif_observation(checker, diagnostic, results.len()));
                    }
                }
            }
            collect_findings(checker, value, &mut results);
        }
    }
    let status = report["command_status"]
        .as_str()
        .unwrap_or("internal_error");
    let decision = report["delivery_decision"].as_str().unwrap_or("incomplete");
    json!({
        "$schema":"https://docs.oasis-open.org/sarif/sarif/v2.1.0/os/schemas/sarif-schema-2.1.0.json",
        "version":"2.1.0",
        "runs":[{
            "tool":{"driver":{"name":"CodeGuard","version":env!("CARGO_PKG_VERSION")}},
            "invocations":[{
                "executionSuccessful":false,
                "toolExecutionNotifications":[{
                    "level":"error",
                    "message":{"text":"CodeGuard has not completed a trusted project check; inspect the structured report and private evidence."}
                }]
            }],
            "results":results,
            "properties":{
                "codeguardReportType":report["report_type"],
                "codeguardCommandStatus":status,
                "codeguardDeliveryDecision":decision,
                "codeguardExitCode":report["exit_code"],
                "codeguardSourceTrust":"local_unverified",
                "codeguardObligationStatus":"unresolved",
                "codeguardExportStatus":report["export"]["status"],
                "codeguardExportReason":report["export"]["reason_code"],
                "nativeFindingCount":results.len(),
                "locationsVisibility":"private_evidence_only"
            }
        }]
    })
}

fn collect_findings(checker: &str, value: &Value, results: &mut Vec<Value>) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if key == "findings" {
                    for finding in child.as_array().into_iter().flatten() {
                        results.push(sarif_observation(checker, finding, results.len()));
                    }
                } else {
                    collect_findings(checker, child, results);
                }
            }
        }
        Value::Array(array) => {
            for child in array {
                collect_findings(checker, child, results);
            }
        }
        _ => {}
    }
}

fn sarif_observation(checker: &str, finding: &Value, ordinal: usize) -> Value {
    let native_rule = finding["rule_id"]
        .as_str()
        .or_else(|| finding["advisory_id"].as_str())
        .unwrap_or("unclassified");
    let rule_identity = format!("{checker}\0{native_rule}");
    let rule_digest = digest(rule_identity.as_bytes());
    // 无稳定 ID 的局部诊断使用本轮顺序作区分，不能冒充跨轮稳定身份。
    let identity = finding["finding_id"].as_str().map_or_else(
        || format!("{checker}\0local:{ordinal}"),
        |id| format!("{checker}\0{id}"),
    );
    json!({
        "ruleId":format!("CG-NATIVE-{}", &rule_digest[..16]),
        "level":"warning",
        "message":{"text":"CodeGuard observed a native diagnostic; project attribution and policy coverage remain unverified."},
        "partialFingerprints":{"codeguardFindingId":digest(identity.as_bytes())},
        "properties":{
            "codeguardObservation":"native_finding_unverified",
            "codeguardFingerprintStability":if finding["finding_id"].as_str().is_some() {"source_id_derived"} else {"current_run_only"},
            "nativeMessageVisibility":"private_evidence_only"
        }
    })
}

fn digest(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

#[cfg(test)]
mod tests {
    use super::partial_check_sarif;
    use serde_json::json;

    #[test]
    fn aborted_report_keeps_sibling_findings_without_claiming_success() {
        let source = json!({
            "report_type":"check_aborted","command_status":"internal_error",
            "delivery_decision":"incomplete","exit_code":4,
            "export":{"status":"not_requested","reason_code":null},
            "native_results":{
                "python_lint":{"files":[{"findings":[
                    {"finding_id":"CG-one","rule_id":"F401","path":"token=private.py"}
                ]}]},
                "rust_lint":{"findings":[{"rule_id":"clippy::needless_return","path":"src/lib.rs"}]}
            }
        });
        let sarif = partial_check_sarif(&source);
        let results = sarif["runs"][0]["results"].as_array().unwrap();
        assert_eq!(results.len(), 2);
        assert_ne!(
            results[0]["partialFingerprints"],
            results[1]["partialFingerprints"]
        );
        assert_eq!(
            sarif["runs"][0]["invocations"][0]["executionSuccessful"],
            false
        );
        assert_eq!(sarif["runs"][0]["properties"]["nativeFindingCount"], 2);
        assert!(!sarif.to_string().contains("token=private"));
    }
}
