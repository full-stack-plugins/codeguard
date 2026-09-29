use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::process::Command;

fn allowed(package: &str, dependency: &str, kind: &str) -> bool {
    match package {
        "codeguard-core" => dependency == "serde" || (dependency == "base64" && kind == "normal"),
        "codeguard-runtime" => {
            matches!(dependency, "codeguard-core" | "libc" | "ring")
                || (kind == "dev" && dependency == "rustls")
                || (kind == "normal"
                    && matches!(
                        dependency,
                        "tar"
                            | "zip"
                            | "flate2"
                            | "reqwest"
                            | "tokio"
                            | "tokio-util"
                            | "futures-util"
                            | "idna_adapter"
                            | "tree-sitter"
                    ))
        }
        "codeguard-adapters" => {
            (dependency == "semver" && kind == "normal")
                || matches!(
                    dependency,
                    "codeguard-core" | "serde" | "serde_json" | "roxmltree" | "toml" | "sha2"
                )
        }
        "codeguard-cli" => {
            (matches!(dependency, "ring" | "tokio" | "base64") && kind == "dev")
                || (dependency == "http" && kind == "normal")
                || matches!(
                    dependency,
                    "codeguard-core"
                        | "codeguard-runtime"
                        | "codeguard-adapters"
                        | "serde"
                        | "serde_json"
                        | "sha1"
                        | "sha2"
                )
        }
        _ => false,
    }
}

fn violations(metadata: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let Some(packages) = metadata.get("packages").and_then(Value::as_array) else {
        return vec!["Cargo metadata 缺 packages".into()];
    };
    let mut seen = BTreeSet::new();
    for package in packages {
        let Some(name) = package.get("name").and_then(Value::as_str) else {
            errors.push("Cargo package 缺名称".into());
            continue;
        };
        if !matches!(
            name,
            "codeguard-core" | "codeguard-runtime" | "codeguard-adapters" | "codeguard-cli"
        ) {
            continue;
        }
        if !seen.insert(name) {
            errors.push(format!("{name}: 重复 package"));
        }
        let Some(dependencies) = package.get("dependencies").and_then(Value::as_array) else {
            errors.push(format!("{name}: 缺依赖表"));
            continue;
        };
        for dependency in dependencies {
            let Some(target) = dependency.get("name").and_then(Value::as_str) else {
                errors.push(format!("{name}: 依赖缺 package 名称"));
                continue;
            };
            let kind = dependency
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("normal");
            if !allowed(name, target, kind) {
                errors.push(format!("{name}: 禁止依赖 {target}"));
            }
        }
    }
    for required in [
        "codeguard-core",
        "codeguard-runtime",
        "codeguard-adapters",
        "codeguard-cli",
    ] {
        if !seen.contains(required) {
            errors.push(format!("{required}: 缺 package"));
        }
    }
    errors
}

fn one_dependency(package: &str, dependency: &str, kind: &str) -> Value {
    let mut packages = vec![
        json!({"name":"codeguard-core","dependencies":[]}),
        json!({"name":"codeguard-runtime","dependencies":[]}),
        json!({"name":"codeguard-adapters","dependencies":[]}),
        json!({"name":"codeguard-cli","dependencies":[]}),
    ];
    let selected = packages
        .iter_mut()
        .find(|candidate| candidate["name"] == package)
        .expect("fixture package");
    selected["dependencies"] =
        json!([{"name":dependency,"rename":"alias","kind":kind,"target":"cfg(unix)"}]);
    json!({"packages":packages})
}

