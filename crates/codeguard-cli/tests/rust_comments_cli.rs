#![cfg(unix)]
use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-comments-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='cg-comments-cli'\nversion='0.1.0'\nedition='2021'\n",
        )
        .unwrap();
        fs::write(
            root.join("Cargo.lock"),
            "version = 4\n[[package]]\nname = \"cg-comments-cli\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn answer() -> i32 { 42 }\n").unwrap();
        Self(root)
    }
    fn tool(&self, body: &str) -> PathBuf {
        let path = self.0.join("fake-cargo");
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }
    fn check(&self, extra: &[&str]) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "comments",
                "rust",
                self.0.to_str().unwrap(),
                "--format=json",
            ])
            .args(extra)
            .env(
                "PATH",
                if extra.contains(&"--cargo-tool") {
                    std::env::var_os("PATH").unwrap_or_default()
                } else {
                    std::ffi::OsString::new()
                },
            )
            .env_remove("CODEGUARD_TIMEOUT")
            .output()
            .unwrap();
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    }
    fn warning(&self) -> String {
        serde_json::json!({"reason":"compiler-message","package_id":"path+file:///fixture#cg-comments-cli@0.1.0",
            "manifest_path":self.0.join("Cargo.toml"),"target":{"kind":["lib"],"src_path":self.0.join("src/lib.rs")},
            "message":{"code":{"code":"missing_docs"},"level":"warning","message":"must not enter conversation",
                "spans":[{"file_name":"src/lib.rs","line_start":1,"column_start":1,"byte_start":0,"byte_end":22,"is_primary":true}]}}).to_string()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn check_all_schedules_documentation_and_lint_with_independent_results() {
    let fixture = Fixture::new();
    Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            fixture.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    let tool=fixture.tool(&format!("case \"$1\" in rustdoc) printf '%s\\n' '{}' ;; esac\nprintf '%s\\n' '{{\"reason\":\"build-finished\",\"success\":true}}'",fixture.warning()));
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            fixture.0.to_str().unwrap(),
            "--cargo-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["native_results"]["rust_comments"]["local_scan_complete"], true,
        "{report}"
    );
    assert_eq!(
        report["native_results"]["rust_comments"]["backlog_status"],
        "synced"
    );
    assert_eq!(
        report["native_results"]["rust_comments"]["findings"][0]["rule_id"],
        "missing_docs"
    );
    assert_eq!(report["execution_budget"]["native_task_count"], 4);
    assert_eq!(report["delivery_decision"], "incomplete");
    let row = report["category_candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["language"] == "rust" && r["category"] == "comments")
        .unwrap();
    assert_eq!(row["status"], "observed_unverified");
}

#[test]
fn queued_recheck_rejects_forged_forced_identity_and_duplicate_input_ownership() {
    let fixture = Fixture::new();
    Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            fixture.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    let tool = fixture.tool(&format!(
        "printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":true}}'",
        fixture.warning()
    ));
    let (_, first) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    let id = first["findings"][0]["finding_id"].as_str().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            fixture.0.to_str().unwrap(),
            "--cargo-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    let verified: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(verified["event_persisted"], true, "{verified}");
    for (index, duplicate) in [(0, false), (1, true), (2, false)] {
        let mut report = verified["native_scan"].clone();
        let run_id = format!("rustdoc-99-{}", 200 + index);
        report["run_id"] = serde_json::json!(run_id);
        report["normal_scan"]["run_id"] = serde_json::json!(run_id);
        if index == 2 {
            report["input_stable"] = serde_json::json!(false);
        } else if duplicate {
            let row = report["input_identities"][0].clone();
            report["input_identities"].as_array_mut().unwrap().push(row);
        } else {
            report["forced_scan"]["findings"][0]["finding_fingerprint"] =
                serde_json::json!("a".repeat(64));
            report["forced_scan"]["findings"][0]["repair_brief"]["evidence"]["finding_fingerprint"] =
                serde_json::json!("a".repeat(64));
        }
        fs::write(
            fixture.0.join(format!(".codeguard/reports/{run_id}.json")),
            report.to_string(),
        )
        .unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["work", "sync", fixture.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["failed_reports"], 2, "{result}");
    assert_eq!(result["new_findings"], 0, "{result}");
    assert_eq!(result["new_blockers"], 1, "{result}");
}

