#![cfg(unix)]

use serde_json::Value;
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Project(std::path::PathBuf);
impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "cg-language-selection-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn check(&self, language: &str, extra: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["check", language, self.0.to_str().unwrap(), "--format=json"])
            .args(extra)
            .env("PATH", "")
            .env_remove("CODEGUARD_TIMEOUT")
            .env_remove("CODEGUARD_JOBS")
            .output()
            .unwrap()
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn every_registered_language_has_a_scoped_check_entry_without_claiming_allow() {
    let p = Project::new();
    for language in codeguard_adapters::legacy_registry().unwrap().languages {
        let output = p.check(&language.id, &[]);
        assert_eq!(
            output.status.code(),
            Some(3),
            "{}: {}",
            language.id,
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["selection"], language.id);
        assert_eq!(report["delivery_decision"], "not_evaluated");
        assert!(report["execution_tasks"].as_array().unwrap().is_empty());
        assert!(report["category_candidates"].as_array().unwrap().is_empty());
    }
}

#[test]
fn python_selection_does_not_schedule_other_languages_in_a_mixed_project() {
    let p = Project::new();
    fs::create_dir(p.0.join("src")).unwrap();
    fs::write(p.0.join("main.py"), "import os\n").unwrap();
    fs::write(p.0.join("src/main.rs"), "fn main() {}\n").unwrap();
    fs::write(p.0.join("Main.java"), "class Main {}\n").unwrap();
    fs::write(
        p.0.join("Cargo.toml"),
        "[package]\nname='test'\nversion='0.1.0'\nedition='2021'\n",
    )
    .unwrap();
    let output = p.check("python", &[]);
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["selection"], "python");
    assert_eq!(report["schema_version"], "0.45.0");
    assert!(
        report["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["language"] == "python")
    );
    assert!(
        report["execution_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["id"].as_str().unwrap().starts_with("python."))
    );
    assert!(report["native_results"]["rust_lint"].is_null());
    assert!(report["native_results"]["java_p3c"].is_null());
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
fn unknown_language_and_other_language_options_fail_before_writes() {
    let p = Project::new();
    for (language, extra) in [
        ("not-a-language", vec![]),
        ("python", vec!["--cargo-tool", "/bin/false"]),
    ] {
        assert_eq!(p.check(language, &extra).status.code(), Some(2));
        assert_eq!(fs::read_dir(&p.0).unwrap().count(), 0);
    }
}

#[cfg(feature = "wasm-precheck")]
#[test]
fn selected_wasm_fallback_does_not_parse_other_languages() {
    let p = Project::new();
    fs::write(p.0.join("main.c"), "int main(void) { return 0; }\n").unwrap();
    fs::write(p.0.join("main.py"), "def broken(:\n").unwrap();
    let output = p.check("c", &[]);
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["syntax_candidates"]["source_file_count"], 1);
    let rows = report["syntax_candidates"]["observations"]
        .as_array()
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["path"], "main.c");
    assert_eq!(rows[0]["language"], "c");
    assert!(report["native_results"]["python_lint"].is_null());
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
#[ignore = "requires explicitly selected installed Ruff; no tool installation"]
fn real_ruff_scoped_check_retains_native_finding() {
    let p = Project::new();
    fs::write(p.0.join("main.py"), "import os\n").unwrap();
    fs::write(p.0.join("Main.java"), "class Main {}\n").unwrap();
    fs::write(p.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let output = p.check("python", &["--ruff-tool", &tool, "--timeout", "60s"]);
    assert_eq!(
        output.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let rows = report["native_results"]["python_lint"]["files"]
        .as_array()
        .unwrap();
    assert!(rows.iter().any(|file| {
        file["findings"]
            .as_array()
            .is_some_and(|findings| findings.iter().any(|finding| finding["rule_id"] == "F401"))
    }));
    assert!(
        report["execution_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|task| task["id"].as_str().unwrap().starts_with("python."))
    );
    assert!(report["native_results"]["java_p3c"].is_null());
    if let Ok(path) = std::env::var("CODEGUARD_SELECTION_REPORT") {
        fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}

#[cfg(feature = "wasm-precheck")]
#[test]
fn selected_candidate_next_uses_its_persisted_task_and_existing_view_protocol() {
    let p = Project::new();
    fs::write(p.0.join("main.c"), "int main( {\n").unwrap();
    fs::write(p.0.join("main.py"), "def broken(:\n").unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["init", p.0.to_str().unwrap(), "--apply", "--format=json"])
        .env("PATH", "")
        .output()
        .unwrap();
    assert_eq!(
        init.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    // 先创建另一语言的历史任务，局部 C 指引不能推荐它。
    let earlier = p.check("python", &[]);
    assert_eq!(earlier.status.code(), Some(3));
    let earlier_report: Value = serde_json::from_slice(&earlier.stdout).unwrap();
    assert!(
        !earlier_report["syntax_tasks"]["tasks"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let output = p.check("c", &[]);
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let tasks = report["syntax_tasks"]["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 1, "{report}");
    assert_eq!(tasks[0]["language"], "c");
    assert_eq!(
        report["next"]["repair_brief"]["task_id"],
        tasks[0]["task_id"]
    );
    assert_eq!(report["next"]["report_type"], "repair_brief_preview");
}
