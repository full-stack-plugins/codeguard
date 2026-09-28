use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("cg-policy-{}-{id}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, value: &Value) {
        fs::write(self.0.join(name), serde_json::to_vec(value).unwrap()).unwrap();
    }

    fn inspect(&self) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "config",
                "validate",
                self.0.to_str().unwrap(),
                "--policy-candidate",
                self.0
                    .join("quality-policy.candidate.json")
                    .to_str()
                    .unwrap(),
                "--format=json",
            ])
            .output()
            .unwrap();
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn candidate() -> Value {
    json!({
        "schema_version":"1.0", "policy_id":"company-codeguard", "revision":"r42",
        "tool_lock_sha256":"a".repeat(64),
        "required_checks":[{
            "id":"python-lint", "language":"python", "category":"lint", "check_kind":"lint.style",
            "rulepack_sha256":"b".repeat(64), "rule_ids":["python.ruff.F401"],
            "source_sets":["src"], "blocking_severities":["error"]
        }],
        "source_exclusions":[], "cve_freshness_hours":24, "tests_required":true
    })
}

#[test]
fn valid_shape_stays_untrusted_and_cannot_authorize_local_exclusion() {
    let project = Project::new();
    project.write("quality-policy.candidate.json", &candidate());
    project.write("codeguard.json", &json!({"exclude":["src/.*"]}));
    let (exit, report) = project.inspect();
    assert_eq!(exit, 3);
    assert_eq!(report["quality_policy"]["status"], "candidate_unverified");
    assert_eq!(report["quality_policy"]["required_check_count"], 1);
    assert_eq!(report["quality_policy"]["source_exclusion_count"], 0);
    assert_eq!(report["authority"], "unverified");
    assert_eq!(report["gate_effect"], "none");
    assert!(
        report
            .to_string()
            .contains("legacy_exclusions_require_approved_policy")
    );
}

#[test]
fn self_approval_unknown_fields_and_wildcard_exclusion_fail_closed() {
    let project = Project::new();
    let mut value = candidate();
    value["approved"] = json!(true);
    project.write("quality-policy.candidate.json", &value);
    let (_, report) = project.inspect();
    assert_eq!(report["quality_policy"]["status"], "invalid");

    let mut value = candidate();
    value["source_exclusions"] = json!([{"id":"x","path":"src/**","file_sha256":"c".repeat(64),"reason":"generated_source","expires_at":1791000000_u64}]);
    project.write("quality-policy.candidate.json", &value);
    let (_, report) = project.inspect();
    assert_eq!(report["quality_policy"]["status"], "invalid");
}

#[test]
fn empty_required_checks_and_duplicate_obligation_ids_are_rejected() {
    let project = Project::new();
    let mut value = candidate();
    value["required_checks"] = json!([]);
    project.write("quality-policy.candidate.json", &value);
    let (_, report) = project.inspect();
    assert_eq!(report["quality_policy"]["status"], "invalid");

    let mut value = candidate();
    let duplicate = value["required_checks"][0].clone();
    value["required_checks"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    project.write("quality-policy.candidate.json", &value);
    let (_, report) = project.inspect();
    assert_eq!(report["quality_policy"]["status"], "invalid");
}

#[test]
fn candidate_tool_lock_reference_matches_bytes_but_remains_untrusted() {
    let project = Project::new();
    let lock = json!({
        "schema_version":"1.0", "lock_id":"local-ruff", "tools":[{
            "id":"ruff", "version":"0.16.8", "binary_sha256":"d".repeat(64),
            "platform":"macos_arm64", "adapter":{"id":"python-ruff","version":"0.1.0"},
            "rule_source":{"kind":"native_builtin","id":"ruff-builtin","sha256":"e".repeat(64)},
            "origin":{"kind":"system","ref":"/usr/local/bin/ruff"}
        }]
    });
    project.write("codeguard.lock.json", &lock);
    let mut policy = candidate();
    policy["tool_lock_sha256"] = json!(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&lock).unwrap())
    ));
    project.write("quality-policy.candidate.json", &policy);
    let (_, report) = project.inspect();
    assert_eq!(
        report["quality_policy"]["tool_lock_reference"],
        "matched_untrusted"
    );
    assert_eq!(report["gate_effect"], "none");

    project.write(
        "codeguard.lock.json",
        &json!({"schema_version":"1.0","lock_id":"tampered","tools":lock["tools"]}),
    );
    let (_, report) = project.inspect();
    assert_eq!(report["quality_policy"]["tool_lock_reference"], "mismatch");
    assert_eq!(report["quality_decision"], "not_evaluated");
}

#[test]
fn exact_content_bound_exclusion_is_a_candidate_not_a_whitelist() {
    let project = Project::new();
    let mut policy = candidate();
    policy["source_exclusions"] = json!([{"id":"generated-one","path":"src/generated.py","file_sha256":"c".repeat(64),"reason":"generated_source","expires_at":1791000000_u64}]);
    project.write("quality-policy.candidate.json", &policy);
    let (_, report) = project.inspect();
    assert_eq!(report["quality_policy"]["status"], "candidate_unverified");
    assert_eq!(report["quality_policy"]["source_exclusion_count"], 1);
    assert_eq!(report["whitelist"]["candidate_effect"], "none");
    assert_eq!(report["gate_effect"], "none");
}

#[test]
fn unknown_language_and_path_traversal_are_invalid() {
    let project = Project::new();
    let mut policy = candidate();
    policy["required_checks"][0]["language"] = json!("unknown_future_language");
    project.write("quality-policy.candidate.json", &policy);
    let (_, report) = project.inspect();
    assert_eq!(report["quality_policy"]["status"], "invalid");

    let mut policy = candidate();
    policy["source_exclusions"] = json!([{"id":"outside","path":"../secrets.py","file_sha256":"c".repeat(64),"reason":"generated_source","expires_at":1791000000_u64}]);
    project.write("quality-policy.candidate.json", &policy);
    let (_, report) = project.inspect();
    assert_eq!(report["quality_policy"]["status"], "invalid");
}

#[test]
fn candidate_schema_contains_no_approval_field() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/quality-policy-candidate.schema.json"
    ))
    .unwrap();
    assert_eq!(schema["additionalProperties"], false);
    assert!(schema["properties"].get("approved").is_none());
    assert_eq!(schema["properties"]["required_checks"]["minItems"], 1);
    assert_eq!(
        schema["properties"]["source_exclusions"]["items"]["additionalProperties"],
        false
    );
}
