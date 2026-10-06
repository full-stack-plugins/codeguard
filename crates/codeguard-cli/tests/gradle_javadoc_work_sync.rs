#![cfg(unix)]
use codeguard_cli::{
    gradle_javadoc_workbench::prepare,
    work_sync::{save_local_report, sync_local_workspace},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Workspace(PathBuf);
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> (Workspace, Value, Value) {
    let dir = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "cg-gradle-sync-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(dir.join("src/main/java")).unwrap();
    let files = [
        ("build.gradle", "plugins { id 'java' }\n"),
        ("settings.gradle", "rootProject.name='sample'\n"),
        ("src/main/java/Sample.java", "public class Sample {}\n"),
    ];
    let inputs = json!(
        files
            .iter()
            .map(|(p, b)| {
                fs::write(dir.join(p), b).unwrap();
                json!({"path":p,"sha256":format!("{:x}",Sha256::digest(b.as_bytes()))})
            })
            .collect::<Vec<_>>()
    );
    let reports: Value = serde_json::from_str(include_str!(
        "../../../tests/acceptance/evidence/gradle-native-javadoc-reports-2026-10-06.json"
    ))
    .unwrap();
    let mut native = reports[0]["report"].clone();
    native["source_snapshot_sha256"] = json!(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&inputs).unwrap())
    ));
    native["findings"] = json!([{"path":"src/main/java/Sample.java","rule_id":"JavadocMissingComment","line":1,"column":8}]);
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["init"])
        .arg(&dir)
        .arg("--apply")
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3), "{init:?}");
    (Workspace(dir), inputs, native)
}
fn read_next(root: &PathBuf) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("next")
        .arg(root)
        .arg("--format=json")
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    serde_json::from_slice(&out.stdout).unwrap()
}
#[test]
fn imported_gradle_tasks_are_stable_and_have_gradle_native_guidance() {
    let (dir, inputs, native) = fixture();
    let report = prepare(&dir.0, &inputs, &native).unwrap();
    save_local_report(&dir.0, &report).unwrap();
    let summary = sync_local_workspace(&dir.0).unwrap();
    assert_eq!(summary.failed_reports, 0);
    assert_eq!(summary.new_findings, 1);
    assert_eq!(summary.new_blockers, 1);
    let next = read_next(&dir.0);
    assert_eq!(
        next["repair_brief"]["checker_id"], "java.gradle.javadoc",
        "{next}"
    );
    assert_eq!(
        next["repair_brief"]["task_verify_status"],
        "local_observation_only"
    );
    let argv = next["repair_brief"]["recheck_argv"].as_array().unwrap();
    assert!(argv.contains(&json!("--gradle-bundle")));
    assert_eq!(argv[1], "task");
    assert_eq!(argv[2], "verify");
    assert_eq!(argv[3], next["repair_brief"]["task_id"]);
    assert!(!argv.contains(&json!("python")));
    let again = prepare(&dir.0, &inputs, &native).unwrap();
    save_local_report(&dir.0, &again).unwrap();
    let summary = sync_local_workspace(&dir.0).unwrap();
    assert_eq!(summary.failed_reports, 0);
    assert_eq!(summary.new_findings, 0);
    assert_eq!(summary.new_blockers, 0);
    for row in report["findings"].as_array().unwrap() {
        let id = row["finding_id"].as_str().unwrap();
        let text = fs::read_to_string(dir.0.join(format!(".codeguard/tasks/{id}.md"))).unwrap();
        for section in [
            "问题证据",
            "规则依据",
            "允许范围",
            "修复步骤",
            "复检命令",
            "历史尝试",
            "关闭条件",
        ] {
            assert!(text.contains(section), "{section}");
        }
    }
    let id = report["findings"][0]["finding_id"].as_str().unwrap();
    fs::remove_file(dir.0.join(format!(".codeguard/tasks/{id}.md"))).unwrap();
    let recovered = sync_local_workspace(&dir.0).unwrap();
    assert_eq!(recovered.restored_task_projections, 1);
    let text = fs::read_to_string(dir.0.join(format!(".codeguard/tasks/{id}.md"))).unwrap();
    assert!(text.contains("--gradle-bundle"));
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "java"])
        .arg(&dir.0)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let historical: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(historical["schema_version"], "0.66.0");
    assert!(historical["gradle_javadoc_tasks"].is_null());
    assert_eq!(
        historical["next"]["repair_brief"]["checker_id"],
        "java.gradle.javadoc"
    );
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_GRADLE_SYNC_REPORT") {
        fs::write(path,serde_json::to_vec_pretty(&json!({"evidence_kind":"constructed_work_sync_fixture","report":report,"next":next,"historical_check":historical})).unwrap()).unwrap();
    }
}
#[test]
fn stale_or_tampered_first_imports_are_rejected_without_source_tasks() {
    for mutation in ["source", "projection", "workspace", "native", "extra"] {
        let (dir, inputs, native) = fixture();
        let mut report = prepare(&dir.0, &inputs, &native).unwrap();
        match mutation {
            "source" => fs::write(
                dir.0.join("src/main/java/Sample.java"),
                "public class Changed {}\n",
            )
            .unwrap(),
            "projection" => report["findings"][0]["line"] = json!(10),
            "workspace" => report["workspace_id"] = json!("wrong"),
            "native" => report["native"]["coverage_proven"] = json!(true),
            _ => report["extra"] = json!(true),
        };
        save_local_report(&dir.0, &report).unwrap();
        let summary = sync_local_workspace(&dir.0).unwrap();
        assert_eq!(summary.failed_reports, 1, "{mutation}");
        assert_eq!(summary.new_findings, 0);
        assert_eq!(summary.new_blockers, 0);
    }
}

