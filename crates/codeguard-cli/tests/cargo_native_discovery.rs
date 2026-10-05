#![cfg(unix)]

use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    bin: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let base = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-cargo-discovery-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let root = base.join("project");
        let bin = base.join("bin");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir(&bin).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='sample'\nversion='0.1.0'\nedition='2021'\n",
        )
        .unwrap();
        fs::write(
            root.join("Cargo.lock"),
            "version=4\n[[package]]\nname='sample'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn answer()->i32 { return 42; }\n",
        )
        .unwrap();
        Self { root, bin }
    }
    fn tool(&self, directory: &Path, failing: bool) -> PathBuf {
        fs::create_dir_all(directory).unwrap();
        let target = directory.join("proxy");
        let diagnostic = json!({"reason":"compiler-message","message":{"level":"warning","code":{"code":"clippy::needless_return"},"spans":[{"file_name":"src/lib.rs","line_start":1,"column_start":1,"is_primary":true}]}});
        let body = if failing {
            "printf 'native failure\\n'; exit 2".to_owned()
        } else {
            format!(
                "if [ \"$1\" = clippy ]; then /bin/cat <<'CG_REPORT'\n{diagnostic}\nCG_REPORT\nfi\nprintf '%s\\n' '{{\"reason\":\"build-finished\",\"success\":true}}'"
            )
        };
        fs::write(&target,format!("#!/bin/sh\n[ \"${{0##*/}}\" = cargo ] || exit 91\n[ \"$RUSTUP_AUTO_INSTALL\" = 0 ] || exit 92\n[ \"$RUSTUP_TOOLCHAIN\" = existing-selected-toolchain ] || exit 93\nprintf '%s\\n' \"$1\" >> '{}/started'\n{body}\n",directory.display())).unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o700)).unwrap();
        let entry = directory.join("cargo");
        symlink(&target, &entry).unwrap();
        entry
    }
    fn command(&self, args: &[&str], path: &str) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .env("PATH", path)
            .env("RUSTUP_TOOLCHAIN", "existing-selected-toolchain")
            .env("RUSTUP_AUTO_INSTALL", "1")
            .env_remove("CODEGUARD_TIMEOUT")
            .env_remove("CODEGUARD_JOBS")
            .output()
            .unwrap();
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).expect("公开命令须输出JSON"),
        )
    }
    fn check(&self, extra: &[&str], path: &str) -> Value {
        let mut args = vec![
            "check",
            "all",
            self.root.to_str().unwrap(),
            "--format=json",
            "--timeout",
            "20s",
            "--jobs",
            "1",
        ];
        args.extend(extra);
        let (exit, report) = self.command(&args, path);
        assert_eq!(exit, 3, "{report}");
        report
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(self.root.parent().unwrap());
    }
}

