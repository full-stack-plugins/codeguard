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
    let (dir, brief, bundle, java, original) = native_fixture();
    let id = brief["task_id"].as_str().unwrap();
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

fn verify_cli(root: &std::path::Path, id: &str, extra: &[&str]) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify", id])
        .arg(root)
        .args(extra)
        .args(["--format", "json"])
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        serde_json::from_slice(&out.stdout).unwrap_or(Value::Null),
    )
}

#[test]
fn public_missing_prerequisites_persist_a_bound_failure_and_next_guidance() {
    let (dir, brief) = fixture();
    let id = brief["task_id"].as_str().unwrap();
    let (exit, report) = verify_cli(&dir.0, id, &[]);
    assert_eq!(exit, 3);
    assert_eq!(report["schema_version"], "0.29.0");
    assert_eq!(report["native_scan"]["reason"], "prerequisites_missing");
    assert_eq!(
        report["native_scan"]["scan"]["native"]["reason"],
        "prerequisites_missing"
    );
    let public_next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("next")
        .arg(&dir.0)
        .arg("--format=json")
        .output()
        .unwrap();
    let public_next: Value = serde_json::from_slice(&public_next.stdout).unwrap();
    assert_eq!(public_next["repair_brief"]["kind"], "blocker");
    assert_eq!(
        public_next["repair_brief"]["action_id"],
        "restore-checker-environment"
    );
    assert_eq!(report["observation"], "incomplete");
    assert_eq!(report["event_persisted"], true, "{report}");
    let next = read_task_brief(&dir.0, id).unwrap();
    assert_eq!(next["schema_version"], "0.23.0");
    assert_eq!(next["task_verify_status"], "local_observation_only");
    assert_eq!(next["verification_observation"], "incomplete");
    assert_eq!(next["recheck_argv"][1], "task");
    assert_eq!(next["recheck_argv"][3], id);
    let run = report["native_scan"]["run_id"].as_str().unwrap();
    assert!(
        dir.0
            .join(format!(".codeguard/state/consumed/{run}.json"))
            .is_file()
    );
    let fact: Value = serde_json::from_slice(
        &fs::read(dir.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_GRADLE_TASK_MISSING_REPORT") {
        fs::write(path,serde_json::to_vec_pretty(&json!({"evidence_kind":"controlled_fixture_missing_tools","report":report,"next":public_next,"source_brief":next})).unwrap()).unwrap();
    }
}

#[test]
fn public_gradle_configuration_change_is_review_not_repaired_or_missing_tool() {
    let (dir, brief) = fixture();
    let id = brief["task_id"].as_str().unwrap();
    fs::write(
        dir.0.join("build.gradle"),
        "plugins { id 'java' }\njavadoc { enabled = false }\n",
    )
    .unwrap();
    let (_, report) = verify_cli(
        &dir.0,
        id,
        &[
            "--gradle-bundle",
            "/unavailable/gradle",
            "--java-home",
            "/unavailable/jdk",
        ],
    );
    assert_eq!(report["observation"], "rule_coverage_requires_review");
    assert_eq!(report["event_persisted"], true, "{report}");
    assert!(report["native_scan"]["scan"].is_null());
    assert_eq!(
        read_task_brief(&dir.0, id).unwrap()["verification_observation"],
        "rule_coverage_requires_review"
    );
}

fn task_json(root: &std::path::Path, prefix: &[&str], args: &[&str]) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("task")
        .args(prefix)
        .arg(root)
        .args(args)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn public_failure_links_ready_attempt_and_preserves_owned_lease() {
    let (dir, brief) = fixture();
    let id = brief["task_id"].as_str().unwrap();
    let action = brief["action_id"].as_str().unwrap();
    let claim = task_json(&dir.0, &["claim", id], &["--owner", "gradle-agent"]);
    let token = claim["lease_token"].as_str().unwrap();
    let started = task_json(
        &dir.0,
        &["attempt", "start", id],
        &[
            "--owner",
            "gradle-agent",
            "--lease-token",
            token,
            "--action-id",
            action,
        ],
    );
    let attempt = started["attempt_id"].as_str().unwrap();
    task_json(
        &dir.0,
        &["attempt", "finish", id],
        &[
            "--owner",
            "gradle-agent",
            "--lease-token",
            token,
            "--attempt-id",
            attempt,
            "--outcome",
            "ready-to-verify",
            "--note-code",
            "source_edit",
        ],
    );
    let (_, report) = verify_cli(
        &dir.0,
        id,
        &["--owner", "gradle-agent", "--lease-token", token],
    );
    assert_eq!(report["event_persisted"], true, "{report}");
    let run = report["native_scan"]["run_id"].as_str().unwrap();
    let event: Value = serde_json::from_slice(
        &fs::read(
            dir.0
                .join(format!(".codeguard/findings/{id}/events/verify-{run}.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(event["attempt_id"], attempt);
    assert_eq!(event["observation"], "incomplete");
    let next = read_task_brief(&dir.0, id).unwrap();
    assert_eq!(next["history"]["attempt_count"], 1);
    assert_eq!(next["history"]["awaiting_verification"], false);
    assert!(next["history"]["no_progress_count"].as_u64().unwrap() >= 1);
    // 环境缺失后不允许继续同一源码修复动作，必须先恢复独立环境任务。
    let retry = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "attempt", "start", id])
        .arg(&dir.0)
        .args([
            "--owner",
            "gradle-agent",
            "--lease-token",
            token,
            "--action-id",
            action,
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(retry.status.code(), Some(3));
    let retry: Value = serde_json::from_slice(&retry.stdout).unwrap();
    assert_eq!(retry["reason"], "task_not_actionable");
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_GRADLE_TASK_ATTEMPT_REPORT") {
        fs::write(path,serde_json::to_vec_pretty(&json!({"evidence_kind":"controlled_fixture_failure_attempt","report":report,"event":event,"brief":next,"retry":retry})).unwrap()).unwrap();
    }
    // 借用租约的复检结束后仍由原owner持有，不将租约误释放给其它智能体。
    task_json(
        &dir.0,
        &["release", id],
        &["--owner", "gradle-agent", "--lease-token", token],
    );
}

#[test]
fn public_gradle_flags_reject_duplicates_relative_paths_and_foreign_context() {
    let (dir, brief) = fixture();
    let id = brief["task_id"].as_str().unwrap();
    for extra in [
        &["--gradle-bundle", "relative"][..],
        &["--gradle-bundle", "/a", "--gradle-bundle", "/b"][..],
        &["--gradle-bundle", "/a", "--ruff-tool", "/ruff"][..],
        &["--java-tool", "/java"][..],
    ] {
        assert_eq!(verify_cli(&dir.0, id, extra).0, 2, "{extra:?}");
    }
    assert_eq!(
        fs::read_dir(dir.0.join(".codeguard/reports"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn first_recheck_import_rejects_forged_origin_scope_and_configuration_claims() {
    for mutation in [
        "origin",
        "task_path",
        "scope_claim",
        "configuration_claim",
        "input_digest",
        "foreign_fact_id",
    ] {
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
        match mutation {
            "origin" => report["origin"]["first_report_sha256"] = json!("0".repeat(64)),
            "task_path" => report["task_path"] = json!("other.java"),
            "scope_claim" => report["source_scope_matches"] = json!(false),
            "configuration_claim" => report["configuration_matches"] = json!(false),
            "input_digest" => {
                report["task_input_stable"] = json!(true);
                report["input_bindings"][2]["sha256"] = json!("0".repeat(64));
            }
            _ => report["task_id"] = json!(format!("CG-{}", "0".repeat(32))),
        }
        save_local_report(&dir.0, &report).unwrap();
        assert_eq!(
            sync_local_workspace(&dir.0).unwrap().failed_reports,
            1,
            "{mutation}"
        );
        let run = report["run_id"].as_str().unwrap();
        assert!(
            !dir.0
                .join(format!(".codeguard/state/consumed/{run}.json"))
                .exists()
        );
    }
}

fn native_fixture() -> (Workspace, Value, PathBuf, PathBuf, Value) {
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

    (dir, brief, bundle, java, original)
}

#[test]
#[ignore = "requires explicit existing Gradle8.10.2/JDK21; never installs"]
fn actual_public_original_recheck_imports_wrapped_findings_and_invalidates_old_observations() {
    let (dir, brief, bundle, java, original) = native_fixture();
    let id = brief["task_id"].as_str().unwrap();
    let args = [
        "--gradle-bundle",
        bundle.to_str().unwrap(),
        "--java-home",
        java.to_str().unwrap(),
    ];
    let (_, present) = verify_cli(&dir.0, id, &args);
    assert_eq!(present["schema_version"], "0.29.0");
    assert_eq!(present["event_persisted"], true, "{present}");
    assert_eq!(present["observation"], "still_present");
    let present_next = read_task_brief(&dir.0, id).unwrap();
    assert_eq!(present_next["verification_observation"], "still_present");
    // 原类/构造函数已补注释，同文件另一方法产生同规则问题；不将新问题当旧问题仍存在。
    fs::write(dir.0.join("src/main/java/Sample.java"),"/** Provides an example. */\npublic class Sample {\n /** Creates the example. */\n public Sample() {}\n public void run() {}\n}\n").unwrap();
    let (_, review) = verify_cli(&dir.0, id, &args);
    assert_eq!(review["event_persisted"], true, "{review}");
    assert_eq!(
        review["observation"], "rule_coverage_requires_review",
        "{review}"
    );
    let new_id = review["native_scan"]["scan"]["findings"][0]["finding_id"]
        .as_str()
        .unwrap();
    assert_ne!(id, new_id);
    let new_brief = read_task_brief(&dir.0, new_id).unwrap();
    assert_eq!(
        new_brief["evidence_ref"]["first_run_id"],
        review["native_scan"]["run_id"]
    );
    assert_eq!(new_brief["recheck_argv"][3], new_id);
    let (_, new_present) = verify_cli(&dir.0, new_id, &args);
    assert_eq!(new_present["event_persisted"], true, "{new_present}");
    assert_eq!(new_present["observation"], "still_present");
    fs::write(dir.0.join("src/main/java/Sample.java"),"/** Provides an example. */\npublic class Sample {\n /** Creates the example. */\n public Sample() {}\n /** Runs the example. */\n public void run() {}\n}\n").unwrap();
    let (_, absent) = verify_cli(&dir.0, id, &args);
    assert_eq!(absent["event_persisted"], true, "{absent}");
    assert_eq!(absent["observation"], "candidate_absent_unverified_policy");
    let absent_next = read_task_brief(&dir.0, id).unwrap();
    assert_eq!(
        absent_next["verification_observation"],
        "candidate_absent_unverified_policy"
    );
    fs::write(
        dir.0.join("settings.gradle"),
        "rootProject.name='changed'\n",
    )
    .unwrap();
    let stale_configuration = read_task_brief(&dir.0, id).unwrap();
    assert_eq!(
        stale_configuration["verification_invalidated_reason"],
        "gradle_inputs_changed_or_unavailable"
    );
    assert!(
        stale_configuration
            .get("verification_observation")
            .is_none()
    );
    fs::write(dir.0.join("settings.gradle"), "rootProject.name='sample'\n").unwrap();
    fs::write(
        dir.0.join("src/main/java/Sample.java"),
        "public class Sample {}\n",
    )
    .unwrap();
    let stale_source = read_task_brief(&dir.0, id).unwrap();
    assert_eq!(
        stale_source["verification_invalidated_reason"],
        "gradle_inputs_changed_or_unavailable"
    );
    assert!(stale_source.get("verification_observation").is_none());
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "java"])
        .arg(&dir.0)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let historical: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(historical["schema_version"], "0.66.0");
    let fact: Value = serde_json::from_slice(
        &fs::read(dir.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    let stored = fs::read_dir(dir.0.join(".codeguard/reports"))
        .unwrap()
        .map(|e| serde_json::from_slice::<Value>(&fs::read(e.unwrap().path()).unwrap()).unwrap())
        .collect::<Vec<_>>();
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_GRADLE_PUBLIC_TASK_REPORTS") {
        fs::write(path,serde_json::to_vec_pretty(&json!({"original":original,"present":present,"review":review,"new_present":new_present,"absent":absent,"present_next":present_next,"absent_next":absent_next,"stale_configuration":stale_configuration,"stale_source":stale_source,"historical":historical,"fact":fact,"stored":stored})).unwrap()).unwrap();
    }
}

#[test]
fn mismatched_tool_identity_stops_execution_and_later_tool_changes_invalidate_the_observation() {
    let (dir, brief) = fixture();
    let id = brief["task_id"].as_str().unwrap();
    let tools = dir.0.join(".codeguard/cache/fixture-tools");
    let bundle = tools.join("gradle");
    let java = tools.join("jdk");
    fs::create_dir_all(bundle.join("bin")).unwrap();
    fs::create_dir_all(java.join("bin")).unwrap();
    fs::write(bundle.join("bin/gradle"), "fixture, must not execute").unwrap();
    fs::write(java.join("bin/java"), "fixture java").unwrap();
    fs::write(java.join("release"), "fixture release").unwrap();
    let (_, report) = verify_cli(
        &dir.0,
        id,
        &[
            "--gradle-bundle",
            bundle.to_str().unwrap(),
            "--java-home",
            java.to_str().unwrap(),
        ],
    );
    assert_eq!(
        report["native_scan"]["reason"],
        "original_tool_identity_mismatch"
    );
    assert!(report["native_scan"]["scan"].is_null());
    assert_eq!(report["event_persisted"], true, "{report}");
    assert_eq!(
        read_task_brief(&dir.0, id).unwrap()["verification_observation"],
        "incomplete"
    );
    fs::write(java.join("bin/java"), "changed fixture java").unwrap();
    let next = read_task_brief(&dir.0, id).unwrap();
    assert_eq!(
        next["verification_invalidated_reason"],
        "gradle_inputs_changed_or_unavailable"
    );
    assert!(next.get("verification_observation").is_none());
}
