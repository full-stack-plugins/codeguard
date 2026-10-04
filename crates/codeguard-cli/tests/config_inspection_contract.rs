use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("cg-config-{}-{id}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, value: &Value) {
        fs::write(self.0.join(name), serde_json::to_vec(value).unwrap()).unwrap();
    }

    fn inspect(&self, action: &str) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["config", action, self.0.to_str().unwrap(), "--format=json"])
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

#[test]
fn legacy_exclusion_and_commands_are_observed_but_never_authorized() {
    let project = Project::new();
    project.write(
        "codeguard.json",
        &json!({"exclude":["src/.*"],"gate_scope":"delta","java":{"commands":[["mvn","test"]]}}),
    );
    let before = fs::read(project.0.join("codeguard.json")).unwrap();
    let (exit, report) = project.inspect("validate");
    assert_eq!(exit, 3);
    assert_eq!(report["report_type"], "config_inspection");
    assert_eq!(report["schema_version"], "0.3.0");
    assert!(
        report["request_id"]
            .as_str()
            .is_some_and(|id| !id.is_empty())
    );
    assert_eq!(report["inspection_status"], "incomplete");
    assert_eq!(report["authority"], "unverified");
    assert_eq!(report["gate_effect"], "none");
    assert_eq!(
        report["legacy_config"]["status"],
        "structurally_valid_untrusted"
    );
    assert_eq!(report["legacy_config"]["exclusion_count"], 1);
    assert_eq!(report["legacy_config"]["custom_command_count"], 1);
    assert!(!report.to_string().contains("mvn"));
    assert_eq!(fs::read(project.0.join("codeguard.json")).unwrap(), before);
}

#[test]
fn unknown_and_malformed_legacy_fields_are_not_silently_ignored() {
    let project = Project::new();
    project.write("codeguard.json", &json!({"approved":true}));
    let (exit, report) = project.inspect("validate");
    assert_eq!(exit, 3);
    assert_eq!(report["legacy_config"]["status"], "invalid");
    assert!(report.to_string().contains("unknown_field"));

    project.write("codeguard.json", &json!({"java":{"commands":[[]]}}));
    let (_, report) = project.inspect("validate");
    assert_eq!(report["legacy_config"]["status"], "invalid");
}

#[test]
fn explain_keeps_sources_partial_without_effective_policy_claim() {
    let project = Project::new();
    project.write("codeguard.json", &json!({"gate_scope":"repo"}));
    let (exit, report) = project.inspect("explain");
    assert_eq!(exit, 3);
    assert_eq!(report["operation"], "explain");
    assert_eq!(report["quality_policy"]["status"], "unbound");
    assert_eq!(report["tool_lock"]["status"], "missing");
    assert_eq!(report["effective_policy"], Value::Null);
}

#[cfg(unix)]
#[test]
fn symlinked_project_config_is_not_followed() {
    use std::os::unix::fs::symlink;
    let project = Project::new();
    symlink("/etc/passwd", project.0.join("codeguard.json")).unwrap();
    let (_, report) = project.inspect("validate");
    assert_eq!(report["legacy_config"]["status"], "invalid");
    assert!(!report.to_string().contains("root:"));
}

#[test]
fn malformed_tool_lock_and_forged_local_decision_never_grant_a_whitelist() {
    let project = Project::new();
    project.write("codeguard.lock.json", &json!({"approved":true}));
    fs::create_dir(project.0.join(".codeguard")).unwrap();
    fs::create_dir(project.0.join(".codeguard/decisions")).unwrap();
    project.write(
        ".codeguard/decisions/allow.json",
        &json!({"approved":true,"path":"*"}),
    );
    let (exit, report) = project.inspect("validate");
    assert_eq!(exit, 3);
    assert_eq!(report["tool_lock"]["status"], "invalid");
    assert_eq!(report["whitelist"]["status"], "authority_unverified");
    assert_eq!(report["whitelist"]["candidate_effect"], "none");
}

#[test]
fn inspection_schema_cannot_express_an_approved_gate() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/config-inspection.schema.json"
    ))
    .unwrap();
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["authority"]["const"], "unverified");
    assert_eq!(schema["properties"]["gate_effect"]["const"], "none");
    assert_eq!(schema["properties"]["effective_policy"]["type"], "null");
}
