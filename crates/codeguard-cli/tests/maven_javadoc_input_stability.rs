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

fn run(mutation: &str, initialized: bool) -> Value {
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
    let edit = if mutation.is_empty() || mutation == "missing_prerequisites" {
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
    if initialized {
        let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init"])
            .arg(&root)
            .args(["--apply", "--format=json"])
            .output()
            .unwrap();
        assert_eq!(init.status.code(), Some(3));
    }
    let execute = || {
        Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["comments", "java"])
            .arg(&root)
            .arg("--maven-tool")
            .arg(if mutation == "missing_prerequisites" {
                root.join("missing-maven")
            } else {
                tool.clone()
            })
            .arg("--java-home")
            .arg(root.join("jdk"))
            .arg("--maven-repo")
            .arg(root.join("repo"))
            .arg("--repo-sha256")
            .arg(hash_bundle_tree(&root.join("repo")).unwrap())
            .arg("--format=json")
            .output()
            .unwrap()
    };
    let out = execute();
    assert_eq!(out.status.code(), Some(3), "{out:?}");
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    let native = report["native_observation"]["maven_multifile_probes"][0]["observation"].clone();
    assert!(native.is_object(), "{report}");
    if initialized {
        let repeated: Value = serde_json::from_slice(&execute().stdout).unwrap();
        assert_eq!(repeated["workbench"]["new_findings"], 0, "{repeated}");
        assert_eq!(repeated["workbench"]["new_blockers"], 0, "{repeated}");
        assert_eq!(
            repeated["workbench"]["next"]["repair_brief"]["task_id"],
            report["workbench"]["next"]["repair_brief"]["task_id"]
        );
        let evidence = &report["workbench"]["next"]["repair_brief"]["evidence_ref"];
        let path = root.join(format!(
            ".codeguard/reports/{}.json",
            evidence["first_run_id"].as_str().unwrap()
        ));
        let saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        if let Ok(dir) = std::env::var("CODEGUARD_MAVEN_WORKBENCH_REPORT_DIR") {
            let dir = std::path::Path::new(&dir);
            fs::create_dir_all(dir).unwrap();
            let name = if mutation.is_empty() {
                "complete"
            } else {
                "preparation"
            };
            fs::write(
                dir.join(format!("{name}-wrapper.json")),
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
            fs::write(
                dir.join(format!("{name}-saved.json")),
                serde_json::to_vec_pretty(&saved).unwrap(),
            )
            .unwrap();
        }
        let mut fake = saved.clone();
        fake["run_id"] = serde_json::json!("javadoc-maven-forged");
        fake["coverage_proven"] = serde_json::json!(true);
        let forged = root.join(".codeguard/reports/javadoc-maven-forged.json");
        fs::write(&forged, serde_json::to_vec(&fake).unwrap()).unwrap();
        let sync = || {
            let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args(["work", "sync"])
                .arg(&root)
                .arg("--format=json")
                .output()
                .unwrap();
            serde_json::from_slice::<Value>(&out.stdout).unwrap()
        };
        assert_eq!(sync()["failed_reports"], 1);
        fs::remove_file(&forged).unwrap();
        if mutation.is_empty() {
            fake["coverage_proven"] = serde_json::json!(false);
            fake["findings"][0]["finding_fingerprint"] = serde_json::json!("0".repeat(64));
            fs::write(&forged, serde_json::to_vec(&fake).unwrap()).unwrap();
            assert_eq!(sync()["failed_reports"], 1);
            fs::remove_file(&forged).unwrap();
            fake = saved;
            fake["run_id"] = serde_json::json!("javadoc-maven-forged");
            fs::write(
                root.join("src/main/java/Bad.java"),
                "/** New. */ public class Bad {}\n",
            )
            .unwrap();
            fs::write(&forged, serde_json::to_vec(&fake).unwrap()).unwrap();
            assert_eq!(sync()["failed_reports"], 1);
        }
        let id = report["workbench"]["next"]["repair_brief"]["task_id"]
            .as_str()
            .unwrap();
        let fact: Value = serde_json::from_slice(
            &fs::read(root.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(fact["state"], "open");
    }
    fs::remove_dir_all(&root).unwrap();
    if initialized { report } else { native }
}

fn observe(mutation: &str) -> Value {
    run(mutation, false)
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

#[test]
fn maven_diagnostics_join_workspace_with_native_recheck_guidance() {
    let r = run("", true);
    assert_eq!(r["workbench_status"], "synced_partial", "{r}");
    assert_eq!(r["workbench"]["new_findings"], 1);
    let brief = &r["workbench"]["next"]["repair_brief"];
    assert_eq!(brief["checker_id"], "java.maven.javadoc", "{r}");
    assert_eq!(brief["recheck_argv"][1], "comments");
    assert_eq!(brief["task_verify_status"], "not_integrated");
    assert_eq!(r["delivery_decision"], "not_evaluated");
}

#[test]
fn missing_maven_prerequisites_generate_preparation_instead_of_source_findings() {
    let r = run("missing_prerequisites", true);
    assert_eq!(r["workbench_status"], "synced_partial", "{r}");
    assert_eq!(r["workbench"]["new_findings"], 0);
    assert_eq!(r["workbench"]["new_blockers"], 1);
    assert_eq!(r["workbench"]["next"]["repair_brief"]["kind"], "blocker");
}
