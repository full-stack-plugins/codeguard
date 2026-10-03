#![cfg(unix)]
use serde_json::Value;
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
fn native_zig_entry_does_not_depend_on_the_optional_wasm_feature() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-zig-base-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let source = root.join("app.zig");
    fs::write(&source, "pub fn main( void {\n").unwrap();
    let tool = root.join("zig");
    fs::write(&tool,"#!/bin/sh\nif [ \"$1\" = version ]; then printf '0.16.0\\n'; exit 0; fi\nwhile IFS= read -r line; do :; done\nprintf '<stdin>:1:13: error: expected token\\n' >&2\nexit 1\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "zig"])
        .arg(&source)
        .arg("--zig-tool")
        .arg(&tool)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["native"]["status"], "diagnostics_observed");
    assert!(r["syntax_precheck"].is_null());
    fs::remove_dir_all(root).unwrap();
}
