use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command};

struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("cg-doc-config-{}", std::process::id()));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "#![warn(clippy::missing_errors_doc)]\n/// 已在源码启用。\npub fn f() {}\n",
        )
        .unwrap();
        Self(root)
    }
    fn observe(&self, manifest: &str) -> Value {
        fs::write(self.0.join("Cargo.toml"), manifest).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["detect", "--format=json"])
            .arg(&self.0)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "requires explicit existing Cargo/Clippy; no installation"]
fn native_cargo_manifest_levels_keep_warnings_and_allowance_distinct() {
    let tool = std::env::var("CODEGUARD_TEST_CARGO").expect("select existing Cargo");
    let p = Project::new();
    let version = Command::new(&tool)
        .args(["clippy", "--version"])
        .env("RUSTUP_AUTO_INSTALL", "0")
        .output()
        .unwrap();
    assert!(version.status.success());
    let version = String::from_utf8(version.stdout).unwrap();
    let cli_sha256 = format!(
        "{:x}",
        Sha256::digest(fs::read(env!("CARGO_BIN_EXE_codeguard")).unwrap())
    );
    fs::write(p.0.join("src/lib.rs"),"/// 返回固定错误。\npub fn failure() -> Result<(), &'static str> { Err(\"unavailable\") }\n/// 总是panic。\npub fn panics() { panic!(\"unavailable\"); }\n/// 无额外前置条件。\npub unsafe fn unsafe_call() {}\n").unwrap();
    fs::write(
        p.0.join("Cargo.lock"),
        "version = 4\n[[package]]\nname=\"sample\"\nversion=\"0.1.0\"\n",
    )
    .unwrap();
    let rules = [
        "missing_errors_doc",
        "missing_panics_doc",
        "missing_safety_doc",
    ];
    let mut evidence = Vec::new();
    let original_source = fs::read_to_string(p.0.join("src/lib.rs")).unwrap();
    for level in ["warn", "allow", "source_attribute"] {
        let mut manifest = "[package]\nname='sample'\nversion='0.1.0'\nedition='2021'\n".to_owned();
        let mut source = original_source.clone();
        if level == "source_attribute" {
            source = format!(
                "#![warn(clippy::missing_errors_doc, clippy::missing_panics_doc, clippy::missing_safety_doc)]\n{source}"
            );
        } else {
            manifest.push_str("[lints.clippy]\n");
            for rule in rules {
                manifest.push_str(&format!("{rule}='{level}'\n"));
            }
        }
        fs::write(p.0.join("src/lib.rs"), source).unwrap();
        let discovery = p.observe(&manifest);
        let c = discovery["checker_configurations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["checker_id"] == "rust.cargo_clippy")
            .unwrap();
        if level == "source_attribute" {
            assert_eq!(c["reason"], "cargo_doc_lints_not_declared_in_manifest");
        } else {
            for rule in rules {
                assert!(
                    c["next_action"]
                        .as_str()
                        .unwrap()
                        .contains(&format!("{rule}={level}"))
                );
            }
        }
        assert_eq!(c["configuration"], "unknown");
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["lint", "rust", "--cargo-tool", &tool, "--format=json"])
            .arg(&p.0)
            .env_remove("CODEGUARD_TIMEOUT")
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let lint: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(lint["native_report"]["local_scan_complete"], true, "{lint}");
        for rule in rules {
            assert_eq!(
                lint["native_report"]["findings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|f| f["rule_id"] == format!("clippy::{rule}")),
                level != "allow",
                "{lint}"
            );
        }
        assert_eq!(lint["delivery_decision"], "not_evaluated");
        evidence
            .push(serde_json::json!({"declared_level":level,"discovery":discovery,"lint":lint}));
    }
    if let Ok(path) = std::env::var("CODEGUARD_CARGO_DOC_CONFIG_EVIDENCE") {
        fs::write(
            path,
            serde_json::to_vec_pretty(
                &serde_json::json!({"qualification":"not_granted","clippy_version":version.trim(),"cli_sha256":cli_sha256,"cases":evidence}),
            )
            .unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn direct_cargo_doc_declarations_are_observed_without_effective_approval() {
    let p = Project::new();
    let package = "[package]\nname='sample'\nversion='0.1.0'\n";
    let r = p.observe(&format!("{package}[lints.clippy]\nmissing_errors_doc='warn'\nmissing_panics_doc={{level='deny',priority=-1}}\nmissing_safety_doc='allow'\n[lints.rust]\nmissing_docs='warn'\n[lints.rustdoc]\nbroken_intra_doc_links='deny'\n"));
    let rows = r["checker_configurations"].as_array().unwrap();
    for (checker, text) in [
        ("rust.cargo_clippy", "missing_errors_doc=warn"),
        ("rust.cargo_rustdoc", "missing_docs=warn"),
    ] {
        let row = rows
            .iter()
            .find(|row| row["checker_id"] == checker)
            .expect("Cargo documentation config");
        assert_eq!(row["configuration"], "unknown");
        assert_eq!(row["configuration_ref"], "Cargo.toml");
        assert!(row["next_action"].as_str().unwrap().contains(text));
    }
    let rustdoc = rows
        .iter()
        .find(|r| r["checker_id"] == "rust.cargo_rustdoc")
        .unwrap();
    assert!(
        rustdoc["next_action"]
            .as_str()
            .unwrap()
            .contains("broken_intra_doc_links=deny")
    );
    let original_manifest = fs::read(p.0.join("Cargo.toml")).unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["init", "--apply", "--format=json"])
        .arg(&p.0)
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let agents = fs::read_to_string(p.0.join("AGENTS.md")).unwrap();
    assert!(agents.contains(".codeguard/project.json"), "{agents}");
    let profile = fs::read_to_string(p.0.join(".codeguard/project.json")).unwrap();
    assert!(profile.contains("missing_errors_doc=warn"), "{profile}");
    assert!(agents.contains("rust.cargo_clippy"));
    assert_eq!(fs::read(p.0.join("Cargo.toml")).unwrap(), original_manifest);
    for (extra, reason) in [
        ("", "cargo_doc_lints_not_declared_in_manifest"),
        (
            "[lints]\nworkspace=true\n",
            "cargo_doc_lints_workspace_inheritance_unresolved",
        ),
        (
            "[lints.clippy]\npedantic='warn'\n",
            "cargo_doc_lints_group_or_default_unresolved",
        ),
    ] {
        let r = p.observe(&format!("{package}{extra}"));
        let c = r["checker_configurations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["checker_id"] == "rust.cargo_clippy")
            .unwrap();
        assert_eq!(c["configuration"], "unknown");
        assert_eq!(c["reason"], reason);
    }
    let r =
        p.observe("[workspace]\nmembers=[]\n[workspace.lints.clippy]\nmissing_errors_doc='warn'\n");
    assert!(
        r["checker_configurations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["reason"] == "cargo_doc_lints_virtual_workspace_unresolved")
    );
}
