use serde_json::{Value, json};
use std::io::Write;
use std::process::{Command, Stdio};

fn run(raw: &[u8]) -> (i32, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["hook", "plan", "--format=json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start hook plan");
    child.stdin.take().unwrap().write_all(raw).unwrap();
    let output = child.wait_with_output().unwrap();
    let body = serde_json::from_slice(&output.stdout).unwrap_or(Value::Null);
    (output.status.code().unwrap(), body)
}

fn request(event: &str) -> Value {
    json!({
        "schema_version": "1.0.0",
        "report_type": "hook_trigger_request",
        "input": {
            "event": event,
            "changed_paths": [],
            "task_id": null,
            "write_outcome": "unknown",
            "host_claims_blocking": false
        }
    })
}

#[test]
fn successful_edit_routes_to_fast_feedback_without_delivery_claim() {
    let mut input = request("file_changed");
    input["input"]["changed_paths"] = json!(["src/A.java", "src/A.java"]);
    input["input"]["write_outcome"] = json!("confirmed");
    let (status, report) = run(&serde_json::to_vec(&input).unwrap());
    assert_eq!(status, 3);
    assert_eq!(report["schema_version"], "1.0.0");
    assert_eq!(report["report_type"], "hook_trigger_plan");
    assert_eq!(report["execution"], "not_run");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["plan"]["action"], "fast_file_check");
    assert_eq!(report["plan"]["target_paths"], json!(["src/A.java"]));
    assert_eq!(report["plan"]["soft_result_reuse_candidate"], true);
    assert_eq!(report["plan"]["may_claim_delivery"], false);
}

#[test]
fn commit_and_push_ignore_host_paths_and_require_fresh_git_snapshot() {
    for (event, action) in [("pre_commit", "commit_gate"), ("pre_push", "push_gate")] {
        let mut input = request(event);
        input["input"]["changed_paths"] = json!(["untrusted/guess.java"]);
        input["input"]["host_claims_blocking"] = json!(true);
        let (status, report) = run(&serde_json::to_vec(&input).unwrap());
        assert_eq!(status, 3);
        assert_eq!(report["plan"]["action"], action);
        assert_eq!(report["plan"]["target_paths"], json!([]));
        assert_eq!(report["plan"]["requires_git_snapshot"], true);
        assert_eq!(report["plan"]["host_blocking_claimed"], true);
        assert_eq!(report["plan"]["may_claim_delivery"], false);
    }
}

#[test]
fn failed_write_and_unknown_scope_do_not_claim_a_scan() {
    let mut input = request("file_changed");
    input["input"]["write_outcome"] = json!("failed");
    assert_eq!(
        run(&serde_json::to_vec(&input).unwrap()).1["plan"]["action"],
        "no_check"
    );
    input["input"]["write_outcome"] = json!("unknown");
    assert_eq!(
        run(&serde_json::to_vec(&input).unwrap()).1["plan"]["action"],
        "resolve_changed_scope"
    );
}

#[test]
fn malformed_or_future_requests_are_rejected_without_plan() {
    let mut input = request("ci");
    input["schema_version"] = json!("2.0.0");
    assert_eq!(run(&serde_json::to_vec(&input).unwrap()), (2, Value::Null));
    input = request("ci");
    input["input"]["extra"] = json!(true);
    assert_eq!(run(&serde_json::to_vec(&input).unwrap()), (2, Value::Null));
    input = request("file_changed");
    input["input"]["changed_paths"] = json!(["../escape.rs"]);
    assert_eq!(run(&serde_json::to_vec(&input).unwrap()), (2, Value::Null));
    let duplicate = b"{\"schema_version\":\"1.0.0\",\"schema_version\":\"1.0.0\",\"report_type\":\"hook_trigger_request\",\"input\":{}}";
    assert_eq!(run(duplicate), (2, Value::Null));
    assert_eq!(run(&vec![b'x'; 64 * 1024 + 1]), (2, Value::Null));
}

#[test]
fn published_request_and_plan_schemas_are_closed() {
    let request_schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/hook-trigger-request.schema.json"
    ))
    .unwrap();
    let plan_schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/hook-trigger-plan.schema.json"
    ))
    .unwrap();
    assert_eq!(
        request_schema["properties"]["schema_version"]["const"],
        "1.0.0"
    );
    assert_eq!(plan_schema["properties"]["execution"]["const"], "not_run");
    assert_eq!(request_schema["additionalProperties"], false);
    assert_eq!(plan_schema["additionalProperties"], false);
    let (_, report) = run(&serde_json::to_vec(&request("session_start")).unwrap());
    for (value, required) in [
        (&report, &plan_schema["required"]),
        (
            &report["plan"],
            &plan_schema["properties"]["plan"]["required"],
        ),
    ] {
        let actual: std::collections::BTreeSet<_> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        let declared: std::collections::BTreeSet<_> = required
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item.as_str().unwrap())
            .collect();
        assert_eq!(actual, declared);
    }
}
