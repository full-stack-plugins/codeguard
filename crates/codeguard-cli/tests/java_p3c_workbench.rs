#![cfg(unix)]

use codeguard_cli::tool_identity::hash_bundle_tree;
use serde_json::Value;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Project {
    root: PathBuf,
    tools: PathBuf,
}

impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-p3c-workbench-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let tools = root.with_extension("tools");
        fs::create_dir_all(root.join("src/main/java")).unwrap();
        fs::create_dir_all(tools.join("jdk/bin")).unwrap();
        fs::create_dir_all(tools.join("repo")).unwrap();
        fs::write(tools.join("jdk/bin/java"), "fixture java").unwrap();
        fs::write(tools.join("repo/artifact.jar"), "fixture repo").unwrap();
        fs::write(
            root.join("src/main/java/Bad_Name.java"),
            "class Bad_Name {}\n",
        )
        .unwrap();
        fs::write(
            root.join("pom.xml"),
            include_str!("../../../tests/fixtures/p3c_native/pom.xml"),
        )
        .unwrap();
        let project = Self { root, tools };
        project.tool("ClassNamingShouldBeCamelRule", "AlibabaJavaNaming");
        project
    }

    fn tool(&self, rule: &str, ruleset: &str) {
        fs::write(self.tools.join("mvn"), format!(
            "#!/bin/sh\nprintf 'run\\n' >> '{}'\nmkdir -p target\ncat > target/pmd.xml <<EOF\n<pmd xmlns=\"http://pmd.sourceforge.net/report/2.0.0\" version=\"6.15.0\"><file name=\"$PWD/src/main/java/Bad_Name.java\"><violation beginline=\"1\" endline=\"1\" begincolumn=\"7\" endcolumn=\"14\" rule=\"{rule}\" ruleset=\"{ruleset}\" priority=\"2\">diagnostic</violation></file></pmd>\nEOF\n", self.tools.join("calls").display()
        )).unwrap();
        fs::set_permissions(self.tools.join("mvn"), fs::Permissions::from_mode(0o700)).unwrap();
    }

    fn execute(&self, args: &[&str], native: bool) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        command
            .args(args)
            .env_remove("CODEGUARD_TIMEOUT")
            .env_remove("CODEGUARD_JOBS");
        if !args.iter().any(|arg| arg.starts_with("--format")) {
            command.arg("--format=json");
        }
        if native {
            command
                .arg("--maven-tool")
                .arg(self.tools.join("mvn"))
                .arg("--java-home")
                .arg(self.tools.join("jdk"))
                .arg("--maven-repo")
                .arg(self.tools.join("repo"))
                .arg("--repo-sha256")
                .arg(hash_bundle_tree(&self.tools.join("repo")).unwrap());
        }
        command.output().unwrap()
    }

    fn json(&self, args: &[&str], native: bool) -> Value {
        let output = self.execute(args, native);
        assert_eq!(output.status.code(), Some(3), "{output:?}");
        let report = serde_json::from_slice(&output.stdout).unwrap();
        if let Ok(directory) = std::env::var("CODEGUARD_P3C_WORKBENCH_REPORT_DIR") {
            fs::create_dir_all(&directory).unwrap();
            fs::write(
                PathBuf::from(directory).join(format!(
                    "{}-{}.json",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                )),
                &output.stdout,
            )
            .unwrap();
        }
        report
    }

    fn init(&self) {
        self.json(&["init", self.root.to_str().unwrap(), "--apply"], false);
    }

    fn lint(&self, source: &str) -> Value {
        self.json(&["lint", "java", source], true)
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
        let _ = fs::remove_dir_all(&self.tools);
    }
}

#[test]
fn single_file_lint_and_project_check_reuse_the_same_p3c_task_and_native_recheck() {
    let project = Project::new();
    project.init();
    let source = project.root.join("src/main/java/Bad_Name.java");
    let first = project.lint(source.to_str().unwrap());
    assert_eq!(first["report_type"], "java_p3c_file_feedback");
    assert_eq!(first["workbench"]["status"], "synced_partial", "{first}");
    assert_eq!(first["project_observation"]["source_file_count"], 1);
    assert_eq!(
        first["project_observation"]["files"][0]["observation"]["declared_rulesets"],
        serde_json::json!(["rulesets/java/ali-naming.xml"])
    );
    let id = first["next"]["repair_brief"]["task_id"].as_str().unwrap();
    let second = project.json(&["check", "java", project.root.to_str().unwrap()], true);
    assert_eq!(second["next"]["repair_brief"]["task_id"], id);
    assert_eq!(
        fs::read_dir(project.root.join(".codeguard/tasks"))
            .unwrap()
            .count(),
        1
    );
    assert_eq!(
        fs::read_to_string(project.tools.join("calls"))
            .unwrap()
            .lines()
            .count(),
        2
    );
    let verify = project.json(
        &["task", "verify", id, project.root.to_str().unwrap()],
        true,
    );
    assert_eq!(verify["observation"], "still_present");
    assert_eq!(verify["event_persisted"], true);
    assert_eq!(verify["delivery_decision"], "not_evaluated");
}

