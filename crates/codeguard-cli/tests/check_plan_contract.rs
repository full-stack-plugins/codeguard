use codeguard_cli::check_plan::parse_check_plan;
use codeguard_core::CHECK_KINDS;
use serde_json::{Value, json};

fn valid() -> Value {
    json!({
        "schema_version":"1.1",
        "report_type":"check_plan",
        "request_id":"run-001",
        "plan_id":"plan-001",
        "content_identity":{"kind":"working_tree","digest":"a".repeat(64)},
        "policy_identity":{"source":"trusted-local","revision":"r1","digest":"b".repeat(64)},
        "discovery_complete":true,
        "obligations":[{
            "id":"python/app/lint/ruff",
            "module":"app",
            "language":"python",
            "category":"lint",
            "check_kind":"lint.style",
            "applicability":"applicable",
            "required":true,
            "expected_targets":["src/main.py"],
            "rule_ids":["F401"]
        }],
        "tasks":[{
            "id":"task-ruff",
            "state":"planned",
            "adapter_id":"python.ruff",
            "obligation_ids":["python/app/lint/ruff"],
            "resource_keys":["python:app"]
        }],
        "edges":[],
        "unresolved_conditions":[]
    })
}

fn parse(value: &Value) -> Result<codeguard_cli::check_plan::CheckPlan, String> {
    parse_check_plan(&serde_json::to_vec(value).expect("测试 JSON"))
}

#[test]
fn complete_static_plan_carries_a_required_obligation_and_task() {
    let plan = parse(&valid()).expect("有效计划");
    assert_eq!(plan.obligation_ids, ["python/app/lint/ruff"]);
    assert_eq!(plan.task_ids, ["task-ruff"]);
}

#[test]
fn no_required_obligation_may_disappear_from_tasks_or_unresolved_conditions() {
    let mut document = valid();
    document["tasks"] = json!([]);
    assert!(parse(&document).is_err());
    document["unresolved_conditions"] = json!([{
        "obligation_id":"python/app/lint/ruff",
        "reason":"effective_model_pending"
    }]);
    assert!(parse(&document).is_ok());
}

#[test]
fn unknown_major_invalid_category_and_duplicate_obligation_are_rejected() {
    let mut document = valid();
    document["schema_version"] = json!("2.0");
    assert!(parse(&document).is_err());
    document = valid();
    document["obligations"][0]["category"] = json!("everything");
    assert!(parse(&document).is_err());
    document = valid();
    let duplicate = document["obligations"][0].clone();
    document["obligations"]
        .as_array_mut()
        .expect("义务")
        .push(duplicate);
    assert!(parse(&document).is_err());
}

#[test]
fn javadoc_dependency_and_cve_obligations_have_distinct_families() {
    for (category, check_kind) in [
        ("comments", "comments.api_docs"),
        ("dependencies", "dependencies.graph"),
        ("cve", "cve.advisory"),
    ] {
        let mut document = valid();
        document["obligations"][0]["category"] = json!(category);
        document["obligations"][0]["check_kind"] = json!(check_kind);
        assert!(parse(&document).is_ok(), "{category}/{check_kind}");
    }
    let mut document = valid();
    document["obligations"][0]["check_kind"] = json!("cve.advisory");
    assert!(parse(&document).is_err());
    document = valid();
    document["obligations"][0]
        .as_object_mut()
        .unwrap()
        .remove("check_kind");
    assert!(parse(&document).is_err());
    document["schema_version"] = json!("1.0");
    assert!(parse(&document).is_ok(), "1.0 兼容读取旧计划");
}

#[test]
fn dependency_cycles_and_dangling_task_references_are_rejected() {
    let mut document = valid();
    document["edges"] = json!([{"before":"task-ruff","after":"missing"}]);
    assert!(parse(&document).is_err());
    document = valid();
    document["edges"] = json!([{"before":"task-ruff","after":"task-ruff"}]);
    assert!(parse(&document).is_err());
    document = valid();
    document["tasks"][0]["obligation_ids"] = json!(["unknown"]);
    assert!(parse(&document).is_err());
}

#[test]
fn published_plan_schema_is_strict_and_parseable() {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/check-plan.schema.json"))
            .expect("计划 schema JSON");
    assert_eq!(schema["$id"], "urn:codeguard:schema:check-plan:1.1");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["$defs"]["obligation"]["additionalProperties"], false);
    let schema_kinds: std::collections::BTreeSet<&str> =
        schema["$defs"]["obligation"]["properties"]["check_kind"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect();
    let core_kinds: std::collections::BTreeSet<&str> =
        CHECK_KINDS.iter().map(|(kind, _)| *kind).collect();
    assert_eq!(schema_kinds, core_kinds);
}
