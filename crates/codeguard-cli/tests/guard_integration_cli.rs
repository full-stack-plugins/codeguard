#![cfg(unix)]
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};
#[path = "support/guard_integration.rs"]
mod fixture;
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Case(PathBuf);
impl Case {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "cg-guard-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let case = Self(root);
        case.put("native", fixture::valid());
        case.put(
            "invocation",
            serde_json::to_value(fixture::invocation()).unwrap(),
        );
        case.put("context", json!({"version":"codeguard.context/v1alpha1","runId":"native-001",
            "binding":{"repoId":"repo","taskId":"task","worktreeId":"worktree","requirementIds":["req"],"candidateOid":"a".repeat(40),"baseOid":"b".repeat(40),"mergeGroupId":null,"sourceSnapshotDigest":format!("sha256:{}", "a".repeat(64)),"baselineDigest":null},
            "requiredTargets":{"python/app/lint/ruff":["src/main.py"]},
            "startedAt":"2026-10-09T00:00:00Z","finishedAt":"2026-10-09T00:01:00Z"}));
        case.put("mapping", json!({"version":"codeguard.mapping/v1alpha1","entries":[
            {"source":{"kind":"finding","tool_id":"ruff","native_rule_id":"F401"},"rule_id":"r"},
            {"source":{"kind":"gap","detail":"native profile unqualified"},"rule_id":"r"},
            {"source":{"kind":"gap","detail":"native aggregate incomplete"},"rule_id":"r"},
            {"source":{"kind":"gap","detail":"incomplete obligation python/app/lint/ruff"},"rule_id":"r"}]}));
        case.put("contract", json!({"apiVersion":guardengine::API_VERSION,"kind":"GuardContract","metadata":{"id":"c","revision":"1"},"spec":{"rules":[{"id":"r","enforcement":"advise","assertion":{"type":"forbid_relation","subject":"code","predicate":"has","object":"gap"}}]}}));
        case
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(format!("{name}.json"))
    }
    fn put(&self, name: &str, value: Value) {
        fs::write(self.path(name), serde_json::to_vec(&value).unwrap()).unwrap();
    }
    fn get(&self, name: &str) -> Value {
        serde_json::from_slice(&fs::read(self.path(name)).unwrap()).unwrap()
    }
    fn run(&self, extra: &[&str]) -> Output {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        cmd.arg("guard-project");
        for (flag, file) in [
            ("--invocation", "invocation"),
            ("--native-report", "native"),
            ("--context", "context"),
            ("--mapping", "mapping"),
            ("--contract", "contract"),
        ] {
            cmd.arg(flag).arg(self.path(file));
        }
        cmd.args(extra)
            .env("PATH", "")
            .current_dir(&self.0)
            .output()
            .unwrap()
    }
    fn native_exit(&self, exit: u64, status: &str) {
        let mut report = self.get("native");
        report["exit_code"] = json!(exit);
        report["command_status"] = json!(status);
        let mut invocation = self.get("invocation");
        invocation["process_exit"] = json!(exit);
        self.put("native", report);
        self.put("invocation", invocation);
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn bundle(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "{e}; stdout={}; stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}
fn assert_unbound(output: Output) {
    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(diagnostic["kind"], "GuardTransportDiagnostic");
    assert_eq!(diagnostic["phase"], "unbound");
}
#[test]
fn explicit_cli_emits_recomputable_partial_bundle_with_original_native_bytes() {
    let case = Case::new();
    for mode in ["enforce", "review", "advise"] {
        let mut c = case.get("contract");
        c["spec"]["rules"][0]["enforcement"] = json!(mode);
        case.put("contract", c);
        let output = case.run(&[]);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stderr.is_empty());
        let result = bundle(&output);
        assert_eq!(result["apiVersion"], "codeguard.projection/v1alpha1");
        assert_eq!(result["kind"], "GuardProjectionBundle");
        assert_eq!(result["nativeExit"], 1);
        assert_eq!(result["envelope"]["coverage"]["status"], "partial");
        assert_eq!(result["envelope"]["decision"], "BLOCK");
        assert_eq!(
            result["artifacts"]["domain"].as_str().unwrap().as_bytes(),
            fs::read(case.path("native")).unwrap()
        );
        let envelope = serde_json::from_value(result["envelope"].clone()).unwrap();
        guardengine::integration::verify_engine_artifacts(
            &envelope,
            result["artifacts"]["contract"].as_str().unwrap().as_bytes(),
            result["artifacts"]["facts"].as_str().unwrap().as_bytes(),
            result["artifacts"]["report"].as_str().unwrap().as_bytes(),
        )
        .unwrap();
    }
}
#[test]
fn invalid_arguments_context_and_policy_are_unbound_stderr_only() {
    assert_unbound(
        Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("guard-project")
            .output()
            .unwrap(),
    );
    let case = Case::new();
    assert_unbound(case.run(&["--repair"]));
    assert_unbound(case.run(&["--context", "duplicate"]));
    for pointer in [
        "/binding/repoId",
        "/binding/candidateOid",
        "/binding/baseOid",
    ] {
        let case = Case::new();
        let mut context = case.get("context");
        *context.pointer_mut(pointer).unwrap() = json!("");
        case.put("context", context);
        assert_unbound(case.run(&[]));
    }
    let case = Case::new();
    let mut context = case.get("context");
    context["requiredTargets"] = json!({});
    case.put("context", context);
    assert_unbound(case.run(&[]));
    let case = Case::new();
    fs::write(case.path("contract"), "&anchor {not: supported YAML}").unwrap();
    assert_unbound(case.run(&[]));
}
#[test]
fn bound_native_errors_and_cancellation_preserve_original_exit_and_null_decision() {
    for (exit, native, expected) in [
        (4, "internal_error", "error"),
        (130, "cancelled", "cancelled"),
    ] {
        let case = Case::new();
        case.native_exit(exit, native);
        let output = case.run(&[]);
        assert_eq!(output.status.code(), Some(4));
        let result = bundle(&output);
        assert_eq!(result["envelope"]["runStatus"], expected);
        assert!(result["envelope"]["decision"].is_null());
        assert_eq!(result["nativeExit"], exit);
        assert!(result["artifacts"]["report"].is_null());
        assert_eq!(
            result["artifacts"]["domain"].as_str().unwrap().as_bytes(),
            fs::read(case.path("native")).unwrap()
        );
    }
}
#[test]
fn failures_after_binding_emit_error_envelopes_without_success_reports() {
    for failure in ["malformed", "missing", "unmapped", "stale"] {
        let case = Case::new();
        match failure {
            "malformed" => fs::write(case.path("native"), "{").unwrap(),
            "missing" => fs::remove_file(case.path("native")).unwrap(),
            "unmapped" => {
                let mut m = case.get("mapping");
                m["entries"].as_array_mut().unwrap().remove(0);
                case.put("mapping", m);
            }
            "stale" => {
                let mut report = case.get("native");
                report["run_id"] = json!("old-run");
                case.put("native", report);
            }
            _ => unreachable!(),
        }
        let output = case.run(&[]);
        assert_eq!(output.status.code(), Some(4), "{failure}");
        let result = bundle(&output);
        assert_eq!(result["envelope"]["runStatus"], "error");
        assert!(result["envelope"]["decision"].is_null());
        assert!(result["artifacts"]["report"].is_null());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["phase"], "bound");
        if failure == "unmapped" {
            assert!(result["artifacts"]["domain"].is_string());
        } else {
            assert!(result["artifacts"]["domain"].is_null());
        }
    }
}
#[test]
fn native_aggregate_three_remains_partial_block_not_approval() {
    let case = Case::new();
    case.native_exit(3, "incomplete");
    let mut native = case.get("native");
    native["results"][0]["completion"] = json!("incomplete");
    native["results"][0]["reason"] = json!("native_timeout");
    native["delivery_gate"]["incomplete_obligation_ids"] = json!(["python/app/lint/ruff"]);
    case.put("native", native);
    let output = case.run(&[]);
    assert_eq!(output.status.code(), Some(2));
    let result = bundle(&output);
    assert_eq!(result["nativeExit"], 3);
    assert_eq!(result["envelope"]["decision"], "BLOCK");
}
#[test]
fn help_exposes_opt_in_partial_entry_without_running_it() {
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["help", "guard-project", "--format=json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["commands"][0]["support"], "partial");
    assert!(
        value["commands"][0]["usage"]
            .as_str()
            .unwrap()
            .contains("--context")
    );
}
#[cfg(unix)]
#[test]
fn native_check_keeps_exit_three_and_stdout_when_output_is_requested() {
    let case = Case::new();
    fs::write(case.0.join("main.rs"), "fn main() {}\n").unwrap();
    let report = case.0.join("native-export.json");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all"])
        .arg(&case.0)
        .arg("--format=json")
        .arg("--output")
        .arg(&report)
        .env("PATH", "")
        .env_remove("CODEGUARD_TIMEOUT")
        .env_remove("CODEGUARD_JOBS")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let stdout: Value = serde_json::from_slice(&output.stdout).unwrap();
    let file: Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
    assert_eq!(stdout, file);
    assert_eq!(stdout["report_type"], "check_feedback");
    assert!(stdout.get("envelope").is_none());
}

#[test]
fn strict_context_and_input_file_limits_do_not_create_success_evidence() {
    for change in ["version", "extra", "oversize", "duplicate"] {
        let case = Case::new();
        match change {
            "oversize" => fs::write(case.path("context"), vec![b' '; 1_048_577]).unwrap(),
            "duplicate" => {
                let bytes = fs::read_to_string(case.path("context")).unwrap();
                fs::write(
                    case.path("context"),
                    bytes.replacen('{', "{\"version\":\"duplicate\",", 1),
                )
                .unwrap();
            }
            field => {
                let mut value = case.get("context");
                value[field] = json!("unsupported");
                case.put("context", value);
            }
        }
        assert_unbound(case.run(&[]));
    }
    let case = Case::new();
    std::os::unix::fs::symlink(case.path("contract"), case.0.join("linked.json")).unwrap();
    fs::rename(case.0.join("linked.json"), case.path("context")).unwrap();
    assert_unbound(case.run(&[]));
}
#[test]
fn oversized_inline_transport_falls_back_to_bound_error_and_keeps_native_bytes() {
    let case = Case::new();
    let mut native = case.get("native");
    let template = native["results"][0]["findings"][0].clone();
    let findings: Vec<_> = (0..40)
        .map(|n| {
            let mut f = template.clone();
            f["id"] = json!(format!("f-{n}"));
            f
        })
        .collect();
    native["delivery_gate"]["blocking_finding_ids"] =
        json!(findings.iter().map(|f| f["id"].clone()).collect::<Vec<_>>());
    native["results"][0]["findings"] = json!(findings);
    case.put("native", native);
    let mut c = case.get("contract");
    for field in ["subject", "predicate", "object"] {
        c["spec"]["rules"][0]["assertion"][field] = json!("\\".repeat(20_000));
    }
    case.put("contract", c);
    let output = case.run(&[]);
    assert_eq!(output.status.code(), Some(4));
    let result = bundle(&output);
    assert_eq!(result["envelope"]["runStatus"], "error");
    assert!(result["envelope"]["decision"].is_null());
    assert_eq!(
        result["envelope"]["diagnostics"][0]["code"],
        "transport_serialization_failed"
    );
    assert!(result["artifacts"]["report"].is_null());
    assert_eq!(
        result["artifacts"]["domain"].as_str().unwrap().as_bytes(),
        fs::read(case.path("native")).unwrap()
    );
}

#[test]
fn native_zero_findings_exit_zero_still_returns_block_two() {
    let case = Case::new();
    case.native_exit(0, "complete");
    let mut native = case.get("native");
    native["results"][0]["findings"] = json!([]);
    native["delivery_gate"]["blocking_finding_ids"] = json!([]);
    case.put("native", native);
    let output = case.run(&[]);
    assert_eq!(output.status.code(), Some(2));
    let result = bundle(&output);
    assert_eq!(result["nativeExit"], 0);
    assert_eq!(result["envelope"]["decision"], "BLOCK");
    assert_eq!(result["envelope"]["coverage"]["status"], "partial");
}
