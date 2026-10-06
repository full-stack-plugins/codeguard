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
