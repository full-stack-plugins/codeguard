#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::Command,
};

fn fixture(name: &str) -> PathBuf {
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "codeguard-erlang-discovery-{name}-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("sample.erl"), "-module(sample).\nf() -> ok\n").unwrap();
    root
}

fn fake(directory: &Path, version: &str) -> PathBuf {
    fs::create_dir_all(directory).unwrap();
    let tool = directory.join("erl");
    fs::write(&tool, format!(
        "#!/bin/sh\ncase \"$*\" in *system_info*) printf '%s\\n' '{version}';; *) /bin/cat >/dev/null; printf '%s\\n' '{{\"schema_version\":\"0.1.0\",\"forms\":1,\"preprocessing\":false,\"diagnostics_truncated\":false,\"diagnostics\":[{{\"line\":2,\"column\":8,\"rule_id\":\"erlang.syntax.error\"}}]}}';; esac\n"
    )).unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    tool
}

fn run(root: &Path, path: &std::ffi::OsStr, extra: &[&str]) -> serde_json::Value {
    let result = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "erlang"])
        .arg(root.join("sample.erl"))
        .args(extra)
        .arg("--format=json")
        .current_dir(root)
        .env("PATH", path)
        .env_remove("CODEGUARD_TIMEOUT")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(3), "{result:?}");
    serde_json::from_slice(&result.stdout).unwrap()
}

#[test]
fn path_native_diagnostic_precedes_candidate_and_records_the_selected_tool() {
    let root = fixture("available");
    let tool = fake(&root.join("bin"), "OTP 28");
    let report = run(&root, root.join("bin").as_os_str(), &[]);
    assert_eq!(report["schema_version"], "0.2.0");
    assert_eq!(report["tool_selection"]["source"], "path");
    assert_eq!(
        report["tool_selection"]["executable"],
        tool.to_str().unwrap()
    );
    assert_eq!(report["native"]["status"], "diagnostics_observed");
    assert_eq!(report["native"]["diagnostics"][0]["column"], 8);
    assert_eq!(report["syntax_precheck"], serde_json::Value::Null);
    assert_eq!(report["coverage_proven"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn explicit_bad_tool_never_uses_a_healthy_path_tool() {
    let root = fixture("explicit-bad");
    let tool = fake(&root.join("bin"), "OTP 28");
    let absent = root.join("absent");
    let report = run(
        &root,
        tool.parent().unwrap().as_os_str(),
        &["--erl-tool", absent.to_str().unwrap()],
    );
    assert_eq!(report["tool_selection"]["source"], "explicit");
    assert_eq!(report["native"]["status"], "incomplete");
    assert_eq!(report["native"]["tool_sha256"], serde_json::Value::Null);
    assert_eq!(report["syntax_precheck"], serde_json::Value::Null);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unsupported_first_path_tool_is_not_replaced_by_another_checker() {
    let root = fixture("unsupported-first");
    let first = fake(&root.join("first"), "OTP 29");
    let second = fake(&root.join("second"), "OTP 28");
    let path = std::env::join_paths([first.parent().unwrap(), second.parent().unwrap()]).unwrap();
    let report = run(&root, &path, &[]);
    assert_eq!(
        report["tool_selection"]["executable"],
        first.to_str().unwrap()
    );
    assert_eq!(
        report["native"]["reason"],
        "erlang_version_unverified_or_unsupported"
    );
    assert_eq!(report["syntax_precheck"], serde_json::Value::Null);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn relative_empty_and_nonexecutable_path_entries_do_not_select_a_tool() {
    let root = fixture("unsafe-entries");
    let tool = fake(&root.join("bin"), "OTP 28");
    let candidate = fake(&root, "OTP 28");
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o600)).unwrap();
    let path =
        std::env::join_paths([Path::new(""), Path::new("bin"), tool.parent().unwrap()]).unwrap();
    let report = run(&root, &path, &[]);
    assert_eq!(report["tool_selection"]["source"], "not_found");
    assert_eq!(
        report["tool_selection"]["executable"],
        serde_json::Value::Null
    );
    assert_eq!(report["native"]["status"], "not_run");
    assert_eq!(report["native"]["reason"], "erlang_tool_not_found_on_path");
    assert!(!report["syntax_precheck"].is_null());
    assert_eq!(
        fs::read_to_string(candidate).unwrap().lines().next(),
        Some("#!/bin/sh")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn path_symlink_is_frozen_to_its_canonical_executable() {
    let root = fixture("symlink");
    let tool = fake(&root.join("real"), "OTP 28");
    fs::create_dir_all(root.join("bin")).unwrap();
    symlink(&tool, root.join("bin/erl")).unwrap();
    let report = run(&root, root.join("bin").as_os_str(), &[]);
    assert_eq!(
        report["tool_selection"]["executable"],
        tool.to_str().unwrap()
    );
    assert_eq!(report["native"]["status"], "diagnostics_observed");
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires existing OTP 28 erl via CODEGUARD_ERL_BIN"]
fn actual_otp_on_path_detects_missing_period_without_an_explicit_flag() {
    let root = fixture("real");
    let tool = PathBuf::from(std::env::var("CODEGUARD_ERL_BIN").unwrap());
    fs::create_dir_all(root.join("bin")).unwrap();
    symlink(&tool, root.join("bin/erl")).unwrap();
    let report = run(&root, root.join("bin").as_os_str(), &[]);
    assert_eq!(report["tool_selection"]["source"], "path");
    assert_eq!(
        report["tool_selection"]["executable"],
        tool.canonicalize().unwrap().to_str().unwrap()
    );
    assert_eq!(report["native"]["version"], "OTP 28");
    assert_eq!(report["native"]["status"], "diagnostics_observed");
    assert_eq!(report["native"]["diagnostics"][0]["line"], 2);
    assert_eq!(report["native"]["diagnostics"][0]["column"], 8);
    assert_eq!(report["syntax_precheck"], serde_json::Value::Null);
    fs::remove_dir_all(root).unwrap();
}
