//! 限定任务生命周期证据的封闭协议校验；结构有效不等于来源可信。
use codeguard_core::{ResolutionCause, TaskLifecycleKind, TaskLifecycleRecord};
use serde_json::Value;

pub(crate) fn valid(record: &TaskLifecycleRecord, value: &Value) -> bool {
    if !keys(
        value,
        &[
            "schema_version",
            "report_type",
            "identity",
            "original_report_sha256",
            "original_source_sha256",
            "current_source_sha256",
            "grammar_sha256",
            "tool_sha256",
            "adapter_sha256",
            "policy_sha256",
            "policy_revision",
            "original_native",
            "current_native",
            "outcome",
        ],
    ) || !value["policy_revision"].as_str().is_some_and(token)
        || ![
            "original_report_sha256",
            "original_source_sha256",
            "current_source_sha256",
            "grammar_sha256",
            "tool_sha256",
            "adapter_sha256",
            "policy_sha256",
        ]
        .iter()
        .all(|key| value[*key].as_str().is_some_and(digest))
        || !native(&value["original_native"])
        || !native(&value["current_native"])
    {
        return false;
    }
    let outcome = value["outcome"].as_str().unwrap_or("");
    match &record.event.kind {
        TaskLifecycleKind::Resolved { cause, .. } => {
            *cause == ResolutionCause::CodeFixed
                && outcome == "code_fixed"
                && value["original_source_sha256"] != value["current_source_sha256"]
                && value["original_native"]["status"] == "diagnostics_observed"
                && value["current_native"]["status"] == "completed"
                && native_bound(value)
        }
        TaskLifecycleKind::Observed | TaskLifecycleKind::Reopened => {
            outcome == "still_present"
                && value["current_native"]["status"] == "diagnostics_observed"
                && native_bound(value)
        }
        TaskLifecycleKind::VerificationRequired { reason_code, .. } => {
            reason_code == outcome
                && matches!(
                    outcome,
                    "inputs_stale"
                        | "native_incomplete"
                        | "false_positive_review_required"
                        | "resolution_evidence_incomplete"
                        | "resolution_history_binding_changed"
                )
        }
    }
}
fn native_bound(value: &Value) -> bool {
    ["original_native", "current_native"].iter().all(|k| {
        value[*k]["tool_sha256"] == value["tool_sha256"] && value[*k]["version"] == "0.16.0"
    })
}
fn native(value: &Value) -> bool {
    let base = ["status", "reason", "version", "tool_sha256", "diagnostics"];
    if !value.as_object().is_some_and(|o| {
        (o.len() == 5 || o.len() == 6)
            && base.iter().all(|k| o.contains_key(*k))
            && o.keys()
                .all(|k| base.contains(&k.as_str()) || k == "diagnostic_count")
    }) || !(value["tool_sha256"].as_str().is_some_and(digest)
        || (value["status"] == "not_run" && value["tool_sha256"].is_null()))
        || !(value["version"].is_null() || value["version"] == "0.16.0")
    {
        return false;
    }
    let Some(rows) = value["diagnostics"].as_array().filter(|r| r.len() <= 32) else {
        return false;
    };
    if value.get("diagnostic_count").is_some()
        && value["diagnostic_count"].as_u64() != Some(rows.len() as u64)
    {
        return false;
    }
    if !rows.iter().all(|r| {
        keys(r, &["line", "column", "rule_id"])
            && r["rule_id"] == "zig.ast_check.error"
            && r["line"]
                .as_u64()
                .is_some_and(|n| n > 0 && n <= u32::MAX as u64)
            && r["column"]
                .as_u64()
                .is_some_and(|n| n > 0 && n <= 1024 * 1024 + 1)
    }) {
        return false;
    }
    match value["status"].as_str() {
        Some("completed") => {
            rows.is_empty()
                && value["diagnostic_count"] == 0
                && value["version"] == "0.16.0"
                && value["reason"] == "ast_check_no_diagnostics"
        }
        Some("diagnostics_observed") => {
            !rows.is_empty()
                && value["diagnostic_count"].as_u64() == Some(rows.len() as u64)
                && value["version"] == "0.16.0"
                && value["reason"] == "ast_check_diagnostics"
        }
        Some("not_run") => {
            rows.is_empty()
                && value["version"].is_null()
                && value["tool_sha256"].is_null()
                && matches!(
                    value["reason"].as_str(),
                    Some("zig_tool_unavailable_or_untrusted" | "native_syntax_source_unavailable")
                )
        }
        Some("incomplete") => matches!(
            value["reason"].as_str(),
            Some(
                "zig_version_unverified_or_unsupported"
                    | "zig_tool_changed_during_check"
                    | "zig_ast_check_incomplete"
            )
        ),
        _ => false,
    }
}
fn keys(value: &Value, expected: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|o| o.len() == expected.len() && expected.iter().all(|k| o.contains_key(*k)))
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
}

#[cfg(test)]
mod tests {
    use super::native;
    use serde_json::json;
    #[test]
    fn missing_tool_evidence_is_valid_only_as_not_run() {
        let mut value = json!({"status":"not_run","reason":"zig_tool_unavailable_or_untrusted","version":null,"tool_sha256":null,"diagnostics":[]});
        assert!(native(&value));
        value["status"] = json!("completed");
        assert!(!native(&value));
    }
    #[test]
    fn unknown_fields_and_mismatched_counts_are_not_native_evidence() {
        let mut value = json!({"status":"completed","reason":"ast_check_no_diagnostics","version":"0.16.0","tool_sha256":"a".repeat(64),"diagnostic_count":0,"diagnostics":[]});
        assert!(native(&value));
        value["diagnostic_count"] = json!(1);
        assert!(!native(&value));
        value["diagnostic_count"] = json!(0);
        value["approved"] = json!(true);
        assert!(!native(&value));
    }
}
