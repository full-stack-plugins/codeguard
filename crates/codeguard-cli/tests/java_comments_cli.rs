#![cfg(unix)]
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
use std::os::unix::fs::PermissionsExt;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    home: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-comments-java-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let home = root.join("jdk");
        fs::create_dir_all(home.join("bin")).unwrap();
        fs::write(home.join("release"), "JAVA_VERSION=\"21.0.12\"\n").unwrap();
        fs::write(home.join("bin/java"), "fixture").unwrap();
        let tool = home.join("bin/javadoc");
        fs::write(&tool, "#!/bin/sh\nprintf invoked >> \"$JAVA_HOME/invoked\"\nmkdir -p docs\nprintf '<html></html>' > docs/index.html\nprintf '%s:1: warning: no comment\\npublic class Bad {}\\n       ^\\n1 warning\\n' \"$PWD/src/Bad.java\" >&2\n").unwrap();
        fs::set_permissions(tool, fs::Permissions::from_mode(0o700)).unwrap();
        Self { root, home }
    }
    fn run(&self, target: &str, extra: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "comments",
                "java",
                target,
                "--java-home",
                self.home.to_str().unwrap(),
                "--format=json",
            ])
            .args(extra)
            .env_remove("CODEGUARD_TIMEOUT")
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
#[test]
fn file_entry_reuses_native_javadoc_without_granting_delivery_or_fake_tasks() {
    let f = Fixture::new();
    let file = f.root.join("Bad.java");
    fs::write(&file, "public class Bad {}\n").unwrap();
    let out = f.run(file.to_str().unwrap(), &[]);
    assert_eq!(out.status.code(), Some(3), "{:?}", out);
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["operation"], "comments");
    assert_eq!(r["language"], "java");
    assert_eq!(r["target_kind"], "file");
    assert_eq!(
        r["native_observation"]["findings"][0]["rule_id"],
        "JavadocMissingComment"
    );
    assert_eq!(r["delivery_decision"], "not_evaluated");
    assert_eq!(r["workbench_status"], "not_integrated");
    assert_eq!(fs::read(&file).unwrap(), b"public class Bad {}\n");
    assert!(!f.root.join(".codeguard").exists());
}
#[test]
fn project_entry_checks_configuration_and_only_selects_java_comments() {
    let f = Fixture::new();
    fs::create_dir_all(f.root.join("src/main/java")).unwrap();
    fs::write(
        f.root.join("src/main/java/Bad.java"),
        "public class Bad {}\n",
    )
    .unwrap();
    fs::write(f.root.join("pom.xml"),"<project><modelVersion>4.0.0</modelVersion><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId><configuration><doclint>all</doclint></configuration></plugin></plugins></build></project>").unwrap();
    fs::write(f.root.join("other.py"), "bad python\n").unwrap();
    let out = f.run(f.root.to_str().unwrap(), &[]);
    assert_eq!(out.status.code(), Some(3), "{:?}", out);
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["target_kind"], "project");
    assert_eq!(r["native_observation"]["source_file_count"], 1);
    assert_eq!(r["native_observation"]["observed_file_count"], 1);
    assert_eq!(
        r["native_observation"]["files"][0]["configuration"],
        "configured"
    );
    assert_eq!(
        r["native_observation"]["files"][0]["observation"]["findings"][0]["rule_id"],
        "JavadocMissingComment"
    );
    assert_eq!(r["coverage_proven"], false);
}
#[test]
fn missing_project_configuration_never_launches_the_native_tool() {
    let f = Fixture::new();
    fs::create_dir_all(f.root.join("src/main/java")).unwrap();
    fs::write(
        f.root.join("src/main/java/Bad.java"),
        "public class Bad {}\n",
    )
    .unwrap();
    let out = f.run(f.root.to_str().unwrap(), &[]);
    assert_eq!(out.status.code(), Some(3));
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["native_observation"]["observed_file_count"], 0);
    assert!(!f.home.join("invoked").exists());
    assert_eq!(
        r["native_observation"]["files"][0]["reason"],
        "javadoc_configuration_not_confirmed"
    );
}
#[test]
fn invalid_or_duplicate_options_are_rejected_before_native_execution() {
    let f = Fixture::new();
    let file = f.root.join("Bad.java");
    fs::write(&file, "public class Bad {}\n").unwrap();
    for extra in [
        vec!["--timeout", "0ms"],
        vec!["--java-home", "/bad"],
        vec!["--format", "human"],
        vec!["--checker", "p3c"],
    ] {
        let out = f.run(file.to_str().unwrap(), &extra);
        assert_eq!(out.status.code(), Some(2), "{:?}", out);
        assert!(out.stdout.is_empty());
        assert!(!f.home.join("invoked").exists());
    }
}

