use codeguard_adapters::parse_cargo_build_json;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture(PathBuf);

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "requires explicit existing CODEGUARD_CARGO_BIN; native Cargo check, no installation"]
fn native_cargo_distinguishes_type_error_success_and_build_script_failure() {
    let cargo = std::env::var("CODEGUARD_CARGO_BIN").expect("既有原生Cargo绝对路径");
    assert!(PathBuf::from(&cargo).is_absolute());
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let fixture = Fixture(std::env::temp_dir().join(format!(
        "codeguard-build-native-{}-{nonce}",
        std::process::id()
    )));
    fs::create_dir_all(fixture.0.join("src")).unwrap();
    fs::write(
        fixture.0.join("Cargo.toml"),
        "[package]\nname='build-sample'\nversion='0.1.0'\nedition='2021'\n",
    )
    .unwrap();
    fs::write(
        fixture.0.join("Cargo.lock"),
        "version = 4\n[[package]]\nname=\"build-sample\"\nversion=\"0.1.0\"\n",
    )
    .unwrap();
    let lock = fs::read(fixture.0.join("Cargo.lock")).unwrap();
    let run = || {
        Command::new(&cargo)
            .args([
                "check",
                "--locked",
                "--offline",
                "--all-targets",
                "--message-format=json",
            ])
            .current_dir(&fixture.0)
            .env("CARGO_TARGET_DIR", fixture.0.join("target"))
            .env("CARGO_NET_OFFLINE", "true")
            .output()
            .unwrap()
    };
    fs::write(
        fixture.0.join("src/lib.rs"),
        "pub fn answer() -> i32 { \"wrong\" }\n",
    )
    .unwrap();
    let output = run();
    assert_eq!(output.status.code(), Some(101));
    let parsed = parse_cargo_build_json(&output.stdout, 101);
    assert_eq!(
        parsed.issue,
        None,
        "{parsed:?}; {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(parsed.build_success, Some(false));
    assert!(parsed.diagnostics.iter().any(|row| row.code == "E0308"));
    assert!(parsed.diagnostics.iter().all(|row| {
        PathBuf::from(&row.manifest_path).canonicalize().unwrap()
            == fixture.0.join("Cargo.toml").canonicalize().unwrap()
    }));

    fs::write(fixture.0.join("src/lib.rs"),"pub fn answer() -> i32 { 42 }\n#[test] fn must_not_run() { panic!(\"type check does not execute tests\"); }\n").unwrap();
    let output = run();
    assert_eq!(output.status.code(), Some(0));
    let parsed = parse_cargo_build_json(&output.stdout, 0);
    assert_eq!(parsed.issue, None, "{parsed:?}");
    assert_eq!(parsed.build_success, Some(true));
    assert!(parsed.diagnostics.is_empty());

    fs::write(
        fixture.0.join("build.rs"),
        "fn main() { panic!(\"fixture build environment failed\"); }\n",
    )
    .unwrap();
    let output = run();
    assert_eq!(output.status.code(), Some(101));
    let parsed = parse_cargo_build_json(&output.stdout, 101);
    assert!(parsed.issue.is_some(), "{parsed:?}");
    assert!(parsed.diagnostics.is_empty());
    assert_eq!(fs::read(fixture.0.join("Cargo.lock")).unwrap(), lock);
}
