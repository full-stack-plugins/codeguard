use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("cg-planned-language-{}-{id}", std::process::id()));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn command(&self, args: &[&str]) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .unwrap();
        assert!(output.stderr.is_empty());
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn cobol_arkts_and_metal_are_explicit_gaps_when_project_requests_check() {
    let project = Project::new();
    for file in ["main.cbl", "main.ets", "main.metal"] {
        fs::write(project.0.join(file), "source\n").unwrap();
    }
    let (exit, report) =
        project.command(&["check", "all", project.0.to_str().unwrap(), "--format=json"]);
    assert_eq!(exit, 3);
    assert_eq!(report["command_status"], "incomplete");
    assert_eq!(report["delivery_decision"], "incomplete");
    assert!(report["required_obligations"].is_null());
    for language in ["cobol", "arkts", "metal"] {
        let candidates: Vec<_> = report["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|candidate| candidate["language"] == language)
            .collect();
        assert_eq!(candidates.len(), 6, "{language}");
        for candidate in candidates {
            assert_eq!(candidate["legacy_status"], "planned");
            assert_eq!(candidate["capability_status"], "gap");
            assert_eq!(candidate["status"], "not_integrated");
            assert_eq!(candidate["reason"], "planned_language_adapter_gap");
            assert!(candidate["checker_id"].is_null());
            assert!(
                candidate["next_action"]
                    .as_str()
                    .is_some_and(|text| !text.is_empty())
            );
        }
        assert!(
            report["unresolved_conditions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|condition| condition == &format!("planned_language_gap:{language}"))
        );
        let (capability_exit, capability) = project.command(&[
            "capabilities",
            language,
            "--platform=macos_arm64",
            "--category=lint",
            "--format=json",
        ]);
        assert_eq!(capability_exit, 0);
        assert_eq!(capability["cells"][0]["legacy_status"], "planned");
        assert_eq!(capability["cells"][0]["status"], "gap");
    }
}