#[test]
fn explicit_maven_context_never_falls_back_to_single_file() {
    let f = Fixture::new();
    fs::create_dir_all(f.root.join("src/main/java")).unwrap();
    fs::write(
        f.root.join("src/main/java/Bad.java"),
        "public class Bad {}\n",
    )
    .unwrap();
    fs::write(f.root.join("pom.xml"), "<project><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId><configuration><doclint>all</doclint></configuration></plugin></plugins></build></project>").unwrap();
    let out = f.run(
        f.root.to_str().unwrap(),
        &["--maven-tool", "/nonexistent/codeguard/mvn"],
    );
    assert_eq!(out.status.code(), Some(3));
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["native_observation"]["probe_mode"], "maven_multifile");
    assert!(!f.home.join("invoked").exists());
    assert_eq!(r["coverage_proven"], false);
}

#[test]
fn budget_and_missing_target_remain_explicit() {
    let f = Fixture::new();
    let out = f.run(
        f.root.join("missing.java").to_str().unwrap(),
        &["--timeout=123ms"],
    );
    assert_eq!(out.status.code(), Some(3));
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["execution_budget"]["timeout_ms"], 123);
    assert_eq!(r["execution_budget"]["source"], "cli");
    assert_eq!(r["target_kind"], "unavailable");
    assert_eq!(r["native_observation"], Value::Null);
    assert!(!f.home.join("invoked").exists());
}

#[test]
#[ignore = "需要显式 CODEGUARD_TEST_JAVA_HOME 指向已有 JDK21，不安装工具"]
fn actual_jdk_comments_change_from_missing_to_documented() {
    let f = Fixture::new();
    let home = std::env::var("CODEGUARD_TEST_JAVA_HOME").unwrap();
    let file = f.root.join("Bad.java");
    fs::write(&file, "public class Bad {}\n").unwrap();
    let run = || {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "comments",
                "java",
                file.to_str().unwrap(),
                "--java-home",
                &home,
                "--format=json",
            ])
            .env_remove("CODEGUARD_TIMEOUT")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3), "{:?}", out);
        serde_json::from_slice::<Value>(&out.stdout).unwrap()
    };
    let before = run();
    assert_eq!(
        before["native_observation"]["findings"][0]["rule_id"],
        "JavadocMissingComment"
    );
    fs::write(
        &file,
        "/** Demonstrates documented public API. */\npublic class Bad { /** Creates an instance. */ public Bad() {} }\n",
    )
    .unwrap();
    let after = run();
    assert_eq!(
        after["native_observation"]["findings"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        after["native_observation"]["local_status"],
        "clean_scope_unproven"
    );
    assert_ne!(
        before["native_observation"]["source_sha256"],
        after["native_observation"]["source_sha256"]
    );
    assert_eq!(
        before["native_observation"]["javadoc_tool_sha256"],
        after["native_observation"]["javadoc_tool_sha256"]
    );
    assert_eq!(after["workbench_status"], "not_integrated");
    assert!(!f.root.join(".codeguard").exists());
}

