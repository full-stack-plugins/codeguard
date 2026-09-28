#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use codeguard_cli::tool_identity::hash_bundle_tree;
use serde_json::Value;

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let path = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "codeguard-cve-native-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::create_dir_all(path.join("src/main/java")).unwrap();
        fs::write(path.join("src/main/java/A.java"), b"class A {}\n").unwrap();
        fs::write(path.join("pom.xml"), b"<project><modelVersion>4.0.0</modelVersion><groupId>demo</groupId><artifactId>app</artifactId><version>1</version><build><plugins><plugin><groupId>org.owasp</groupId><artifactId>dependency-check-maven</artifactId><version>12.1.0</version></plugin></plugins></build><dependencies><dependency><groupId>org.demo</groupId><artifactId>lib</artifactId><version>2.0</version></dependency></dependencies></project>").unwrap();
        Self(path)
    }

    fn check(&self, args: &[&str]) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["check", "java", self.0.to_str().unwrap(), "--format=json"])
            .args(args)
            .output()
            .unwrap();
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    }

    fn init(&self) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init", self.0.to_str().unwrap(), "--apply", "--format=json"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn pinned_native_owasp_advisory_reaches_feedback_without_cve_gate_claim() {
    let project = Project::new();
    project.init();
    let marker = project.0.join("native-started");
    let maven = project.0.join("mvn");
    fs::write(&maven, format!(r##"#!/bin/sh
pwd > '{}'
for arg in "$@"; do
 case "$arg" in -Dodc.outputDirectory=*) out=${{arg#-Dodc.outputDirectory=}} ;; esac
done
mkdir -p "$out"
cat > "$out/dependency-check-report.json" <<'EOF'
{{"reportSchema":"1.1","scanInfo":{{"engineVersion":"12.1.0","dataSource":[{{"name":"NVD","timestamp":"2026-09-25T00:00:00Z"}}]}},"projectInfo":{{"name":"app","reportDate":"2026-09-26T00:00:00Z"}},"dependencies":[{{"fileName":"lib.jar","isVirtual":false,"sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","packages":[{{"id":"pkg:maven/org.demo/lib@2.0"}},{{"id":"pkg:maven/org.demo/lib@2.0?repository_url=https://private.example/token"}}],"vulnerabilities":[{{"source":"NVD","name":"CVE-2026-1234","cvssv3":{{"baseScore":8.0}}}}]}}]}}
EOF
printf '[INFO] --- dependency-check:12.1.0:check (default-cli) @ app ---\n[INFO] BUILD SUCCESS\n'
"##, marker.display())).unwrap();
    fs::set_permissions(&maven, fs::Permissions::from_mode(0o700)).unwrap();
    let jdk = project.0.join("jdk");
    fs::create_dir_all(jdk.join("bin")).unwrap();
    fs::write(jdk.join("bin/java"), b"fake java").unwrap();
    fs::write(jdk.join("release"), b"JAVA_VERSION=21\n").unwrap();
    let repo = project.0.join("repo");
    fs::create_dir(&repo).unwrap();
    let db = project.0.join("db");
    fs::create_dir(&db).unwrap();
    fs::write(db.join("odc.mv.db"), b"test-db").unwrap();
    let repo_sha = hash_bundle_tree(&repo).unwrap();
    let db_sha = hash_bundle_tree(&db).unwrap();
    let (exit, report) = project.check(&[
        "--maven-tool",
        maven.to_str().unwrap(),
        "--java-home",
        jdk.to_str().unwrap(),
        "--maven-repo",
        repo.to_str().unwrap(),
        "--repo-sha256",
        &repo_sha,
        "--cve-data-dir",
        db.to_str().unwrap(),
        "--cve-data-sha256",
        &db_sha,
    ]);
    assert_eq!(exit, 3);
    assert!(marker.exists(), "{report}");
    let cve = &report["native_results"]["java_cve"]["probes"][0]["observation"];
    assert_eq!(
        cve["native_status"], "findings_observed_untrusted",
        "{report}"
    );
    assert_eq!(cve["advisories"][0]["advisory_id"], "CVE-2026-1234");
    assert_eq!(cve["advisories"][0]["score"], 8.0);
    assert_eq!(
        cve["advisories"][0]["package_ids"][0],
        "pkg:maven/org.demo/lib@2.0"
    );
    assert!(
        cve["advisories"][0]["package_ids"][1]
            .as_str()
            .unwrap()
            .starts_with("redacted-sha256:")
    );
    assert!(!report.to_string().contains("private.example"));
    assert_eq!(cve["database_freshness"], "unverified");
    assert_eq!(
        report["native_results"]["java_cve"]["backlog_status"],
        "synced_partial"
    );
    let blockers: Vec<_> = fs::read_dir(project.0.join("codeguard/findings"))
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path().join("finding.json");
            serde_json::from_slice::<Value>(&fs::read(path).unwrap()).unwrap()
        })
        .filter(|fact| fact["checker_id"] == "java.maven.dependency_check")
        .collect();
    assert_eq!(blockers.len(), 1);
    assert_eq!(blockers[0]["kind"], "blocker");
    assert_eq!(
        blockers[0]["reason_code"],
        "cve_database_freshness_unverified"
    );
    let blocker_id = blockers[0]["id"].as_str().unwrap();
    let verify = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            blocker_id,
            project.0.to_str().unwrap(),
            "--format=json",
        ])
        .args([
            "--maven-tool",
            maven.to_str().unwrap(),
            "--java-home",
            jdk.to_str().unwrap(),
            "--maven-repo",
            repo.to_str().unwrap(),
            "--repo-sha256",
            &repo_sha,
            "--cve-data-dir",
            db.to_str().unwrap(),
            "--cve-data-sha256",
            &db_sha,
        ])
        .output()
        .unwrap();
    assert_eq!(verify.status.code(), Some(3));
    let verify: Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(
        verify["native_scan"]["probes"][0]["observation"]["native_status"],
        "findings_observed_untrusted"
    );
    assert_eq!(verify["observation"], "still_blocked");
    assert_eq!(verify["event_persisted"], true, "{verify}");
    assert_eq!(verify["reason"], Value::Null);
    let task = fs::read_to_string(project.0.join(format!(
        "codeguard/tasks/{}.md",
        blockers[0]["id"].as_str().unwrap()
    )))
    .unwrap();
    assert!(task.contains("漏洞库"));
    assert!(task.contains("复检"));
    assert_eq!(
        report["native_results"]["java_cve"]["next"]["repair_brief"]["checker_id"],
        "java.maven.dependency_check"
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
    let human = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "java",
            project.0.to_str().unwrap(),
            "--format=human",
        ])
        .args([
            "--maven-tool",
            maven.to_str().unwrap(),
            "--java-home",
            jdk.to_str().unwrap(),
            "--maven-repo",
            repo.to_str().unwrap(),
            "--repo-sha256",
            &repo_sha,
            "--cve-data-dir",
            db.to_str().unwrap(),
            "--cve-data-sha256",
            &db_sha,
        ])
        .output()
        .unwrap();
    assert!(
        String::from_utf8(human.stdout)
            .unwrap()
            .contains("CVE-2026-1234")
    );
    fs::create_dir_all(project.0.join("child/src/main/java")).unwrap();
    fs::write(
        project.0.join("child/build.gradle.kts"),
        b"plugins { java }\n",
    )
    .unwrap();
    fs::write(
        project.0.join("child/src/main/java/Child.java"),
        b"class Child {}\n",
    )
    .unwrap();
    let (_, mixed) = project.check(&[
        "--maven-tool",
        maven.to_str().unwrap(),
        "--java-home",
        jdk.to_str().unwrap(),
        "--maven-repo",
        repo.to_str().unwrap(),
        "--repo-sha256",
        &repo_sha,
        "--cve-data-dir",
        db.to_str().unwrap(),
        "--cve-data-sha256",
        &db_sha,
    ]);
    let candidate = mixed["category_candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["language"] == "java" && entry["category"] == "cve")
        .unwrap();
    assert_eq!(candidate["checker_id"], Value::Null);
    assert_eq!(candidate["status"], "configuration_unresolved");
    assert_eq!(candidate["reason"], "checker_build_systems_mixed");
    assert_eq!(
        mixed["native_results"]["java_cve"]["observed_report_count"],
        1
    );
    let repeated_database_blockers = fs::read_dir(project.0.join("codeguard/findings"))
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path().join("finding.json");
            serde_json::from_slice::<Value>(&fs::read(path).unwrap()).unwrap()
        })
        .filter(|fact| {
            fact["checker_id"] == "java.maven.dependency_check"
                && fact["reason_code"] == "cve_database_freshness_unverified"
        })
        .count();
    assert_eq!(repeated_database_blockers, 1);
    let script = fs::read_to_string(&maven).unwrap();
    fs::write(
        &maven,
        script.replace("\"name\":\"app\"", "\"name\":\"other\""),
    )
    .unwrap();
    let (_, wrong_project) = project.check(&[
        "--maven-tool",
        maven.to_str().unwrap(),
        "--java-home",
        jdk.to_str().unwrap(),
        "--maven-repo",
        repo.to_str().unwrap(),
        "--repo-sha256",
        &repo_sha,
        "--cve-data-dir",
        db.to_str().unwrap(),
        "--cve-data-sha256",
        &db_sha,
    ]);
    let observation = &wrong_project["native_results"]["java_cve"]["probes"][0]["observation"];
    assert_eq!(observation["native_status"], "incomplete");
    assert_eq!(observation["reason"], "native_report_project_mismatch");
    assert!(observation["advisories"].as_array().unwrap().is_empty());
    fs::write(db.join("odc.mv.db"), b"changed-db").unwrap();
    let (_, changed_database) = project.check(&[
        "--maven-tool",
        maven.to_str().unwrap(),
        "--java-home",
        jdk.to_str().unwrap(),
        "--maven-repo",
        repo.to_str().unwrap(),
        "--repo-sha256",
        &repo_sha,
        "--cve-data-dir",
        db.to_str().unwrap(),
        "--cve-data-sha256",
        &db_sha,
    ]);
    let observation = &changed_database["native_results"]["java_cve"]["probes"][0]["observation"];
    assert_eq!(observation["native_status"], "incomplete");
    assert_eq!(observation["reason"], "native_identity_mismatch");
    assert!(observation["advisories"].as_array().unwrap().is_empty());
}