#[test]
fn path_cargo_proxy_runs_all_three_native_checks_with_selected_toolchain_and_no_install() {
    let f = Fixture::new();
    f.tool(&f.bin, false);
    let report = f.check(&[], f.bin.to_str().unwrap());
    for field in ["rust_lint", "rust_comments", "rust_build"] {
        assert_eq!(
            report["native_results"][field]["local_scan_complete"], true,
            "{field}: {report}"
        );
    }
    assert_eq!(
        report["native_results"]["rust_lint"]["findings"][0]["rule_id"],
        "clippy::needless_return"
    );
    let calls = fs::read_to_string(f.bin.join("started")).unwrap();
    for operation in ["clippy", "check", "rustdoc"] {
        assert!(calls.lines().any(|line| line == operation), "{calls}");
    }
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn comments_and_build_entries_resolve_the_existing_cargo_proxy() {
    let f = Fixture::new();
    f.tool(&f.bin, false);
    for command in ["comments", "build"] {
        let (exit, report) = f.command(
            &[command, "rust", f.root.to_str().unwrap(), "--format=json"],
            f.bin.to_str().unwrap(),
        );
        assert_eq!(exit, 3);
        assert_eq!(report["local_scan_complete"], true, "{report}");
    }
}

#[test]
fn explicitly_invalid_cargo_does_not_fall_back_to_path() {
    let f = Fixture::new();
    f.tool(&f.bin, false);
    let absent = f.bin.join("absent");
    let report = f.check(
        &["--cargo-tool", absent.to_str().unwrap()],
        f.bin.to_str().unwrap(),
    );
    for field in ["rust_lint", "rust_comments", "rust_build"] {
        assert_eq!(
            report["native_results"][field]["reason"], "cargo_tool_unavailable",
            "{report}"
        );
    }
    assert!(!f.bin.join("started").exists());
}

#[test]
fn first_selected_cargo_failure_does_not_try_a_second_cargo() {
    let f = Fixture::new();
    f.tool(&f.bin, true);
    let second = f.bin.parent().unwrap().join("second");
    f.tool(&second, false);
    let path = format!("{}:{}", f.bin.display(), second.display());
    let report = f.check(&[], &path);
    assert!(!second.join("started").exists());
    assert!(f.bin.join("started").exists());
    assert_eq!(
        report["native_results"]["rust_lint"]["local_scan_complete"],
        false
    );
}

#[test]
fn relative_empty_and_non_executable_path_entries_are_not_cargo_candidates() {
    let f = Fixture::new();
    let unavailable = f.bin.parent().unwrap().join("nonexec");
    fs::create_dir(&unavailable).unwrap();
    fs::write(unavailable.join("cargo"), "must not run").unwrap();
    fs::set_permissions(unavailable.join("cargo"), fs::Permissions::from_mode(0o600)).unwrap();
    f.tool(&f.bin, false);
    let path = format!("relative::{}:{}", unavailable.display(), f.bin.display());
    let report = f.check(&[], &path);
    assert_eq!(
        report["native_results"]["rust_lint"]["local_scan_complete"], true,
        "{report}"
    );
}

#[test]
fn truly_missing_cargo_stays_an_environment_blocker_without_starting_native_code() {
    let f = Fixture::new();
    let report = f.check(&[], f.bin.to_str().unwrap());
    assert_eq!(
        report["native_results"]["rust_lint"]["reason"],
        "cargo_tool_not_selected"
    );
    assert!(!f.bin.join("started").exists());
}

#[test]
fn auto_selected_cargo_rechecks_the_same_stable_clippy_task() {
    let f = Fixture::new();
    f.tool(&f.bin, false);
    assert_eq!(
        f.command(
            &["init", f.root.to_str().unwrap(), "--apply", "--format=json"],
            f.bin.to_str().unwrap()
        )
        .0,
        3
    );
    let report = f.check(&[], f.bin.to_str().unwrap());
    let repeated = f.check(&[], f.bin.to_str().unwrap());
    let id = report["native_results"]["rust_lint"]["findings"][0]["finding_id"]
        .as_str()
        .expect("真实原生问题");
    assert_eq!(
        repeated["native_results"]["rust_lint"]["findings"][0]["finding_id"],
        id
    );
    let (exit, verified) = f.command(
        &[
            "task",
            "verify",
            id,
            f.root.to_str().unwrap(),
            "--format=json",
        ],
        f.bin.to_str().unwrap(),
    );
    assert_eq!(exit, 3);
    assert_eq!(verified["observation"], "still_present", "{verified}");
}

#[test]
#[ignore = "requires existing Cargo/rustup and installed toolchain; no installation permitted"]
fn actual_path_cargo_runs_native_checks_and_preserves_the_selected_toolchain() {
    let fixture = Fixture::new();
    let selected = std::env::var("CODEGUARD_TEST_INSTALLED_TOOLCHAIN")
        .expect("explicit existing toolchain required for this real acceptance test");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            fixture.root.to_str().unwrap(),
            "--format=json",
            "--jobs",
            "1",
            "--timeout",
            "60s",
        ])
        .env("RUSTUP_TOOLCHAIN", selected)
        .env("RUSTUP_AUTO_INSTALL", "1")
        .env_remove("CODEGUARD_TIMEOUT")
        .env_remove("CODEGUARD_JOBS")
        .output()
        .unwrap();
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(3), "{report}");
    for field in ["rust_lint", "rust_comments", "rust_build"] {
        assert_eq!(
            report["native_results"][field]["local_scan_complete"], true,
            "{field}: {report}"
        );
        assert!(report["native_results"][field]["tool_sha256"].is_string());
    }
    assert!(
        report["native_results"]["rust_lint"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["rule_id"] == "clippy::needless_return"),
        "{report}"
    );
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
#[ignore = "requires existing Cargo/rustup; verifies missing custom toolchain failure without installation"]
fn actual_missing_custom_toolchain_is_an_environment_failure_without_installation() {
    let fixture = Fixture::new();
    let rustup_home = fixture.bin.join("rustup_home");
    fs::create_dir(&rustup_home).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            fixture.root.to_str().unwrap(),
            "--format=json",
            "--jobs",
            "1",
            "--timeout",
            "20s",
        ])
        .env("RUSTUP_HOME", &rustup_home)
        .env("RUSTUP_TOOLCHAIN", "codeguard-absent-custom-toolchain")
        .env("RUSTUP_AUTO_INSTALL", "1")
        .env_remove("CODEGUARD_TIMEOUT")
        .env_remove("CODEGUARD_JOBS")
        .output()
        .unwrap();
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(3), "{report}");
    for field in ["rust_lint", "rust_comments", "rust_build"] {
        assert_eq!(
            report["native_results"][field]["local_scan_complete"], false,
            "{report}"
        );
        assert_ne!(
            report["native_results"][field]["reason"],
            "cargo_tool_not_selected"
        );
        assert!(
            report["native_results"][field]["findings"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    let toolchains = rustup_home.join("toolchains");
    assert!(!toolchains.exists() || fs::read_dir(toolchains).unwrap().next().is_none());
    assert_eq!(report["delivery_decision"], "incomplete");
}