#[test]
#[ignore = "requires explicit existing Gradle8.10.2/JDK21; never installs"]
fn actual_public_check_persists_and_reuses_gradle_documentation_tasks() {
    let (dir, _, _) = fixture();
    fs::write(dir.0.join("build.gradle"),"plugins { id 'java' }\ntasks.named('javadoc') { options.addBooleanOption('Xdoclint:missing',true); options.addBooleanOption('quiet',true) }\n").unwrap();
    let gradle = std::env::var_os("CODEGUARD_TEST_GRADLE_BUNDLE").expect("existing Gradle");
    let java = std::env::var_os("CODEGUARD_TEST_JAVA_HOME").expect("existing JDK");
    let run = |selection: &str| {
        let mut c = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        c.args(["check", selection])
            .arg(&dir.0)
            .arg("--gradle-javadoc")
            .arg("--gradle-bundle")
            .arg(&gradle)
            .arg("--java-home")
            .arg(&java);
        for path in [
            "build.gradle",
            "settings.gradle",
            "src/main/java/Sample.java",
        ] {
            c.arg("--gradle-project-file").arg(path);
        }
        let out = c.arg("--format=json").output().unwrap();
        assert_eq!(out.status.code(), Some(3), "{out:?}");
        serde_json::from_slice::<Value>(&out.stdout).unwrap()
    };
    let first = run("java");
    assert_eq!(first["schema_version"], "0.66.0");
    assert_eq!(
        first["gradle_javadoc_tasks"]["status"], "synced_partial",
        "{first}"
    );
    assert_eq!(
        first["gradle_javadoc_tasks"]["new_findings"],
        first["native_results"]["java_gradle_javadoc"]["findings"]
            .as_array()
            .unwrap()
            .len()
    );
    assert_eq!(first["gradle_javadoc_tasks"]["new_findings"], 2);
    assert_eq!(
        first["next"]["repair_brief"]["checker_id"],
        "java.gradle.javadoc"
    );
    let id = read_next(&dir.0)["repair_brief"]["task_id"].clone();
    let second = run("all");
    assert_eq!(second["gradle_javadoc_tasks"]["new_findings"], 0);
    assert_eq!(
        second["next"]["repair_brief"]["checker_id"],
        "java.gradle.javadoc"
    );
    assert_eq!(second["gradle_javadoc_tasks"]["new_blockers"], 0);
    assert_eq!(read_next(&dir.0)["repair_brief"]["task_id"], id);
    fs::write(
        dir.0.join("src/main/java/Sample.java"),
        "/** Example. */\npublic class Sample { /** Creates the example. */ public Sample() {} }\n",
    )
    .unwrap();
    let repaired = run("java");
    assert_eq!(
        repaired["native_results"]["java_gradle_javadoc"]["native_status"],
        "empty_output_unverified"
    );
    assert_eq!(repaired["gradle_javadoc_tasks"]["new_findings"], 0);
    let fact: Value = serde_json::from_slice(
        &fs::read(dir.0.join(format!(
            ".codeguard/findings/{}/finding.json",
            id.as_str().unwrap()
        )))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    let stored_reports = fs::read_dir(dir.0.join(".codeguard/reports"))
        .unwrap()
        .map(|e| serde_json::from_slice::<Value>(&fs::read(e.unwrap().path()).unwrap()).unwrap())
        .filter(|r| r["report_type"] == "gradle_javadoc_workbench_observation")
        .collect::<Vec<_>>();
    assert_eq!(stored_reports.len(), 3);
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_GRADLE_PUBLIC_SYNC_REPORT") {
        fs::write(path,serde_json::to_vec_pretty(&json!({"evidence_kind":"actual_public_check_work_sync","first":first,"second":second,"repaired":repaired,"retained_fact":fact,"stored_reports":stored_reports,"next":read_next(&dir.0)})).unwrap()).unwrap();
    }
}