#[test]
fn single_file_lint_does_not_scan_sibling_sources() {
    let project = Project::new();
    project.init();
    fs::write(
        project.root.join("src/main/java/Other.java"),
        "class Other {}\n",
    )
    .unwrap();
    let first = project.lint(
        project
            .root
            .join("src/main/java/Bad_Name.java")
            .to_str()
            .unwrap(),
    );
    assert_eq!(first["project_observation"]["source_file_count"], 1);
    assert_eq!(
        fs::read_to_string(project.tools.join("calls"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}

#[test]
fn unselected_rules_are_not_imported_as_project_findings() {
    let project = Project::new();
    project.init();
    project.tool("ClassMustHaveAuthorRule", "AlibabaJavaComments");
    let report = project.lint(
        project
            .root
            .join("src/main/java/Bad_Name.java")
            .to_str()
            .unwrap(),
    );
    assert_eq!(
        report["project_observation"]["findings"],
        serde_json::json!([])
    );
    assert_eq!(
        report["project_observation"]["files"][0]["observation"]["reason"],
        "native_rule_outside_selected_rulesets"
    );
    assert_eq!(report["next"]["repair_brief"]["kind"], "blocker");
}

#[test]
fn nearest_unconfigured_pom_shadows_configured_parent_without_native_launch() {
    let project = Project::new();
    project.init();
    fs::create_dir_all(project.root.join("child/src")).unwrap();
    fs::write(
        project.root.join("child/pom.xml"),
        "<project><modelVersion>4.0.0</modelVersion></project>",
    )
    .unwrap();
    fs::write(
        project.root.join("child/src/Bad_Name.java"),
        "class Bad_Name {}\n",
    )
    .unwrap();
    let report = project.lint(
        project
            .root
            .join("child/src/Bad_Name.java")
            .to_str()
            .unwrap(),
    );
    assert_eq!(
        report["project_observation"]["files"][0]["build_root"],
        "child"
    );
    assert_eq!(
        report["project_observation"]["files"][0]["configuration"],
        "missing"
    );
    assert!(!project.tools.join("calls").exists());
}

#[test]
fn damaged_nearest_workspace_is_not_bypassed_or_reinitialized() {
    let project = Project::new();
    project.init();
    fs::create_dir_all(project.root.join("child/.codeguard")).unwrap();
    fs::write(
        project.root.join("child/.codeguard/workspace.json"),
        "broken",
    )
    .unwrap();
    fs::write(
        project.root.join("child/Bad_Name.java"),
        "class Bad_Name {}\n",
    )
    .unwrap();
    let report = project.lint(project.root.join("child/Bad_Name.java").to_str().unwrap());
    assert_eq!(report["workbench"]["reason"], "workspace_invalid");
    assert!(report["next"].is_null());
    assert!(!project.tools.join("calls").exists());
    assert_eq!(
        fs::read_to_string(project.root.join("child/.codeguard/workspace.json")).unwrap(),
        "broken"
    );
}

#[test]
fn persistence_failure_preserves_diagnostics_without_a_fake_task() {
    let project = Project::new();
    project.init();
    fs::remove_dir_all(project.root.join(".codeguard/reports")).unwrap();
    fs::write(project.root.join(".codeguard/reports"), "blocked").unwrap();
    let report = project.lint(
        project
            .root
            .join("src/main/java/Bad_Name.java")
            .to_str()
            .unwrap(),
    );
    assert_eq!(report["project_observation"]["finding_count"], 1);
    assert_eq!(
        report["workbench"]["reason"],
        "reports_directory_unavailable"
    );
    assert!(report["next"].is_null());
}

#[test]
fn explicit_workspace_rejects_outside_source_and_source_links_before_native_launch() {
    let project = Project::new();
    project.init();
    let external = project.tools.join("Bad_Name.java");
    fs::write(&external, "class Bad_Name {}\n").unwrap();
    let report = project.json(
        &[
            "lint",
            "java",
            external.to_str().unwrap(),
            "--workspace",
            project.root.to_str().unwrap(),
        ],
        true,
    );
    assert_eq!(report["workbench"]["reason"], "source_outside_workspace");
    symlink(
        project.root.join("src/main/java/Bad_Name.java"),
        project.root.join("Linked.java"),
    )
    .unwrap();
    let report = project.json(
        &[
            "lint",
            "java",
            project.root.join("Linked.java").to_str().unwrap(),
            "--workspace",
            project.root.to_str().unwrap(),
        ],
        true,
    );
    assert_eq!(report["workbench"]["reason"], "source_unavailable");
    assert!(!project.tools.join("calls").exists());
}

#[test]
fn zero_native_diagnostics_do_not_close_a_task_with_unproven_rule_coverage() {
    let project = Project::new();
    project.init();
    let first = project.lint(
        project
            .root
            .join("src/main/java/Bad_Name.java")
            .to_str()
            .unwrap(),
    );
    let id = first["next"]["repair_brief"]["task_id"].as_str().unwrap();
    fs::write(project.tools.join("mvn"), "#!/bin/sh\nmkdir -p target\nprintf '%s\\n' '<pmd xmlns=\"http://pmd.sourceforge.net/report/2.0.0\" version=\"6.15.0\"></pmd>' > target/pmd.xml\n").unwrap();
    fs::set_permissions(project.tools.join("mvn"), fs::Permissions::from_mode(0o700)).unwrap();
    let verify = project.json(
        &["task", "verify", id, project.root.to_str().unwrap()],
        true,
    );
    assert_eq!(verify["observation"], "rule_coverage_requires_review");
    let fact: Value = serde_json::from_slice(
        &fs::read(
            project
                .root
                .join(format!(".codeguard/findings/{id}/finding.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}

#[test]
fn unbound_single_file_retains_existing_report_and_never_initializes_a_workspace() {
    let project = Project::new();
    let report = project.lint(
        project
            .root
            .join("src/main/java/Bad_Name.java")
            .to_str()
            .unwrap(),
    );
    assert_eq!(report["report_type"], "java_p3c_local_feedback");
    assert_eq!(report["schema_version"], "0.2.0");
    assert_eq!(report["declared_rulesets"].as_array().unwrap().len(), 10);
    assert!(!project.root.join(".codeguard").exists());
}

#[test]
fn linked_parent_directory_is_rejected_before_native_launch() {
    let project = Project::new();
    project.init();
    symlink(
        project.root.join("src/main/java"),
        project.root.join("alias"),
    )
    .unwrap();
    let report = project.json(
        &[
            "lint",
            "java",
            project.root.join("alias/Bad_Name.java").to_str().unwrap(),
            "--workspace",
            project.root.to_str().unwrap(),
        ],
        true,
    );
    assert_eq!(report["workbench"]["reason"], "source_unavailable");
    assert!(!project.tools.join("calls").exists());
}

#[test]
fn human_feedback_contains_current_native_rule_and_saved_task_identity() {
    let project = Project::new();
    project.init();
    let source = project.root.join("src/main/java/Bad_Name.java");
    let first = project.lint(source.to_str().unwrap());
    let id = first["next"]["repair_brief"]["task_id"].as_str().unwrap();
    let output = project.execute(
        &["lint", "java", source.to_str().unwrap(), "--format=human"],
        true,
    );
    assert_eq!(output.status.code(), Some(3));
    let human = String::from_utf8(output.stdout).unwrap();
    assert!(human.contains("ClassNamingShouldBeCamelRule"));
    assert!(human.contains(id));
    assert!(human.contains("不能签发质量通过"));
}

#[test]
fn explicit_p3c_selection_preserves_configuration_blocker_without_wasm() {
    let project = Project::new();
    project.init();
    fs::write(
        project.root.join("pom.xml"),
        "<project><modelVersion>4.0.0</modelVersion></project>",
    )
    .unwrap();
    let report = project.json(
        &[
            "lint",
            "java",
            project
                .root
                .join("src/main/java/Bad_Name.java")
                .to_str()
                .unwrap(),
            "--checker=p3c",
        ],
        false,
    );
    assert_eq!(report["report_type"], "java_p3c_file_feedback");
    assert_eq!(
        report["project_observation"]["files"][0]["configuration"],
        "missing"
    );
    assert_eq!(report["next"]["repair_brief"]["kind"], "blocker");
    assert!(!project.tools.join("calls").exists());
}

#[test]
fn comment_rule_tasks_do_not_instruct_the_agent_to_fix_naming() {
    let project = Project::new();
    project.init();
    let pom = fs::read_to_string(project.root.join("pom.xml"))
        .unwrap()
        .replace("ali-naming.xml", "ali-comment.xml");
    fs::write(project.root.join("pom.xml"), pom).unwrap();
    project.tool("ClassMustHaveAuthorRule", "AlibabaJavaComments");
    let report = project.lint(
        project
            .root
            .join("src/main/java/Bad_Name.java")
            .to_str()
            .unwrap(),
    );
    let id = report["next"]["repair_brief"]["task_id"].as_str().unwrap();
    let task = fs::read_to_string(project.root.join(format!(".codeguard/tasks/{id}.md"))).unwrap();
    assert!(task.contains("ClassMustHaveAuthorRule"));
    assert!(!task.contains("命名规则"));
    assert!(!task.contains("命名语义"));
    for field in [
        "问题证据",
        "规则依据",
        "允许范围",
        "修复步骤",
        "复检 argv",
        "历史尝试",
        "关闭条件",
    ] {
        assert!(task.contains(field), "missing {field}: {task}");
    }
    assert!(task.contains("p3c"));
}
