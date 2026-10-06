#![cfg(unix)]
use codeguard_cli::{
    gradle_javadoc_task_recheck::{run, classify},
    gradle_javadoc_workbench::prepare,
    work_sync::{save_local_report, sync_local_workspace},
    next_command::read_task_brief,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering, AtomicBool},
    time::{Instant, Duration},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Workspace(PathBuf);
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> (Workspace, Value) {
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "cg-gradle-task-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(root.join("src/main/java")).unwrap();
    let inputs = json!(
        [
            ("build.gradle", "plugins { id 'java' }\n"),
            ("settings.gradle", "rootProject.name='sample'\n"),
            ("src/main/java/Sample.java", "public class Sample {}\n")
        ]
        .into_iter()
        .map(|(p, b)| {
            fs::write(root.join(p), b).unwrap();
            json!({"path":p,"sha256":format!("{:x}",Sha256::digest(b.as_bytes()))})
        })
        .collect::<Vec<_>>()
    );
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("init")
        .arg(&root)
        .arg("--apply")
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let old: Value = serde_json::from_str(include_str!(
        "../../../tests/acceptance/evidence/gradle-native-javadoc-reports-2026-10-06.json"
    ))
    .unwrap();
    let mut native = old[0]["report"].clone();
    native["source_snapshot_sha256"] = json!(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&inputs).unwrap())
    ));
    native["findings"] = json!([{"path":"src/main/java/Sample.java","rule_id":"JavadocMissingComment","line":1,"column":8}]);
    let report = prepare(&root, &inputs, &native).unwrap();
    save_local_report(&root, &report).unwrap();
    assert_eq!(sync_local_workspace(&root).unwrap().failed_reports, 0);
    let id = report["findings"][0]["finding_id"].as_str().unwrap();
    let brief = read_task_brief(&root, id).unwrap();
    (Workspace(root), brief)
}
#[test]
fn missing_tools_or_changed_original_bindings_never_become_absent_candidates() {
    let (dir, brief) = fixture();
    let report = run(
        &dir.0,
        &brief,
        None,
        None,
        Instant::now() + Duration::from_secs(10),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(report["reason"], "prerequisites_missing");
    assert_eq!(classify(&brief, &report), "incomplete");
    let mut bad = brief.clone();
    bad["evidence_ref"]["first_report_sha256"] = json!("0".repeat(64));
    assert!(
        run(
            &dir.0,
            &bad,
            None,
            None,
            Instant::now() + Duration::from_secs(10),
            &AtomicBool::new(false)
        )
        .is_err()
    );
}

#[test]
fn task_scope_and_rule_must_match_the_consumed_original_fact() {
    let (dir, brief) = fixture();
    for key in ["scope", "native_rule_id", "checker_id", "kind"] {
        let mut changed = brief.clone();
        changed[key] = json!("forged");
        assert!(
            run(
                &dir.0,
                &changed,
                None,
                None,
                Instant::now() + Duration::from_secs(10),
                &AtomicBool::new(false)
            )
            .is_err(),
            "{key}"
        );
    }
}

#[test]
fn incomplete_or_malformed_scan_cannot_claim_candidate_absence() {
    let (dir, brief) = fixture();
    let mut report = run(
        &dir.0,
        &brief,
        None,
        None,
        Instant::now() + Duration::from_secs(10),
        &AtomicBool::new(false),
    )
    .unwrap();
    for key in [
        "task_input_stable",
        "configuration_matches",
        "tool_identity_matches",
        "original_tool_identity_complete",
        "source_scope_matches",
    ] {
        report[key] = json!(true);
    }
    report["scan"] = json!({"native":{"native_status":"empty_output_unverified"}});
    assert_eq!(classify(&brief, &report), "incomplete");
}

#[test]
fn changed_build_configuration_requires_review_before_native_execution() {
    let (dir, brief) = fixture();
    fs::write(
        dir.0.join("build.gradle"),
        "plugins { id 'java' }\njavadoc { enabled = false }\n",
    )
    .unwrap();
    let report = run(
        &dir.0,
        &brief,
        None,
        None,
        Instant::now() + Duration::from_secs(10),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(report["reason"], "original_configuration_changed");
    assert!(report["scan"].is_null());
    assert_eq!(classify(&brief, &report), "rule_coverage_requires_review");
}

#[test]
#[ignore = "requires explicit existing Gradle8.10.2/JDK21; never installs"]
fn actual_original_gradle_task_recheck_preserves_findings_and_untrusted_absence() {
    use codeguard_cli::{gradle_javadoc_probe::observe, gradle_model_probe::Request};
    use std::collections::BTreeSet;
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "cg-gradle-native-task-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let dir = Workspace(root);
    fs::create_dir_all(dir.0.join("src/main/java")).unwrap();
    fs::write(dir.0.join("settings.gradle"), "rootProject.name='sample'\n").unwrap();
    fs::write(dir.0.join("build.gradle"),"plugins { id 'java' }\ntasks.named('javadoc') { options.addBooleanOption('Xdoclint:missing', true); options.addBooleanOption('quiet', true); options.locale = 'en_US'; options.encoding = 'UTF-8' }\n").unwrap();
    fs::write(
        dir.0.join("src/main/java/Sample.java"),
        "public class Sample {\n public Sample() {}\n}\n",
    )
    .unwrap();
    assert_eq!(
        Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("init")
            .arg(&dir.0)
            .arg("--apply")
            .output()
            .unwrap()
            .status
            .code(),
        Some(3)
    );
    let paths = BTreeSet::from([
        PathBuf::from("build.gradle"),
        PathBuf::from("settings.gradle"),
        PathBuf::from("src/main/java/Sample.java"),
    ]);
    let inputs=json!(paths.iter().map(|p|json!({"path":p,"sha256":format!("{:x}",Sha256::digest(fs::read(dir.0.join(p)).unwrap()))})).collect::<Vec<_>>());
    let bundle =
        PathBuf::from(std::env::var_os("CODEGUARD_TEST_GRADLE_BUNDLE").expect("existing Gradle"));
    let java = PathBuf::from(std::env::var_os("CODEGUARD_TEST_JAVA_HOME").expect("existing JDK"));
    let native = observe(
        &Request {
            project_root: dir.0.clone(),
            project_files: paths,
            gradle_bundle: bundle.clone(),
            java_home: java.clone(),
            deadline: Instant::now() + Duration::from_secs(90),
        },
        &AtomicBool::new(false),
    );
    assert_eq!(
        native["native_status"], "findings_observed_unverified",
        "{native}"
    );
    let original = prepare(&dir.0, &inputs, &native).unwrap();
    save_local_report(&dir.0, &original).unwrap();
    assert_eq!(sync_local_workspace(&dir.0).unwrap().failed_reports, 0);
    let id = original["findings"][0]["finding_id"].as_str().unwrap();
    let brief = read_task_brief(&dir.0, id).unwrap();
    let present = run(
        &dir.0,
        &brief,
        Some(&bundle),
        Some(&java),
        Instant::now() + Duration::from_secs(90),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(classify(&brief, &present), "still_present", "{present}");
    fs::write(dir.0.join("src/main/java/Sample.java"),"/** Provides an example. */\npublic class Sample {\n /** Creates the example. */\n public Sample() {}\n}\n").unwrap();
    assert!(!codeguard_cli::gradle_javadoc_task_recheck::inputs_current(
        &dir.0, &present
    ));
    let absent = run(
        &dir.0,
        &brief,
        Some(&bundle),
        Some(&java),
        Instant::now() + Duration::from_secs(90),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        classify(&brief, &absent),
        "candidate_absent_unverified_policy",
        "{absent}"
    );
    assert_eq!(absent["coverage_proven"], false);
    assert_eq!(absent["delivery_decision"], "not_evaluated");
    let cancelled = run(
        &dir.0,
        &brief,
        Some(&bundle),
        Some(&java),
        Instant::now() + Duration::from_secs(90),
        &AtomicBool::new(true),
    )
    .unwrap();
    assert_eq!(cancelled["scan"]["native"]["reason"], "request_cancelled");
    assert_eq!(classify(&brief, &cancelled), "incomplete");
    let fact: Value = serde_json::from_slice(
        &fs::read(dir.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_GRADLE_TASK_REPORTS") {
        fs::write(path,serde_json::to_vec_pretty(&json!({"original":original,"present":present,"absent":absent,"cancelled":cancelled})).unwrap()).unwrap();
    }
}

#[test]
fn missing_projected_findings_cannot_hide_valid_native_diagnostics() {
    let (dir, brief) = fixture();
    let original: Value = serde_json::from_slice(
        &fs::read(dir.0.join(format!(
            ".codeguard/reports/{}.json",
            brief["evidence_ref"]["first_run_id"].as_str().unwrap()
        )))
        .unwrap(),
    )
    .unwrap();
    let mut report = run(
        &dir.0,
        &brief,
        None,
        None,
        Instant::now() + Duration::from_secs(10),
        &AtomicBool::new(false),
    )
    .unwrap();
    for key in [
        "task_input_stable",
        "configuration_matches",
        "tool_identity_matches",
        "original_tool_identity_complete",
        "source_scope_matches",
    ] {
        report[key] = json!(true);
    }
    let mut scan = original;
    scan["run_id"] = report["run_id"].clone();
    scan["findings"] = json!([]);
    report["scan"] = scan;
    assert_ne!(
        classify(&brief, &report),
        "candidate_absent_unverified_policy"
    );
}
