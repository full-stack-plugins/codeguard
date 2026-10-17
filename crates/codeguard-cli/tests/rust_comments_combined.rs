#![cfg(unix)]
//! 独立注释入口的双原生检查契约；受控进程不授予真实工具资格。
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-combined-comments-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='cg-combined'\nversion='0.1.0'\nedition='2021'\n",
        )
        .unwrap();
        fs::write(
            root.join("Cargo.lock"),
            "version = 4\n[[package]]\nname='cg-combined'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn fail() -> Result<(), ()> { Err(()) }\n",
        )
        .unwrap();
        let p = Self(root);
        p.call(&["init", "--apply", "--format=json"]);
        p
    }
    fn call(&self, args: &[&str]) -> Value {
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
    fn diagnostic(&self, rule: &str) -> String {
        json!({"reason":"compiler-message","package_id":"path+file:///fixture#cg-combined@0.1.0",
            "manifest_path":self.0.join("Cargo.toml"),"target":{"kind":["lib"],"src_path":self.0.join("src/lib.rs")},
            "message":{"code":{"code":rule},"level":"warning","message":"untrusted message",
                "spans":[{"file_name":"src/lib.rs","line_start":1,"column_start":1,"byte_start":0,"byte_end":11,"is_primary":true}]}}).to_string()
    }
    fn tool(&self, clippy_body: &str) -> PathBuf {
        let path = self.0.join("cargo");
        fs::write(&path, format!("#!/bin/sh\nprintf '%s\\n' \"$1\" >> calls\ncase \"$1\" in\nrustdoc) printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":true}}';;\nclippy) {clippy_body};;\nesac\n", self.diagnostic("missing_docs"))).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn standalone_comments_keeps_both_native_rules_and_stable_original_tasks() {
    let p = Project::new();
    let tool = p.tool(&format!(
        "printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":true}}'",
        p.diagnostic("clippy::missing_errors_doc")
    ));
    let scan = || {
        p.call(&[
            "comments",
            "rust",
            "--cargo-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ])
    };
    let first = scan();
    assert_eq!(first["report_type"], "rust_comments_feedback", "{first}");
    assert_eq!(first["local_scan_complete"], true, "{first}");
    assert_eq!(
        first["native_results"]["rustdoc"]["findings"][0]["rule_id"],
        "missing_docs"
    );
    assert_eq!(
        first["documentation_findings"][0]["rule_id"],
        "clippy::missing_errors_doc"
    );
    assert_eq!(first["coverage_proven"], false);
    let id = first["documentation_findings"][0]["finding_id"]
        .as_str()
        .unwrap();
    let repeat = scan();
    assert_eq!(repeat["documentation_findings"][0]["finding_id"], id);
    let verified = p.call(&[
        "task",
        "verify",
        id,
        "--cargo-tool",
        tool.to_str().unwrap(),
        "--format=json",
    ]);
    assert_eq!(verified["observation"], "still_present", "{verified}");
    assert_eq!(
        fs::read_to_string(p.0.join("calls"))
            .unwrap()
            .lines()
            .take(4)
            .collect::<Vec<_>>(),
        ["rustdoc", "clippy", "rustdoc", "clippy"]
    );
    assert!(!first.to_string().contains("untrusted message"));
}

#[test]
fn clippy_failure_keeps_rustdoc_findings_without_fake_full_completion() {
    let p = Project::new();
    let tool = p.tool("exit 1");
    let report = p.call(&[
        "comments",
        "rust",
        "--cargo-tool",
        tool.to_str().unwrap(),
        "--format=json",
    ]);
    assert_eq!(
        report["native_results"]["rustdoc"]["local_scan_complete"], true,
        "{report}"
    );
    assert_eq!(
        report["native_results"]["clippy"]["local_scan_complete"],
        false
    );
    assert_eq!(report["local_scan_complete"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(
        report["native_results"]["rustdoc"]["findings"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn unrelated_clippy_rules_are_retained_but_not_documentation_candidates() {
    let p = Project::new();
    let tool = p.tool(&format!(
        "printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":true}}'",
        p.diagnostic("clippy::needless_return")
    ));
    let report = p.call(&[
        "comments",
        "rust",
        "--cargo-tool",
        tool.to_str().unwrap(),
        "--format=json",
    ]);
    assert_eq!(
        report["native_results"]["clippy"]["findings"][0]["rule_id"],
        "clippy::needless_return"
    );
    assert_eq!(report["documentation_findings"], json!([]));
    assert_eq!(
        report["next"]["repair_brief"]["checker_id"], "rust.cargo_rustdoc",
        "{report}"
    );
}

#[test]
fn second_scan_source_change_invalidates_combined_input() {
    let p = Project::new();
    let tool = p.tool("printf 'changed\\n' > src/lib.rs; printf '%s\\n' '{\"reason\":\"build-finished\",\"success\":true}'");
    let report = p.call(&[
        "comments",
        "rust",
        "--cargo-tool",
        tool.to_str().unwrap(),
        "--format=json",
    ]);
    assert_eq!(
        report["native_results"]["rustdoc"]["local_scan_complete"],
        true
    );
    assert_eq!(report["input_stable"], false);
    assert_eq!(report["local_scan_complete"], false);
    if report["next"]["repair_brief"]["kind"] == "blocker" {
        assert_eq!(
            report["next"]["repair_brief"]["action_id"],
            "restore-checker-environment"
        );
    } else {
        assert_ne!(report["next"]["repair_brief"]["disposition"], "actionable");
    }
}

#[test]
fn clean_rescan_keeps_historical_documentation_task_visible_until_verified() {
    let p = Project::new();
    let tool = p.tool(&format!("[ -f fixed ] || printf '%s\\n' '{}'; printf '%s\\n' '{{\"reason\":\"build-finished\",\"success\":true}}'", p.diagnostic("clippy::missing_errors_doc")));
    let body = fs::read_to_string(&tool)
        .unwrap()
        .replace(&format!("rustdoc) printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":true}}'", p.diagnostic("missing_docs")),
            &format!("rustdoc) [ -f fixed ] || printf '%s\\n' '{}'; printf '%s\\n' '{{\"reason\":\"build-finished\",\"success\":true}}'", p.diagnostic("missing_docs")));
    fs::write(&tool, body).unwrap();
    let scan = || {
        p.call(&[
            "comments",
            "rust",
            "--cargo-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ])
    };
    let first = scan();
    let ids = [
        first["documentation_findings"][0]["finding_id"].clone(),
        first["native_results"]["rustdoc"]["findings"][0]["finding_id"].clone(),
    ];
    fs::write(p.0.join("fixed"), "").unwrap();
    let second = scan();
    assert_eq!(second["local_scan_complete"], true, "{second}");
    assert_eq!(second["documentation_findings"], json!([]));
    assert_eq!(second["native_results"]["rustdoc"]["findings"], json!([]));
    assert!(
        ids.contains(&second["next"]["repair_brief"]["task_id"]),
        "{second}"
    );
}

#[test]
fn deadline_is_shared_and_second_native_process_does_not_get_a_fresh_budget() {
    let p = Project::new();
    let tool = p.tool("printf '%s\\n' '{\"reason\":\"build-finished\",\"success\":true}'");
    let source = fs::read_to_string(&tool)
        .unwrap()
        .replace("rustdoc)", "rustdoc) /bin/sleep 1;");
    fs::write(&tool, source).unwrap();
    let report = p.call(&[
        "comments",
        "rust",
        "--cargo-tool",
        tool.to_str().unwrap(),
        "--timeout",
        "100ms",
        "--format=json",
    ]);
    assert_eq!(report["execution_budget"]["timeout_ms"], 100);
    assert_eq!(report["local_scan_complete"], false);
    assert!(
        !fs::read_to_string(p.0.join("calls"))
            .unwrap_or_default()
            .lines()
            .any(|line| line == "clippy")
    );
}

#[test]
#[ignore = "requires explicitly selected installed Cargo with Clippy; no installation"]
fn actual_combined_comments_three_contract_rules_repair_with_original_clippy() {
    let tool = std::env::var("CODEGUARD_TEST_CARGO").expect("select installed Cargo");
    let version = Command::new(&tool)
        .args(["clippy", "--version"])
        .output()
        .unwrap();
    assert!(version.status.success());
    let mut evidence = Vec::new();
    for (rule, section, body, detail) in [
        (
            "missing_errors_doc",
            "Errors",
            "pub fn fail() -> Result<(), &'static str> { Err(\"unavailable\") }",
            "调用总是返回 unavailable 错误。",
        ),
        (
            "missing_panics_doc",
            "Panics",
            "pub fn fail() { panic!(\"unavailable\"); }",
            "调用总是因 unavailable 而 panic。",
        ),
        (
            "missing_safety_doc",
            "Safety",
            "pub unsafe fn fail() {}",
            "本接口没有额外调用前置条件。",
        ),
    ] {
        let p = Project::new();
        let source = |doc: &str| {
            format!(
                "//! 真实原生注释验收样例。\n#![warn(clippy::{rule})]\n/// 演示契约。\n{doc}\n{body}\n"
            )
        };
        fs::write(p.0.join("src/lib.rs"), source("")).unwrap();
        let first = p.call(&["comments", "rust", "--cargo-tool", &tool, "--format=json"]);
        assert_eq!(first["local_scan_complete"], true, "{first}");
        let native_rule = format!("clippy::{rule}");
        let finding = first["documentation_findings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["rule_id"] == native_rule)
            .unwrap();
        let id = finding["finding_id"].as_str().unwrap();
        let present = p.call(&["task", "verify", id, "--cargo-tool", &tool, "--format=json"]);
        assert_eq!(present["observation"], "still_present", "{present}");
        fs::write(
            p.0.join("src/lib.rs"),
            source(&format!("/// # {section}\n/// {detail}")),
        )
        .unwrap();
        let repaired = p.call(&["task", "verify", id, "--cargo-tool", &tool, "--format=json"]);
        assert_eq!(
            repaired["observation"], "candidate_absent_unverified_policy",
            "{repaired}"
        );
        let fact: Value = serde_json::from_slice(
            &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(fact["state"], "open");
        evidence.push(json!({"rule":native_rule,"first":first,"present":present,"repaired":repaired,"state":fact["state"]}));
    }
    if let Ok(path) = std::env::var("CODEGUARD_COMBINED_DOC_EVIDENCE") {
        fs::write(path, serde_json::to_vec_pretty(&json!({"qualification":"not_granted","cli_sha256":format!("{:x}",Sha256::digest(fs::read(env!("CARGO_BIN_EXE_codeguard")).unwrap())),"clippy_version":String::from_utf8(version.stdout).unwrap().trim(),"cases":evidence})).unwrap()).unwrap();
    }
}
