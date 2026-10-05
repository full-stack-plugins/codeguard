//! Rust原生工具缺失时复用统一有界WASM候选及保守初检结论。
use serde_json::{Value, json};
use std::{path::Path, time::Instant};
#[cfg(feature = "wasm-precheck")]
pub(crate) fn observe(root: &Path, deadline: Instant) -> Value {
    let Ok(registry) = codeguard_adapters::legacy_registry() else {
        return unavailable("grammar_registry_unavailable");
    };
    let discovery =
        crate::discovery::discover(root, &registry, &codeguard_runtime::NativeObservation);
    crate::check_syntax_candidates::observe(
        root,
        &discovery,
        crate::check_syntax_candidates::NativeCoverage {
            node_lint: &Value::Null,
            python_lint: &Value::Null,
            go_lint: &Value::Null,
            erlang_lint: &Value::Null,
            kotlin_lint: &Value::Null,
            zig_lint: &Value::Null,
            swift_lint: &Value::Null,
            ruby_lint: &Value::Null,
            rust_targets: &crate::rust_native_syntax_coverage::RustNativeSyntaxCoverage::default(),
            go_tool: None,
        },
        Some("rust"),
        2,
        deadline,
        if discovery.observation_complete {
            None
        } else {
            Some("rust_source_discovery_incomplete")
        },
    )
}
#[cfg(not(feature = "wasm-precheck"))]
pub(crate) fn observe(_root: &Path, _deadline: Instant) -> Value {
    unavailable("wasm_feature_not_built")
}
pub(crate) fn unavailable(reason: &str) -> Value {
    json!({"status":"not_run","reason":reason,"execution_phase":"after_native","authority":"candidate_unqualified","delivery_decision":"incomplete",
        "source_file_count":0,"skipped_count":0,"unrouted_count":0,"native_preferred_count":0,"observations":[],
        "next_action":"准备项目适用Rust原生工具；初检不可用不能认定源码通过"})
}
pub(crate) fn preliminary_result(syntax: &Value) -> &'static str {
    let Some(rows) = syntax["observations"].as_array() else {
        return "incomplete";
    };
    let candidate = rows.iter().any(|row| {
        row["recovery_count"].as_u64().unwrap_or(0) > 0
            || row["structural_observation_count"].as_u64().unwrap_or(0) > 0
    });
    if candidate {
        return "candidates_observed";
    }
    if syntax["status"] == "observed_partial"
        && syntax["source_file_count"]
            .as_u64()
            .is_some_and(|n| n > 0 && n == rows.len() as u64)
        && syntax["skipped_count"] == 0
        && syntax["unrouted_count"] == 0
        && rows
            .iter()
            .all(|row| row["status"] == "candidate_observed" && row["reason"].is_null())
    {
        "no_candidates_observed"
    } else {
        "incomplete"
    }
}

#[cfg(test)]
mod tests {
    use super::{preliminary_result, unavailable};
    use serde_json::json;
    #[test]
    fn empty_skipped_failed_or_unbuilt_observations_require_native_tools() {
        assert_eq!(
            preliminary_result(&unavailable("wasm_feature_not_built")),
            "incomplete"
        );
        let good = json!({"status":"observed_partial","source_file_count":1,"skipped_count":0,"unrouted_count":0,
            "observations":[{"status":"candidate_observed","reason":null,"recovery_count":0}]});
        assert_eq!(preliminary_result(&good), "no_candidates_observed");
        for (field, value) in [
            ("skipped_count", json!(1)),
            ("unrouted_count", json!(1)),
            ("source_file_count", json!(2)),
            ("status", json!("not_run")),
        ] {
            let mut bad = good.clone();
            bad[field] = value;
            assert_eq!(preliminary_result(&bad), "incomplete");
        }
        let mut bad = good.clone();
        bad["observations"][0]["reason"] = json!("syntax_recovery_incomplete");
        assert_eq!(preliminary_result(&bad), "incomplete");
    }
}
