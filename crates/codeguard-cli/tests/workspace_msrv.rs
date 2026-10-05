use serde_json::Value;
use std::collections::BTreeSet;
use std::process::Command;

fn version(value: &str) -> (u32, u32, u32) {
    let mut parts = value.split('.');
    (
        parts.next().unwrap().parse().unwrap(),
        parts.next().unwrap().parse().unwrap(),
        parts.next().unwrap_or("0").parse().unwrap(),
    )
}

#[test]
fn a_higher_patch_minimum_is_not_the_declared_baseline() {
    assert!(version("1.85.1") > version("1.85"));
    assert_eq!(version("1.85.0"), version("1.85"));
    assert!(version("1.84.99") < version("1.85"));
}

#[test]
fn selected_dependency_msrv_does_not_exceed_workspace_baseline() {
    let rustc = Command::new("rustc").arg("-vV").output().unwrap();
    assert!(rustc.status.success());
    let banner = String::from_utf8(rustc.stdout).unwrap();
    let target = banner
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .unwrap();
    let mut cargo = Command::new("cargo");
    cargo.args([
        "metadata",
        "--locked",
        "--offline",
        "--format-version",
        "1",
        "--filter-platform",
        target,
    ]);
    if cfg!(feature = "wasm-precheck") {
        cargo.args(["--features", "wasm-precheck"]);
    }
    let output = cargo
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Cargo metadata: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value = serde_json::from_slice(&output.stdout).unwrap();
    let selected: BTreeSet<&str> = metadata["resolve"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node["id"].as_str().unwrap())
        .collect();
    let workspace: BTreeSet<&str> = metadata["workspace_members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_str().unwrap())
        .collect();
    let mut failures = Vec::new();
    let mut undeclared = 0;
    for package in metadata["packages"].as_array().unwrap() {
        let id = package["id"].as_str().unwrap();
        if !selected.contains(id) {
            continue;
        }
        if workspace.contains(id) {
            assert_eq!(
                version(package["rust_version"].as_str().unwrap()),
                (1, 85, 0)
            );
        }
        match package["rust_version"].as_str() {
            Some(minimum) if version(minimum) > (1, 85, 0) => failures.push(format!(
                "{} {} requires Rust {}",
                package["name"].as_str().unwrap(),
                package["version"].as_str().unwrap(),
                minimum
            )),
            None => undeclared += 1,
            _ => {}
        }
    }
    assert!(failures.is_empty(), "MSRV incompatible: {failures:?}");
    // 声明相容仅是静态下界检查；无声明的依赖及实际 Rust 1.85 编译仍须 CI 证明。
    println!(
        "target={target}; selected={}; rust_version_undeclared={undeclared}; actual_msrv_build=not_run",
        selected.len()
    );
}