#[test]
fn missing_database_keeps_configured_checker_incomplete_before_native_start() {
    let project = Project::new();
    project.init();
    let (exit, report) = project.check(&[]);
    assert_eq!(exit, 3);
    let cve = &report["native_results"]["java_cve"]["probes"][0]["observation"];
    assert_eq!(cve["native_status"], "incomplete");
    assert_eq!(cve["reason"], "prerequisites_missing");
    assert!(cve["advisories"].as_array().unwrap().is_empty());
    assert_eq!(
        report["native_results"]["java_cve"]["backlog_status"],
        "synced_partial"
    );
    let facts: Vec<_> = fs::read_dir(project.0.join("codeguard/findings"))
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path().join("finding.json");
            serde_json::from_slice::<Value>(&fs::read(path).unwrap()).unwrap()
        })
        .filter(|fact| fact["checker_id"] == "java.maven.dependency_check")
        .collect();
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0]["kind"], "blocker");
    assert_eq!(facts[0]["reason_code"], "prerequisites_missing");

    let run_id = report["native_results"]["java_cve"]["run_id"]
        .as_str()
        .unwrap();
    let local_report = project.0.join(format!("codeguard/reports/{run_id}.json"));
    let bytes = fs::read_to_string(&local_report).unwrap();
    fs::write(&local_report, bytes.replace("\"unverified\"", "\"fresh\"")).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["work", "sync", project.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let sync: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(sync["failed_reports"].as_u64().unwrap() >= 1);
}

