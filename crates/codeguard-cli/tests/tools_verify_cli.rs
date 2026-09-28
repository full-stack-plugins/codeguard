#![cfg(unix)]
use codeguard_cli::tool_identity::current_platform_id;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cg-tools-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn run(&self, args: &[&str]) -> (i32, Value) {
        self.run_operation("verify", args)
    }
    fn run_operation(&self, operation: &str, args: &[&str]) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["tools", operation])
            .arg(&self.0)
            .args(args)
            .arg("--format=json")
            .output()
            .unwrap();
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    }
    fn lock(&self, kind: &str, reference: &str, bytes: &[u8]) -> Value {
        let hash = format!("{:x}", Sha256::digest(bytes));
        json!({"schema_version":"1.0","lock_id":"test-local","tools":[{"id":"mock","version":"1","binary_sha256":hash,"platform":current_platform_id().unwrap(),"adapter":{"id":"test","version":"1"},"rule_source":{"kind":"native_builtin","id":"test","sha256":hash},"origin":{"kind":kind,"ref":reference}}]})
    }
    fn write_lock(&self, lock: &Value) {
        fs::write(
            self.0.join("codeguard.lock.json"),
            serde_json::to_vec(lock).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn list_reports_declared_inventory_and_artifact_gaps_without_execution() {
    let project = Project::new();
    let bytes = b"#!/bin/sh\ntouch marker\n";
    fs::write(project.0.join("wrapper"), bytes).unwrap();
    fs::set_permissions(project.0.join("wrapper"), fs::Permissions::from_mode(0o700)).unwrap();
    project.write_lock(&project.lock("project_wrapper", "wrapper", bytes));
    let (exit, report) = project.run_operation("list", &[]);
    assert_eq!(exit, 3);
    assert_eq!(report["report_type"], "tool_inventory_observation");
    assert_eq!(report["inventory_scope"], "declared_lock_only");
    assert_eq!(report["required_inventory"], "unverified");
    assert_eq!(report["tools"][0]["declared_version"], "1");
    assert_eq!(report["tools"][0]["required_by_policy"], Value::Null);
    assert_eq!(report["tools"][0]["applicability"], "current_platform");
    assert_eq!(report["tools"][0]["artifact_status"], "matched_untrusted");
    assert_eq!(report["tools"][0]["execution"], "not_run");
    assert!(!project.0.join("marker").exists());
    assert!(!project.0.join("codeguard").exists());
    assert!(!report.to_string().contains(project.0.to_str().unwrap()));
}

#[test]
fn list_retains_foreign_platform_inventory_without_local_missing_claim() {
    let project = Project::new();
    let mut lock = project.lock("project_wrapper", "unavailable", b"tool");
    lock["tools"][0]["platform"] = codeguard_core::CANDIDATE_PLATFORMS
        .iter()
        .find(|platform| Some(**platform) != current_platform_id())
        .unwrap()
        .to_string()
        .into();
    project.write_lock(&lock);
    let row = project.run_operation("list", &[]).1["tools"][0].clone();
    assert_eq!(row["applicability"], "other_platform");
    assert_eq!(row["artifact_status"], "not_inspected");
    assert_eq!(row["tool"], Value::Null);
    assert_eq!(row["required_by_policy"], Value::Null);
}

#[test]
fn list_distinguishes_runtime_declaration_from_runtime_artifact() {
    let project = Project::new();
    let mut lock = project.lock("project_wrapper", "missing", b"tool");
    lock["tools"][0]["runtime"] = json!({"id":"jdk","version":"17","binary_sha256":"a".repeat(64)});
    project.write_lock(&lock);
    let row = project.run_operation("list", &[]).1["tools"][0].clone();
    assert_eq!(row["declared_runtime"]["id"], "jdk");
    assert_eq!(row["declared_runtime"]["version"], "17");
    assert_eq!(row["runtime"]["issue"], "runtime_path_missing");
    assert_eq!(row["artifact_status"], "incomplete");
}

#[test]
fn list_without_a_lock_does_not_claim_empty_inventory_ready() {
    let project = Project::new();
    let report = project.run_operation("list", &[]).1;
    assert_eq!(report["lock_status"], "missing");
    assert_eq!(report["required_inventory"], "unverified");
    assert_eq!(report["readiness"], "unknown");
    assert_eq!(report["authority"], "unverified");
    assert_eq!(report["gate_effect"], "none");
    assert_eq!(report["tools"], json!([]));
}

#[test]
fn list_rejects_self_assigned_policy_authority_in_lock() {
    let project = Project::new();
    let mut lock = project.lock("project_wrapper", "tool", b"tool");
    lock["tools"][0]["required_by_policy"] = true.into();
    project.write_lock(&lock);
    let report = project.run_operation("list", &[]).1;
    assert_eq!(report["lock_status"], "invalid");
    assert_eq!(report["tools"], json!([]));
    assert_eq!(report["required_inventory"], "unverified");
}

#[test]
fn list_order_is_stable_and_rule_source_declaration_remains_untrusted() {
    let project = Project::new();
    let mut lock = project.lock("project_wrapper", "tool", b"tool");
    let mut second = lock["tools"][0].clone();
    lock["tools"][0]["id"] = "z-tool".into();
    second["id"] = "a-tool".into();
    second["rule_source"]["kind"] = "external_rulepack".into();
    lock["tools"].as_array_mut().unwrap().push(second);
    project.write_lock(&lock);
    let report = project.run_operation("list", &[]).1;
    assert_eq!(report["tools"][0]["tool_id"], "a-tool");
    assert_eq!(report["tools"][1]["tool_id"], "z-tool");
    assert_eq!(
        report["tools"][0]["rule_source"]["kind"],
        "external_rulepack"
    );
    assert_eq!(report["authority"], "unverified");
}

#[test]
fn published_cache_bytes_are_verified_without_gaining_policy_or_execution() {
    let project = Project::new();
    let cache = project.0.join("cache");
    fs::create_dir(&cache).unwrap();
    fs::set_permissions(&cache, fs::Permissions::from_mode(0o700)).unwrap();
    let bytes = b"#!/bin/sh\ntouch marker\n";
    let receipt = codeguard_runtime::publish_tool_bytes(
        &cache,
        bytes,
        Sha256::digest(bytes).into(),
        std::time::Instant::now() + std::time::Duration::from_secs(5),
        &std::sync::atomic::AtomicBool::new(false),
    )
    .unwrap();
    project.write_lock(&project.lock("managed_cache", &receipt.relative_path, bytes));
    let report = project.run(&["--managed-cache", cache.to_str().unwrap()]).1;
    assert_eq!(report["tools"][0]["artifact_status"], "matched_untrusted");
    assert_eq!(report["tools"][0]["execution"], "not_run");
    assert_eq!(report["authority"], "unverified");
    assert_eq!(report["readiness"], "unknown");
    assert!(!project.0.join("marker").exists());
}

#[test]
fn install_defaults_to_preview_and_never_writes_from_a_candidate_lock() {
    let project = Project::new();
    project.write_lock(&project.lock("managed_cache", "tool", b"tool"));
    let lock = project.0.join("codeguard.lock.json");
    let (exit, report) = project.run_operation("install", &["--lock", lock.to_str().unwrap()]);
    assert_eq!(exit, 3);
    assert_eq!(report["report_type"], "tool_install_preview");
    assert_eq!(report["mode"], "dry_run");
    assert_eq!(report["writes_performed"], false);
    assert_eq!(report["installation_status"], "plan_incomplete");
    assert_eq!(
        report["tools"][0]["install_action"],
        "bind_approved_distribution_manifest"
    );
    assert!(!project.0.join("codeguard").exists());
}

#[test]
fn install_apply_is_blocked_before_writing_without_trusted_source() {
    let project = Project::new();
    project.write_lock(&project.lock("managed_cache", "tool", b"tool"));
    let lock = project.0.join("codeguard.lock.json");
    let report = project
        .run_operation("install", &["--lock", lock.to_str().unwrap(), "--apply"])
        .1;
    assert_eq!(report["mode"], "apply_requested");
    assert_eq!(report["reason"], "approved_install_source_unbound");
    assert_eq!(report["installation_status"], "blocked_before_mutation");
    assert_eq!(report["writes_performed"], false);
    assert!(!project.0.join("codeguard").exists());
}

#[test]
fn install_preview_explains_wrapper_recovery_without_running_wrapper() {
    let project = Project::new();
    let bytes = b"#!/bin/sh\ntouch marker\n";
    fs::write(project.0.join("wrapper"), bytes).unwrap();
    fs::set_permissions(project.0.join("wrapper"), fs::Permissions::from_mode(0o700)).unwrap();
    project.write_lock(&project.lock("project_wrapper", "wrapper", bytes));
    let lock = project.0.join("codeguard.lock.json");
    let report = project
        .run_operation("install", &["--lock", lock.to_str().unwrap(), "--dry-run"])
        .1;
    assert_eq!(
        report["tools"][0]["install_action"],
        "verify_approved_toolchain_before_doctor"
    );
    assert_eq!(report["tools"][0]["execution"], "not_run");
    assert!(!project.0.join("marker").exists());
}
impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn wrapper_hash_match_is_read_only_and_never_proves_runtime_or_policy() {
    let project = Project::new();
    let bytes = b"#!/bin/sh\ntouch marker\n";
    fs::write(project.0.join("wrapper"), bytes).unwrap();
    fs::set_permissions(project.0.join("wrapper"), fs::Permissions::from_mode(0o700)).unwrap();
    project.write_lock(&project.lock("project_wrapper", "wrapper", bytes));
    let (exit, report) = project.run(&[]);
    assert_eq!(exit, 3);
    assert_eq!(report["report_type"], "tool_artifact_inspection");
    assert_eq!(report["tools"][0]["artifact_status"], "matched_untrusted");
    assert_eq!(report["tools"][0]["execution"], "not_run");
    assert_eq!(report["authority"], "unverified");
    assert_eq!(report["readiness"], "unknown");
    assert_eq!(report["gate_effect"], "none");
    assert!(!project.0.join("marker").exists());
    assert!(!project.0.join("codeguard").exists());
    assert!(!report.to_string().contains(project.0.to_str().unwrap()));
}

#[test]
fn changed_nonexecutable_and_missing_tools_have_distinct_recovery_codes() {
    let project = Project::new();
    project.write_lock(&project.lock("project_wrapper", "wrapper", b"expected"));
    assert_eq!(
        project.run(&[]).1["tools"][0]["tool"]["issue"],
        "tool_missing"
    );
    fs::write(project.0.join("wrapper"), b"changed").unwrap();
    assert_eq!(
        project.run(&[]).1["tools"][0]["tool"]["issue"],
        "artifact_digest_mismatch"
    );
    fs::write(project.0.join("wrapper"), b"expected").unwrap();
    let report = project.run(&[]).1;
    assert_eq!(
        report["tools"][0]["tool"]["issue"],
        "artifact_not_executable"
    );
    assert!(
        !report["tools"][0]["next_action"]
            .as_str()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn missing_invalid_and_symlinked_lock_do_not_create_tool_or_code_findings() {
    let project = Project::new();
    assert_eq!(project.run(&[]).1["lock_status"], "missing");
    fs::write(project.0.join("codeguard.lock.json"), b"broken").unwrap();
    assert_eq!(project.run(&[]).1["lock_status"], "invalid");
    fs::rename(
        project.0.join("codeguard.lock.json"),
        project.0.join("other.json"),
    )
    .unwrap();
    symlink("other.json", project.0.join("codeguard.lock.json")).unwrap();
    let report = project.run(&[]).1;
    assert_eq!(report["lock_status"], "unreadable");
    assert!(report["tools"].as_array().unwrap().is_empty());
    assert!(report.get("findings").is_none());
}

#[test]
fn required_runtime_and_bundle_are_verified_independently() {
    let project = Project::new();
    let tool = project.0.join("tool");
    fs::write(&tool, b"tool").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let mut lock = project.lock("system", tool.to_str().unwrap(), b"tool");
    lock["tools"][0]["runtime"] = json!({"id":"jdk","version":"17","binary_sha256":"a".repeat(64)});
    lock["tools"][0]["bundle"] =
        json!({"root":project.0.join("missing-bundle"),"tree_sha256":"a".repeat(64)});
    project.write_lock(&lock);
    let report = project.run(&[]).1;
    assert_eq!(report["tools"][0]["tool"]["digest_matched"], true);
    assert_eq!(
        report["tools"][0]["runtime"]["issue"],
        "runtime_path_missing"
    );
    assert_eq!(report["tools"][0]["bundle"]["issue"], "bundle_missing");
    assert_eq!(report["tools"][0]["artifact_status"], "incomplete");
}

#[test]
fn managed_cache_and_explicit_runtime_match_without_execution() {
    let project = Project::new();
    fs::create_dir(project.0.join("cache")).unwrap();
    let tool = project.0.join("cache/tool");
    let runtime = project.0.join("runtime");
    for path in [&tool, &runtime] {
        fs::write(path, b"#!/bin/sh\ntouch marker\n").unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let bytes = fs::read(&tool).unwrap();
    let mut lock = project.lock("managed_cache", "tool", &bytes);
    lock["tools"][0]["runtime"] =
        json!({"id":"jdk","version":"17","binary_sha256":format!("{:x}", Sha256::digest(&bytes))});
    project.write_lock(&lock);
    let cache = project.0.join("cache");
    let runtime_argument = format!("jdk={}", runtime.display());
    let report = project
        .run(&[
            "--managed-cache",
            cache.to_str().unwrap(),
            "--runtime",
            &runtime_argument,
        ])
        .1;
    assert_eq!(report["tools"][0]["artifact_status"], "matched_untrusted");
    assert_eq!(report["tools"][0]["runtime"]["digest_matched"], true);
    assert!(!project.0.join("marker").exists());
    assert!(!project.0.join("codeguard").exists());
}

#[test]
fn foreign_platform_lock_has_no_current_platform_artifact_claim() {
    let project = Project::new();
    let mut lock = project.lock("project_wrapper", "tool", b"tool");
    lock["tools"][0]["platform"] = codeguard_core::CANDIDATE_PLATFORMS
        .iter()
        .find(|platform| Some(**platform) != current_platform_id())
        .unwrap()
        .to_string()
        .into();
    project.write_lock(&lock);
    let report = project.run(&[]).1;
    assert!(report["tools"].as_array().unwrap().is_empty());
    assert_eq!(report["readiness"], "unknown");
    assert!(report["next_action"].as_str().unwrap().contains("当前平台"));
}

#[test]
fn installation_and_ambiguous_arguments_are_rejected_before_observation() {
    let project = Project::new();
    for args in [
        vec!["--install"],
        vec!["--format=json", "--format=human"],
        vec!["--managed-cache", "relative"],
        vec!["--runtime", "jdk=relative"],
        vec!["--runtime", "jdk=/a", "--runtime", "jdk=/b"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["tools", "verify"])
            .arg(&project.0)
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
    assert!(!project.0.join("codeguard").exists());
}

#[test]
fn human_feedback_escapes_untrusted_labels_and_keeps_incomplete_semantics() {
    let project = Project::new();
    let mut lock = project.lock("project_wrapper", "missing", b"tool");
    lock["tools"][0]["id"] = "mock\nFORGED_READY\u{1b}".into();
    project.write_lock(&lock);
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["tools", "verify"])
        .arg(&project.0)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("准备 unknown"));
    assert!(text.contains("执行 not_run"));
    assert!(!text.contains("\nFORGED_READY"));
    assert!(!text.contains('\u{1b}'));
}

fn write_distribution(project: &Project) -> PathBuf {
    let lock = fs::read(project.0.join("codeguard.lock.json")).unwrap();
    let parsed: Value = serde_json::from_slice(&lock).unwrap();
    let tool = &parsed["tools"][0];
    let manifest = json!({"schema_version":"1.0","manifest_id":"test","lock_sha256":format!("{:x}",Sha256::digest(&lock)),"artifacts":[{"tool_id":tool["id"],"version":tool["version"],"platform":tool["platform"],"binary_sha256":tool["binary_sha256"],"origin_ref_sha256":format!("{:x}",Sha256::digest(tool["origin"]["ref"].as_str().unwrap().as_bytes())),"download_url":"https://example.org/private/tool","package_sha256":tool["binary_sha256"],"package_size_bytes":4,"format":"raw","unpacked_size_limit_bytes":4}]});
    let path = project.0.join("distribution.json");
    fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    path
}

#[test]
fn install_manifest_binding_is_observed_without_approval_or_mutation() {
    let project = Project::new();
    project.write_lock(&project.lock("managed_cache", "tool", b"tool"));
    let path = write_distribution(&project);
    let lock = project.0.join("codeguard.lock.json");
    let (exit, report) = project.run_operation(
        "install",
        &[
            "--lock",
            lock.to_str().unwrap(),
            "--distribution-manifest",
            path.to_str().unwrap(),
            "--apply",
        ],
    );
    assert_eq!(exit, 3);
    assert_eq!(report["schema_version"], "0.3.0");
    assert_eq!(report["distribution_manifest"]["status"], "bound_untrusted");
    assert_eq!(report["distribution_manifest"]["bound_artifacts"], 1);
    assert_eq!(report["tools"][0]["distribution"]["format"], "raw");
    assert_eq!(
        report["tools"][0]["distribution"]["layout"]["status"],
        "not_applicable_raw"
    );
    assert_eq!(
        report["tools"][0]["distribution"]["layout"]["content_verification"],
        "not_run"
    );
    assert_eq!(report["installation_status"], "blocked_before_mutation");
    assert_eq!(report["authority"], "unverified");
    assert_eq!(report["writes_performed"], false);
    assert!(!report.to_string().contains("https://example.org"));
    assert!(!project.0.join("codeguard").exists());
}

#[test]
fn install_manifest_mismatch_invalid_and_symlink_have_fixed_diagnostics() {
    let project = Project::new();
    project.write_lock(&project.lock("managed_cache", "tool", b"tool"));
    let path = write_distribution(&project);
    let lock = project.0.join("codeguard.lock.json");
    use std::io::Write;
    fs::OpenOptions::new()
        .append(true)
        .open(&lock)
        .unwrap()
        .write_all(b" ")
        .unwrap();
    let args = [
        "--lock",
        lock.to_str().unwrap(),
        "--distribution-manifest",
        path.to_str().unwrap(),
    ];
    let report = project.run_operation("install", &args).1;
    assert_eq!(report["distribution_manifest"]["status"], "unbound");
    assert_eq!(
        report["distribution_manifest"]["reason"],
        "distribution_lock_digest_mismatch"
    );
    assert_eq!(report["tools"][0]["distribution"], Value::Null);
    fs::write(&path, b"{\"secret\":\"DO_NOT_ECHO\"}").unwrap();
    let report = project.run_operation("install", &args).1;
    assert_eq!(report["distribution_manifest"]["status"], "invalid");
    assert!(!report.to_string().contains("DO_NOT_ECHO"));
    fs::remove_file(&path).unwrap();
    symlink(&lock, &path).unwrap();
    assert_eq!(
        project.run_operation("install", &args).1["distribution_manifest"]["status"],
        "unreadable"
    );
}

#[test]
fn distribution_flag_is_install_only_and_unique() {
    let project = Project::new();
    for (operation, args) in [
        ("verify", vec!["--distribution-manifest", "/a"]),
        ("list", vec!["--distribution-manifest", "/a"]),
        (
            "install",
            vec![
                "--lock",
                "/a",
                "--distribution-manifest",
                "/a",
                "--distribution-manifest",
                "/b",
            ],
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["tools", operation])
            .arg(&project.0)
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn manifest_subset_does_not_claim_other_tools_or_runtime_coverage() {
    let project = Project::new();
    let mut lock = project.lock("managed_cache", "tool", b"tool");
    let mut second = lock["tools"][0].clone();
    second["id"] = "second".into();
    second["origin"]["ref"] = "second".into();
    lock["tools"].as_array_mut().unwrap().push(second);
    lock["tools"][0]["runtime"] = json!({"id":"jdk","version":"17","binary_sha256":"a".repeat(64)});
    project.write_lock(&lock);
    let path = write_distribution(&project);
    let lock_path = project.0.join("codeguard.lock.json");
    let report = project
        .run_operation(
            "install",
            &[
                "--lock",
                lock_path.to_str().unwrap(),
                "--distribution-manifest",
                path.to_str().unwrap(),
            ],
        )
        .1;
    assert_eq!(report["distribution_manifest"]["bound_artifacts"], 1);
    assert_eq!(report["tools"][1]["distribution"], Value::Null);
    assert_eq!(
        report["tools"][0]["runtime"]["issue"],
        "runtime_path_missing"
    );
    assert_eq!(report["required_inventory"], "unverified");
    assert!(
        report["missing_inputs"]
            .as_array()
            .unwrap()
            .contains(&json!("required_inventory_runtime_coverage"))
    );
}

#[test]
fn fifo_lock_and_manifest_return_incomplete_without_hanging_install() {
    use std::time::{Duration, Instant};
    let project = Project::new();
    project.write_lock(&project.lock("managed_cache", "tool", b"tool"));
    let lock = project.0.join("codeguard.lock.json");
    let fifo = project.0.join("fifo.json");
    assert!(
        Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    for manifest_case in [false, true] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        cmd.args(["tools", "install"])
            .arg(&project.0)
            .arg("--lock")
            .arg(if manifest_case { &lock } else { &fifo })
            .arg("--format=json");
        if manifest_case {
            cmd.arg("--distribution-manifest").arg(&fifo);
        }
        let mut child = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("install FIFO read blocked; child reaped");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(3));
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        if manifest_case {
            assert_eq!(report["distribution_manifest"]["status"], "unreadable");
        } else {
            assert_eq!(report["lock_status"], "unreadable");
        }
        assert_eq!(report["writes_performed"], false);
        assert_eq!(report["readiness"], "unknown");
    }
    assert!(!project.0.join("codeguard").exists());
}

#[test]
fn install_preview_explains_full_layout_without_reading_packages_or_writing() {
    let project = Project::new();
    let tree = "e".repeat(64);
    let private_entry = "release/private-name/bin/tool";
    let private_bundle = "release/private-name/libexec";
    let reference = format!("{tree}.bundle/{private_entry}");
    let mut lock = project.lock("managed_cache", &reference, b"tool");
    lock["tools"][0]["bundle"] =
        json!({"root":format!("{tree}.bundle/{private_bundle}"),"tree_sha256":"d".repeat(64)});
    project.write_lock(&lock);
    let path = write_distribution(&project);
    let mut manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    manifest["schema_version"] = "1.2".into();
    manifest["artifacts"][0]["format"] = "zip".into();
    manifest["artifacts"][0]["entrypoint"] = private_entry.into();
    manifest["artifacts"][0]["bundle_tree_sha256"] = "d".repeat(64).into();
    manifest["artifacts"][0]["bundle_archive_root"] = private_bundle.into();
    manifest["artifacts"][0]["install_tree_sha256"] = tree.clone().into();
    fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let lock_path = project.0.join("codeguard.lock.json");
    let args = [
        "--lock",
        lock_path.to_str().unwrap(),
        "--distribution-manifest",
        path.to_str().unwrap(),
    ];
    for apply in [false, true] {
        let mut options = args.to_vec();
        if apply {
            options.push("--apply");
        }
        let (exit, report) = project.run_operation("install", &options);
        assert_eq!(exit, 3);
        assert_eq!(report["schema_version"], "0.3.0");
        let layout = &report["tools"][0]["distribution"]["layout"];
        assert_eq!(layout["status"], "declared_paths_bound_untrusted");
        assert_eq!(layout["cache_directory"], format!("{tree}.bundle"));
        assert_eq!(layout["content_verification"], "not_run");
        assert_eq!(
            layout["entrypoint_ref_sha256"],
            format!("{:x}", Sha256::digest(reference.as_bytes()))
        );
        assert_eq!(
            layout["bundle_root_ref_sha256"],
            format!(
                "{:x}",
                Sha256::digest(format!("{tree}.bundle/{private_bundle}").as_bytes())
            )
        );
        assert_eq!(report["writes_performed"], false);
        assert_eq!(report["readiness"], "unknown");
        assert!(!report.to_string().contains("private-name"));
        assert!(!project.0.join("codeguard").exists());
    }
    let human = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["tools", "install"])
        .arg(&project.0)
        .args(args)
        .output()
        .unwrap();
    assert_eq!(human.status.code(), Some(3));
    let text = String::from_utf8(human.stdout).unwrap();
    assert!(
        text.contains("布局阶段")
            && text.contains("declared_paths_bound_untrusted")
            && text.contains("包内容核验 not_run")
    );
    assert!(!text.contains("private-name"));
    manifest["schema_version"] = "1.1".into();
    manifest["artifacts"][0]
        .as_object_mut()
        .unwrap()
        .remove("install_tree_sha256");
    fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let report = project.run_operation("install", &args).1;
    assert_eq!(
        report["tools"][0]["distribution"]["layout"]["status"],
        "missing_full_tree_declaration"
    );
    assert_eq!(
        report["tools"][0]["distribution"]["layout"]["cache_directory"],
        Value::Null
    );
}

#[test]
fn raw_versioned_locator_is_bound_before_preview_advances_to_content_verification() {
    let project = Project::new();
    let bytes = b"tool";
    let reference = format!("{:x}.bin", Sha256::digest(bytes));
    project.write_lock(&project.lock("managed_cache", &reference, bytes));
    let path = write_distribution(&project);
    let mut m: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    m["schema_version"] = "1.2".into();
    fs::write(&path, serde_json::to_vec(&m).unwrap()).unwrap();
    let lock_path = project.0.join("codeguard.lock.json");
    let args = [
        "--lock",
        lock_path.to_str().unwrap(),
        "--distribution-manifest",
        path.to_str().unwrap(),
    ];
    let (exit, report) = project.run_operation("install", &args);
    assert_eq!(exit, 3);
    assert_eq!(
        report["tools"][0]["distribution"]["layout"]["status"],
        "not_applicable_raw"
    );
    assert_eq!(
        report["tools"][0]["distribution"]["layout"]["next_action"],
        "verify_source_then_raw_package"
    );
    assert_eq!(
        report["tools"][0]["distribution"]["layout"]["content_verification"],
        "not_run"
    );
    project.write_lock(&project.lock("managed_cache", "wrong/location", bytes));
    let lock = fs::read(&lock_path).unwrap();
    m["lock_sha256"] = format!("{:x}", Sha256::digest(&lock)).into();
    m["artifacts"][0]["origin_ref_sha256"] =
        format!("{:x}", Sha256::digest(b"wrong/location")).into();
    fs::write(&path, serde_json::to_vec(&m).unwrap()).unwrap();
    let report = project.run_operation("install", &args).1;
    assert_eq!(
        report["distribution_manifest"]["reason"],
        "distribution_raw_locator_mismatch"
    );
    assert_eq!(report["tools"][0]["distribution"], Value::Null);
    assert!(!project.0.join("codeguard").exists());
}