#[test]
fn rustdoc_task_verify_records_original_checker_and_native_suppression_without_closing() {
    let fixture = Fixture::new();
    Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            fixture.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    let tool = fixture.tool(&format!(
        "printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":true}}'",
        fixture.warning()
    ));
    let (_, first) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    let id = first["findings"][0]["finding_id"].as_str().unwrap();
    let verify = || {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "task",
                "verify",
                id,
                fixture.0.to_str().unwrap(),
                "--cargo-tool",
                tool.to_str().unwrap(),
                "--format=json",
            ])
            .output()
            .unwrap();
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    let present = verify();
    assert_eq!(present["observation"], "still_present", "{present}");
    assert_eq!(present["event_persisted"], true, "{present}");
    fixture.tool(&format!("case \"$*\" in *--force-warn*) printf '%s\\n' '{}' ;; esac\nprintf '%s\\n' '{{\"reason\":\"build-finished\",\"success\":true}}'",fixture.warning()));
    let suppressed = verify();
    assert_eq!(
        suppressed["observation"], "suppression_requires_review",
        "{suppressed}"
    );
    assert_eq!(suppressed["event_persisted"], true, "{suppressed}");
    fixture.tool("printf '%s\\n' '{\"reason\":\"build-finished\",\"success\":true}'");
    let absent = verify();
    assert_eq!(
        absent["observation"], "candidate_absent_unverified_policy",
        "{absent}"
    );
    let fact: Value = serde_json::from_slice(
        &fs::read(
            fixture
                .0
                .join(format!(".codeguard/findings/{id}/finding.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}

#[test]
fn initialized_comments_automatically_persist_one_task_across_repeated_scans() {
    let fixture = Fixture::new();
    Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            fixture.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert!(fixture.0.join(".codeguard/workspace.json").is_file());
    let tool = fixture.tool(&format!(
        "printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":true}}'",
        fixture.warning()
    ));
    let (_, first) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    assert_eq!(first["backlog_status"], "synced");
    let id = first["findings"][0]["finding_id"].as_str().unwrap();
    let task = fixture.0.join(format!(".codeguard/tasks/{id}.md"));
    assert!(task.is_file());
    let (_, second) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    assert_eq!(second["backlog_status"], "synced");
    assert_eq!(
        first["findings"][0]["finding_id"],
        second["findings"][0]["finding_id"]
    );
    assert_eq!(second["backlog_sync"]["new_findings"], 0);
    assert!(fs::read_to_string(task).unwrap().contains("rustdoc"));
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", fixture.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(next.to_string().contains("rust.cargo_rustdoc"), "{next}");
    assert!(next.to_string().contains("verify"), "{next}");
}

#[test]
fn queued_report_with_changed_manifest_never_creates_a_source_repair_task() {
    let fixture = Fixture::new();
    Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            fixture.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    let tool = fixture.tool(&format!(
        "printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":true}}'",
        fixture.warning()
    ));
    // 取得未绑定观察再绑定到队列，确保导入时重新核验而非依赖命令的先前检查。
    let workspace: Value =
        serde_json::from_slice(&fs::read(fixture.0.join(".codeguard/workspace.json")).unwrap())
            .unwrap();
    let (_, mut report) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    report["run_id"] = serde_json::json!("rustdoc-99-100");
    report["workspace_id"] = workspace["workspace_id"].clone();
    report["backlog_status"] = serde_json::json!("queued");
    report["backlog_sync"] = Value::Null;
    fs::write(fixture.0.join("Cargo.toml"), "changed manifest\n").unwrap();
    fs::write(
        fixture.0.join(".codeguard/reports/rustdoc-99-100.json"),
        report.to_string(),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["work", "sync", fixture.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["new_findings"], 0, "{result}");
    assert_eq!(result["new_blockers"], 1, "{result}");
    assert_eq!(result["historical_findings"], 1, "{result}");
}

#[test]
fn queued_report_with_forged_fingerprint_is_rejected() {
    let fixture = Fixture::new();
    Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            fixture.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    let tool = fixture.tool(&format!(
        "printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":true}}'",
        fixture.warning()
    ));
    let (_, mut report) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    report["run_id"] = serde_json::json!("rustdoc-99-101");
    report["backlog_status"] = serde_json::json!("queued");
    report["backlog_sync"] = Value::Null;
    report["findings"][0]["finding_fingerprint"] = serde_json::json!("a".repeat(64));
    report["findings"][0]["repair_brief"]["evidence"]["finding_fingerprint"] =
        serde_json::json!("a".repeat(64));
    fs::write(
        fixture.0.join(".codeguard/reports/rustdoc-99-101.json"),
        report.to_string(),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["work", "sync", fixture.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["failed_reports"], 1, "{result}");
    assert_eq!(result["new_findings"], 0, "{result}");
}