fn initialize(f: &Fixture) {
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["init", f.root.to_str().unwrap(), "--apply", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
}

#[test]
fn initialized_project_syncs_one_stable_javadoc_task_and_real_next() {
    let f = Fixture::new();
    fs::create_dir_all(f.root.join("src/main/java")).unwrap();
    fs::write(
        f.root.join("src/main/java/Bad.java"),
        "public class Bad {}\n",
    )
    .unwrap();
    fs::write(f.root.join("pom.xml"), "<project><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId><configuration><doclint>all</doclint></configuration></plugin></plugins></build></project>").unwrap();
    initialize(&f);
    let mut task = Value::Null;
    for iteration in 0..2 {
        let out = f.run(f.root.to_str().unwrap(), &[]);
        assert_eq!(out.status.code(), Some(3));
        let r: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(r["workbench"]["status"], "synced_partial", "{r}");
        assert_eq!(
            r["workbench"]["new_findings"],
            if iteration == 0 { 1 } else { 0 }
        );
        let brief = &r["workbench"]["next"]["repair_brief"];
        assert_eq!(brief["checker_id"], "java.jdk.javadoc");
        assert_eq!(brief["kind"], "finding");
        assert_eq!(brief["recheck_argv"][1], "task");
        if iteration == 0 {
            task = brief["task_id"].clone();
        } else {
            assert_eq!(task, brief["task_id"]);
        }
    }
    assert_eq!(
        fs::read_dir(f.root.join(".codeguard/tasks"))
            .unwrap()
            .count(),
        1
    );
    let fact: Value = serde_json::from_slice(
        &fs::read(f.root.join(format!(
            ".codeguard/findings/{}/finding.json",
            task.as_str().unwrap()
        )))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}

#[test]
fn initialized_project_missing_configuration_creates_only_environment_task() {
    let f = Fixture::new();
    fs::create_dir_all(f.root.join("src/main/java")).unwrap();
    fs::write(
        f.root.join("src/main/java/Bad.java"),
        "public class Bad {}\n",
    )
    .unwrap();
    initialize(&f);
    let out = f.run(f.root.to_str().unwrap(), &[]);
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["workbench"]["new_findings"], 0, "{r}");
    assert_eq!(r["workbench"]["new_blockers"], 1);
    assert_eq!(r["workbench"]["next"]["repair_brief"]["kind"], "blocker");
    assert!(!f.home.join("invoked").exists());
}

fn configured_project() -> Fixture {
    let f = Fixture::new();
    fs::create_dir_all(f.root.join("src/main/java")).unwrap();
    fs::write(
        f.root.join("src/main/java/Bad.java"),
        "public class Bad {}\n",
    )
    .unwrap();
    fs::write(f.root.join("pom.xml"), "<project><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId><configuration><doclint>all</doclint></configuration></plugin></plugins></build></project>").unwrap();
    initialize(&f);
    f
}

#[test]
fn failed_report_persistence_is_visible_then_recovers_without_fake_tasks() {
    let f = configured_project();
    let reports = f.root.join(".codeguard/reports");
    fs::remove_dir(&reports).unwrap();
    fs::write(&reports, "blocked").unwrap();
    let out = f.run(f.root.to_str().unwrap(), &[]);
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_ne!(r["workbench"]["status"], "synced_partial");
    assert_eq!(r["workbench"]["next"], Value::Null);
    assert_eq!(
        fs::read_dir(f.root.join(".codeguard/tasks"))
            .unwrap()
            .count(),
        0
    );
    fs::remove_file(&reports).unwrap();
    fs::create_dir(&reports).unwrap();
    let out = f.run(f.root.to_str().unwrap(), &[]);
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["workbench"]["new_findings"], 1);
}

