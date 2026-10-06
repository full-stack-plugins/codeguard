#![cfg(unix)]
//! 原项目文档契约的实际 Clippy 检查；不安装工具，不作为独立精度或生产资格。
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command};

struct Project(PathBuf);
impl Project {
    fn new(index: usize, source: &str) -> Self {
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-clippy-doc-{}-{index}", std::process::id()));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='documentation-sample'\nversion='0.1.0'\nedition='2021'\n",
        )
        .unwrap();
        fs::write(
            root.join("Cargo.lock"),
            "version = 4\n[[package]]\nname=\"documentation-sample\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        fs::write(root.join("src/lib.rs"), source).unwrap();
        let init = Self(root);
        init.invoke(&["init", "--apply", "--format=json"]);
        init
    }
    fn invoke(&self, args: &[&str]) -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .arg(&self.0)
            .env_remove("CODEGUARD_TIMEOUT")
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "requires explicitly selected installed Cargo with Clippy; no installation"]
fn native_documentation_contracts_reuse_tasks_and_preserve_semantic_limits() {
    let tool = std::env::var("CODEGUARD_TEST_CARGO").expect("select existing Cargo");
    let version = Command::new(&tool)
        .args(["clippy", "--version"])
        .output()
        .unwrap();
    assert!(version.status.success());
    let version = String::from_utf8(version.stdout).unwrap();
    let cli_sha256 = format!(
        "{:x}",
        Sha256::digest(fs::read(env!("CARGO_BIN_EXE_codeguard")).unwrap())
    );
    let mut evidence = Vec::new();
    for (index, (rule, section, body, detail)) in [
        (
            "missing_errors_doc",
            "Errors",
            "pub fn fail() -> Result<(), &'static str> { Err(\"unavailable\") }",
            "不可用时返回 unavailable 错误。",
        ),
        (
            "missing_panics_doc",
            "Panics",
            "pub fn fail() { panic!(\"unavailable\"); }",
            "每次调用都会因 unavailable 而 panic。",
        ),
        (
            "missing_safety_doc",
            "Safety",
            "pub unsafe fn fail() {}",
            "本接口没有额外调用前置条件。",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let source = |doc: &str, suppression: &str| {
            format!(
                "#![warn(clippy::{rule})]\n{}/// 演示原生契约检查。\n{}{body}\n",
                if suppression.is_empty() {
                    String::new()
                } else {
                    format!("{suppression}\n")
                },
                if doc.is_empty() {
                    String::new()
                } else {
                    format!("{doc}\n")
                }
            )
        };
        let p = Project::new(index, &source("", ""));
        let lint = || p.invoke(&["lint", "rust", "--cargo-tool", &tool, "--format=json"]);
        let first = lint();
        let native_rule = format!("clippy::{rule}");
        assert_eq!(
            first["native_report"]["local_scan_complete"], true,
            "{first}"
        );
        let finding = first["native_report"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["rule_id"] == native_rule)
            .expect("native documentation finding");
        let id = finding["finding_id"].as_str().unwrap();
        let brief = &first["next"]["repair_brief"];
        assert_eq!(brief["task_id"], id, "{first}");
        assert!(brief["step"].as_str().unwrap().contains(section), "{brief}");
        let repeat = lint();
        assert_eq!(repeat["next"]["repair_brief"]["task_id"], id);
        let verify = || p.invoke(&["task", "verify", id, "--cargo-tool", &tool, "--format=json"]);
        let present = verify();
        assert_eq!(present["observation"], "still_present", "{present}");
        fs::write(
            p.0.join("src/lib.rs"),
            source("", &format!("#[allow(clippy::{rule})]")),
        )
        .unwrap();
        let suppressed = verify();
        assert_eq!(
            suppressed["observation"], "suppression_requires_review",
            "{suppressed}"
        );
        fs::write(
            p.0.join("src/lib.rs"),
            source(&format!("/// # {section}"), ""),
        )
        .unwrap();
        let bare = lint();
        assert_eq!(bare["native_report"]["local_scan_complete"], true, "{bare}");
        assert!(
            bare["native_report"]["findings"]
                .as_array()
                .unwrap()
                .iter()
                .all(|f| f["rule_id"] != native_rule),
            "{bare}"
        );
        assert_eq!(bare["delivery_decision"], "not_evaluated");
        fs::write(
            p.0.join("src/lib.rs"),
            source(&format!("/// # {section}\n/// {detail}"), ""),
        )
        .unwrap();
        let repaired = verify();
        assert_eq!(
            repaired["observation"], "candidate_absent_unverified_policy",
            "{repaired}"
        );
        let fact: Value = serde_json::from_slice(
            &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(fact["state"], "open");
        evidence.push(json!({"rule":native_rule,"first":first,"repeat":repeat,"present":present,"suppressed":suppressed,"bare_heading":bare,"repaired":repaired,"state":fact["state"]}));
    }
    let p = Project::new(
        3,
        "/// 返回固定错误。\npub fn fail() -> Result<(), &'static str> { Err(\"unavailable\") }\n",
    );
    let unselected = p.invoke(&["lint", "rust", "--cargo-tool", &tool, "--format=json"]);
    assert_eq!(
        unselected["native_report"]["local_scan_complete"], true,
        "{unselected}"
    );
    assert!(
        unselected["native_report"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["rule_id"] != "clippy::missing_errors_doc")
    );
    if let Ok(path) = std::env::var("CODEGUARD_CLIPPY_DOC_EVIDENCE") {
        fs::write(path, serde_json::to_vec_pretty(&json!({"qualification":"not_granted","cli_sha256":cli_sha256,"clippy_version":version.trim(),"cases":evidence,"unselected_pedantic":unselected})).unwrap()).unwrap();
    }
}
