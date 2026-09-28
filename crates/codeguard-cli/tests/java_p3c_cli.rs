#![cfg(unix)]

use codeguard_cli::tool_identity::hash_bundle_tree;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    source: PathBuf,
    maven: PathBuf,
    java_home: PathBuf,
    repo: PathBuf,
}

impl Fixture {
    fn new(script: &str) -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-java-cli-{}-{id}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let source = root.join("Bad_Name.java");
        fs::write(&source, b"class Bad_Name {}\n").unwrap();
        let maven = root.join("mvn");
        fs::write(&maven, script).unwrap();
        fs::set_permissions(&maven, fs::Permissions::from_mode(0o700)).unwrap();
        let java_home = root.join("jdk");
        fs::create_dir(&java_home).unwrap();
        fs::create_dir(java_home.join("bin")).unwrap();
        fs::write(java_home.join("bin/java"), b"fixture java").unwrap();
        let repo = root.join("repo");
        fs::create_dir(&repo).unwrap();
        fs::write(repo.join("artifact.jar"), b"fixture").unwrap();
        Self {
            root,
            source,
            maven,
            java_home,
            repo,
        }
    }

    fn run(&self, digest: &str) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "java",
                self.source.to_str().unwrap(),
                "--maven-tool",
                self.maven.to_str().unwrap(),
                "--java-home",
                self.java_home.to_str().unwrap(),
                "--maven-repo",
                self.repo.to_str().unwrap(),
                "--repo-sha256",
                digest,
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

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn successful_native_exit_without_fresh_pmd_report_is_incomplete() {
    let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
    let digest = hash_bundle_tree(&fixture.repo).unwrap();
    let before = fs::read(&fixture.source).unwrap();
    let (exit, report) = fixture.run(&digest);
    assert_eq!(exit, 3);
    assert_eq!(report["report_type"], "java_p3c_local_feedback");
    assert_eq!(report["local_status"], "incomplete");
    assert_eq!(report["reason"], "native_report_missing_or_invalid");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(fs::read(&fixture.source).unwrap(), before);
    assert!(!fixture.root.join("target").exists());
}

#[test]
fn changed_dependency_closure_prevents_native_launch() {
    let fixture = Fixture::new("#!/bin/sh\nprintf touched > ../launched\nexit 0\n");
    let (exit, report) = fixture.run(&"0".repeat(64));
    assert_eq!(exit, 3);
    assert_eq!(report["reason"], "dependency_closure_mismatch");
    assert!(!fixture.root.join("launched").exists());
}

#[test]
fn native_mutation_of_copied_source_invalidates_observation() {
    let fixture = Fixture::new(
        "#!/bin/sh\nprintf 'class Changed {}\\n' > src/main/java/Bad_Name.java\nexit 0\n",
    );
    let digest = hash_bundle_tree(&fixture.repo).unwrap();
    let (exit, report) = fixture.run(&digest);
    assert_eq!(exit, 3);
    assert_eq!(report["reason"], "source_changed_during_scan");
    assert_eq!(fs::read(&fixture.source).unwrap(), b"class Bad_Name {}\n");
}

#[test]
fn feedback_protocol_cannot_claim_quality_pass() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/java-p3c-local-feedback.schema.json"
    ))
    .unwrap();
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        schema["properties"]["delivery_decision"]["const"],
        "not_evaluated"
    );
    assert_eq!(schema["properties"]["coverage_proven"]["const"], false);
    assert_eq!(schema["properties"]["exit_code"]["const"], 3);
    assert_eq!(schema["properties"]["schema_version"]["const"], "0.2.0");
    assert_eq!(schema["properties"]["declared_rulesets"]["minItems"], 10);
}

#[test]
fn reported_declared_rulesets_match_embedded_native_plan() {
    let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
    let digest = hash_bundle_tree(&fixture.repo).unwrap();
    let (_, report) = fixture.run(&digest);
    let pom = include_str!("../resources/p3c_single_file_pom.xml");
    let rulesets = report["declared_rulesets"].as_array().unwrap();
    assert_eq!(rulesets.len(), 10);
    assert_eq!(pom.matches("<ruleset>").count(), rulesets.len());
    for ruleset in rulesets {
        let name = ruleset.as_str().unwrap();
        assert!(pom.contains(&format!("<ruleset>{name}</ruleset>")));
    }
    assert_eq!(
        report["native_plan_sha256"],
        format!("{:x}", Sha256::digest(pom.as_bytes()))
    );
}

#[test]
fn human_feedback_uses_plain_status_and_recovery_reason() {
    let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
    let digest = hash_bundle_tree(&fixture.repo).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "java",
            fixture.source.to_str().unwrap(),
            "--maven-tool",
            fixture.maven.to_str().unwrap(),
            "--java-home",
            fixture.java_home.to_str().unwrap(),
            "--maven-repo",
            fixture.repo.to_str().unwrap(),
            "--repo-sha256",
            &digest,
            "--format=human",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("原生诊断：incomplete"));
    assert!(stdout.contains("待核实：native_report_missing_or_invalid"));
    assert!(!stdout.contains("\"incomplete\""));
}

#[test]
#[ignore = "requires native Maven, JDK and pinned offline P3C dependency closure"]
fn native_p3c_finding_is_visible_in_public_java_lint() {
    let tool = std::env::var("CODEGUARD_MAVEN_BIN").unwrap();
    let java_home = std::env::var("CODEGUARD_JAVA_HOME").unwrap();
    let repo = std::env::var("CODEGUARD_P3C_MAVEN_REPO").unwrap();
    let repo_sha256 = std::env::var("CODEGUARD_P3C_REPO_TREE_SHA256").unwrap();
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/p3c_native/violating/Bad_Name.java")
        .canonicalize()
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "java",
            source.to_str().unwrap(),
            "--maven-tool",
            &tool,
            "--java-home",
            &java_home,
            "--maven-repo",
            &repo,
            "--repo-sha256",
            &repo_sha256,
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["local_status"], "findings_observed_untrusted",
        "{report}"
    );
    assert!(
        report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| {
                finding["rule_id"] == "ClassNamingShouldBeCamelRule"
                    && finding["ruleset"] == "AlibabaJavaNaming"
            })
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
#[ignore = "requires native Maven, JDK and pinned offline P3C dependency closure"]
fn native_clean_xml_never_becomes_a_quality_pass() {
    let tool = std::env::var("CODEGUARD_MAVEN_BIN").unwrap();
    let java_home = std::env::var("CODEGUARD_JAVA_HOME").unwrap();
    let repo = std::env::var("CODEGUARD_P3C_MAVEN_REPO").unwrap();
    let repo_sha256 = std::env::var("CODEGUARD_P3C_REPO_TREE_SHA256").unwrap();
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/p3c_native/clean/GoodName.java")
        .canonicalize()
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "java",
            source.to_str().unwrap(),
            "--maven-tool",
            &tool,
            "--java-home",
            &java_home,
            "--maven-repo",
            &repo,
            "--repo-sha256",
            &repo_sha256,
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["local_status"], "clean_scope_unproven", "{report}");
    assert!(report["findings"].as_array().unwrap().is_empty());
    assert_eq!(report["coverage_proven"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
}