#[test]
fn initialized_missing_tool_creates_preparation_task_without_source_violation() {
    let fixture = Fixture::new();
    Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            fixture.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    let (_, report) = fixture.check(&[]);
    assert_eq!(report["backlog_status"], "synced");
    assert_eq!(report["backlog_sync"]["new_findings"], 0);
    assert_eq!(report["backlog_sync"]["new_blockers"], 1);
    let tasks: Vec<_> = fs::read_dir(fixture.0.join(".codeguard/tasks"))
        .unwrap()
        .flatten()
        .collect();
    assert_eq!(tasks.len(), 1);
    let text = fs::read_to_string(tasks[0].path()).unwrap();
    assert!(text.contains("rustdoc"));
    assert!(text.contains("不是源码违规"));
}

#[test]
fn observed_comments_never_grant_delivery_or_modify_inputs() {
    let fixture = Fixture::new();
    let tool = fixture.tool(&format!(
        "printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":true}}'",
        fixture.warning()
    ));
    let (exit, report) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    assert_eq!(exit, 3);
    assert_eq!(report["report_type"], "rustdoc_local_observation");
    assert_eq!(report["local_scan_complete"], true);
    assert_eq!(report["coverage_proven"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["findings"][0]["rule_id"], "missing_docs");
    let brief = &report["findings"][0]["repair_brief"];
    assert_eq!(brief["status"], "repair_guidance");
    assert_eq!(brief["allowed_paths"], serde_json::json!(["src/lib.rs"]));
    assert_eq!(
        brief["evidence"]["source_sha256"],
        report["findings"][0]["source_sha256"]
    );
    assert_eq!(brief["recheck_argv"], report["recheck_argv"]);
    assert_eq!(brief["attempt_history"], serde_json::json!([]));
    assert!(brief["closure_conditions"].to_string().contains("原工具"));
    assert!(!report.to_string().contains("must not enter conversation"));
    assert!(!fixture.0.join("target").exists());
    assert!(!fixture.0.join(".codeguard").exists());
    assert_eq!(
        fs::read_to_string(fixture.0.join("src/lib.rs")).unwrap(),
        "pub fn answer() -> i32 { 42 }\n"
    );
}

