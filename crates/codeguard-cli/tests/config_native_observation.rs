use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "cg-config-native-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn inspect(&self, action: &str) -> Value {
        let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["config", action])
            .arg(&self.0)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(3));
        serde_json::from_slice(&o.stdout).unwrap()
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
const POM: &str = r#"<project><modelVersion>4.0.0</modelVersion><groupId>example</groupId><artifactId>demo</artifactId><version>1</version><build><plugins><plugin><groupId>org.apache.maven.plugins</groupId><artifactId>maven-pmd-plugin</artifactId><version>3.25.0</version><configuration><skipPmdError>false</skipPmdError><rulesets><ruleset>rulesets/java/ali-naming.xml</ruleset></rulesets></configuration><dependencies><dependency><groupId>com.alibaba.p3c</groupId><artifactId>p3c-pmd</artifactId><version>2.1.1</version></dependency></dependencies></plugin></plugins></build></project>"#;
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
#[test]
fn config_explain_reports_native_configuration_sources_without_running_them() {
    let p = Project::new();
    for dir in ["java", "python", "node"] {
        fs::create_dir(p.0.join(dir)).unwrap();
    }
    fs::write(p.0.join("java/pom.xml"), POM).unwrap();
    fs::write(p.0.join("python/a.py"), "pass\n").unwrap();
    fs::write(p.0.join("python/.ruff.toml"), "[lint]\nselect=['E']\n").unwrap();
    fs::write(
        p.0.join("node/package.json"),
        json!({"devDependencies":{"eslint":"10.0.0"}}).to_string(),
    )
    .unwrap();
    fs::write(p.0.join("node/a.ts"), "export const n=1;\n").unwrap();
    fs::write(
        p.0.join("node/eslint.config.js"),
        "require('fs').writeFileSync('executed-config', 'bad');module.exports=[];",
    )
    .unwrap();
    let report = p.inspect("explain");
    assert_eq!(report["schema_version"], "0.3.0");
    let observation = &report["project_configuration"];
    assert_eq!(observation["basis"], "static_project_discovery");
    assert_eq!(observation["native_execution"], "not_run");
    assert_eq!(observation["effective_rules"], "unresolved");
    assert_eq!(observation["suppressions"], "unresolved");
    let rows = observation["checker_configurations"].as_array().unwrap();
    let p3c = rows
        .iter()
        .find(|r| r["checker_id"] == "java.maven.p3c")
        .unwrap();
    assert_eq!(p3c["configuration"], "configured");
    assert_eq!(p3c["configuration_ref"], "java/pom.xml");
    let ruff = rows
        .iter()
        .find(|r| r["checker_id"] == "python.ruff")
        .unwrap();
    assert_eq!(ruff["configuration"], "configured");
    let eslint = rows
        .iter()
        .find(|r| r["checker_id"] == "node.eslint")
        .unwrap();
    assert_eq!(eslint["configuration"], "unknown");
    assert_eq!(
        eslint["reason"],
        "eslint_dynamic_configuration_not_evaluated"
    );
    assert_eq!(eslint["configuration_ref"], "node/eslint.config.js");
    assert_eq!(
        observation["manifest_sha256"]["java/pom.xml"],
        digest(POM.as_bytes())
    );
    assert_eq!(
        observation["checker_config_sha256"]["python/.ruff.toml"],
        digest(b"[lint]\nselect=['E']\n")
    );
    assert_eq!(report["effective_policy"], Value::Null);
    assert_eq!(report["quality_decision"], "not_evaluated");
    assert!(!p.0.join("executed-config").exists());
    assert!(!p.0.join("node/executed-config").exists());
    assert!(!p.0.join(".codeguard").exists());
    if let Ok(dir) = std::env::var("CODEGUARD_CONFIG_REPORT_DIR") {
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            PathBuf::from(dir).join("native-config-explain.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }
}
#[test]
fn p3c_unresolved_error_policy_is_not_reported_as_configured() {
    let p = Project::new();
    fs::write(
        p.0.join("pom.xml"),
        POM.replace("<skipPmdError>false</skipPmdError>", ""),
    )
    .unwrap();
    let report = p.inspect("explain");
    let rows = report["project_configuration"]["checker_configurations"]
        .as_array()
        .unwrap();
    let p3c = rows
        .iter()
        .find(|r| r["checker_id"] == "java.maven.p3c")
        .unwrap();
    assert_eq!(p3c["configuration"], "unknown");
    assert_eq!(p3c["reason"], "p3c_processing_error_policy_unresolved");
    assert!(
        p3c["next_action"]
            .as_str()
            .unwrap()
            .contains("skipPmdError")
    );
}
#[test]
fn configuration_rows_are_bounded_without_hiding_total_scope() {
    let p = Project::new();
    for index in 0..40 {
        let d = p.0.join(format!("module-{index:02}"));
        fs::create_dir(&d).unwrap();
        fs::write(d.join("pom.xml"), "<project/>").unwrap();
    }
    let report = p.inspect("validate");
    let obs = &report["project_configuration"];
    assert_eq!(obs["checker_count"], 280);
    assert_eq!(obs["checker_configurations"].as_array().unwrap().len(), 256);
    assert_eq!(obs["truncated"], true);
    assert_eq!(obs["observation_status"], "incomplete");
    assert_eq!(report["quality_decision"], "not_evaluated");
}
#[test]
fn malformed_native_config_is_distinct_from_missing_config_and_policy() {
    let p = Project::new();
    fs::write(p.0.join("a.py"), "pass\n").unwrap();
    fs::write(p.0.join(".ruff.toml"), "[lint\n").unwrap();
    let report = p.inspect("validate");
    let rows = report["project_configuration"]["checker_configurations"]
        .as_array()
        .unwrap();
    assert_eq!(
        rows.iter()
            .find(|r| r["checker_id"] == "python.ruff")
            .unwrap()["configuration"],
        "invalid"
    );
    assert_eq!(report["quality_policy"]["status"], "unbound");
    fs::remove_file(p.0.join(".ruff.toml")).unwrap();
    let report = p.inspect("validate");
    let rows = report["project_configuration"]["checker_configurations"]
        .as_array()
        .unwrap();
    assert_eq!(
        rows.iter()
            .find(|r| r["checker_id"] == "python.ruff")
            .unwrap()["configuration"],
        "missing"
    );
}
#[test]
fn empty_project_and_inaccessible_scope_are_not_quality_passes() {
    let p = Project::new();
    let report = p.inspect("explain");
    assert_eq!(report["project_configuration"]["checker_count"], 0);
    assert_eq!(report["quality_decision"], "not_evaluated");
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["config", "explain"])
        .arg(p.0.join("missing"))
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(
        report["project_configuration"]["observation_status"],
        "incomplete"
    );
}
#[test]
fn legacy_duplicate_keys_do_not_hide_exclusions_or_commands() {
    for raw in [
        r#"{"exclude":["src/**"],"exclude":[]}"#,
        r#"{"java":{"commands":[["mvn","verify"]],"commands":[]}}"#,
    ] {
        let p = Project::new();
        fs::write(p.0.join("codeguard.json"), raw).unwrap();
        let report = p.inspect("validate");
        assert_eq!(report["legacy_config"]["status"], "invalid");
    }
}
#[test]
fn human_explain_shows_static_checker_state_and_unresolved_suppressions() {
    let p = Project::new();
    fs::write(p.0.join("pom.xml"), POM).unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["config", "explain"])
        .arg(&p.0)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(3));
    let text = String::from_utf8(o.stdout).unwrap();
    assert!(text.contains("java.maven.p3c"));
    assert!(text.contains("configured"));
    assert!(text.contains("抑制") && text.contains("未解析"));
    assert!(text.contains("pom.xml"));
    assert!(text.contains("p3c_artifact_ruleset_and_error_policy_declared"));
    assert!(text.contains("运行原生 Maven PMD"));
}