#[test]
fn cve_blocker_verify_uses_original_checker_and_keeps_task_open() {
    let project = Project::new();
    project.init();
    let (_, first) = project.check(&[]);
    let id = first["native_results"]["java_cve"]["next"]["repair_brief"]["task_id"]
        .as_str()
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            project.0.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let verify: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        verify["native_scan"]["report_type"],
        "java_cve_project_probe"
    );
    assert_eq!(
        verify["native_scan"]["checker_id"],
        "java.maven.dependency_check"
    );
    assert_eq!(verify["observation"], "still_blocked");
    assert_eq!(verify["event_persisted"], true, "{verify}");
    let fact: Value = serde_json::from_slice(
        &fs::read(
            project
                .0
                .join(format!("codeguard/findings/{id}/finding.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", project.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    assert_eq!(
        next.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&next.stdout)
    );
    let next: Value = serde_json::from_slice(&next.stdout).unwrap();
    assert!(next["repair_brief"]["checker_id"].is_string());
}

#[test]
fn repeated_cve_blocker_scans_keep_local_evidence_without_tracked_event_noise() {
    let project = Project::new();
    project.init();
    let (_, first) = project.check(&[]);
    let id = first["native_results"]["java_cve"]["next"]["repair_brief"]["task_id"]
        .as_str()
        .unwrap();
    let events = project.0.join(format!("codeguard/findings/{id}/events"));
    assert_eq!(fs::read_dir(&events).unwrap().count(), 1);
    let (_, repeated) = project.check(&[]);
    let run_id = repeated["native_results"]["java_cve"]["run_id"]
        .as_str()
        .unwrap();
    assert_eq!(fs::read_dir(&events).unwrap().count(), 1);
    let observation = project
        .0
        .join(format!("codeguard/state/observations/{id}/{run_id}.json"));
    let observation: Value = serde_json::from_slice(&fs::read(observation).unwrap()).unwrap();
    assert_eq!(observation["record_type"], "local_blocker_observation");
    assert_eq!(observation["blocker_id"], id);
    assert_eq!(observation["report_sha256"].as_str().unwrap().len(), 64);
    let verify = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            project.0.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(verify.status.code(), Some(3));
    let verify: Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(verify["event_persisted"], true, "{verify}");
    assert_eq!(fs::read_dir(&events).unwrap().count(), 2);
    project.check(&[]);
    assert_eq!(fs::read_dir(&events).unwrap().count(), 3);
    project.check(&[]);
    assert_eq!(fs::read_dir(&events).unwrap().count(), 3);
}