#[test]
fn changed_inputs_keep_evidence_but_request_rescan_before_source_repair() {
    let fixture = Fixture::new();
    let tool = fixture.tool(&format!(
        "printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":true}}'\nprintf 'changed\\n' > src/lib.rs",
        fixture.warning()
    ));
    let (_, report) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    assert_eq!(report["reason"], "inputs_changed_during_scan");
    let brief = &report["findings"][0]["repair_brief"];
    assert_eq!(brief["status"], "investigation_required");
    assert_eq!(brief["allowed_paths"], serde_json::json!([]));
    assert!(brief["steps"].to_string().contains("重新检查"));
}
#[test]
fn missing_tool_lock_and_malformed_report_have_specific_incomplete_reasons() {
    let fixture = Fixture::new();
    assert_eq!(fixture.check(&[]).1["reason"], "cargo_tool_not_selected");
    let tool = fixture.tool("printf 'not-json\\n'");
    assert_eq!(
        fixture.check(&["--cargo-tool", tool.to_str().unwrap()]).1["reason"],
        "native_report_malformed"
    );
    fs::remove_file(fixture.0.join("Cargo.lock")).unwrap();
    assert_eq!(
        fixture.check(&["--cargo-tool", tool.to_str().unwrap()]).1["reason"],
        "locked_cargo_inputs_unavailable"
    );
}
#[test]
fn target_mismatch_and_input_change_cannot_be_complete_observations() {
    let fixture = Fixture::new();
    let bad = fixture.warning().replace("src/lib.rs", "src/other.rs");
    let tool = fixture.tool(&format!(
        "printf '%s\\n' '{bad}' '{{\"reason\":\"build-finished\",\"success\":true}}'"
    ));
    assert_eq!(
        fixture.check(&["--cargo-tool", tool.to_str().unwrap()]).1["reason"],
        "native_target_outside_observed_scope"
    );
    let tool=fixture.tool("printf 'changed\\n' > src/lib.rs\nprintf '%s\\n' '{\"reason\":\"build-finished\",\"success\":true}'");
    assert_eq!(
        fixture.check(&["--cargo-tool", tool.to_str().unwrap()]).1["reason"],
        "inputs_changed_during_scan"
    );
}
#[test]
fn invalid_arguments_stop_before_tool_execution() {
    let fixture = Fixture::new();
    let marker = fixture.0.join("unexpected-native-execution");
    let tool = fixture.tool(&format!(": > '{}'", marker.display()));
    fs::copy(tool, fixture.0.join("cargo")).unwrap();
    for args in [
        vec!["comments", "rust", "--cargo-tool", "relative"],
        vec!["comments", "unknown-language"],
        vec!["comments", "rust", "--format=sarif"],
        vec!["comments", "rust", "--timeout", "0s"],
    ] {
        assert_eq!(
            Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args(args)
                .current_dir(&fixture.0)
                .env("PATH", &fixture.0)
                .output()
                .unwrap()
                .status
                .code(),
            Some(2)
        );
        assert!(
            !marker.exists(),
            "invalid arguments must not execute native tools"
        );
    }
}

#[test]
fn supported_java_comments_without_java_sources_is_incomplete_not_usage_error() {
    let fixture = Fixture::new();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["comments", "java", "--format=json"])
        .current_dir(&fixture.0)
        .env("PATH", &fixture.0)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["report_type"], "java_comments_feedback");
    assert_eq!(report["language"], "java");
    assert_eq!(report["command_status"], "incomplete");
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
fn cancellation_keeps_exit_130_even_when_inputs_changed() {
    let fixture = Fixture::new();
    let tool = fixture.tool("printf 'changed\\n' > src/lib.rs\nkill -INT \"$PPID\"\nsleep 1");
    let (exit, report) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    assert_eq!(exit, 130);
    assert_eq!(report["command_status"], "cancelled");
    assert_eq!(report["reason"], "request_cancelled");
    assert_eq!(report["local_scan_complete"], false);
}

