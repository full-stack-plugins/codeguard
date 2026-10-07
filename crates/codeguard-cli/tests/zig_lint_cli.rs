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
        "#!/bin/sh\nif [ \"$1\" = version ]; then printf '0.16.0\\n'; exit 0; fi\nif [ \"$1\" = ast-check ]; then while IFS= read -r line; do :; done; printf '<stdin>:1:23: error: expected closing brace\\n' >&2; exit 1; fi\nexit 2\n",
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
    assert_eq!(
        report["native"]["status"], "diagnostics_observed",
        "{report}"
    );
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
        .env("PATH", "")
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
        "#!/bin/sh\nif [ \"$1\" = version ]; then printf '0.16.0\\n'; exit 0; fi\nif [ \"$1\" = ast-check ]; then while IFS= read -r line; do :; done; exit 0; fi\nexit 2\n",
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
    assert_eq!(report["native"]["status"], "completed", "{report}");
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
        "#!/bin/sh\nif [ \"$1\" = version ]; then printf '0.16.0\\n'; exit 0; fi\nif [ \"$1\" = ast-check ]; then while IFS= read -r line; do :; done; printf '#changed\\n' >> \"$0\"; exit 0; fi\nexit 2\n",
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

#[test]
fn cancelled_native_zig_check_returns_130_without_running_wasm_fallback() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-zig-cancel-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("empty.zig");
    fs::write(&source, "const Empty = struct {};\n").unwrap();
    let tool = root.join("zig-tool");
    fs::write(
        &tool,
        "#!/bin/sh\nprintf started > \"$0.started\"\nexec /bin/sleep 30\n",
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "zig"])
        .arg(&source)
        .args(["--zig-tool", tool.to_str().unwrap(), "--format=json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let marker = tool.with_extension("started");
    let wait_start = std::time::Instant::now();
    while !marker.exists() && wait_start.elapsed() < std::time::Duration::from_secs(10) {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(marker.exists(), "原生版本探测必须先启动");
    let kill = Command::new("/bin/kill")
        .arg("-INT")
        .arg(child.id().to_string())
        .status()
        .unwrap();
    assert!(kill.success(), "SIGINT 必须送达 CLI 进程");
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(130),
        "取消必须返回 130 而不是普通未完成 3；stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["command_status"], "cancelled", "{report}");
    assert_eq!(report["native"]["status"], "incomplete", "{report}");
    assert_eq!(report["native"]["reason"], "request_cancelled", "{report}");
    // 取消后不得继续执行 WASM 候选初检。
    assert_eq!(
        report["syntax_precheck"],
        serde_json::Value::Null,
        "{report}"
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
    fs::remove_dir_all(root).unwrap();
}
