//! ESLint 稳定任务原工具复检；局部观察不能关闭任务或批准策略。
#[cfg(unix)]
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
#[cfg(unix)]
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::{
    collections::BTreeMap,
    path::Path,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
pub(crate) fn run(
    root: &Path,
    brief: &Value,
    options: &BTreeMap<String, String>,
    deadline: Instant,
) -> Value {
    let workspace = crate::workspace_refresh::read_workspace_baseline(root)
        .ok()
        .flatten()
        .and_then(|b| b.workspace_id().map(str::to_owned));
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |v| v.as_nanos());
    let mut report = json!({"schema_version":"0.2.0","report_type":"eslint_task_recheck","workspace_binding":"bound","workspace_id":workspace,
        "run_id":format!("eslint-task-{}-{nanos}",std::process::id()),"authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated",
        "target":{"task_id":brief["task_id"],"kind":brief["kind"],"checker_id":brief["checker_id"],"path":brief["scope"],"rule_id":brief["native_rule_id"]},
        "reason":"eslint_execution_context_missing","scan":null,"original_context_matches":false,"effective_rule":crate::eslint_effective_settings::unavailable(brief["native_rule_id"].as_str())});
    let Some(scope) = brief["scope"].as_str() else {
        return report;
    };
    let mut args = vec![root.join(scope).to_string_lossy().into_owned()];
    for key in [
        "--node-tool",
        "--eslint-entry",
        "--eslint-version",
        "--config",
        "--cwd",
    ] {
        let Some(value) = options.get(key) else {
            return report;
        };
        args.extend([key.into(), value.clone()]);
    }
    let args = match crate::eslint_lint_arguments::EslintLintArguments::parse(&args) {
        Ok(args) => args,
        Err(_) => {
            report["reason"] = json!("eslint_recheck_options_invalid");
            return report;
        }
    };
    let (feedback, scan) = crate::eslint_lint_command::observe_for_task(root, &args, deadline);
    report["reason"] = feedback["reason"].clone();
    let Some(mut scan) = scan else {
        return report;
    };
    scan["run_id"] = report["run_id"].clone();
    let original = brief["evidence_ref"]["first_run_id"]
        .as_str()
        .and_then(|run| {
            read_bounded_regular_file(
                &root.join(".codeguard/reports").join(format!("{run}.json")),
                16 * 1024 * 1024,
            )
            .ok()
        })
        .filter(|bytes| {
            brief["evidence_ref"]["first_report_sha256"] == format!("{:x}", Sha256::digest(bytes))
        })
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .map(|report| {
            if report["report_type"] == "eslint_task_recheck" && valid_shape(&report) {
                report["scan"].clone()
            } else {
                report
            }
        });
    report["original_context_matches"] = json!(original.is_some_and(|old| {
        old["report_type"] == "eslint_workbench_observation"
            && old["workspace_id"] == scan["workspace_id"]
            && old["source_path"] == scope
            && ["config", "node", "eslint"]
                .iter()
                .all(|key| old["inputs"][*key] == scan["inputs"][*key])
            && old["cwd"] == scan["cwd"]
            && old["native_version"] == scan["native_version"]
    }));
    if brief["kind"] == "finding" && scan["local_coherent"] == true {
        if let Some(rule) = brief["native_rule_id"].as_str() {
            report["effective_rule"] =
                crate::eslint_effective_settings::observe(rule, &scan, deadline);
            if report["effective_rule"]["status"] == "incomplete" {
                report["reason"] = report["effective_rule"]["reason"].clone();
            }
        }
    }
    report["scan"] = scan;
    report
}

pub(crate) fn valid_shape(report: &Value) -> bool {
    let keys = [
        "schema_version",
        "report_type",
        "workspace_binding",
        "workspace_id",
        "run_id",
        "authority",
        "coverage_proven",
        "delivery_decision",
        "target",
        "reason",
        "scan",
        "original_context_matches",
    ];
    report.as_object().is_some_and(|m| {
        keys.iter().all(|k| m.contains_key(*k))
            && ((report["schema_version"] == "0.1.0" && m.len() == keys.len())
                || (report["schema_version"] == "0.2.0"
                    && m.len() == keys.len() + 1
                    && effective_shape(&report["effective_rule"], &report["target"]["rule_id"])))
    }) && report["report_type"] == "eslint_task_recheck"
        && report["workspace_binding"] == "bound"
        && report["authority"] == "local_unverified"
        && report["coverage_proven"] == false
        && report["delivery_decision"] == "not_evaluated"
        && report["reason"].as_str().is_some_and(|s| {
            !s.is_empty()
                && s.len() <= 128
                && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        })
        && report["original_context_matches"].is_boolean()
        && report["target"].as_object().is_some_and(|m| {
            m.len() == 5
                && ["task_id", "kind", "checker_id", "path", "rule_id"]
                    .iter()
                    .all(|k| m.contains_key(*k))
        })
        && (report["scan"].is_null() || report["scan"].is_object())
}

