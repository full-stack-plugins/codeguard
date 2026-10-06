#![cfg(unix)]
use codeguard_cli::tool_identity::hash_bundle_tree;
use serde_json::Value;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);

fn observe(mutation: &str) -> Value {
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "cg-maven-javadoc-input-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(root.join("src/main/java")).unwrap();
    fs::create_dir_all(root.join("jdk/bin")).unwrap();
    fs::create_dir(root.join("repo")).unwrap();
    fs::write(root.join("src/main/java/Bad.java"), "public class Bad {}\n").unwrap();
    fs::write(root.join("pom.xml"),"<project><modelVersion>4.0.0</modelVersion><groupId>demo</groupId><artifactId>demo</artifactId><version>1</version><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId><version>3.12.0</version><configuration><doclint>missing</doclint></configuration></plugin></plugins></build></project>").unwrap();
    fs::write(root.join("jdk/bin/java"), "fixture").unwrap();
    fs::write(root.join("jdk/release"), "JAVA_VERSION=\"21.0.12\"\n").unwrap();
    let edit = if mutation.is_empty() {
        String::new()
    } else {
        format!(
            "printf 'changed\\n' > '{}'\n",
            root.join(mutation).display()
        )
    };
    let tool = root.join("mvn");
    fs::write(&tool,format!("#!/bin/sh\n{edit}mkdir -p target/reports/apidocs\nprintf '<html></html>' > target/reports/apidocs/index.html\nprintf '[INFO] --- javadoc:3.12.0:javadoc (default-cli) @ demo ---\\n[WARNING] Javadoc Warnings\\n[WARNING] %s:1: warning: no comment\\n[WARNING] public class Bad {{}}\\n[WARNING] ^\\n[WARNING] 1 warning\\n[INFO] BUILD SUCCESS\\n' \"$PWD/src/main/java/Bad.java\"\n")).unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["comments", "java"])
        .arg(&root)
        .arg("--maven-tool")
        .arg(&tool)
        .arg("--java-home")
        .arg(root.join("jdk"))
        .arg("--maven-repo")
        .arg(root.join("repo"))
        .arg("--repo-sha256")
        .arg(hash_bundle_tree(&root.join("repo")).unwrap())
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{out:?}");
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    let native = report["native_observation"]["maven_multifile_probes"][0]["observation"].clone();
    assert!(native.is_object(), "{report}");
    fs::remove_dir_all(&root).unwrap();
    native
}

#[test]
fn unchanged_original_inputs_keep_native_diagnostics() {
    let r = observe("");
    assert_eq!(r["native_status"], "findings_observed_untrusted", "{r}");
    assert_eq!(r["findings"].as_array().unwrap().len(), 1);
}
#[test]
fn original_source_and_pom_changes_discard_frozen_copy_diagnostics() {
    for path in ["src/main/java/Bad.java", "pom.xml"] {
        let r = observe(path);
        assert_eq!(r["native_status"], "incomplete", "{path}: {r}");
        assert_eq!(r["reason"], "native_inputs_changed_during_scan");
        assert!(r["findings"].as_array().unwrap().is_empty());
        assert_eq!(r["observed_source_count"], 0);
    }
}
#[test]
fn changed_jdk_release_disallows_old_runtime_diagnostics() {
    let r = observe("jdk/release");
    assert_eq!(r["native_status"], "incomplete", "{r}");
    assert_eq!(r["reason"], "native_identity_changed_during_scan");
    assert!(r["findings"].as_array().unwrap().is_empty());
}
