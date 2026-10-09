//! Java task observations and authority boundaries. Native disappearance is
//! persisted, but authoritative close/reopen still requires a trusted service.
use serde_json::Value;
use std::{fs, process::Command};

fn run(args: &[&str]) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(args)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{out:?}");
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn java_check_exposes_unresolved_authority() {
    let root = std::env::temp_dir().join(format!("codeguard-java-feedback-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    fs::write(root.join("Sample.java"), "public class Sample {}\n").unwrap();
    let report = run(&["check", "java", root.to_str().unwrap()]);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["command_status"], "incomplete");
    assert_eq!(report["authority"], "local_unverified");
    let candidates = report["category_candidates"].as_array().unwrap();
    for expected in ["lint", "comments", "dependencies", "cve"] {
        assert!(
            candidates.iter().any(|c| c["category"] == expected),
            "{report}"
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires explicit CODEGUARD_TEST_JAVA_HOME pointing to JDK 21"]
fn native_bad_fixed_bad_keeps_same_task_open_without_approved_closure() {
    let home = std::env::var("CODEGUARD_TEST_JAVA_HOME").unwrap();
    let root =
        std::env::temp_dir().join(format!("codeguard-java-native-loop-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let file = root.join("Sample.java");
    let bad = "public class Sample { /** Creates a sample. */ public Sample() {} }\n";
    let fixed = "/** Demonstrates the sample API. */\npublic class Sample { /** Creates a sample. */ public Sample() {} }\n";
    fs::write(&file, bad).unwrap();
    let path = root.to_str().unwrap();
    run(&["init", path, "--apply"]);
    let scan = || {
        run(&[
            "comments",
            "java",
            file.to_str().unwrap(),
            "--workspace",
            path,
            "--java-home",
            &home,
        ])
    };
    let first = scan();
    assert_eq!(first["workbench"]["new_findings"], 1, "{first}");
    let brief = &first["workbench"]["next"]["repair_brief"];
    assert_eq!(brief["checker_id"], "java.jdk.javadoc");
    let id = brief["task_id"].as_str().unwrap();
    for (source, expected) in [
        (bad, "still_present"),
        (fixed, "candidate_absent_unverified_policy"),
        (bad, "still_present"),
    ] {
        fs::write(&file, source).unwrap();
        let verified = run(&["task", "verify", id, path, "--java-home", &home]);
        assert_eq!(verified["observation"], expected, "{verified}");
        assert_eq!(verified["event_persisted"], true, "{verified}");
        let fact: Value = serde_json::from_slice(
            &fs::read(root.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(fact["state"], "open");
        let again = scan();
        assert_eq!(again["workbench"]["new_findings"], 0, "{again}");
        assert_eq!(
            fs::read_dir(root.join(".codeguard/tasks")).unwrap().count(),
            1
        );
    }
    fs::remove_dir_all(root).unwrap();
}