fn effective_shape(value: &Value, rule: &Value) -> bool {
    let keys = [
        "rule_id",
        "status",
        "reason",
        "severity",
        "input_stable",
        "stdout_sha256",
    ];
    let hash_valid = value["stdout_sha256"].as_str().is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    });
    value
        .as_object()
        .is_some_and(|m| m.len() == keys.len() && keys.iter().all(|k| m.contains_key(*k)))
        && value["rule_id"] == *rule
        && value["input_stable"].is_boolean()
        && (value["severity"].is_null() || value["severity"].as_u64().is_some_and(|n| n <= 2))
        && value["reason"].as_str().is_some_and(|s| {
            !s.is_empty()
                && s.len() <= 128
                && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        })
        && if value["status"] == "observed" {
            value["input_stable"] == true
                && hash_valid
                && if value["severity"].is_null() {
                    value["reason"] == "eslint_effective_rule_not_present"
                } else {
                    value["reason"] == "eslint_effective_rule_observed"
                }
        } else {
            value["status"] == "incomplete"
                && value["severity"].is_null()
                && (value["stdout_sha256"].is_null() || hash_valid)
        }
}

pub(crate) fn classify(brief: &Value, report: &Value) -> &'static str {
    if !valid_shape(report)
        || report["target"]["task_id"] != brief["task_id"]
        || report["target"]["kind"] != brief["kind"]
        || report["target"]["checker_id"] != brief["checker_id"]
        || report["target"]["path"] != brief["scope"]
        || report["target"]["rule_id"] != brief["native_rule_id"]
    {
        return "incomplete";
    }
    let scan = &report["scan"];
    if matches!(
        report["reason"].as_str(),
        Some("request_cancelled" | "request_deadline_exceeded")
    ) {
        return "incomplete";
    }
    if scan["reason"] == "eslint_suppression_requires_review" {
        return "suppression_requires_review";
    }
    if scan["local_coherent"] != true {
        return if brief["kind"] == "blocker" && !scan.is_null() {
            "still_blocked"
        } else {
            "incomplete"
        };
    }
    if brief["kind"] == "blocker" {
        return "environment_restored_unverified_policy";
    }
    if report["schema_version"] == "0.2.0" && report["effective_rule"]["input_stable"] != true {
        return "incomplete";
    }
    if report["original_context_matches"] != true {
        return "rule_coverage_requires_review";
    }
    if scan["findings"].as_array().is_some_and(|items| {
        items
            .iter()
            .any(|f| f["finding_id"] == brief["task_id"] && f["rule_id"] == brief["native_rule_id"])
    }) {
        "still_present"
    } else {
        if report["schema_version"] == "0.2.0" {
            let settings = &report["effective_rule"];
            if settings["status"] != "observed" {
                return "incomplete";
            }
            if settings["severity"].is_null() || settings["severity"] == 0 {
                return "rule_coverage_requires_review";
            }
        }
        "candidate_absent_unverified_policy"
    }
}

#[cfg(test)]
mod tests {
    use super::classify;
    use serde_json::{Value, json};
    #[test]
    fn original_context_target_and_suppression_cannot_be_treated_as_resolution() {
        let brief = json!({"task_id":"CG-abc","kind":"finding","checker_id":"node.eslint","scope":"app.js","native_rule_id":"no-debugger"});
        let mut report = json!({"schema_version":"0.1.0","report_type":"eslint_task_recheck","workspace_binding":"bound","workspace_id":"ws-local","run_id":"eslint-task-1-1",
            "authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated",
            "target":{"task_id":"CG-abc","kind":"finding","checker_id":"node.eslint","path":"app.js","rule_id":"no-debugger"},
            "reason":"project_context_and_policy_unverified","scan":{"local_coherent":true,"reason":null,"findings":[]},"original_context_matches":true});
        assert_eq!(
            classify(&brief, &report),
            "candidate_absent_unverified_policy"
        );
        report["original_context_matches"] = json!(false);
        assert_eq!(classify(&brief, &report), "rule_coverage_requires_review");
        report["original_context_matches"] = json!(true);
        report["scan"]["findings"] = json!([{"finding_id":"CG-abc","rule_id":"no-debugger"}]);
        assert_eq!(classify(&brief, &report), "still_present");
        report["scan"]["reason"] = json!("eslint_suppression_requires_review");
        assert_eq!(classify(&brief, &report), "suppression_requires_review");
        report["target"]["path"] = json!("other.js");
        assert_eq!(classify(&brief, &report), "incomplete");
        report["target"]["path"] = json!("app.js");
        report["scan"]["reason"] = Value::Null;
        report["scan"]["findings"] = json!([]);
        report["schema_version"] = json!("0.2.0");
        report["effective_rule"] = json!({"rule_id":"no-debugger","status":"observed","reason":"eslint_effective_rule_observed","severity":2,"input_stable":true,"stdout_sha256":"0".repeat(64)});
        assert_eq!(
            classify(&brief, &report),
            "candidate_absent_unverified_policy"
        );
        report["effective_rule"]["severity"] = json!(0);
        assert_eq!(classify(&brief, &report), "rule_coverage_requires_review");
        report["effective_rule"]["severity"] = Value::Null;
        report["effective_rule"]["reason"] = json!("eslint_effective_rule_not_present");
        assert_eq!(classify(&brief, &report), "rule_coverage_requires_review");
        report["effective_rule"]["status"] = json!("incomplete");
        report["effective_rule"]["reason"] = json!("eslint_effective_settings_invalid");
        assert_eq!(classify(&brief, &report), "incomplete");
    }
}
