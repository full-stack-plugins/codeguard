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
    if mutation == "outside_scope" {
        fs::write(root.join("Extra.java"), "public class Extra {}\n").unwrap();
    }
    let edit =
        if mutation.is_empty() || matches!(mutation, "missing_prerequisites" | "outside_scope") {
            String::new()
        } else {
            format!(
                "printf 'changed\\n' > '{}'\n",
                root.join(mutation).display()
            )
        };
    let tool = root.join("mvn");
    fs::write(&tool,format!("#!/bin/sh\n{edit}mkdir -p target/reports/apidocs\nprintf '<html></html>' > target/reports/apidocs/index.html\nif /usr/bin/grep -Fq 'Documented' src/main/java/Bad.java; then printf '[INFO] --- javadoc:3.12.0:javadoc (default-cli) @ demo ---\\n[INFO] BUILD SUCCESS\\n'; exit 0; fi\nprintf '[INFO] --- javadoc:3.12.0:javadoc (default-cli) @ demo ---\\n[WARNING] Javadoc Warnings\\n[WARNING] %s:1: warning: no comment\\n[WARNING] public class Bad {{}}\\n[WARNING] ^\\n[WARNING] 1 warning\\n[INFO] BUILD SUCCESS\\n' \"$PWD/src/main/java/Bad.java\"\n")).unwrap();
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
        if mutation.is_empty() {
            let id = report["workbench"]["next"]["repair_brief"]["task_id"]
                .as_str()
                .unwrap();
            let verify = |maven: bool| {
                let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
                command
                    .args(["task", "verify", id])
                    .arg(&root)
                    .arg("--java-home")
                    .arg(root.join("jdk"));
                if maven {
                    command
                        .arg("--maven-tool")
                        .arg(&tool)
                        .arg("--maven-repo")
                        .arg(root.join("repo"))
                        .arg("--repo-sha256")
                        .arg(hash_bundle_tree(&root.join("repo")).unwrap());
                }
                let out = command.arg("--format=json").output().unwrap();
                assert_eq!(out.status.code(), Some(3), "{out:?}");
                serde_json::from_slice::<Value>(&out.stdout).unwrap()
            };
            let still = verify(true);
            assert_eq!(still["observation"], "still_present", "{still}");
            assert_eq!(still["event_persisted"], true);
            let missing = verify(false);
            assert_eq!(missing["observation"], "incomplete");
            assert_eq!(missing["event_persisted"], true);
            fs::write(root.join("jdk/bin/java"), "changed-runtime").unwrap();
            let tool_change = verify(true);
            assert_eq!(tool_change["observation"], "incomplete");
            assert_eq!(
                tool_change["native_scan"]["reason"],
                "original_tool_identity_mismatch"
            );
            assert_eq!(tool_change["event_persisted"], true);
            fs::write(root.join("jdk/bin/java"), "fixture").unwrap();
            fs::write(
                root.join("src/main/java/Bad.java"),
                "/** Documented. */ public class Bad {}\n",
            )
            .unwrap();
            let absent = verify(true);
            assert_eq!(
                absent["observation"], "candidate_absent_unverified_policy",
                "{absent}"
            );
            assert_eq!(absent["event_persisted"], true);
            fs::write(
                root.join("src/main/java/Second.java"),
                "public class Second {}\n",
            )
            .unwrap();
            let scope_change = verify(true);
            assert_eq!(
                scope_change["observation"], "rule_coverage_requires_review",
                "{scope_change}"
            );
            assert_eq!(scope_change["event_persisted"], true);
            fs::remove_file(root.join("src/main/java/Second.java")).unwrap();
            let pom = fs::read_to_string(root.join("pom.xml")).unwrap();
            fs::write(
                root.join("pom.xml"),
                pom.replace("<version>1</version>", "<version>2</version>"),
            )
            .unwrap();
            let config_change = verify(true);
            assert_eq!(
                config_change["observation"], "rule_coverage_requires_review",
                "{config_change}"
            );
            assert_eq!(config_change["event_persisted"], true);
            fs::write(root.join("pom.xml"), pom).unwrap();
            let queried = Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args(["next"])
                .arg(&root)
                .arg("--format=json")
                .output()
                .unwrap();
            let queried: Value = serde_json::from_slice(&queried.stdout).unwrap();
            assert_eq!(
                queried["repair_brief"]["checker_id"], "java.maven.javadoc",
                "{queried}"
            );
            let aggregate = Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args(["check", "java"])
                .arg(&root)
                .arg("--format=json")
                .output()
                .unwrap();
            assert_eq!(aggregate.status.code(), Some(3), "{aggregate:?}");
            let aggregate: Value = serde_json::from_slice(&aggregate.stdout).unwrap();
            assert_eq!(aggregate["schema_version"], "0.38.0", "{aggregate}");
            assert_eq!(
                aggregate["next"]["repair_brief"]["checker_id"],
                "java.maven.p3c"
            );
            if let Ok(dir) = std::env::var("CODEGUARD_MAVEN_WORKBENCH_REPORT_DIR") {
                let dir = std::path::Path::new(&dir);
                fs::create_dir_all(dir).unwrap();
                fs::write(
                    dir.join("aggregate.json"),
                    serde_json::to_vec_pretty(&aggregate).unwrap(),
                )
                .unwrap();
                fs::write(
                    dir.join("next.json"),
                    serde_json::to_vec_pretty(&queried).unwrap(),
                )
                .unwrap();
                for (name, r) in [
                    ("still", still),
                    ("missing", missing),
                    ("tool-change", tool_change),
                    ("absent", absent),
                    ("scope-change", scope_change),
                    ("config-change", config_change),
                ] {
                    fs::write(
                        dir.join(format!("verify-{name}.json")),
                        serde_json::to_vec_pretty(&r).unwrap(),
                    )
                    .unwrap();
                }
            }
            fs::write(root.join("src/main/java/Bad.java"), "public class Bad {}\n").unwrap();
        }
        if mutation == "outside_scope" {
            let id = report["workbench"]["next"]["repair_brief"]["task_id"]
                .as_str()
                .unwrap();
            let verified = Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args(["task", "verify", id])
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
            let v: Value = serde_json::from_slice(&verified.stdout).unwrap();
            assert_eq!(v["observation"], "incomplete", "{v}");
            assert_eq!(v["event_persisted"], true);
        }
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
        if mutation.is_empty() {
            fs::remove_file(&forged).unwrap();
            fs::write(root.join("src/main/java/Bad.java"), "public class Bad {}\n").unwrap();
            let invoke = |args: &[&str]| {
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
            let root_text = root.to_str().unwrap();
            let action = report["workbench"]["next"]["repair_brief"]["action_id"]
                .as_str()
                .unwrap();
            let (exit, claim) = invoke(&["task", "claim", id, root_text, "--owner", "maven-agent"]);
            assert_eq!(exit, 0, "{claim}");
            let token = claim["lease_token"].as_str().unwrap();
            for _ in 0..2 {
                let (exit, started) = invoke(&[
                    "task",
                    "attempt",
                    "start",
                    id,
                    root_text,
                    "--owner",
                    "maven-agent",
                    "--lease-token",
                    token,
                    "--action-id",
                    action,
                ]);
                assert_eq!(exit, 0, "{started}");
                let attempt = started["attempt_id"].as_str().unwrap();
                let (exit, finished) = invoke(&[
                    "task",
                    "attempt",
                    "finish",
                    id,
                    root_text,
                    "--owner",
                    "maven-agent",
                    "--lease-token",
                    token,
                    "--attempt-id",
                    attempt,
                    "--outcome",
                    "ready-to-verify",
                    "--note-code",
                    "source_edit",
                ]);
                assert_eq!(exit, 0, "{finished}");
                let (_, v) = invoke(&[
                    "task",
                    "verify",
                    id,
                    root_text,
                    "--owner",
                    "maven-agent",
                    "--lease-token",
                    token,
                    "--maven-tool",
                    tool.to_str().unwrap(),
                    "--java-home",
                    root.join("jdk").to_str().unwrap(),
                    "--maven-repo",
                    root.join("repo").to_str().unwrap(),
                    "--repo-sha256",
                    &hash_bundle_tree(&root.join("repo")).unwrap(),
                ]);
                assert_eq!(v["observation"], "still_present", "{v}");
                assert_eq!(v["event_persisted"], true);
            }
            let (_, q) = invoke(&["next", root_text]);
            assert_eq!(q["repair_brief"]["disposition"], "needs_decision", "{q}");
        }
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
    assert_eq!(brief["recheck_argv"][1], "task");
    assert_eq!(brief["task_verify_status"], "local_observation_only");
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

#[test]
fn maven_scope_blocker_survives_a_complete_main_source_probe() {
    let r = run("outside_scope", true);
    assert_eq!(r["workbench"]["new_blockers"], 1);
    assert_eq!(r["workbench"]["new_findings"], 1);
}