#[test]
fn current_workspace_respects_dependency_direction() {
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--offline",
            "--format-version",
            "1",
            "--no-deps",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Cargo metadata 应可运行");
    assert!(output.status.success());
    let metadata: Value = serde_json::from_slice(&output.stdout).expect("Cargo metadata JSON");
    let errors = violations(&metadata);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn forbidden_edges_stay_forbidden_under_alias_build_and_target_variants() {
    for (package, dependency, kind) in [
        ("codeguard-core", "codeguard-runtime", "normal"),
        ("codeguard-core", "codex-sdk", "dev"),
        ("codeguard-adapters", "codeguard-runtime", "build"),
        ("codeguard-adapters", "codeguard-runtime", "dev"),
        ("codeguard-adapters", "codeguard-runtime", "normal"),
        ("codeguard-core", "tree-sitter", "normal"),
        ("codeguard-adapters", "tree-sitter", "normal"),
        ("codeguard-cli", "tree-sitter", "normal"),
        ("codeguard-runtime", "tree-sitter", "dev"),
        ("codeguard-cli", "ring", "normal"),
        ("codeguard-cli", "ring", "build"),
        ("codeguard-core", "ring", "normal"),
        ("codeguard-core", "base64", "dev"),
        ("codeguard-core", "base64", "build"),
        ("codeguard-cli", "base64", "normal"),
        ("codeguard-core", "semver", "normal"),
        ("codeguard-runtime", "semver", "normal"),
        ("codeguard-cli", "semver", "normal"),
        ("codeguard-adapters", "semver", "build"),
        ("codeguard-core", "http", "normal"),
        ("codeguard-runtime", "http", "normal"),
        ("codeguard-adapters", "http", "normal"),
        ("codeguard-cli", "http", "build"),
        ("codeguard-cli", "http", "dev"),
        ("codeguard-core", "tar", "normal"),
        ("codeguard-cli", "zip", "normal"),
        ("codeguard-adapters", "flate2", "normal"),
        ("codeguard-runtime", "tar", "build"),
        ("codeguard-runtime", "zip", "dev"),
    ] {
        let metadata = one_dependency(package, dependency, kind);
        assert!(!violations(&metadata).is_empty());
    }
}

#[test]
fn semver_parser_is_scoped_to_adapter_normal_dependencies() {
    assert!(violations(&one_dependency("codeguard-adapters", "semver", "normal")).is_empty());
}

#[test]
fn base64_decoder_is_scoped_to_pure_core_and_cli_test_fixtures() {
    assert!(violations(&one_dependency("codeguard-core", "base64", "normal")).is_empty());
    assert!(violations(&one_dependency("codeguard-cli", "base64", "dev")).is_empty());
    assert!(!violations(&one_dependency("codeguard-runtime", "base64", "normal")).is_empty());
    assert!(!violations(&one_dependency("codeguard-adapters", "base64", "normal")).is_empty());
}

#[test]
fn dependency_checker_rejects_missing_metadata() {
    assert!(!violations(&json!({})).is_empty());
    assert!(!violations(&json!({"packages":[{"name":"codeguard-core"}]})).is_empty());
}

#[test]
fn archive_libraries_are_runtime_normal_dependencies_only() {
    for dependency in ["tar", "zip", "flate2"] {
        assert!(violations(&one_dependency("codeguard-runtime", dependency, "normal")).is_empty());
    }
}

#[test]
fn network_dependencies_are_runtime_only_and_async_cli_support_is_test_only() {
    for dependency in [
        "reqwest",
        "tokio",
        "tokio-util",
        "futures-util",
        "idna_adapter",
    ] {
        assert!(violations(&one_dependency("codeguard-runtime", dependency, "normal")).is_empty());
        for package in ["codeguard-core", "codeguard-adapters", "codeguard-cli"] {
            assert!(!violations(&one_dependency(package, dependency, "normal")).is_empty());
        }
        for kind in ["build", "dev"] {
            assert!(!violations(&one_dependency("codeguard-runtime", dependency, kind)).is_empty());
        }
    }
    assert!(violations(&one_dependency("codeguard-cli", "tokio", "dev")).is_empty());
    assert!(!violations(&one_dependency("codeguard-cli", "tokio", "build")).is_empty());
    assert!(violations(&one_dependency("codeguard-runtime", "rustls", "dev")).is_empty());
    assert!(!violations(&one_dependency("codeguard-runtime", "rustls", "normal")).is_empty());
    for package in ["codeguard-core", "codeguard-adapters", "codeguard-cli"] {
        assert!(!violations(&one_dependency(package, "rustls", "dev")).is_empty());
    }
}
