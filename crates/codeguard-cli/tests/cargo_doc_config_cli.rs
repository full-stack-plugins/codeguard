use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command};

struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "cg-doc-config-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
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

#[test]
fn workspace_documentation_declarations_require_member_opt_in() {
    let p = Project::new();
    fs::create_dir_all(p.0.join("member/src")).unwrap();
    fs::write(p.0.join("member/src/lib.rs"), "pub fn f() {}\n").unwrap();
    let member = "[package]\nname='member'\nversion='0.1.0'\n[lints]\nworkspace=true\n";
    fs::write(p.0.join("member/Cargo.toml"), member).unwrap();
    let workspace = "[workspace]\nmembers=['member']\n[workspace.lints.clippy]\nmissing_errors_doc='warn'\n[workspace.lints.rust]\nmissing_docs='deny'\n";
    let r = p.observe(workspace);
    let rows = r["checker_configurations"].as_array().unwrap();
    let c = rows
        .iter()
        .find(|c| c["build_root"] == "member" && c["checker_id"] == "rust.cargo_clippy")
        .unwrap();
    assert_eq!(c["configuration"], "unknown");
    assert_eq!(c["configuration_ref"], "member/Cargo.toml");
    assert_eq!(
        c["reason"],
        "cargo_doc_lints_workspace_declared_scope_unverified"
    );
    assert!(
        c["next_action"]
            .as_str()
            .unwrap()
            .contains("missing_errors_doc=warn")
    );
    assert!(c["next_action"].as_str().unwrap().contains("Cargo.toml"));
    fs::write(
        p.0.join("member/Cargo.toml"),
        "[package]\nname='member'\nversion='0.1.0'\n",
    )
    .unwrap();
    let r = p.observe(workspace);
    let c = r["checker_configurations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["build_root"] == "member" && c["checker_id"] == "rust.cargo_clippy")
        .unwrap();
    assert_eq!(c["reason"], "cargo_doc_lints_not_declared_in_manifest");
    assert!(
        !c["next_action"]
            .as_str()
            .unwrap()
            .contains("missing_errors_doc=warn")
    );
}

#[test]
fn nearest_workspace_and_explicit_reference_do_not_borrow_farther_rules() {
    let p = Project::new();
    fs::create_dir_all(p.0.join("nested/member/src")).unwrap();
    fs::write(p.0.join("nested/member/src/lib.rs"), "pub fn f() {}\n").unwrap();
    let member = "[package]\nname='member'\nversion='0.1.0'\n[lints]\nworkspace=true\n";
    fs::write(p.0.join("nested/member/Cargo.toml"), member).unwrap();
    fs::write(
        p.0.join("nested/Cargo.toml"),
        "[workspace]\nmembers=['member']\n[workspace.lints.clippy]\nmissing_errors_doc='allow'\n",
    )
    .unwrap();
    let outer = "[workspace]\nmembers=[]\n[workspace.lints.clippy]\nmissing_errors_doc='forbid'\n";
    for (nested, reason, expected) in [
        (
            "[workspace]\nmembers=['member']\n[workspace.lints.clippy]\nmissing_errors_doc='allow'\n",
            "cargo_doc_lints_workspace_declared_scope_unverified",
            Some("missing_errors_doc=allow"),
        ),
        (
            "[workspace]\nmembers=['member']\n",
            "cargo_doc_lints_workspace_rules_not_declared",
            None,
        ),
        (
            "[workspace",
            "cargo_doc_lints_workspace_manifest_unavailable_or_invalid",
            None,
        ),
    ] {
        fs::write(p.0.join("nested/Cargo.toml"), nested).unwrap();
        let r = p.observe(outer);
        let c = r["checker_configurations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["build_root"] == "nested/member" && c["checker_id"] == "rust.cargo_clippy")
            .unwrap();
        assert_eq!(c["reason"], reason);
        assert_eq!(c["configuration"], "unknown");
        assert!(
            !c["next_action"]
                .as_str()
                .unwrap()
                .contains("missing_errors_doc=forbid")
        );
        if let Some(text) = expected {
            assert!(c["next_action"].as_str().unwrap().contains(text));
        }
    }
    fs::write(
        p.0.join("nested/member/Cargo.toml"),
        "[package]\nname='member'\nversion='0.1.0'\nworkspace='../..'\n[lints]\nworkspace=true\n",
    )
    .unwrap();
    let r = p.observe(outer);
    let c = r["checker_configurations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["build_root"] == "nested/member" && c["checker_id"] == "rust.cargo_clippy")
        .unwrap();
    assert_eq!(
        c["reason"],
        "cargo_doc_lints_explicit_workspace_reference_unresolved"
    );
    assert!(
        !c["next_action"]
            .as_str()
            .unwrap()
            .contains("missing_errors_doc=forbid")
    );
}

#[test]
#[ignore = "requires explicit existing Cargo/Clippy; no installation"]
fn native_workspace_documentation_opt_in_matches_cargo() {
    let tool = std::env::var("CODEGUARD_TEST_CARGO").expect("select existing Cargo");
    let p = Project::new();
    let source = "/// 返回固定错误。\npub fn failure() -> Result<(), &'static str> { Err(\"unavailable\") }\n";
    for name in ["member", "independent"] {
        fs::create_dir_all(p.0.join(name).join("src")).unwrap();
        fs::write(p.0.join(name).join("src/lib.rs"), source).unwrap();
        let opt_in = if name == "member" {
            "[lints]\nworkspace=true\n"
        } else {
            ""
        };
        fs::write(
            p.0.join(name).join("Cargo.toml"),
            format!("[package]\nname='{name}'\nversion='0.1.0'\nedition='2021'\n{opt_in}"),
        )
        .unwrap();
    }
    fs::write(p.0.join("Cargo.lock"), "version = 4\n[[package]]\nname=\"member\"\nversion=\"0.1.0\"\n[[package]]\nname=\"independent\"\nversion=\"0.1.0\"\n").unwrap();
    let version = Command::new(&tool)
        .args(["clippy", "--version"])
        .env("RUSTUP_AUTO_INSTALL", "0")
        .output()
        .unwrap();
    assert!(version.status.success());
    let mut evidence = Vec::new();
    for level in ["warn", "allow"] {
        let manifest = format!(
            "[workspace]\nmembers=['member','independent']\nresolver='2'\n[workspace.lints.clippy]\nmissing_errors_doc='{level}'\n"
        );
        let discovery = p.observe(&manifest);
        let mut oracle = Vec::new();
        for name in ["member", "independent"] {
            let row = discovery["checker_configurations"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["build_root"] == name && c["checker_id"] == "rust.cargo_clippy")
                .unwrap();
            assert_eq!(row["configuration"], "unknown");
            if name == "member" {
                assert_eq!(
                    row["reason"],
                    "cargo_doc_lints_workspace_declared_scope_unverified"
                );
                assert!(
                    row["next_action"]
                        .as_str()
                        .unwrap()
                        .contains(&format!("missing_errors_doc={level}"))
                );
            } else {
                assert_eq!(row["reason"], "cargo_doc_lints_not_declared_in_manifest");
            }
            let out = Command::new(&tool)
                .args([
                    "clippy",
                    "--offline",
                    "--locked",
                    "--message-format=json",
                    "--manifest-path",
                ])
                .arg(p.0.join(name).join("Cargo.toml"))
                .arg("--target-dir")
                .arg(p.0.join(".native-target"))
                .env("RUSTUP_AUTO_INSTALL", "0")
                .env("CARGO_NET_OFFLINE", "true")
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            let events: Vec<Value> = String::from_utf8(out.stdout)
                .unwrap()
                .lines()
                .map(|l| serde_json::from_str(l).unwrap())
                .collect();
            assert_eq!(events.last().unwrap()["reason"], "build-finished");
            assert_eq!(events.last().unwrap()["success"], true);
            let count = events
                .iter()
                .filter(|e| {
                    e["reason"] == "compiler-message"
                        && e["message"]["code"]["code"] == "clippy::missing_errors_doc"
                })
                .count();
            assert_eq!(count, usize::from(level == "warn" && name == "member"));
            assert_eq!(
                fs::read_to_string(p.0.join(name).join("src/lib.rs")).unwrap(),
                source
            );
            oracle.push(serde_json::json!({"package": name, "count": count, "original_cargo_events": events}));
        }
        evidence
            .push(serde_json::json!({"level": level, "discovery": discovery, "oracle": oracle}));
    }
    if let Ok(path) = std::env::var("CODEGUARD_CARGO_DOC_WORKSPACE_EVIDENCE") {
        fs::write(path, serde_json::to_vec_pretty(&serde_json::json!({"qualification":"not_granted", "clippy_version": String::from_utf8(version.stdout).unwrap().trim(), "cli_sha256":format!("{:x}", Sha256::digest(fs::read(env!("CARGO_BIN_EXE_codeguard")).unwrap())), "cases": evidence})).unwrap()).unwrap();
    }
}
