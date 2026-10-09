use codeguard_cli::guard_integration::profile::InvocationDescriptor;
use serde_json::{Value, json};
pub fn valid() -> Value {
    json!({
        "schema_version":"1.0",
        "report_type":"run_report",
        "operation":"lint",
        "request_id":"run-001",
        "run_id":"native-001",
        "command_status":"complete",
        "exit_code":1,
        "request":{
            "schema_version":"1.0",
            "report_type":"check_request",
            "request_id":"run-001",
            "command":"lint",
            "selection":{"language":"python"},
            "root":"/workspace/example",
            "content_source":{"kind":"working_tree"},
            "options":{"format":"json","offline":false,"timeout_ms":30000,"jobs":2}
        },
        "plan_id":"plan-001",
        "identities":{
            "content":{"kind":"working_tree","digest":"a".repeat(64)},
            "policy":{"source":"trusted-local","revision":"r1","digest":"b".repeat(64)},
            "tools":[{"id":"ruff","version":"0.16.8","binary_sha256":"c".repeat(64)}]
        },
        "results":[{
            "obligation_id":"python/app/lint/ruff",
            "completion":"complete",
            "findings":[{
                "id":"f-1",
                "native_rule_id":"F401",
                "tool_id":"ruff",
                "severity":"error",
                "gate_impact":"blocking",
                "message":"unused import",
                "obligation_id":"python/app/lint/ruff",
                "evidence_refs":["run:native-001:ruff"]
            }],
            "coverage":{"expected":["src/main.py"],"observed":["src/main.py"],"unresolved":[],"proof_kind":"native_report"},
            "execution_refs":["run:native-001:ruff"]
        }],
        "delivery_gate":{
            "decision":"not_evaluated",
            "eligible":false,
            "blocking_finding_ids":["f-1"],
            "incomplete_obligation_ids":[],
            "coverage_mismatch_ids":[],
            "invalid_ledger_ids":[]
        },
        "warnings":[],
        "next_actions":["修复 F401 后用原生 Ruff 复检"]
    })
}

pub fn invocation() -> InvocationDescriptor {
    InvocationDescriptor::parse(br#"{"version":"codeguard.invocation/v1alpha1","package_version":"0.1.4","command":"lint","flags":["--format=json"],"native_schema":"1.0","request_id":"run-001","run_id":"native-001","process_exit":1}"#).unwrap()
}