#[test]
fn forged_fingerprint_report_is_rejected_without_extra_task() {
    let f = configured_project();
    f.run(f.root.to_str().unwrap(), &[]);
    let report = fs::read_dir(f.root.join(".codeguard/reports"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let mut data: Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
    data["run_id"] = serde_json::json!("javadoc-forged-999999999999999999999");
    data["sources"][0]["findings"][0]["finding_fingerprint"] = serde_json::json!("0".repeat(64));
    fs::write(
        f.root
            .join(".codeguard/reports/javadoc-forged-999999999999999999999.json"),
        serde_json::to_vec(&data).unwrap(),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["work", "sync", f.root.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["failed_reports"], 1, "{r}");
    assert_eq!(
        fs::read_dir(f.root.join(".codeguard/tasks"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
#[ignore = "需要显式已有JDK21，验证工作台中的诊断消失不能自行关闭"]
fn actual_jdk_project_clean_observation_keeps_previous_tasks_open() {
    let f = configured_project();
    let home = std::env::var("CODEGUARD_TEST_JAVA_HOME").unwrap();
    let run = || {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "comments",
                "java",
                f.root.to_str().unwrap(),
                "--java-home",
                &home,
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
        serde_json::from_slice::<Value>(&out.stdout).unwrap()
    };
    let first = run();
    assert_eq!(first["workbench"]["status"], "synced_partial", "{first}");
    assert!(first["workbench"]["new_findings"].as_u64().unwrap() > 0);
    fs::write(f.root.join("src/main/java/Bad.java"),"/** Documented class. */\npublic class Bad { /** Documented constructor. */ public Bad() {} }\n").unwrap();
    let second = run();
    assert_eq!(second["workbench"]["status"], "synced_partial", "{second}");
    assert_eq!(second["workbench"]["new_findings"], 0);
    assert_eq!(
        second["native_observation"]["files"][0]["observation"]["local_status"],
        "clean_scope_unproven"
    );
    for entry in fs::read_dir(f.root.join(".codeguard/findings")).unwrap() {
        let fact: Value =
            serde_json::from_slice(&fs::read(entry.unwrap().path().join("finding.json")).unwrap())
                .unwrap();
        assert_eq!(fact["state"], "open");
    }
    assert_eq!(
        second["workbench"]["task_verify_status"],
        "local_observation_only"
    );
}

#[test]
fn javadoc_task_verify_records_present_and_missing_tool_without_closing() {
    let f = configured_project();
    let r: Value = serde_json::from_slice(&f.run(f.root.to_str().unwrap(), &[]).stdout).unwrap();
    let id = r["workbench"]["next"]["repair_brief"]["task_id"]
        .as_str()
        .unwrap();
    for home in [Some(f.home.to_str().unwrap()), None] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        command.args([
            "task",
            "verify",
            id,
            f.root.to_str().unwrap(),
            "--format=json",
        ]);
        if let Some(home) = home {
            command.args(["--java-home", home]);
        }
        let out = command.output().unwrap();
        assert_eq!(out.status.code(), Some(3));
        let report: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(
            report["observation"],
            if home.is_some() {
                "still_present"
            } else {
                "incomplete"
            },
            "{report}"
        );
        assert_eq!(report["event_persisted"], true, "{report}");
        let fact: Value = serde_json::from_slice(
            &fs::read(
                f.root
                    .join(format!(".codeguard/findings/{id}/finding.json")),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(fact["state"], "open");
    }
}

#[test]
fn javadoc_verify_rejects_unrelated_tool_parameters_before_native_execution() {
    let f = configured_project();
    let r: Value = serde_json::from_slice(&f.run(f.root.to_str().unwrap(), &[]).stdout).unwrap();
    let id = r["workbench"]["next"]["repair_brief"]["task_id"]
        .as_str()
        .unwrap();
    fs::remove_file(f.home.join("invoked")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            f.root.to_str().unwrap(),
            "--java-home",
            f.home.to_str().unwrap(),
            "--maven-tool",
            "/bad/mvn",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(!f.home.join("invoked").exists());
}

#[test]
fn javadoc_task_verify_changed_sdk_cannot_claim_absence() {
    let f = configured_project();
    let r: Value = serde_json::from_slice(&f.run(f.root.to_str().unwrap(), &[]).stdout).unwrap();
    let id = r["workbench"]["next"]["repair_brief"]["task_id"]
        .as_str()
        .unwrap();
    fs::write(f.home.join("bin/java"), "different runtime identity").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            f.root.to_str().unwrap(),
            "--java-home",
            f.home.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["observation"], "incomplete", "{r}");
    assert_eq!(r["native_scan"]["tool_identity_matches"], false);
    assert_eq!(r["event_persisted"], true);
}

#[test]
fn javadoc_failed_attempts_bind_verification_and_stop_repetition() {
    let f = configured_project();
    let root = f.root.to_str().unwrap();
    let r: Value = serde_json::from_slice(&f.run(root, &[]).stdout).unwrap();
    let brief = &r["workbench"]["next"]["repair_brief"];
    let id = brief["task_id"].as_str().unwrap();
    let action = brief["action_id"].as_str().unwrap();
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .arg("--format=json")
            .output()
            .unwrap();
        (
            out.status.code().unwrap(),
            serde_json::from_slice::<Value>(&out.stdout).unwrap(),
        )
    };
    let (exit, claim) = run(&["task", "claim", id, root, "--owner", "javadoc-agent"]);
    assert_eq!(exit, 0);
    let token = claim["lease_token"].as_str().unwrap();
    for _ in 0..2 {
        let (exit, start) = run(&[
            "task",
            "attempt",
            "start",
            id,
            root,
            "--owner",
            "javadoc-agent",
            "--lease-token",
            token,
            "--action-id",
            action,
        ]);
        assert_eq!(exit, 0, "{start}");
        let attempt = start["attempt_id"].as_str().unwrap();
        let (exit, finish) = run(&[
            "task",
            "attempt",
            "finish",
            id,
            root,
            "--owner",
            "javadoc-agent",
            "--lease-token",
            token,
            "--attempt-id",
            attempt,
            "--outcome",
            "ready-to-verify",
            "--note-code",
            "source_edit",
        ]);
        assert_eq!(exit, 0, "{finish}");
        let (_, verified) = run(&[
            "task",
            "verify",
            id,
            root,
            "--java-home",
            f.home.to_str().unwrap(),
            "--owner",
            "javadoc-agent",
            "--lease-token",
            token,
        ]);
        assert_eq!(verified["observation"], "still_present", "{verified}");
        assert_eq!(verified["event_persisted"], true, "{verified}");
    }
    let (_, next) = run(&["next", root]);
    assert_eq!(
        next["repair_brief"]["disposition"], "needs_decision",
        "{next}"
    );
}

#[test]
#[ignore = "需要已有JDK21，验证注释任务原工具复检和原配置变化分流"]
fn actual_jdk_task_verify_absence_and_configuration_change_keep_open() {
    let f = configured_project();
    let home = std::env::var("CODEGUARD_TEST_JAVA_HOME").unwrap();
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3), "{:?}", out);
        serde_json::from_slice::<Value>(&out.stdout).unwrap()
    };
    let root = f.root.to_str().unwrap();
    let first = run(&["comments", "java", root, "--java-home", &home]);
    assert_eq!(first["workbench"]["status"], "synced_partial");
    let id = fs::read_dir(f.root.join(".codeguard/findings"))
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            let fact: Value =
                serde_json::from_slice(&fs::read(e.path().join("finding.json")).unwrap()).unwrap();
            fact
        })
        .find(|f| f["native_rule_id"] == "JavadocMissingComment")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let verify = || run(&["task", "verify", &id, root, "--java-home", &home]);
    let present = verify();
    assert_eq!(present["observation"], "still_present", "{present}");
    assert_eq!(present["event_persisted"], true);
    fs::write(f.root.join("src/main/java/Bad.java"),"/** Documented class. */\npublic class Bad { /** Documented constructor. */ public Bad() {} }\n").unwrap();
    let absent = verify();
    assert_eq!(
        absent["observation"], "candidate_absent_unverified_policy",
        "{absent}"
    );
    assert_eq!(absent["event_persisted"], true);
    let pom = f.root.join("pom.xml");
    fs::write(
        &pom,
        format!(
            "<!-- Configuration changed -->\n{}",
            fs::read_to_string(&pom).unwrap()
        ),
    )
    .unwrap();
    let changed = verify();
    assert_eq!(
        changed["observation"], "rule_coverage_requires_review",
        "{changed}"
    );
    assert_eq!(changed["event_persisted"], true);
    let fact: Value = serde_json::from_slice(
        &fs::read(
            f.root
                .join(format!(".codeguard/findings/{id}/finding.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}

#[test]
fn explicit_file_workspace_syncs_stable_task_without_project_configuration() {
    let f = Fixture::new();
    let file = f.root.join("Bad.java");
    fs::write(&file, "public class Bad {}\n").unwrap();
    initialize(&f);
    let mut id = String::new();
    for i in 0..2 {
        let out = f.run(
            file.to_str().unwrap(),
            &["--workspace", f.root.to_str().unwrap()],
        );
        assert_eq!(out.status.code(), Some(3));
        let r: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(r["workbench"]["status"], "synced_partial", "{r}");
        assert_eq!(r["workbench"]["new_findings"], if i == 0 { 1 } else { 0 });
        let current = r["workbench"]["next"]["repair_brief"]["task_id"]
            .as_str()
            .unwrap();
        if i == 0 {
            id = current.into();
        } else {
            assert_eq!(current, id);
        }
    }
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            &id,
            f.root.to_str().unwrap(),
            "--java-home",
            f.home.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["observation"], "still_present", "{r}");
    assert_eq!(r["event_persisted"], true);
    assert!(!f.root.join("pom.xml").exists());
    assert_eq!(
        fs::read_dir(f.root.join(".codeguard/tasks"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn explicit_file_workspace_never_initializes_or_accepts_outside_source() {
    let f = Fixture::new();
    let file = f.root.join("Bad.java");
    fs::write(&file, "public class Bad {}\n").unwrap();
    let out = f.run(
        file.to_str().unwrap(),
        &["--workspace", f.root.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(3));
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["workbench"]["status"], "workspace_not_initialized");
    assert!(!f.root.join(".codeguard").exists());
    fs::remove_file(f.home.join("invoked")).unwrap();
    let outside = Fixture::new();
    let out = f.run(
        file.to_str().unwrap(),
        &["--workspace", outside.root.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(2));
    assert!(!f.home.join("invoked").exists());
    assert!(out.stdout.is_empty());
}

#[test]
fn explicit_file_missing_jdk_creates_preparation_task_without_source_finding() {
    let f = Fixture::new();
    let file = f.root.join("Bad.java");
    fs::write(&file, "public class Bad {}\n").unwrap();
    initialize(&f);
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "comments",
            "java",
            file.to_str().unwrap(),
            "--workspace",
            f.root.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["workbench"]["status"], "synced_partial", "{r}");
    assert_eq!(r["workbench"]["new_findings"], 0);
    assert_eq!(r["workbench"]["new_blockers"], 1);
    assert_eq!(
        r["workbench"]["next"]["repair_brief"]["observation_scope"],
        "explicit_file_probe"
    );
    assert!(!f.home.join("invoked").exists());
}

#[test]
#[ignore = "需要已有JDK21，验证显式文件模式不借用新增项目配置"]
fn actual_jdk_explicit_file_task_preserves_probe_mode_after_pom_added() {
    let f = Fixture::new();
    let file = f.root.join("Bad.java");
    fs::write(&file, "public class Bad {}\n").unwrap();
    initialize(&f);
    let home = std::env::var("CODEGUARD_TEST_JAVA_HOME").unwrap();
    let root = f.root.to_str().unwrap();
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
        serde_json::from_slice::<Value>(&out.stdout).unwrap()
    };
    let initial = run(&[
        "comments",
        "java",
        file.to_str().unwrap(),
        "--workspace",
        root,
        "--java-home",
        &home,
    ]);
    assert_eq!(initial["workbench"]["status"], "synced_partial");
    let brief = &initial["workbench"]["next"]["repair_brief"];
    let id = brief["task_id"].as_str().unwrap();
    assert_eq!(brief["observation_scope"], "explicit_file_probe");
    let still = run(&["task", "verify", id, root, "--java-home", &home]);
    assert_eq!(still["observation"], "still_present", "{still}");
    assert_eq!(still["event_persisted"], true);
    fs::write(f.root.join("pom.xml"), "<project>invalid configuration").unwrap();
    fs::write(&file,"/** Documented class. */\npublic class Bad { /** Documented constructor. */ public Bad() {} }\n").unwrap();
    let fixed = run(&["task", "verify", id, root, "--java-home", &home]);
    assert_eq!(
        fixed["observation"], "candidate_absent_unverified_policy",
        "{fixed}"
    );
    assert_eq!(
        fixed["native_scan"]["observation_scope"],
        "explicit_file_probe"
    );
    assert_eq!(fixed["event_persisted"], true);
    let fact: Value = serde_json::from_slice(
        &fs::read(
            f.root
                .join(format!(".codeguard/findings/{id}/finding.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}

#[test]
fn human_file_workspace_feedback_includes_real_task_and_recheck() {
    let f = Fixture::new();
    let file = f.root.join("Bad.java");
    fs::write(&file, "public class Bad {}\n").unwrap();
    initialize(&f);
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "comments",
            "java",
            file.to_str().unwrap(),
            "--workspace",
            f.root.to_str().unwrap(),
            "--java-home",
            f.home.to_str().unwrap(),
            "--format=human",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("工作台："));
    assert!(text.contains("synced_partial"));
    assert!(text.contains("CG-"));
    assert!(text.contains("explicit_file_probe"));
    assert!(text.contains("verify"));
}

#[test]
fn detailed_workbench_rejects_native_protocol_downgrade_on_first_import() {
    let f = configured_project();
    let tool = f.home.join("bin/javadoc");
    fs::write(&tool, "#!/bin/sh\nmkdir -p docs\nprintf '<html></html>' > docs/index.html\nprintf '%s:1: warning: empty comment\\npublic class Bad {}\\n       ^\\n1 warning\\n' \"$PWD/src/Bad.java\" >&2\n").unwrap();
    let out = f.run(f.root.to_str().unwrap(), &[]);
    let feedback: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(feedback["workbench"]["new_findings"], 1, "{feedback}");
    let path = fs::read_dir(f.root.join(".codeguard/reports"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let report: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for (name, outer, inner) in [
        ("outer", "0.2.0", "0.2.0"),
        ("inner", "0.3.0", "0.1.0"),
        ("both", "0.2.0", "0.1.0"),
    ] {
        let mut forged = report.clone();
        let run = format!("javadoc-downgrade-{name}");
        forged["run_id"] = serde_json::json!(run);
        forged["schema_version"] = serde_json::json!(outer);
        forged["sources"][0]["native"]["schema_version"] = serde_json::json!(inner);
        fs::write(
            f.root.join(format!(".codeguard/reports/{run}.json")),
            serde_json::to_vec(&forged).unwrap(),
        )
        .unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["work", "sync", f.root.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let sync: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(sync["failed_reports"], 3, "{sync}");
    assert_eq!(
        fs::read_dir(f.root.join(".codeguard/tasks"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
#[ignore = "需要显式已有JDK21，验证详细描述的公开检查、稳定任务及原工具复检"]
fn actual_jdk_detailed_descriptions_file_and_configured_project() {
    use serde_json::json;
    use sha2::{Digest, Sha256};
    let home = PathBuf::from(std::env::var_os("CODEGUARD_TEST_JAVA_HOME").unwrap());
    let binary = env!("CARGO_BIN_EXE_codeguard");
    let binary_sha = format!("{:x}", Sha256::digest(fs::read(binary).unwrap()));
    let documented = "/** 提供数值计算示例。 */\npublic class Sample {\n /** 创建计算器。 */ public Sample() {}\n /** 输出结果的初始值。 */ public int value;\n /** 返回输入数值。\n  * @param input 待返回的输入数值\n  * @return {@code input} 的原值\n  * @throws IllegalArgumentException 输入为负数时抛出\n  */\n public int run(int input) throws IllegalArgumentException { if (input < 0) { throw new IllegalArgumentException(); } return input; }\n /** {@inheritDoc} */\n @Override public String toString() { return \"sample\"; }\n}\n";
    let cases = [
        (
            "empty_declarations",
            "/** */\npublic class Sample {\n /** */ public Sample() {}\n /** */ public int value;\n /** */ public void run() {}\n}\n",
            vec!["JavadocEmptyComment"; 4],
        ),
        (
            "bare_tags",
            "/** Sample API. */\npublic class Sample { /** Creates sample. */ public Sample() {}\n/** Computes value.\n * @param value\n * @return\n * @throws IllegalArgumentException\n */\npublic int run(int value) throws IllegalArgumentException { return value; } }\n",
            vec![
                "JavadocEmptyParamDescription",
                "JavadocEmptyReturnDescription",
                "JavadocEmptyThrowsDescription",
            ],
        ),
        (
            "tags_without_purpose",
            "/** Sample API. */\npublic class Sample { /** Creates sample. */ public Sample() {}\n/** @param value the input\n * @return the value\n */\npublic int run(int value) { return value; } }\n",
            vec!["JavadocMissingMainDescription"],
        ),
        ("documented_inheritance", documented, vec![]),
    ];
    let mut evidence = Vec::new();
    for (name, source, expected) in cases {
        for mode in ["explicit_file_probe", "configured_project_probe"] {
            let mut f = Fixture::new();
            f.home = home.clone();
            let relative = if mode == "explicit_file_probe" {
                "Sample.java"
            } else {
                "src/main/java/Sample.java"
            };
            let file = f.root.join(relative);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(&file, source).unwrap();
            if mode == "configured_project_probe" {
                fs::write(f.root.join("pom.xml"), "<project><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId><configuration><doclint>all</doclint></configuration></plugin></plugins></build></project>").unwrap();
            }
            initialize(&f);
            let run = |args: &[&str]| {
                let out = Command::new(binary)
                    .args(args)
                    .arg("--format=json")
                    .output()
                    .unwrap();
                assert!(
                    out.status.code() == Some(3)
                        || (args[0] == "next" && out.status.code() == Some(0)),
                    "{}",
                    String::from_utf8_lossy(&out.stderr)
                );
                serde_json::from_slice::<Value>(&out.stdout).unwrap()
            };
            let root = f.root.to_str().unwrap();
            let java = home.to_str().unwrap();
            let target = if mode == "explicit_file_probe" {
                file.to_str().unwrap()
            } else {
                root
            };
            let feedback = run(&[
                "comments",
                "java",
                target,
                "--workspace",
                root,
                "--java-home",
                java,
            ]);
            assert_eq!(feedback["schema_version"], "0.8.0");
            assert_eq!(
                feedback["workbench"]["status"], "synced_partial",
                "{feedback}"
            );
            assert_eq!(
                feedback["workbench"]["new_findings"],
                expected.len(),
                "{feedback}"
            );
            let observation = if mode == "explicit_file_probe" {
                &feedback["native_observation"]
            } else {
                &feedback["native_observation"]["files"][0]["observation"]
            };
            let mut rules: Vec<_> = observation["findings"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["rule_id"].as_str().unwrap())
                .collect();
            let mut wanted = expected.clone();
            rules.sort();
            wanted.sort();
            assert_eq!(rules, wanted, "{feedback}");
            assert_eq!(
                observation["local_status"],
                if expected.is_empty() {
                    "clean_scope_unproven"
                } else {
                    "findings_observed_untrusted"
                }
            );
            let first_path = fs::read_dir(f.root.join(".codeguard/reports"))
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            let first: Value = serde_json::from_slice(&fs::read(&first_path).unwrap()).unwrap();
            let mut rechecks = Vec::new();
            for finding in first["sources"][0]["findings"].as_array().unwrap() {
                let id = finding["finding_id"].as_str().unwrap();
                let present = run(&["task", "verify", id, root, "--java-home", java]);
                assert_eq!(present["schema_version"], "0.31.0");
                assert_eq!(present["observation"], "still_present", "{present}");
                assert_eq!(present["event_persisted"], true);
                rechecks.push(present);
            }
            let next = run(&["next", root]);
            if !expected.is_empty() {
                assert_eq!(next["repair_brief"]["schema_version"], "0.4.0");
                assert!(
                    next["repair_brief"]["step"]
                        .as_str()
                        .unwrap()
                        .contains("详细说明")
                );
            }
            fs::write(&file, documented).unwrap();
            let mut repaired = Vec::new();
            for finding in first["sources"][0]["findings"].as_array().unwrap() {
                let id = finding["finding_id"].as_str().unwrap();
                let absent = run(&["task", "verify", id, root, "--java-home", java]);
                assert_eq!(
                    absent["observation"], "candidate_absent_unverified_policy",
                    "{absent}"
                );
                assert_eq!(absent["event_persisted"], true);
                let fact: Value = serde_json::from_slice(
                    &fs::read(
                        f.root
                            .join(format!(".codeguard/findings/{id}/finding.json")),
                    )
                    .unwrap(),
                )
                .unwrap();
                assert_eq!(fact["state"], "open");
                repaired.push(absent);
            }
            let direct = if mode == "explicit_file_probe" {
                fs::write(&file, source).unwrap();
                let scan = run(&[
                    "lint",
                    "java",
                    file.to_str().unwrap(),
                    "--checker",
                    "javadoc",
                    "--java-home",
                    java,
                ]);
                assert_eq!(scan["findings"].as_array().unwrap().len(), expected.len());
                scan
            } else {
                Value::Null
            };
            let aggregate = if mode == "configured_project_probe" {
                let scan = run(&["check", "java", root, "--java-home", java]);
                assert_eq!(scan["schema_version"], "0.68.0", "{scan}");
                scan
            } else {
                Value::Null
            };
            evidence.push(json!({"direct":direct,"aggregate":aggregate,"case":name,"mode":mode,"source":source,"source_sha256":format!("{:x}",Sha256::digest(source.as_bytes())),"feedback":feedback,"workbench":first,"next":next,"present":rechecks,"repaired":repaired}));
        }
    }
    assert_eq!(
        format!("{:x}", Sha256::digest(fs::read(binary).unwrap())),
        binary_sha
    );
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_JDK_DETAILED_EVIDENCE") {
        fs::write(path,serde_json::to_vec_pretty(&json!({"evidence_kind":"actual_existing_jdk_public_detailed_descriptions","codeguard_binary_sha256":binary_sha,"reports":evidence})).unwrap()).unwrap();
    }
}
