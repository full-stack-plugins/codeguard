#![cfg(all(feature = "wasm-precheck", unix))]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

#[test]
fn native_zig_diagnostic_takes_priority_over_wasm_candidate() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-zig-native-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("broken.zig");
    fs::write(&source, "const Empty = struct {\n").unwrap();
    let tool = root.join("zig-tool");
    fs::write(
        &tool,
        "#!/bin/sh\nif [ \"$1\" = version ]; then printf '0.16.0\\n'; exit 0; fi\nif [ \"$1\" = ast-check ]; then printf '<stdin>:1:23: error: expected closing brace\\n' >&2; exit 1; fi\nexit 2\n",
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "zig"])
        .arg(&source)
        .args(["--zig-tool", tool.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["native"]["status"], "diagnostics_observed");
    assert_eq!(report["native"]["diagnostic_count"], 1);
    assert_eq!(report["native"]["tool_sha256"].as_str().unwrap().len(), 64);
    assert_eq!(report["syntax_precheck"], serde_json::Value::Null);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn absent_explicit_zig_tool_uses_unqualified_wasm_without_claiming_clean() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-zig-fallback-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("empty.zig");
    fs::write(&source, "const Empty = struct {};\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "zig"])
        .arg(&source)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["native"]["status"], "not_run");
    assert_eq!(
        report["syntax_precheck"]["precheck"]["status"],
        "incomplete"
    );
    assert_eq!(report["syntax_precheck"]["grammar_qualified"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn clean_native_ast_check_does_not_claim_full_lint_or_run_wasm() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-zig-native-clean-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("empty.zig");
    fs::write(&source, "const Empty = struct {};\n").unwrap();
    let tool = root.join("zig-tool");
    fs::write(
        &tool,
        "#!/bin/sh\nif [ \"$1\" = version ]; then printf '0.16.0\\n'; exit 0; fi\nif [ \"$1\" = ast-check ]; then exit 0; fi\nexit 2\n",
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "zig"])
        .arg(&source)
        .args(["--zig-tool", tool.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["native"]["status"], "completed");
    assert_eq!(report["syntax_precheck"], serde_json::Value::Null);
    assert_eq!(report["status"], "incomplete");
    assert_eq!(report["coverage_proven"], false);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn changed_zig_executable_cannot_authorize_native_completion() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-zig-changed-tool-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("empty.zig");
    fs::write(&source, "const Empty = struct {};\n").unwrap();
    let tool = root.join("zig-tool");
    fs::write(
        &tool,
        "#!/bin/sh\nif [ \"$1\" = version ]; then printf '0.16.0\\n'; exit 0; fi\nif [ \"$1\" = ast-check ]; then printf '#changed\\n' >> \"$0\"; exit 0; fi\nexit 2\n",
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "zig"])
        .arg(&source)
        .args(["--zig-tool", tool.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["native"]["status"], "incomplete");
    assert_eq!(report["native"]["reason"], "zig_tool_changed_during_check");
    assert_eq!(report["syntax_precheck"]["grammar_qualified"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    fs::remove_dir_all(root).unwrap();
}
