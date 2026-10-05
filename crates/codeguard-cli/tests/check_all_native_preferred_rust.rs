#![cfg(all(feature = "wasm-precheck", unix))]

use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "codeguard-rust-native-preferred-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='coverage-sample'\nversion='0.1.0'\nedition='2021'\n",
        )
        .unwrap();
        fs::write(
            root.join("Cargo.lock"),
            "version=4\n[[package]]\nname='coverage-sample'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "#[cfg(any())] mod excluded;\nmod child;\npub fn answer() -> i32 { child::value() }\n",
        )
        .unwrap();
        fs::write(root.join("src/child.rs"), "pub fn value() -> i32 { 42 }\n").unwrap();
        fs::write(root.join("src/excluded.rs"), "pub fn broken( {\n").unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        Self(root)
    }
    fn artifact(&self, source: &str) -> Value {
        json!({"reason":"compiler-artifact","package_id":"path+file:///sample#coverage-sample@0.1.0","manifest_path":self.0.join("Cargo.toml"),"target":{"kind":["lib"],"src_path":self.0.join(source)},"fresh":false})
    }
    fn tool(&self, stream: &str, exit: i32, mutation: &str) -> PathBuf {
        let tool = self.0.join("cargo-fixture");
        fs::write(&tool, format!("#!/bin/sh\ncase \"$1\" in\nclippy) {mutation}\ncat <<'CG_NATIVE_REPORT'\n{stream}\nCG_NATIVE_REPORT\nexit {exit} ;;\ncheck|rustdoc) printf '%s\\n' '{{\"reason\":\"build-finished\",\"success\":true}}'; exit 0 ;;\nesac\nexit 2\n")).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        tool
    }
    fn check(&self, tool: Option<&std::path::Path>) -> Value {
        let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        command.args(["check", "all"]).arg(&self.0).args([
            "--format=json",
            "--timeout",
            "60s",
            "--jobs",
            "1",
        ]);
        if let Some(tool) = tool {
            command.arg("--cargo-tool").arg(tool);
        } else {
            command.env("PATH", "");
        }
        let output = command
            .env_remove("CODEGUARD_TIMEOUT")
            .env_remove("CODEGUARD_JOBS")
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
    fn init(&self) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init"])
            .arg(&self.0)
            .args(["--apply", "--format=json"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const FINISH: &str = "{\"reason\":\"build-finished\",\"success\":true}";
fn candidate_paths(report: &Value) -> Vec<&str> {
    report["syntax_candidates"]["observations"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|row| row["path"].as_str())
        .collect()
}

#[test]
fn completed_clippy_artifacts_preempt_only_the_matching_target_roots() {
    let fixture = Fixture::new();
    let stream = format!(
        "{}\n{}\n{FINISH}",
        fixture.artifact("src/lib.rs"),
        fixture.artifact("src/main.rs")
    );
    let tool = fixture.tool(&stream, 0, "");
    let report = fixture.check(Some(&tool));
    assert_eq!(
        report["native_results"]["rust_lint"]["local_scan_complete"], true,
        "{report}"
    );
    assert_eq!(
        report["syntax_candidates"]["native_preferred_count"], 2,
        "Clippy 编译的目标入口不能重复启动 WASM: {report}"
    );
    assert_eq!(
        candidate_paths(&report),
        ["src/child.rs", "src/excluded.rs"]
    );
    assert!(
        report["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["path"] == "src/excluded.rs"
                && row["recovery_count"].as_u64().unwrap_or(0) > 0)
    );
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn a_clean_finish_without_artifacts_does_not_cover_all_rust_files() {
    let fixture = Fixture::new();
    let tool = fixture.tool(FINISH, 0, "");
    let report = fixture.check(Some(&tool));
    assert_eq!(report["syntax_candidates"]["native_preferred_count"], 0);
    assert_eq!(candidate_paths(&report).len(), 4);
}

#[test]
fn failed_clippy_with_artifacts_still_runs_candidate_checks() {
    let fixture = Fixture::new();
    let tool = fixture.tool(
        &format!("{}\n{FINISH}", fixture.artifact("src/lib.rs")),
        101,
        "",
    );
    let report = fixture.check(Some(&tool));
    assert_eq!(
        report["native_results"]["rust_lint"]["local_scan_complete"],
        false
    );
    assert_eq!(report["syntax_candidates"]["native_preferred_count"], 0);
    assert_eq!(candidate_paths(&report).len(), 4);
}

#[test]
fn absent_cargo_keeps_rust_candidate_observations() {
    let fixture = Fixture::new();
    let report = fixture.check(None);
    assert_eq!(
        report["native_results"]["rust_lint"]["reason"],
        "cargo_tool_not_selected"
    );
    assert_eq!(report["syntax_candidates"]["native_preferred_count"], 0);
    assert_eq!(candidate_paths(&report).len(), 4);
}

#[test]
fn wrong_manifest_target_or_cached_artifact_never_grants_coverage() {
    let fixture = Fixture::new();
    for variant in ["manifest", "source", "fresh", "kind", "relative_source"] {
        let mut artifact = fixture.artifact("src/lib.rs");
        match variant {
            "manifest" => artifact["manifest_path"] = json!(fixture.0.join("other/Cargo.toml")),
            "source" => artifact["target"]["src_path"] = json!("/outside/lib.rs"),
            "fresh" => artifact["fresh"] = json!(true),
            "kind" => artifact["target"]["kind"] = json!(["unknown"]),
            _ => artifact["target"]["src_path"] = json!("src/lib.rs"),
        }
        let tool = fixture.tool(&format!("{artifact}\n{FINISH}"), 0, "");
        let report = fixture.check(Some(&tool));
        assert_eq!(
            report["syntax_candidates"]["native_preferred_count"], 0,
            "{variant}: {report}"
        );
        assert_eq!(candidate_paths(&report).len(), 4, "{variant}");
    }
}

#[test]
fn duplicated_keys_and_events_after_finish_do_not_skip_wasm() {
    let fixture = Fixture::new();
    let artifact = fixture.artifact("src/lib.rs").to_string();
    for stream in [
        format!(
            "{}\n{FINISH}",
            artifact.replacen("\"fresh\":false", "\"fresh\":true,\"fresh\":false", 1)
        ),
        format!("{FINISH}\n{artifact}"),
        format!("{artifact}\n{FINISH}\n{{\"reason\":\"text-line\",\"message\":\"late\"}}"),
    ] {
        let tool = fixture.tool(&stream, 0, "");
        let report = fixture.check(Some(&tool));
        assert_eq!(
            report["syntax_candidates"]["native_preferred_count"], 0,
            "{report}"
        );
        assert_eq!(candidate_paths(&report).len(), 4);
    }
}

#[test]
fn native_warning_remains_visible_when_its_target_skips_wasm() {
    let fixture = Fixture::new();
    let warning = json!({"reason":"compiler-message","message":{"level":"warning","code":{"code":"clippy::needless_return"},"spans":[{"is_primary":true,"file_name":"src/lib.rs","line_start":3,"column_start":1}]}});
    let tool = fixture.tool(
        &format!("{warning}\n{}\n{FINISH}", fixture.artifact("src/lib.rs")),
        0,
        "",
    );
    let report = fixture.check(Some(&tool));
    assert_eq!(
        report["native_results"]["rust_lint"]["findings"][0]["rule_id"],
        "clippy::needless_return"
    );
    assert_eq!(report["syntax_candidates"]["native_preferred_count"], 1);
    assert!(!candidate_paths(&report).contains(&"src/lib.rs"));
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn changed_configuration_or_tool_cannot_preempt_candidate_parsing() {
    for (mutation, reason) in [
        (
            "printf 'too-many-arguments-threshold = 9\\n' > clippy.toml",
            "rust_inputs_changed_during_scan",
        ),
        (
            "printf '# replaced\\n' >> \"$0\"",
            "cargo_tool_changed_during_scan",
        ),
    ] {
        let fixture = Fixture::new();
        let tool = fixture.tool(
            &format!("{}\n{FINISH}", fixture.artifact("src/lib.rs")),
            0,
            mutation,
        );
        let report = fixture.check(Some(&tool));
        assert_eq!(
            report["native_results"]["rust_lint"]["reason"], reason,
            "{report}"
        );
        assert_eq!(report["syntax_candidates"]["native_preferred_count"], 0);
        if reason == "rust_inputs_changed_during_scan" {
            // 新增配置也改变静态源码范围；聚合保护应阻止按旧范围启动候选。
            assert_eq!(report["syntax_candidates"]["status"], "not_run");
            assert_eq!(
                report["syntax_candidates"]["reason"],
                "source_or_scope_changed"
            );
            assert!(candidate_paths(&report).is_empty());
        } else {
            assert_eq!(candidate_paths(&report).len(), 4);
        }
    }
}

#[test]
fn identical_clippy_diagnostics_across_targets_keep_one_stable_task() {
    let fixture = Fixture::new();
    fixture.init();
    let warning = json!({"reason":"compiler-message","message":{"level":"warning","code":{"code":"clippy::needless_return"},"spans":[{"is_primary":true,"file_name":"src/lib.rs","line_start":3,"column_start":1}]}});
    let tool = fixture.tool(
        &format!(
            "{warning}\n{warning}\n{}\n{FINISH}",
            fixture.artifact("src/lib.rs")
        ),
        0,
        "",
    );
    let first = fixture.check(Some(&tool));
    let repeated = fixture.check(Some(&tool));
    for report in [&first, &repeated] {
        assert_eq!(
            report["native_results"]["rust_lint"]["findings"]
                .as_array()
                .unwrap()
                .len(),
            1,
            "同一个原生位置不能因库/测试目标重复建问题: {report}"
        );
        assert_eq!(report["syntax_candidates"]["native_preferred_count"], 1);
        assert_eq!(
            report["native_results"]["rust_lint"]["backlog_status"],
            "synced_partial"
        );
        assert_eq!(
            report["native_results"]["rust_lint"]["backlog_sync"]["failed_reports"],
            0
        );
    }
    assert_eq!(
        first["native_results"]["rust_lint"]["findings"][0]["finding_id"],
        repeated["native_results"]["rust_lint"]["findings"][0]["finding_id"]
    );
    let clippy_facts = fs::read_dir(fixture.0.join(".codeguard/findings"))
        .unwrap()
        .filter_map(|entry| {
            let path = entry.ok()?.path().join("finding.json");
            let fact: Value = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
            (fact["checker_id"] == "rust.cargo_clippy" && fact["kind"] != "blocker").then_some(fact)
        })
        .count();
    assert_eq!(clippy_facts, 1);
}

#[test]
fn duplicate_native_levels_keep_the_strongest_level_without_merging_positions() {
    let fixture = Fixture::new();
    let warning = json!({"reason":"compiler-message","message":{"level":"warning","code":{"code":"clippy::needless_return"},"spans":[{"is_primary":true,"file_name":"src/lib.rs","line_start":3,"column_start":1}]}});
    let mut error = warning.clone();
    error["message"]["level"] = json!("error");
    let mut different_position = warning.clone();
    different_position["message"]["spans"][0]["column_start"] = json!(5);
    let tool = fixture.tool(&format!("{warning}\n{error}\n{warning}\n{different_position}\n{{\"reason\":\"build-finished\",\"success\":false}}"), 101, "");
    let report = fixture.check(Some(&tool));
    let findings = report["native_results"]["rust_lint"]["findings"]
        .as_array()
        .unwrap();
    assert_eq!(findings.len(), 2, "{report}");
    assert_eq!(findings[0]["level"], "error");
    assert_eq!(findings[1]["column"], 5);
    assert_ne!(findings[0]["finding_id"], findings[1]["finding_id"]);
    assert_eq!(
        report["native_results"]["rust_lint"]["local_scan_complete"],
        false
    );
    assert_eq!(report["syntax_candidates"]["native_preferred_count"], 0);
}

#[test]
#[ignore = "requires an already installed Cargo/Clippy via CODEGUARD_CARGO_BIN"]
fn real_clippy_preempts_lib_and_bin_without_hiding_cfg_excluded_syntax() {
    let fixture = Fixture::new();
    let cargo =
        PathBuf::from(std::env::var("CODEGUARD_CARGO_BIN").expect("explicit existing Cargo"));
    let report = fixture.check(Some(&cargo));
    assert_eq!(
        report["native_results"]["rust_lint"]["local_scan_complete"], true,
        "{report}"
    );
    assert_eq!(
        report["syntax_candidates"]["native_preferred_count"], 2,
        "{report}"
    );
    assert_eq!(
        candidate_paths(&report),
        ["src/child.rs", "src/excluded.rs"]
    );
    assert!(
        report["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["path"] == "src/excluded.rs"
                && row["recovery_count"].as_u64().unwrap_or(0) > 0)
    );
    assert_eq!(report["delivery_decision"], "incomplete");
}