#[test]
fn identical_declarations_are_ambiguous_instead_of_borrowing_ordinal_task_ids() {
    let fixture = Fixture::new();
    let source = "pub mod a { pub fn same() {} }\npub mod b { pub fn same() {} }\n";
    fs::write(fixture.0.join("src/lib.rs"), source).unwrap();
    let mut first: Value = serde_json::from_str(&fixture.warning()).unwrap();
    let mut second = first.clone();
    for (row, start) in [
        (&mut first, source.find("pub fn same()").unwrap()),
        (&mut second, source.rfind("pub fn same()").unwrap()),
    ] {
        row["message"]["spans"][0]["byte_start"] = serde_json::json!(start);
        row["message"]["spans"][0]["byte_end"] = serde_json::json!(start + 13);
        let prefix = &source[..start];
        row["message"]["spans"][0]["line_start"] =
            serde_json::json!(prefix.bytes().filter(|byte| *byte == b'\n').count() + 1);
        row["message"]["spans"][0]["column_start"] =
            serde_json::json!(prefix.rsplit('\n').next().unwrap().chars().count() + 1);
    }
    let tool = fixture.tool(&format!(
        "printf '%s\\n' '{first}' '{second}' '{{\"reason\":\"build-finished\",\"success\":true}}'"
    ));
    let (_, report) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    assert_eq!(report["reason"], "native_finding_identity_ambiguous");
    assert_eq!(report["local_scan_complete"], false);
    assert_eq!(report["findings"].as_array().unwrap().len(), 2);
    for finding in report["findings"].as_array().unwrap() {
        assert_eq!(finding["identity_status"], "ambiguous");
        assert!(finding["finding_id"].is_null());
        assert_eq!(finding["repair_brief"]["status"], "investigation_required");
        assert_eq!(
            finding["repair_brief"]["allowed_paths"],
            serde_json::json!([])
        );
    }
}
#[test]
#[ignore = "requires explicit existing CODEGUARD_CARGO_BIN; no installation"]
fn actual_cli_rustdoc_finds_comments_and_rechecks_clean_source_without_claiming_repair() {
    let fixture = Fixture::new();
    Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            fixture.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    let tool = std::env::var("CODEGUARD_CARGO_BIN").unwrap();
    let (exit, report) = fixture.check(&["--cargo-tool", &tool]);
    assert_eq!(exit, 3);
    assert_eq!(report["local_scan_complete"], true);
    assert!(!report["findings"].as_array().unwrap().is_empty());
    let task_id = report["findings"][0]["finding_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let verify = || {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "task",
                "verify",
                &task_id,
                fixture.0.to_str().unwrap(),
                "--cargo-tool",
                &tool,
                "--format=json",
            ])
            .output()
            .unwrap();
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    assert_eq!(verify()["observation"], "still_present");
    assert_eq!(report["backlog_status"], "synced");
    for finding in report["findings"].as_array().unwrap() {
        assert!(
            fixture
                .0
                .join(format!(
                    ".codeguard/tasks/{}.md",
                    finding["finding_id"].as_str().unwrap()
                ))
                .is_file()
        );
    }
    fs::write(
        fixture.0.join("src/lib.rs"),
        "//! Sample crate.\n/// Computes an answer.\npub fn answer() -> i32 { 42 }\n",
    )
    .unwrap();
    let (exit, report) = fixture.check(&["--cargo-tool", &tool]);
    assert_eq!(exit, 3);
    assert_eq!(report["local_scan_complete"], true);
    assert!(report["findings"].as_array().unwrap().is_empty());
    assert_eq!(report["delivery_decision"], "not_evaluated");
    let repaired = verify();
    assert_eq!(
        repaired["observation"], "candidate_absent_unverified_policy",
        "{repaired}"
    );
    assert_eq!(repaired["event_persisted"], true, "{repaired}");
    fs::write(
        fixture.0.join("src/lib.rs"),
        "#![allow(missing_docs)]\npub fn answer() -> i32 { 42 }\n",
    )
    .unwrap();
    let suppressed = verify();
    assert_eq!(
        suppressed["observation"], "suppression_requires_review",
        "{suppressed}"
    );
    fs::write(
        fixture.0.join("src/lib.rs"),
        "//! 样例文档。\n/// 中文链接 [NoSuchType]。\npub fn answer() -> i32 { 42 }\n",
    )
    .unwrap();
    let (_, unicode) = fixture.check(&["--cargo-tool", &tool]);
    assert_eq!(unicode["local_scan_complete"], true);
    assert_eq!(
        unicode["findings"][0]["rule_id"],
        "rustdoc::broken_intra_doc_links"
    );
    assert!(
        unicode["findings"][0]["repair_brief"]["rule_basis"]
            .to_string()
            .contains("可解析性")
    );
    assert_eq!(
        unicode["findings"][0]["repair_brief"]["status"],
        "repair_guidance"
    );
}

#[test]
fn cargo_proxy_preserves_selected_entrypoint_and_rejects_retargeting() {
    for retarget in [false, true] {
        let fixture = Fixture::new();
        let tools = Fixture::new();
        let mutation = if retarget {
            "ln -sf alternate-proxy \"$0\""
        } else {
            ""
        };
        let dispatcher = tools.tool(&format!(
            "case \"$0\" in */cargo-entry) ;; *) exit 91 ;; esac\n{mutation}\nprintf '%s\\n' '{{\"reason\":\"build-finished\",\"success\":true}}'"
        ));
        let alternate = tools.0.join("alternate-proxy");
        fs::copy(&dispatcher, &alternate).unwrap();
        let entry = tools.0.join("cargo-entry");
        std::os::unix::fs::symlink(&dispatcher, &entry).unwrap();
        let (exit, report) = fixture.check(&["--cargo-tool", entry.to_str().unwrap()]);
        assert_eq!(exit, 3);
        assert_eq!(report["local_scan_complete"], !retarget, "{report}");
        assert_eq!(
            report["reason"],
            if retarget {
                "tool_changed_during_scan"
            } else {
                "native_observed_unverified"
            }
        );
    }
}
