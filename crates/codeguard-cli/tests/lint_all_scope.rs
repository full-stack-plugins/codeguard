#![cfg(unix)]
use std::{fs, process::Command};
#[test]
fn mixed_project_lint_all_selects_lint_nodes_without_build_comments_or_cve() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-lint-all-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("app.py"), "import os\n").unwrap();
    fs::write(root.join("src/lib.rs"), "pub fn value() -> u8 { 1 }\n").unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='demo'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "all"])
        .arg(&root)
        .args(["--format=json", "--jobs", "1"])
        .env("PATH", &root)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{out:?}");
    let r: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["requested_categories"], serde_json::json!(["lint"]));
    let mut nodes: Vec<_> = r["execution_tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["id"].as_str().unwrap())
        .collect();
    nodes.sort();
    assert_eq!(nodes, ["python.lint", "rust.lint"]);
    assert!(
        r["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .all(|x| x["category"] == "lint")
    );
    for key in [
        "rust_build",
        "rust_comments",
        "rust_cve",
        "java_cve",
        "java_javadoc",
        "java_dependencies",
    ] {
        assert!(r["native_results"][key].is_null());
    }
    if let Ok(path) = std::env::var("CODEGUARD_LINT_ALL_REPORT") {
        fs::write(path, serde_json::to_vec_pretty(&r).unwrap()).unwrap();
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn lint_all_rejects_cve_options_before_reading_project() {
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "all",
            "/nonexistent-project",
            "--cargo-audit-tool",
            "/missing-audit",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
}

#[test]
fn unsupported_language_stays_visible_in_lint_scope() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-lint-all-sql-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("query.sql"), "SELECT 1;\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "all"])
        .arg(&root)
        .arg("--format=json")
        .env("PATH", &root)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let r: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["category_candidates"].as_array().unwrap().len(), 1);
    assert_eq!(r["category_candidates"][0]["language"], "sql");
    assert_eq!(r["category_candidates"][0]["category"], "lint");
    assert_eq!(r["category_candidates"][0]["status"], "not_integrated");
    assert_eq!(r["requested_categories"], serde_json::json!(["lint"]));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn selected_cargo_is_invoked_only_for_lint_and_not_independent_categories() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-lint-all-cargo-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='demo'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "pub fn value() -> u8 { 1 }\n").unwrap();
    fs::write(
        root.join("Cargo.lock"),
        "version = 4\n[[package]]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let marker = root.join("invocations.log");
    fs::write(&marker, "").unwrap();
    let tool = root.join("cargo");
    fs::write(&tool,format!("#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"$1\" in --version) printf 'cargo 1.94.0\\n' ;; *) printf '%s\\n' '{{\"reason\":\"build-finished\",\"success\":true}}' ;; esac\n",marker.display())).unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "all"])
        .arg(&root)
        .arg("--cargo-tool")
        .arg(&tool)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{out:?}");
    let invocations = fs::read_to_string(&marker).unwrap();
    assert!(
        invocations.lines().any(|line| line.starts_with("clippy ")),
        "{invocations}"
    );
    assert!(
        invocations
            .lines()
            .all(|line| line.starts_with("clippy ") || line == "--version"),
        "{invocations}"
    );
    fs::remove_dir_all(root).unwrap();
}
