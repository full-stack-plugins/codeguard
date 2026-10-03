#![cfg(all(feature = "wasm-precheck", unix))]

use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

#[test]
fn malformed_go_package_scope_cannot_preempt_candidate_worker() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-go-scope-reject-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("go.mod"),
        "module example.com/scope-reject\n\ngo 1.23.4\n",
    )
    .unwrap();
    fs::write(root.join("main.go"), "package main\nfunc main() {}\n").unwrap();
    let tool = root.join("go-fake");
    fs::write(
        &tool,
        "#!/bin/sh\ncase \"$1\" in\nversion) echo 'go version go1.23.4 darwin/arm64' ;;\nvet) printf '# example.com/scope-reject\\n{}\\n' >&2 ;;\nlist) printf '%s\\n' '{\"Dir\":\"/outside\",\"ImportPath\":\"example.com/scope-reject\",\"Name\":\"main\",\"GoFiles\":[\"main.go\"]}' ;;\nesac\n",
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all"])
        .arg(&root)
        .args([
            "--go-tool",
            tool.to_str().unwrap(),
            "--format=json",
            "--timeout",
            "40s",
        ])
        .output()
        .unwrap();
    fs::remove_dir_all(&root).unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["native_results"]["go_lint"]["native_status"], "clean_observed_unverified",
        "{report}"
    );
    assert_eq!(report["syntax_candidates"]["native_preferred_count"], 0);
    assert!(
        report["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"] == "main.go" && item["language"] == "go")
    );
}

#[test]
#[ignore = "requires explicit existing Go 1.23.4 via CODEGUARD_GO_BIN"]
fn completed_go_vet_preempts_only_a_selected_source_file() {
    let go = std::env::var("CODEGUARD_GO_BIN").expect("provide an existing Go executable");
    let version = Command::new(&go).arg("version").output().unwrap();
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).starts_with("go version go1.23.4 "));
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "codeguard-go-native-preferred-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("go.mod"),
        "module example.com/native-preferred\n\ngo 1.23.4\n",
    )
    .unwrap();
    fs::write(
        root.join("main.go"),
        "package main\nimport \"fmt\"\nfunc main() { fmt.Printf(\"%d\", \"bad\") }\n",
    )
    .unwrap();
    fs::write(
        root.join("excluded.go"),
        "//go:build never\n\npackage main\nfunc broken( { }\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all"])
        .arg(&root)
        .args(["--go-tool", &go, "--format=json", "--timeout", "120s"])
        .output()
        .unwrap();
    fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        output.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        matches!(
            report["native_results"]["go_lint"]["native_status"].as_str(),
            Some("clean_observed_unverified" | "findings_observed_unverified")
        ),
        "{report}"
    );
    assert_eq!(
        report["native_results"]["go_lint"]["findings"][0]["rule_id"], "printf",
        "{report}"
    );
    assert_eq!(report["syntax_candidates"]["native_preferred_count"], 1);
    let observations = report["syntax_candidates"]["observations"]
        .as_array()
        .unwrap();
    assert!(!observations.iter().any(|item| item["path"] == "main.go"));
    assert!(observations.iter().any(|item| {
        item["path"] == "excluded.go"
            && item["language"] == "go"
            && item["status"] == "candidate_observed"
            && item["recovery_count"]
                .as_u64()
                .is_some_and(|count| count > 0)
    }));
    assert_eq!(report["delivery_decision"], "incomplete");
}