#[test]
fn preparation_identity_survives_failure_cancellation_and_restored_empty_output() {
    let (dir, inputs, mut native) = fixture();
    native["findings"] = json!([]);
    let mut id = Value::Null;
    let mut previews = Vec::new();
    for (index, (status, reason)) in [
        ("incomplete", "native_execution_incomplete"),
        ("incomplete", "request_cancelled"),
        (
            "empty_output_unverified",
            "selected_sources_and_complete_documentation_rules_unverified",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        native["native_status"] = json!(status);
        native["reason"] = json!(reason);
        let report = prepare(&dir.0, &inputs, &native).unwrap();
        save_local_report(&dir.0, &report).unwrap();
        let summary = sync_local_workspace(&dir.0).unwrap();
        assert_eq!(summary.failed_reports, 0, "{reason}");
        assert_eq!(summary.new_findings, 0);
        assert_eq!(summary.new_blockers, u64::from(index == 0));
        let next = read_next(&dir.0);
        let brief = &next["repair_brief"];
        if id.is_null() {
            id = brief["task_id"].clone();
        }
        assert_eq!(brief["task_id"], id);
        assert_eq!(brief["latest_diagnostic_reason"], reason);
        assert_eq!(
            brief["action_id"],
            if index == 2 {
                "review-project-policy"
            } else {
                "restore-checker-environment"
            }
        );
        previews.push(next);
    }
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_GRADLE_PREPARATION_REPORT") {
        fs::write(
            path,
            serde_json::to_vec_pretty(
                &json!({"evidence_kind":"constructed_preparation_transitions","previews":previews}),
            )
            .unwrap(),
        )
        .unwrap();
    }
    let run = previews.last().unwrap()["repair_brief"]["latest_preparation_ref"]["run_id"]
        .as_str()
        .unwrap();
    let path = dir.0.join(format!(
        ".codeguard/state/observations/{}/{run}.json",
        id.as_str().unwrap()
    ));
    let mut observation: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    observation["diagnostic_reason"] = json!("request_cancelled");
    fs::write(&path, serde_json::to_vec_pretty(&observation).unwrap()).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("next")
        .arg(&dir.0)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let rejected: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(rejected["reason"], "gradle_preparation_report_invalid");
}
