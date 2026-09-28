use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("cg-init-{}-{id}", std::process::id()));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn run(&self, args: &[&str]) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("init")
            .arg(&self.0)
            .args(args)
            .output()
            .unwrap();
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
fn default_init_is_read_only_and_does_not_execute_project_wrapper() {
    let project = Project::new();
    fs::write(
        project.0.join("pom.xml"),
        "<project><modelVersion>4.0.0</modelVersion></project>",
    )
    .unwrap();
    fs::write(project.0.join("mvnw"), "#!/bin/sh\ntouch wrapper-ran\n").unwrap();
    let (exit, report) = project.run(&["--format=json"]);
    assert_eq!(exit, 0);
    assert_eq!(report["init_status"], "planned");
    assert_eq!(report["readiness"], "unknown");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert!(!project.0.join("codeguard").exists());
    assert!(!project.0.join("wrapper-ran").exists());
    assert!(!report["planned_files"].as_array().unwrap().is_empty());
}

#[test]
fn apply_creates_bounded_workspace_and_is_idempotent() {
    let project = Project::new();
    fs::write(
        project.0.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let (first_exit, first) = project.run(&["--apply", "--format=json"]);
    assert_eq!(first_exit, 3);
    assert_eq!(first["init_status"], "partial");
    for path in [
        ".gitignore",
        "README.md",
        "workspace.json",
        "project.json",
        "module-graph.json",
        "architecture.md",
    ] {
        assert!(project.0.join("codeguard").join(path).is_file(), "{path}");
    }
    for directory in [
        "findings",
        "tasks",
        "decisions",
        "reports",
        "runs",
        "cache",
        "worktrees",
        "state",
    ] {
        assert!(
            project.0.join("codeguard").join(directory).is_dir(),
            "{directory}"
        );
    }
    let project_json: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/project.json")).unwrap())
            .unwrap();
    assert_eq!(project_json["delivery_decision"], "not_evaluated");
    assert!(
        project_json["languages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == "rust")
    );
    let workspace_before = fs::read(project.0.join("codeguard/workspace.json")).unwrap();
    let workspace: Value = serde_json::from_slice(&workspace_before).unwrap();
    assert_eq!(workspace["schema_version"], "0.3.0");
    assert_eq!(workspace["workspace_id"], first["workspace_id"]);
    assert!(
        workspace["workspace_id"]
            .as_str()
            .unwrap()
            .starts_with("ws-")
    );
    assert_eq!(
        workspace["project_sha256"],
        format!(
            "{:x}",
            Sha256::digest(fs::read(project.0.join("codeguard/project.json")).unwrap())
        )
    );
    assert_eq!(
        workspace["module_graph_sha256"],
        format!(
            "{:x}",
            Sha256::digest(fs::read(project.0.join("codeguard/module-graph.json")).unwrap())
        )
    );
    assert_eq!(
        workspace["planned_agents_block_sha256"],
        format!(
            "{:x}",
            Sha256::digest(fs::read(project.0.join("AGENTS.md")).unwrap())
        )
    );
    let (again_exit, again) = project.run(&["--apply", "--format=json"]);
    assert_eq!(again_exit, 3);
    assert_eq!(again["init_status"], "partial", "{again}");
    assert_eq!(again["workspace_id"], workspace["workspace_id"]);
    assert_eq!(
        fs::read(project.0.join("codeguard/workspace.json")).unwrap(),
        workspace_before
    );
}

#[test]
fn conflicting_user_file_is_preserved_and_does_not_create_other_artifacts() {
    let project = Project::new();
    fs::create_dir(project.0.join("codeguard")).unwrap();
    fs::write(project.0.join("codeguard/README.md"), "user owned\n").unwrap();
    let (exit, report) = project.run(&["--apply", "--format=json"]);
    assert_eq!(exit, 3);
    assert_eq!(report["init_status"], "conflict");
    assert_eq!(
        fs::read_to_string(project.0.join("codeguard/README.md")).unwrap(),
        "user owned\n"
    );
    assert!(!project.0.join("codeguard/workspace.json").exists());
}

#[test]
fn discovery_skips_managed_records_but_keeps_user_source_under_codeguard() {
    let project = Project::new();
    let (exit, _) = project.run(&["--apply", "--format=json"]);
    assert_eq!(exit, 3);
    fs::create_dir(project.0.join("codeguard/src")).unwrap();
    fs::write(project.0.join("codeguard/src/user.py"), "print('kept')\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("detect")
        .arg(&project.0)
        .arg("--format=json")
        .output()
        .unwrap();
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let source_paths: Vec<&str> = report["languages"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|language| language["source_files"].as_array().unwrap())
        .filter_map(Value::as_str)
        .collect();
    assert!(source_paths.contains(&"codeguard/src/user.py"));
    assert!(!source_paths.contains(&"codeguard/README.md"));
}

#[test]
fn project_human_instructions_are_not_overwritten_by_partial_init() {
    let project = Project::new();
    fs::write(
        project.0.join("AGENTS.md"),
        "# Human rules\nKeep this text.\n",
    )
    .unwrap();
    let (exit, report) = project.run(&["--apply", "--format=json"]);
    assert_eq!(exit, 3);
    assert_eq!(report["init_status"], "partial");
    let generated = fs::read_to_string(project.0.join("AGENTS.md")).unwrap();
    assert!(generated.starts_with("# Human rules\nKeep this text.\n"));
    assert!(generated.contains("<!-- CODEGUARD:BEGIN project-context -->"));
    assert!(generated.contains("<!-- CODEGUARD:END project-context -->"));
    assert!(generated.contains("codeguard/project.json"));
    let (again_exit, again) = project.run(&["--apply", "--format=json"]);
    assert_eq!(again_exit, 3);
    assert_eq!(again["init_status"], "partial");
    assert_eq!(
        fs::read_to_string(project.0.join("AGENTS.md")).unwrap(),
        generated
    );
}

#[test]
fn malformed_or_manually_modified_agents_block_is_a_preflight_conflict() {
    let project = Project::new();
    fs::write(
        project.0.join("AGENTS.md"),
        "<!-- CODEGUARD:BEGIN project-context -->\nunfinished\n",
    )
    .unwrap();
    let (exit, report) = project.run(&["--apply", "--format=json"]);
    assert_eq!(exit, 3);
    assert_eq!(report["init_status"], "conflict");
    assert_eq!(report["conflict_file"], "AGENTS.md");
    assert!(!project.0.join("codeguard").exists());
    fs::write(project.0.join("AGENTS.md"), "# Human\n").unwrap();
    let (applied_exit, _) = project.run(&["--apply", "--format=json"]);
    assert_eq!(applied_exit, 3);
    let before = fs::read_to_string(project.0.join("AGENTS.md")).unwrap();
    fs::write(
        project.0.join("AGENTS.md"),
        before.replace("codeguard/project.json", "tampered/project.json"),
    )
    .unwrap();
    let (conflict_exit, conflict) = project.run(&["--apply", "--format=json"]);
    assert_eq!(conflict_exit, 3);
    assert_eq!(conflict["init_status"], "conflict");
    assert_eq!(conflict["conflict_file"], "AGENTS.md");
}

#[test]
fn other_managed_sections_remain_byte_identical() {
    let project = Project::new();
    let human = "# Human\n<!-- CODEGRAPH_START -->\nkeep exactly\n<!-- CODEGRAPH_END -->\n";
    fs::write(project.0.join("AGENTS.md"), human).unwrap();
    let (exit, _) = project.run(&["--apply", "--format=json"]);
    assert_eq!(exit, 3);
    assert!(
        fs::read_to_string(project.0.join("AGENTS.md"))
            .unwrap()
            .starts_with(human)
    );
}

#[test]
fn duplicate_codeguard_markers_are_rejected_before_workspace_writes() {
    let project = Project::new();
    let marker = "<!-- CODEGUARD:BEGIN project-context -->";
    fs::write(
        project.0.join("AGENTS.md"),
        format!("{marker}\n{marker}\n<!-- CODEGUARD:END project-context -->\n"),
    )
    .unwrap();
    let (exit, report) = project.run(&["--apply", "--format=json"]);
    assert_eq!(exit, 3);
    assert_eq!(report["init_status"], "conflict");
    assert_eq!(report["conflict_file"], "AGENTS.md");
    assert!(!project.0.join("codeguard").exists());
}

#[cfg(unix)]
#[test]
fn symlinked_agents_file_is_not_followed() {
    use std::os::unix::fs::symlink;
    let project = Project::new();
    let outside = Project::new();
    fs::write(outside.0.join("AGENTS.md"), "# Outside\n").unwrap();
    symlink(outside.0.join("AGENTS.md"), project.0.join("AGENTS.md")).unwrap();
    let (exit, report) = project.run(&["--apply", "--format=json"]);
    assert_eq!(exit, 3);
    assert_eq!(report["conflict_file"], "AGENTS.md");
    assert_eq!(
        fs::read_to_string(outside.0.join("AGENTS.md")).unwrap(),
        "# Outside\n"
    );
    assert!(!project.0.join("codeguard").exists());
}

#[test]
fn changed_manifest_refreshes_owned_profile_without_clearing_work_records() {
    let project = Project::new();
    fs::write(
        project.0.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    let before = fs::read(project.0.join("codeguard/project.json")).unwrap();
    let agents_before = fs::read(project.0.join("AGENTS.md")).unwrap();
    fs::write(
        project.0.join("codeguard/tasks/CG-1.md"),
        "# Human note\nkeep\n",
    )
    .unwrap();
    fs::write(
        project.0.join("codeguard/findings/CG-1.json"),
        "{\"id\":\"CG-1\"}\n",
    )
    .unwrap();
    fs::write(
        project.0.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.2.0\"\n",
    )
    .unwrap();
    let (dry_exit, dry) = project.run(&["--format=json"]);
    assert_eq!(dry_exit, 0);
    assert_eq!(dry["profile_stale"], true);
    assert!(dry["changed_files"].as_array().unwrap().is_empty());
    assert_eq!(
        fs::read(project.0.join("codeguard/project.json")).unwrap(),
        before
    );
    let (apply_exit, apply) = project.run(&["--apply", "--format=json"]);
    assert_eq!(apply_exit, 3);
    assert_eq!(apply["init_status"], "partial");
    assert_ne!(
        fs::read(project.0.join("codeguard/project.json")).unwrap(),
        before
    );
    assert_ne!(
        fs::read(project.0.join("AGENTS.md")).unwrap(),
        agents_before
    );
    assert_eq!(
        fs::read_to_string(project.0.join("codeguard/tasks/CG-1.md")).unwrap(),
        "# Human note\nkeep\n"
    );
    assert_eq!(
        fs::read_to_string(project.0.join("codeguard/findings/CG-1.json")).unwrap(),
        "{\"id\":\"CG-1\"}\n"
    );
    let (again_exit, again) = project.run(&["--apply", "--format=json"]);
    assert_eq!(again_exit, 3);
    assert_eq!(again["profile_stale"], false);
    assert!(again["changed_files"].as_array().unwrap().is_empty());
}

#[test]
fn added_build_root_changes_graph_and_removal_changes_it_back() {
    let project = Project::new();
    fs::create_dir(project.0.join("service-a")).unwrap();
    fs::write(
        project.0.join("service-a/Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    fs::create_dir(project.0.join("service-b")).unwrap();
    fs::write(
        project.0.join("service-b/Cargo.toml"),
        "[package]\nname = \"b\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    let graph: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/module-graph.json")).unwrap())
            .unwrap();
    assert!(
        graph["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["kind"] == "contains" && edge["to"] == "service-b")
    );
    assert!(
        !graph["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["kind"] == "build_dependency")
    );
    assert!(
        fs::read_to_string(project.0.join("AGENTS.md"))
            .unwrap()
            .contains("构建根 \"service-b\"")
    );
    fs::remove_file(project.0.join("service-b/Cargo.toml")).unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    let graph: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/module-graph.json")).unwrap())
            .unwrap();
    assert!(
        !graph["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["to"] == "service-b")
    );
    assert!(
        !fs::read_to_string(project.0.join("AGENTS.md"))
            .unwrap()
            .contains("构建根 \"service-b\"")
    );
}

#[test]
fn manually_changed_profile_cannot_be_refreshed_as_owned_content() {
    let project = Project::new();
    fs::write(
        project.0.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    fs::write(project.0.join("codeguard/project.json"), "human edit\n").unwrap();
    fs::write(
        project.0.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.2.0\"\n",
    )
    .unwrap();
    let (exit, report) = project.run(&["--apply", "--format=json"]);
    assert_eq!(exit, 3);
    assert_eq!(report["init_status"], "conflict");
    assert_eq!(report["conflict_file"], "codeguard/project.json");
    assert_eq!(
        fs::read_to_string(project.0.join("codeguard/project.json")).unwrap(),
        "human edit\n"
    );
}

#[test]
fn lockfile_change_refreshes_profile_without_claiming_dependency_analysis() {
    let project = Project::new();
    fs::write(
        project.0.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(project.0.join("Cargo.lock"), "first\n").unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    let prior: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/project.json")).unwrap())
            .unwrap();
    fs::write(project.0.join("Cargo.lock"), "second\n").unwrap();
    let (exit, report) = project.run(&["--apply", "--format=json"]);
    assert_eq!(exit, 3);
    assert_eq!(report["profile_stale"], true);
    let current: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/project.json")).unwrap())
            .unwrap();
    assert_ne!(
        prior["lock_sha256"]["Cargo.lock"],
        current["lock_sha256"]["Cargo.lock"]
    );
    assert_eq!(current["delivery_decision"], "not_evaluated");
}

#[test]
fn ruff_rule_change_and_removal_refresh_profile_without_erasing_findings() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "print(1)\n").unwrap();
    fs::write(project.0.join(".ruff.toml"), "[lint]\nselect = [\"E\"]\n").unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    fs::write(
        project.0.join("codeguard/findings/CG-1.json"),
        "{\"id\":\"CG-1\"}\n",
    )
    .unwrap();
    let prior: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/project.json")).unwrap())
            .unwrap();
    fs::write(project.0.join(".ruff.toml"), "[lint]\nselect = [\"F\"]\n").unwrap();
    let (changed_exit, changed) = project.run(&["--apply", "--format=json"]);
    assert_eq!(changed_exit, 3);
    assert_eq!(changed["profile_stale"], true);
    let refreshed: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/project.json")).unwrap())
            .unwrap();
    assert_ne!(
        prior["checker_config_sha256"][".ruff.toml"],
        refreshed["checker_config_sha256"][".ruff.toml"]
    );
    fs::remove_file(project.0.join(".ruff.toml")).unwrap();
    let (removed_exit, removed) = project.run(&["--apply", "--format=json"]);
    assert_eq!(removed_exit, 3);
    assert_eq!(removed["profile_stale"], true);
    let current: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/project.json")).unwrap())
            .unwrap();
    assert!(
        current["checker_config_sha256"]
            .as_object()
            .unwrap()
            .is_empty()
    );
    fs::write(
        project.0.join(".ruff.toml"),
        "[lint]\nselect = [\"E\", \"F\"]\n",
    )
    .unwrap();
    let (added_exit, added) = project.run(&["--apply", "--format=json"]);
    assert_eq!(added_exit, 3);
    assert_eq!(added["profile_stale"], true);
    let readded: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/project.json")).unwrap())
            .unwrap();
    assert!(readded["checker_config_sha256"][".ruff.toml"].is_string());
    assert_eq!(
        fs::read_to_string(project.0.join("codeguard/findings/CG-1.json")).unwrap(),
        "{\"id\":\"CG-1\"}\n"
    );
}

#[test]
fn unreadable_rule_input_never_claims_profile_is_fresh() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "print(1)\n").unwrap();
    fs::write(project.0.join(".ruff.toml"), "[lint]\nselect = [\"E\"]\n").unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    fs::write(project.0.join(".ruff.toml"), vec![b'x'; 256 * 1024 + 1]).unwrap();
    let (exit, report) = project.run(&["--format=json"]);
    assert_eq!(exit, 0);
    assert_eq!(report["observation_complete"], false);
    assert!(report["profile_stale"].is_null());
    assert!(
        report["unresolved"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "static_discovery_incomplete")
    );
}

#[test]
fn known_visible_rule_files_across_languages_invalidate_profile_without_claiming_checker_ready() {
    let project = Project::new();
    fs::write(project.0.join("App.java"), "class App {}\n").unwrap();
    fs::write(project.0.join("index.ts"), "export const value = 1;\n").unwrap();
    fs::write(project.0.join("checkstyle.xml"), "<rules version=\"1\"/>\n").unwrap();
    fs::write(project.0.join("eslint.config.mjs"), "export default [];\n").unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    let prior: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/project.json")).unwrap())
            .unwrap();
    assert!(prior["checker_config_sha256"]["checkstyle.xml"].is_string());
    assert!(prior["checker_config_sha256"]["eslint.config.mjs"].is_string());
    let configurations = prior["checker_configurations"].as_array().unwrap();
    assert_eq!(configurations.len(), 1);
    assert_eq!(configurations[0]["checker_id"], "node.eslint");
    assert_eq!(configurations[0]["configuration"], "unknown");
    assert_eq!(
        configurations[0]["reason"],
        "eslint_dynamic_configuration_not_evaluated"
    );
    fs::write(project.0.join("checkstyle.xml"), "<rules version=\"2\"/>\n").unwrap();
    let (changed_exit, changed) = project.run(&["--apply", "--format=json"]);
    assert_eq!(changed_exit, 3);
    assert_eq!(changed["profile_stale"], true);
    let refreshed: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/project.json")).unwrap())
            .unwrap();
    assert_ne!(
        prior["checker_config_sha256"]["checkstyle.xml"],
        refreshed["checker_config_sha256"]["checkstyle.xml"]
    );
    fs::remove_file(project.0.join("eslint.config.mjs")).unwrap();
    let (removed_exit, removed) = project.run(&["--apply", "--format=json"]);
    assert_eq!(removed_exit, 3);
    assert_eq!(removed["profile_stale"], true);
    let current: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/project.json")).unwrap())
            .unwrap();
    assert!(
        current["checker_config_sha256"]
            .get("eslint.config.mjs")
            .is_none()
    );
}

#[test]
fn registry_declared_dot_config_is_observed_without_scanning_other_dot_files() {
    let project = Project::new();
    fs::write(project.0.join("index.ts"), "export const value = 1;\n").unwrap();
    fs::write(project.0.join(".eslintrc"), "{\"rules\":{}}\n").unwrap();
    fs::write(project.0.join(".secret-source.ts"), "not ordinary source\n").unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    let prior: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/project.json")).unwrap())
            .unwrap();
    assert!(prior["checker_config_sha256"][".eslintrc"].is_string());
    assert!(
        prior["languages"]
            .as_array()
            .unwrap()
            .iter()
            .all(|language| language["source_file_count"].as_u64().unwrap() <= 1)
    );
    fs::write(project.0.join(".eslintrc"), "{\"rules\":{\"semi\":2}}\n").unwrap();
    let (exit, report) = project.run(&["--format=json"]);
    assert_eq!(exit, 0);
    assert_eq!(report["profile_stale"], true);
}

#[test]
fn changed_source_path_with_same_count_refreshes_profile() {
    let project = Project::new();
    fs::write(project.0.join("old.py"), "print(1)\n").unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    let prior: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/project.json")).unwrap())
            .unwrap();
    fs::rename(project.0.join("old.py"), project.0.join("new.py")).unwrap();
    let (exit, report) = project.run(&["--apply", "--format=json"]);
    assert_eq!(exit, 3);
    assert_eq!(report["profile_stale"], true);
    let current: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/project.json")).unwrap())
            .unwrap();
    assert_eq!(
        prior["languages"][0]["source_file_count"],
        current["languages"][0]["source_file_count"]
    );
    assert_ne!(
        prior["languages"][0]["source_set_sha256"],
        current["languages"][0]["source_set_sha256"]
    );
}

#[test]
fn package_version_is_not_misreported_as_language_target_version() {
    let project = Project::new();
    fs::write(
        project.0.join("package.json"),
        "{\"name\":\"demo\",\"version\":\"2.4.6\"}\n",
    )
    .unwrap();
    fs::write(project.0.join("index.ts"), "export const value = 1;\n").unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    let profile: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/project.json")).unwrap())
            .unwrap();
    assert_eq!(
        profile["package_declared_versions"]["package.json"],
        "2.4.6"
    );
    for language in profile["languages"].as_array().unwrap() {
        assert!(language["declared_version"].is_null());
        assert_eq!(language["version_status"], "unknown");
    }
    assert_eq!(profile["delivery_decision"], "not_evaluated");
}

#[test]
fn refresh_preserves_human_text_added_outside_agents_managed_block() {
    let project = Project::new();
    fs::write(
        project.0.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(project.0.join("AGENTS.md"), "# Human rules\nKeep this.\n").unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    let mut agents = fs::read_to_string(project.0.join("AGENTS.md")).unwrap();
    agents.push_str("\n# Human appendix\nNever erase this.\n");
    fs::write(project.0.join("AGENTS.md"), &agents).unwrap();
    fs::write(
        project.0.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.2.0\"\n",
    )
    .unwrap();
    let (exit, report) = project.run(&["--apply", "--format=json"]);
    assert_eq!(exit, 3);
    assert_eq!(report["init_status"], "partial");
    let updated = fs::read_to_string(project.0.join("AGENTS.md")).unwrap();
    assert!(updated.starts_with("# Human rules\nKeep this.\n"));
    assert!(updated.ends_with("\n# Human appendix\nNever erase this.\n"));
    assert_ne!(updated, agents);
}

#[test]
fn interrupted_refresh_accepts_target_projection_and_finishes_workspace_marker() {
    let project = Project::new();
    let reference = Project::new();
    for root in [&project.0, &reference.0] {
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
    }
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    assert_eq!(reference.run(&["--apply", "--format=json"]).0, 3);
    let old_workspace = fs::read(project.0.join("codeguard/workspace.json")).unwrap();
    for root in [&project.0, &reference.0] {
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.2.0\"\n",
        )
        .unwrap();
    }
    assert_eq!(reference.run(&["--apply", "--format=json"]).0, 3);
    fs::write(
        project.0.join("codeguard/project.json"),
        fs::read(reference.0.join("codeguard/project.json")).unwrap(),
    )
    .unwrap();
    let (exit, report) = project.run(&["--apply", "--format=json"]);
    assert_eq!(exit, 3);
    assert_eq!(report["init_status"], "partial");
    assert_ne!(
        fs::read(project.0.join("codeguard/workspace.json")).unwrap(),
        old_workspace
    );
    let mut updated_workspace: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/workspace.json")).unwrap())
            .unwrap();
    let mut reference_workspace: Value =
        serde_json::from_slice(&fs::read(reference.0.join("codeguard/workspace.json")).unwrap())
            .unwrap();
    assert_ne!(
        updated_workspace["workspace_id"],
        reference_workspace["workspace_id"]
    );
    assert_eq!(
        updated_workspace["workspace_id"],
        serde_json::from_slice::<Value>(&old_workspace).unwrap()["workspace_id"]
    );
    updated_workspace
        .as_object_mut()
        .unwrap()
        .remove("workspace_id");
    reference_workspace
        .as_object_mut()
        .unwrap()
        .remove("workspace_id");
    assert_eq!(updated_workspace, reference_workspace);
    assert_eq!(
        fs::read(project.0.join("AGENTS.md")).unwrap(),
        fs::read(reference.0.join("AGENTS.md")).unwrap()
    );
    let (again_exit, again) = project.run(&["--apply", "--format=json"]);
    assert_eq!(again_exit, 3);
    assert!(again["changed_files"].as_array().unwrap().is_empty());
}

#[test]
fn malformed_workspace_identity_blocks_refresh_without_writes() {
    let project = Project::new();
    fs::write(
        project.0.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    let before = fs::read(project.0.join("codeguard/project.json")).unwrap();
    fs::write(
        project.0.join("codeguard/workspace.json"),
        "{\"approved\":true}\n",
    )
    .unwrap();
    fs::write(
        project.0.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.2.0\"\n",
    )
    .unwrap();
    let (exit, report) = project.run(&["--apply", "--format=json"]);
    assert_eq!(exit, 3);
    assert_eq!(report["init_status"], "conflict");
    assert_eq!(report["conflict_file"], "codeguard/workspace.json");
    assert_eq!(
        fs::read(project.0.join("codeguard/project.json")).unwrap(),
        before
    );
}

#[test]
fn legacy_workspace_upgrades_identity_without_erasing_findings() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "pass\n").unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    let path = project.0.join("codeguard/workspace.json");
    let mut legacy: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    legacy["schema_version"] = serde_json::json!("0.2.0");
    legacy.as_object_mut().unwrap().remove("workspace_id");
    fs::write(&path, serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();
    let finding = project.0.join("codeguard/findings/user-note.txt");
    fs::write(&finding, "keep this\n").unwrap();
    let (exit, report) = project.run(&["--apply", "--format=json"]);
    assert_eq!(exit, 3);
    assert_eq!(report["init_status"], "partial");
    let upgraded: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(upgraded["schema_version"], "0.3.0");
    assert_eq!(upgraded["workspace_id"], report["workspace_id"]);
    assert_eq!(fs::read_to_string(finding).unwrap(), "keep this\n");
}

#[cfg(unix)]
#[test]
fn symlinked_workspace_is_a_conflict_without_following_it() {
    use std::os::unix::fs::symlink;
    let project = Project::new();
    let outside = Project::new();
    symlink(&outside.0, project.0.join("codeguard")).unwrap();
    let (exit, report) = project.run(&["--apply", "--format=json"]);
    assert_eq!(exit, 3);
    assert_eq!(report["init_status"], "conflict");
    assert_eq!(report["conflict_file"], "codeguard");
    assert!(!outside.0.join("workspace.json").exists());
}

#[test]
fn published_partial_init_schemas_cannot_claim_quality_allow() {
    for source in [
        include_str!("../../../schemas/init-plan.schema.json"),
        include_str!("../../../schemas/codeguard-workspace.schema.json"),
        include_str!("../../../schemas/project-profile.schema.json"),
        include_str!("../../../schemas/module-graph.schema.json"),
    ] {
        let schema: Value = serde_json::from_str(source).unwrap();
        assert_eq!(schema["additionalProperties"], false);
        assert!(matches!(
            schema["properties"]["schema_version"]["const"].as_str(),
            Some("0.1.0" | "0.2.0" | "0.3.0" | "0.4.0" | "0.5.0")
        ));
        assert!(!source.contains("\"allow\""));
    }
}

#[test]
fn init_separates_maven_aggregation_and_explicit_local_dependency_with_evidence() {
    let project = Project::new();
    fs::create_dir(project.0.join("api")).unwrap();
    fs::create_dir(project.0.join("service")).unwrap();
    let api = "<project><groupId>example</groupId><artifactId>api</artifactId><version>1</version></project>";
    let service = "<project><groupId>example</groupId><artifactId>service</artifactId><version>1</version><dependencies><dependency><groupId>example</groupId><artifactId>api</artifactId><version>1</version><scope>test</scope></dependency></dependencies></project>";
    fs::write(
        project.0.join("pom.xml"),
        "<project><modules><module>api</module><module>service</module></modules></project>",
    )
    .unwrap();
    fs::write(project.0.join("api/pom.xml"), api).unwrap();
    fs::write(project.0.join("service/pom.xml"), service).unwrap();
    let (exit, _) = project.run(&["--apply", "--format=json"]);
    assert_eq!(exit, 3);
    let graph: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/module-graph.json")).unwrap())
            .unwrap();
    assert_eq!(graph["schema_version"], "0.3.0");
    let edges = graph["edges"].as_array().unwrap();
    assert!(
        edges
            .iter()
            .any(|e| e["kind"] == "aggregation" && e["from"] == "." && e["to"] == "api")
    );
    let edge = edges
        .iter()
        .find(|e| e["kind"] == "build_dependency")
        .expect("显式本地依赖应被观察");
    assert_eq!(edge["from"], "service");
    assert_eq!(edge["to"], "api");
    assert_eq!(edge["status"], "declared");
    assert_eq!(edge["scope"], "test");
    assert_eq!(edge["condition"], "unconditional_declaration");
    assert_eq!(edge["manifest_ref"], "service/pom.xml");
    assert_eq!(
        edge["manifest_sha256"],
        format!("{:x}", Sha256::digest(service.as_bytes()))
    );
    assert_eq!(graph["dependency_edges_complete"], false);
    assert!(!project.0.join("target").exists());
}

#[test]
fn init_keeps_ambiguous_conditional_and_escaping_maven_relations_unresolved() {
    let project = Project::new();
    for directory in ["api", "duplicate", "service"] {
        fs::create_dir(project.0.join(directory)).unwrap();
    }
    let api = "<project><groupId>example</groupId><artifactId>api</artifactId><version>1</version></project>";
    fs::write(project.0.join("api/pom.xml"), api).unwrap();
    fs::write(project.0.join("duplicate/pom.xml"), api).unwrap();
    fs::write(project.0.join("pom.xml"), "<project><modules><module>../escape</module><module>/absolute</module><module>missing</module><module>api</module></modules><profiles><profile><modules><module>service</module></modules></profile></profiles></project>").unwrap();
    fs::write(project.0.join("service/pom.xml"), "<project><groupId>example</groupId><artifactId>service</artifactId><version>1</version><parent/><dependencies><dependency><groupId>example</groupId><artifactId>api</artifactId><version>1</version></dependency><dependency><groupId>example</groupId><artifactId>other</artifactId><version>${version}</version></dependency></dependencies></project>").unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    let graph: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/module-graph.json")).unwrap())
            .unwrap();
    assert!(
        !graph["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["kind"] == "build_dependency")
    );
    assert!(
        !graph["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["kind"] == "aggregation" && edge["to"] == "service")
    );
    for reason in [
        "maven_profiles_not_evaluated:pom.xml",
        "maven_module_path_unresolved:pom.xml",
        "maven_module_target_unresolved:pom.xml",
        "maven_dependency_target_unresolved:service/pom.xml",
        "maven_dependency_coordinates_unresolved:service/pom.xml",
        "maven_project_coordinates_ambiguous:api/pom.xml",
    ] {
        assert!(
            graph["unresolved"]
                .as_array()
                .unwrap()
                .contains(&Value::from(reason)),
            "{reason}"
        );
    }
}

#[test]
fn refreshed_maven_dependency_graph_changes_with_manifest_bytes_and_preserves_work() {
    let project = Project::new();
    for name in ["api", "service"] {
        fs::create_dir(project.0.join(name)).unwrap();
    }
    fs::write(project.0.join("api/pom.xml"), "<project><groupId>example</groupId><artifactId>api</artifactId><version>1</version></project>").unwrap();
    let service = "<project><groupId>example</groupId><artifactId>service</artifactId><version>1</version><dependencies><dependency><groupId>example</groupId><artifactId>api</artifactId><version>1</version></dependency></dependencies></project>";
    fs::write(project.0.join("service/pom.xml"), service).unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    let before = fs::read(project.0.join("codeguard/module-graph.json")).unwrap();
    fs::write(
        project.0.join("codeguard/tasks/user-notes.md"),
        "人工备注，不允许清空",
    )
    .unwrap();
    fs::write(
        project.0.join("service/pom.xml"),
        service.replace(
            "<version>1</version></dependency>",
            "<version>2</version></dependency>",
        ),
    )
    .unwrap();
    let (exit, report) = project.run(&["--apply", "--format=json"]);
    assert_eq!(exit, 3);
    assert_eq!(report["profile_stale"], true);
    let after = fs::read(project.0.join("codeguard/module-graph.json")).unwrap();
    assert_ne!(before, after);
    let graph: Value = serde_json::from_slice(&after).unwrap();
    assert!(
        !graph["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["kind"] == "build_dependency")
    );
    assert_eq!(
        fs::read_to_string(project.0.join("codeguard/tasks/user-notes.md")).unwrap(),
        "人工备注，不允许清空"
    );
    assert_eq!(
        project.run(&["--apply", "--format=json"]).1["profile_stale"],
        false
    );
}

#[test]
fn init_observes_cargo_members_and_renamed_local_path_dependencies_without_execution() {
    let project = Project::new();
    for name in ["api", "service"] {
        fs::create_dir(project.0.join(name)).unwrap();
    }
    fs::write(
        project.0.join("Cargo.toml"),
        "[workspace]\nmembers = [\"api\", \"service\"]\n",
    )
    .unwrap();
    fs::write(
        project.0.join("api/Cargo.toml"),
        "[package]\nname = \"actual-api\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    let service = "[package]\nname = \"service\"\nversion = \"1.0.0\"\n[dependencies]\napi_alias = { path = \"../api\", package = \"actual-api\" }\n[build-dependencies]\nactual-api = { path = \"../api\" }\n";
    fs::write(project.0.join("service/Cargo.toml"), service).unwrap();
    fs::write(
        project.0.join("service/build.rs"),
        "fn main(){panic!(\"init must never run build.rs\");}",
    )
    .unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    let graph: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/module-graph.json")).unwrap())
            .unwrap();
    assert_eq!(graph["schema_version"], "0.3.0");
    let edges = graph["edges"].as_array().unwrap();
    assert!(edges.iter().any(|edge| edge["kind"] == "aggregation"
        && edge["basis"] == "cargo_members_declaration"
        && edge["to"] == "api"));
    for scope in ["normal", "build"] {
        let edge = edges
            .iter()
            .find(|edge| edge["kind"] == "build_dependency" && edge["scope"] == scope)
            .unwrap();
        assert_eq!(edge["from"], "service");
        assert_eq!(edge["to"], "api");
        assert_eq!(edge["status"], "declared");
        assert_eq!(edge["basis"], "cargo_path_dependency_declaration");
        assert_eq!(
            edge["manifest_sha256"],
            format!("{:x}", Sha256::digest(service.as_bytes()))
        );
    }
    assert_eq!(graph["dependency_edges_complete"], false);
    let agents = fs::read_to_string(project.0.join("AGENTS.md")).unwrap();
    assert!(agents.contains("构建根"));
    assert!(
        agents.contains("\"service\" → \"api\"：build_dependency（scope=\"normal\"）"),
        "{agents}"
    );
    assert!(agents.contains("模块依赖完整性：未解析"));
    assert!(!project.0.join("Cargo.lock").exists());
    assert!(!project.0.join("target").exists());
}

#[test]
fn cargo_graph_refuses_wrong_alias_missing_target_escaping_path_and_conditions() {
    let project = Project::new();
    for name in ["api", "service"] {
        fs::create_dir(project.0.join(name)).unwrap();
    }
    fs::write(
        project.0.join("Cargo.toml"),
        "[workspace]\nmembers=['api','crates/*']\nexclude=['api']\n",
    )
    .unwrap();
    fs::write(
        project.0.join("api/Cargo.toml"),
        "[package]\nname='actual-api'\nversion='1.0.0'\n",
    )
    .unwrap();
    fs::write(project.0.join("service/Cargo.toml"),"[package]\nname='service'\nversion='1.0.0'\n[dependencies]\nwrong={path='../api'}\nmissing={path='../missing'}\nescape={path='../../outside'}\nconditional={path='../api',package='actual-api',optional=true}\ninherited={workspace=true}\n[target.'cfg(unix)'.dependencies]\nactual-api={path='../api'}\n").unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    let graph: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/module-graph.json")).unwrap())
            .unwrap();
    assert!(
        !graph["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| matches!(
                edge["kind"].as_str(),
                Some("aggregation" | "build_dependency")
            ))
    );
    for reason in [
        "cargo_workspace_exclusions_not_resolved:Cargo.toml",
        "cargo_workspace_member_unresolved:Cargo.toml",
        "cargo_dependency_target_unresolved:service/Cargo.toml",
        "cargo_dependency_path_unresolved:service/Cargo.toml",
        "cargo_optional_dependency_not_resolved:service/Cargo.toml",
        "cargo_dependency_workspace_not_resolved:service/Cargo.toml",
        "cargo_target_not_resolved:service/Cargo.toml",
    ] {
        assert!(
            graph["unresolved"]
                .as_array()
                .unwrap()
                .contains(&Value::from(reason)),
            "{reason}"
        );
    }
}

#[test]
fn cargo_dependency_refresh_removes_old_relationship_and_preserves_notes() {
    let project = Project::new();
    for name in ["api", "service"] {
        fs::create_dir(project.0.join(name)).unwrap();
    }
    fs::write(
        project.0.join("api/Cargo.toml"),
        "[package]\nname='actual-api'\nversion='1.0.0'\n",
    )
    .unwrap();
    let service = "[package]\nname='service'\nversion='1.0.0'\n[dev-dependencies]\nalias={path='../api',package='actual-api'}\n";
    fs::write(project.0.join("service/Cargo.toml"), service).unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    let before = fs::read(project.0.join("codeguard/module-graph.json")).unwrap();
    fs::write(project.0.join("codeguard/tasks/notes.md"), "保留记录").unwrap();
    fs::write(
        project.0.join("service/Cargo.toml"),
        service.replace("actual-api", "wrong-name"),
    )
    .unwrap();
    assert_eq!(
        project.run(&["--apply", "--format=json"]).1["profile_stale"],
        true
    );
    let after = fs::read(project.0.join("codeguard/module-graph.json")).unwrap();
    assert_ne!(before, after);
    let graph: Value = serde_json::from_slice(&after).unwrap();
    assert!(
        !graph["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["kind"] == "build_dependency")
    );
    assert_eq!(
        fs::read_to_string(project.0.join("codeguard/tasks/notes.md")).unwrap(),
        "保留记录"
    );
    assert_eq!(
        project.run(&["--apply", "--format=json"]).1["profile_stale"],
        false
    );
}

#[test]
fn init_keeps_per_manifest_java_and_rust_targets_separate_from_package_and_installed_versions() {
    let project = Project::new();
    for name in ["java17", "java21", "rust2021", "rust2024"] {
        fs::create_dir(project.0.join(name)).unwrap();
    }
    let java17 = "<project><properties><maven.compiler.release>17</maven.compiler.release></properties></project>";
    fs::write(project.0.join("java17/pom.xml"), java17).unwrap();
    fs::write(project.0.join("java21/pom.xml"),"<project><properties><maven.compiler.source>21</maven.compiler.source><maven.compiler.target>21</maven.compiler.target></properties></project>").unwrap();
    fs::write(
        project.0.join("rust2021/Cargo.toml"),
        "[package]\nname='a'\nversion='9.9.9'\nrust-version='1.70'\nedition='2021'\n",
    )
    .unwrap();
    fs::write(
        project.0.join("rust2024/Cargo.toml"),
        "[package]\nname='b'\nversion='0.1.0'\nrust-version='1.85'\nedition='2024'\n",
    )
    .unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    let profile: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/project.json")).unwrap())
            .unwrap();
    assert_eq!(profile["schema_version"], "0.3.0");
    let targets = profile["language_targets"]
        .as_array()
        .expect("画像应保留逐清单语言目标");
    assert_eq!(targets.len(), 7);
    let target = targets
        .iter()
        .find(|value| value["manifest_ref"] == "java17/pom.xml")
        .unwrap();
    assert_eq!(target["language_id"], "java");
    assert_eq!(target["target_kind"], "maven.compiler.release");
    assert_eq!(target["value"], "17");
    assert_eq!(target["status"], "declared_only");
    assert_eq!(
        target["manifest_sha256"],
        format!("{:x}", Sha256::digest(java17.as_bytes()))
    );
    for (manifest, value, kind) in [
        ("java21/pom.xml", "21", "maven.compiler.target"),
        ("rust2021/Cargo.toml", "1.70", "rust-version"),
        ("rust2024/Cargo.toml", "2024", "edition"),
    ] {
        assert!(
            targets
                .iter()
                .any(|target| target["manifest_ref"] == manifest
                    && target["value"] == value
                    && target["target_kind"] == kind)
        );
    }
    for language in profile["languages"].as_array().unwrap() {
        assert!(language["installed_version"].is_null());
        assert!(language["declared_version"].is_null());
    }
    let agents = fs::read_to_string(project.0.join("AGENTS.md")).unwrap();
    assert!(agents.contains("语言 java"), "{agents}");
    assert!(agents.contains("语言 rust"), "{agents}");
    assert!(agents.contains("构建器线索 Maven"), "{agents}");
    assert!(agents.contains("构建器线索 Cargo"), "{agents}");
    assert!(
        agents.contains("\"java17/pom.xml\" 声明 java maven.compiler.release=\"17\""),
        "{agents}"
    );
    assert!(
        agents.contains("\"rust2021/Cargo.toml\" 声明 rust rust-version=\"1.70\""),
        "{agents}"
    );
    assert!(agents.contains("本机版本：unknown"), "{agents}");
    assert!(!targets.iter().any(|target| target["value"] == "9.9.9"));
}

#[test]
fn unresolved_language_targets_are_visible_and_refresh_preserves_notes() {
    let project = Project::new();
    fs::create_dir(project.0.join("java")).unwrap();
    fs::write(project.0.join("java/pom.xml"),"<project><properties><maven.compiler.release>${java.version}</maven.compiler.release></properties></project>").unwrap();
    fs::write(project.0.join("Cargo.toml"),"[package]\nname='app'\nversion='1.0.0'\nrust-version={workspace=true}\nedition={workspace=true}\n").unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    let before: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/project.json")).unwrap())
            .unwrap();
    assert!(before["language_targets"].as_array().unwrap().is_empty());
    for reason in [
        "maven_language_target_unresolved:maven.compiler.release:java/pom.xml",
        "cargo_language_target_unresolved:rust-version:Cargo.toml",
        "cargo_language_target_unresolved:edition:Cargo.toml",
    ] {
        assert!(
            before["unknown_conditions"]
                .as_array()
                .unwrap()
                .contains(&Value::from(reason)),
            "{reason}"
        );
    }
    fs::write(project.0.join("codeguard/tasks/notes.md"), "保留人工备注").unwrap();
    fs::write(project.0.join("java/pom.xml"),"<project><properties><maven.compiler.release>17</maven.compiler.release></properties></project>").unwrap();
    assert_eq!(
        project.run(&["--apply", "--format=json"]).1["profile_stale"],
        true
    );
    let after: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/project.json")).unwrap())
            .unwrap();
    assert_eq!(after["language_targets"][0]["value"], "17");
    assert!(
        !after["unknown_conditions"]
            .as_array()
            .unwrap()
            .contains(&Value::from(
                "maven_language_target_unresolved:maven.compiler.release:java/pom.xml"
            ))
    );
    assert_eq!(
        fs::read_to_string(project.0.join("codeguard/tasks/notes.md")).unwrap(),
        "保留人工备注"
    );
    assert_eq!(
        project.run(&["--apply", "--format=json"]).1["profile_stale"],
        false
    );
}

#[test]
fn dry_run_returns_the_profile_summary_without_creating_files() {
    let project = Project::new();
    fs::write(project.0.join("pom.xml"),"<project><properties><maven.compiler.release>17</maven.compiler.release></properties></project>").unwrap();
    let (exit, report) = project.run(&["--format=json"]);
    assert_eq!(exit, 0);
    assert_eq!(report["schema_version"], "0.5.0");
    let summary = &report["profile_summary"];
    assert_eq!(summary["installed_versions"], "not_probed");
    assert_eq!(summary["build_model"], "not_resolved");
    assert_eq!(summary["architecture"], "unknown");
    assert_eq!(summary["languages"][0]["id"], "java");
    assert_eq!(summary["languages"][0]["manifest_count"], 1);
    assert_eq!(summary["build_roots"][0]["path"], ".");
    assert_eq!(summary["language_targets"][0]["value"], "17");
    assert_eq!(summary["language_targets"][0]["status"], "declared_only");
    assert_eq!(report["readiness"], "unknown");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert!(!project.0.join("codeguard").exists());
    assert!(!project.0.join("AGENTS.md").exists());
    let (_, applied) = project.run(&["--apply", "--format=json"]);
    assert_eq!(applied["profile_summary"], report["profile_summary"]);
}

#[test]
fn human_init_reports_the_same_declared_target_and_unknown_readiness() {
    let project = Project::new();
    fs::write(
        project.0.join("Cargo.toml"),
        "[package]\nname='app'\nversion='1.0.0'\nrust-version='1.85'\nedition='2024'\n",
    )
    .unwrap();
    let (_, report) = project.run(&["--format=json"]);
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("init")
        .arg(&project.0)
        .arg("--format=human")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("检查准备：unknown；交付：未评估"));
    assert!(text.contains("本机版本：未探测；构建模型：未解析；架构：unknown"));
    for target in report["profile_summary"]["language_targets"]
        .as_array()
        .unwrap()
    {
        for field in ["language_id", "target_kind", "value", "manifest_ref"] {
            assert!(text.contains(&target[field].to_string()));
        }
    }
    assert!(text.contains("状态 declared_only"));
    assert!(text.contains("待确认："));
    assert!(!text.contains(&project.0.to_string_lossy().to_string()));
    assert!(!project.0.join("codeguard").exists());
}

#[test]
#[cfg(unix)]
fn human_init_cannot_turn_project_path_control_characters_into_terminal_status_lines() {
    let project = Project::new();
    let hostile = "module\nFORGED_STATUS\u{1b}[31m";
    fs::create_dir(project.0.join(hostile)).unwrap();
    fs::write(project.0.join(hostile).join("pom.xml"),"<project><properties><maven.compiler.release>17</maven.compiler.release></properties></project>").unwrap();
    let (_, report) = project.run(&["--format=json"]);
    assert_eq!(report["profile_summary"]["build_roots"][0]["path"], hostile);
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("init")
        .arg(&project.0)
        .arg("--format=human")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(!text.contains(hostile));
    assert!(!text.contains('\u{1b}'));
    assert!(!text.lines().any(|line| line.starts_with("FORGED_STATUS")));
    assert!(text.contains("\\nFORGED_STATUS\\u001b[31m"));
    assert!(!project.0.join("codeguard").exists());
}

#[test]
fn init_feedback_separates_checker_configuration_from_execution_and_required_policy() {
    let project = Project::new();
    fs::write(project.0.join("pom.xml"),"<project><build><plugins><plugin><groupId>org.apache.maven.plugins</groupId><artifactId>maven-javadoc-plugin</artifactId></plugin></plugins></build></project>").unwrap();
    let (exit, report) = project.run(&["--format=json"]);
    assert_eq!(exit, 0);
    assert_eq!(report["schema_version"], "0.5.0");
    let summary = &report["profile_summary"];
    assert_eq!(summary["checker_inventory"], "partial");
    let checkers = summary["checkers"]
        .as_array()
        .expect("初始化应反馈检查器配置");
    let javadoc = checkers
        .iter()
        .find(|entry| entry["checker_id"] == "java.maven.javadoc")
        .unwrap();
    assert_eq!(javadoc["configuration"], "configured");
    assert_eq!(javadoc["execution"], "not_run");
    assert_eq!(javadoc["configuration_ref"], "pom.xml");
    assert_eq!(javadoc["build_root"], ".");
    assert!(javadoc["required_by_policy"].is_null());
    assert_eq!(javadoc["gate_effect"], "none");
    let cve = checkers
        .iter()
        .find(|entry| entry["checker_id"] == "java.maven.dependency_check")
        .unwrap();
    assert_eq!(cve["configuration"], "missing");
    assert_eq!(cve["execution"], "not_run");
    assert_eq!(report["readiness"], "unknown");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert!(!project.0.join("codeguard").exists());
    assert!(!project.0.join("target").exists());
}

#[test]
fn init_checker_feedback_preserves_four_configuration_states_per_build_root() {
    let project = Project::new();
    for (name, pom) in [
        (
            "configured",
            "<project><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId></plugin></plugins></build></project>",
        ),
        ("missing", "<project/>"),
        ("invalid", "<project"),
        (
            "unknown",
            "<project><parent><groupId>external</groupId><artifactId>parent</artifactId><version>1</version></parent></project>",
        ),
    ] {
        fs::create_dir(project.0.join(name)).unwrap();
        fs::write(project.0.join(name).join("pom.xml"), pom).unwrap();
    }
    let (_, report) = project.run(&["--format=json"]);
    let checkers = report["profile_summary"]["checkers"].as_array().unwrap();
    for state in ["configured", "missing", "invalid", "unknown"] {
        let row = checkers
            .iter()
            .find(|row| row["checker_id"] == "java.maven.javadoc" && row["build_root"] == state)
            .unwrap();
        assert_eq!(row["configuration"], state);
        assert_eq!(row["execution"], "not_run");
        assert_eq!(row["gate_effect"], "none");
        assert!(row["required_by_policy"].is_null());
        assert!(!row["reason"].as_str().unwrap().is_empty());
        assert!(!row["next_action"].as_str().unwrap().is_empty());
    }
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("init")
        .arg(&project.0)
        .arg("--format=human")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("检查器清单：partial；检查义务：未绑定"));
    for row in checkers
        .iter()
        .filter(|row| row["checker_id"] == "java.maven.javadoc")
    {
        let line = format!(
            "检查配置 {}（构建根 {}）：{}；执行：not_run",
            row["checker_id"], row["build_root"], row["configuration"]
        );
        assert!(text.contains(&line));
    }
    assert!(text.contains("必需性：未绑定"));
    assert!(text.contains("配置依据："));
    assert!(text.contains("下一步："));
    assert!(!project.0.join("codeguard").exists());

    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    let agents = fs::read_to_string(project.0.join("AGENTS.md")).unwrap();
    for state in ["configured", "invalid", "missing"] {
        assert!(agents.contains(&format!(
            "检查配置 \"java.maven.javadoc\"（根 \"{state}\"）：{state}；来源"
        )));
    }
    assert!(agents.contains("其余 8 项检查配置见项目画像"));
    assert!(agents.contains("原生执行 not_run，必需性未绑定"));
}

#[test]
fn init_missing_python_configuration_stays_not_run_without_claiming_code_failure() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    let (_, report) = project.run(&["--format=json"]);
    let row = report["profile_summary"]["checkers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["checker_id"] == "python.ruff")
        .unwrap();
    assert_eq!(row["configuration"], "missing");
    assert_eq!(row["execution"], "not_run");
    assert_eq!(row["reason"], "project_ruff_config_not_found");
    assert!(report.get("findings").is_none());
    assert!(row["required_by_policy"].is_null());
    assert_eq!(report["readiness"], "unknown");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(
        fs::read_to_string(project.0.join("app.py")).unwrap(),
        "import os\n"
    );
}

#[test]
fn maven_and_cargo_package_versions_are_per_manifest_and_not_language_targets() {
    let project = Project::new();
    fs::create_dir(project.0.join("java")).unwrap();
    fs::create_dir(project.0.join("rust")).unwrap();
    let pom = "<project><version>2.3.4-SNAPSHOT</version><properties><maven.compiler.release>17</maven.compiler.release></properties></project>";
    let cargo =
        "[package]\nname='app'\nversion='9.8.7+build.2'\nrust-version='1.85'\nedition='2024'\n";
    fs::write(project.0.join("java/pom.xml"), pom).unwrap();
    fs::write(project.0.join("rust/Cargo.toml"), cargo).unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    let profile: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/project.json")).unwrap())
            .unwrap();
    assert_eq!(
        profile["package_declared_versions"]["java/pom.xml"],
        "2.3.4-SNAPSHOT"
    );
    assert_eq!(
        profile["package_declared_versions"]["rust/Cargo.toml"],
        "9.8.7+build.2"
    );
    assert_eq!(
        profile["manifest_sha256"]["java/pom.xml"],
        format!("{:x}", Sha256::digest(pom.as_bytes()))
    );
    assert_eq!(
        profile["manifest_sha256"]["rust/Cargo.toml"],
        format!("{:x}", Sha256::digest(cargo.as_bytes()))
    );
    for target in profile["language_targets"].as_array().unwrap() {
        assert_ne!(target["value"], "2.3.4-SNAPSHOT");
        assert_ne!(target["value"], "9.8.7+build.2");
    }
    for language in profile["languages"].as_array().unwrap() {
        assert!(language["installed_version"].is_null());
        assert!(language["declared_version"].is_null());
    }
}

#[test]
fn unresolved_package_versions_refresh_without_erasing_work_or_guessing_inheritance() {
    let project = Project::new();
    fs::write(
        project.0.join("pom.xml"),
        "<project><parent><version>8.9.0</version></parent></project>",
    )
    .unwrap();
    fs::write(
        project.0.join("Cargo.toml"),
        "[package]\nname='app'\nversion={workspace=true}\n[workspace.package]\nversion='8.9.0'\n",
    )
    .unwrap();
    assert_eq!(project.run(&["--apply", "--format=json"]).0, 3);
    let before: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/project.json")).unwrap())
            .unwrap();
    assert!(
        before["package_declared_versions"]
            .as_object()
            .unwrap()
            .is_empty()
    );
    for reason in [
        "maven_package_version_not_declared:pom.xml",
        "cargo_package_version_unresolved:Cargo.toml",
    ] {
        assert!(
            before["unknown_conditions"]
                .as_array()
                .unwrap()
                .contains(&Value::from(reason)),
            "{reason}"
        );
    }
    fs::write(project.0.join("codeguard/tasks/notes.md"), "保留人工备注").unwrap();
    let pom = "<project><version>2.0-SNAPSHOT</version></project>";
    let cargo = "[package]\nname='app'\nversion='3.0.0-rc.1'\n";
    fs::write(project.0.join("pom.xml"), pom).unwrap();
    fs::write(project.0.join("Cargo.toml"), cargo).unwrap();
    assert_eq!(
        project.run(&["--apply", "--format=json"]).1["profile_stale"],
        true
    );
    let after: Value =
        serde_json::from_slice(&fs::read(project.0.join("codeguard/project.json")).unwrap())
            .unwrap();
    for (manifest, version, content) in [
        ("pom.xml", "2.0-SNAPSHOT", pom),
        ("Cargo.toml", "3.0.0-rc.1", cargo),
    ] {
        assert_eq!(after["package_declared_versions"][manifest], version);
        assert_eq!(
            after["manifest_sha256"][manifest],
            format!("{:x}", Sha256::digest(content.as_bytes()))
        );
    }
    assert_eq!(
        fs::read_to_string(project.0.join("codeguard/tasks/notes.md")).unwrap(),
        "保留人工备注"
    );
    assert_eq!(
        project.run(&["--apply", "--format=json"]).1["profile_stale"],
        false
    );
    assert!(!project.0.join("Cargo.lock").exists());
    assert!(!project.0.join("target").exists());
}
