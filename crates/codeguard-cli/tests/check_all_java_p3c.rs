#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use codeguard_cli::tool_identity::hash_bundle_tree;
use serde_json::Value;
use sha2::{Digest, Sha256};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[test]
fn javadoc_main_source_scope_is_relative_to_its_build_root() {
    let project = Project::new(false);
    fs::remove_file(project.0.join("src/main/java/Bad_Name.java")).unwrap();
    fs::write(project.0.join("pom.xml"), "<project><modelVersion>4.0.0</modelVersion><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId><configuration><doclint>missing</doclint></configuration></plugin></plugins></build></project>").unwrap();
    fs::create_dir_all(project.0.join("vendor/src/main/java")).unwrap();
    fs::write(
        project.0.join("vendor/src/main/java/Demo.java"),
        "public class Demo {}\n",
    )
    .unwrap();
    let (_, report) = project.check_java(&[]);
    let javadoc = &report["native_results"]["java_javadoc"];
    assert_eq!(
        javadoc["files"][0]["configuration"], "configured",
        "{report}"
    );
    assert_eq!(
        javadoc["files"][0]["reason"], "javadoc_source_scope_unverified",
        "{report}"
    );
    assert!(javadoc["files"][0]["observation"].is_null(), "{report}");
    assert_eq!(javadoc["observed_file_count"], 0);
    assert_eq!(javadoc["local_probe_complete"], false);
}

#[test]
fn javadoc_multifile_does_not_borrow_nested_build_sources() {
    let project = Project::new(false);
    let pom = "<project><modelVersion>4.0.0</modelVersion><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId><configuration><doclint>missing</doclint></configuration></plugin></plugins></build></project>";
    fs::write(project.0.join("pom.xml"), pom).unwrap();
    let nested = project.0.join("src/main/java/nested");
    fs::create_dir_all(nested.join("src/main/java")).unwrap();
    fs::write(nested.join("pom.xml"), pom).unwrap();
    fs::write(
        nested.join("src/main/java/Child.java"),
        "public class Child {}\n",
    )
    .unwrap();
    let missing = project.0.join("missing-maven");
    let (_, report) = project.check_java(&["--maven-tool", missing.to_str().unwrap()]);
    let probes = report["native_results"]["java_javadoc"]["maven_multifile_probes"]
        .as_array()
        .unwrap();
    assert_eq!(probes.len(), 2, "{report}");
    for probe in probes {
        assert_eq!(probe["observation"]["source_count"], 1, "{report}");
        assert_eq!(probe["observation"]["observed_source_count"], 0);
    }
    // 子构建根未配置仍遮蔽父根，不能借父配置重新纳入扫描。
    fs::write(
        nested.join("pom.xml"),
        "<project><modelVersion>4.0.0</modelVersion></project>",
    )
    .unwrap();
    let (_, shadowed) = project.check_java(&["--maven-tool", missing.to_str().unwrap()]);
    let javadoc = &shadowed["native_results"]["java_javadoc"];
    assert_eq!(
        javadoc["maven_multifile_probes"].as_array().unwrap().len(),
        1,
        "{shadowed}"
    );
    assert_eq!(
        javadoc["maven_multifile_probes"][0]["observation"]["source_count"],
        1
    );
    let child = javadoc["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|file| file["path"] == "src/main/java/nested/src/main/java/Child.java")
        .unwrap();
    assert_eq!(child["configuration"], "missing");
    assert_eq!(child["reason"], "javadoc_configuration_not_confirmed");
}

#[test]
#[ignore = "requires explicit Maven, JDK 21 and isolated offline Javadoc plugin repository"]
fn real_maven_javadoc_multifile_probe_keeps_project_authority_unverified() {
    let project = Project::new(false);
    fs::remove_file(project.0.join("src/main/java/Bad_Name.java")).unwrap();
    fs::write(
        project.0.join("src/main/java/First.java"),
        b"package demo;\npublic class First { public Second peer; }\n",
    )
    .unwrap();
    fs::write(
        project.0.join("src/main/java/Second.java"),
        b"package demo;\npublic class Second {}\n",
    )
    .unwrap();
    fs::write(
        project.0.join("pom.xml"),
        "<project><modelVersion>4.0.0</modelVersion><groupId>demo</groupId><artifactId>javadoc-native</artifactId><version>1</version><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId><version>3.12.0</version><configuration><doclint>missing</doclint></configuration></plugin></plugins></build></project>",
    )
    .unwrap();
    let maven = std::env::var("CODEGUARD_MAVEN_BIN").unwrap();
    let java_home = std::env::var("CODEGUARD_JAVA_HOME").unwrap();
    let repo = std::env::var("CODEGUARD_JAVADOC_MAVEN_REPO").unwrap();
    let digest = hash_bundle_tree(&PathBuf::from(&repo)).unwrap();
    let (exit, report) = project.check_java(&[
        "--maven-tool",
        &maven,
        "--java-home",
        &java_home,
        "--maven-repo",
        &repo,
        "--repo-sha256",
        &digest,
    ]);
    assert_eq!(exit, 3);
    if let Ok(directory) = std::env::var("CODEGUARD_TEST_REPORT_DIR") {
        fs::write(
            PathBuf::from(directory).join("maven-javadoc-warning.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }

    let probes = report["native_results"]["java_javadoc"]["maven_multifile_probes"]
        .as_array()
        .unwrap();
    assert_eq!(probes.len(), 1);
    let observation = &probes[0]["observation"];
    assert_eq!(
        observation["native_status"], "findings_observed_untrusted",
        "{observation}"
    );
    assert_eq!(observation["source_count"], 2);
    assert_eq!(observation["observed_source_count"], 2);
    let files = report["native_results"]["java_javadoc"]["files"]
        .as_array()
        .unwrap();
    assert!(
        files.iter().all(|file| file["observation"].is_null()
            && file["reason"] == "maven_multifile_probe_selected"),
        "{files:?}"
    );
    assert_eq!(
        report["native_results"]["java_javadoc"]["probe_mode"],
        "maven_multifile"
    );

    assert_eq!(observation["pom_mode"], "direct_pom_replay");
    assert_eq!(
        observation["native_plan_sha256"],
        format!(
            "{:x}",
            sha2::Sha256::digest(fs::read(project.0.join("pom.xml")).unwrap())
        )
    );
    assert!(observation["findings"].as_array().unwrap().len() >= 2);
    assert_eq!(observation["coverage_proven"], false);
    assert_eq!(observation["project_checker_attribution"], "unverified");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert!(
        report["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| {
                entry["language"] == "java"
                    && entry["category"] == "comments"
                    && entry["reason"] == "javadoc_multifile_probe_unverified_project_coverage"
            })
    );

    fs::write(
        project.0.join("src/main/java/First.java"),
        b"package demo;\n/** First class. */\npublic class First {\n/** Creates first. */ public First() {}\n}\n",
    )
    .unwrap();
    fs::write(
        project.0.join("src/main/java/Second.java"),
        b"package demo;\n/** Second class. */\npublic class Second {\n/** Creates second. */ public Second() {}\n}\n",
    )
    .unwrap();
    let (exit, cleaned) = project.check_java(&[
        "--maven-tool",
        &maven,
        "--java-home",
        &java_home,
        "--maven-repo",
        &repo,
        "--repo-sha256",
        &digest,
    ]);
    assert_eq!(exit, 3);
    let clean_probe =
        &cleaned["native_results"]["java_javadoc"]["maven_multifile_probes"][0]["observation"];
    assert_eq!(
        clean_probe["native_status"], "clean_log_unverified",
        "{clean_probe}"
    );
    assert!(clean_probe["findings"].as_array().unwrap().is_empty());
    assert_eq!(cleaned["delivery_decision"], "not_evaluated");
    if let Ok(directory) = std::env::var("CODEGUARD_TEST_REPORT_DIR") {
        fs::write(
            PathBuf::from(directory).join("maven-javadoc-clean.json"),
            serde_json::to_vec_pretty(&cleaned).unwrap(),
        )
        .unwrap();
    }
}

struct Project(PathBuf);

impl Project {
    fn new(configured: bool) -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-check-java-p3c-{}-{id}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::create_dir_all(root.join("src/main/java")).unwrap();
        fs::write(
            root.join("src/main/java/Bad_Name.java"),
            "class Bad_Name {}\n",
        )
        .unwrap();
        fs::write(
            root.join("pom.xml"),
            if configured {
                include_str!("../../../tests/fixtures/p3c_native/pom.xml")
            } else {
                "<project><modelVersion>4.0.0</modelVersion></project>"
            },
        )
        .unwrap();
        Self(root)
    }

    fn check(&self, extra: &[&str]) -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["check", "all", self.0.to_str().unwrap(), "--format=json"])
            .args(extra)
            .env_remove("CODEGUARD_TIMEOUT")
            .env_remove("CODEGUARD_JOBS")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3), "{:?}", output);
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn check_java(&self, extra: &[&str]) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["check", "java", self.0.to_str().unwrap(), "--format=json"])
            .args(extra)
            .env_remove("CODEGUARD_TIMEOUT")
            .env_remove("CODEGUARD_JOBS")
            .output()
            .unwrap();
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    }

    fn init(&self) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init", self.0.to_str().unwrap(), "--apply", "--format=json"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3), "{:?}", output);
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn missing_p3c_configuration_is_reported_without_running_maven() {
    let project = Project::new(false);
    let marker = project.0.join("native-launched");
    let tool = project.0.join("mvn");
    fs::write(&tool, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let report = project.check(&["--maven-tool", tool.to_str().unwrap()]);
    assert!(!marker.exists());
    assert_eq!(report["schema_version"], "0.38.0");
    assert_eq!(report["execution_tasks"][0]["id"], "java.p3c");
    assert_eq!(report["execution_tasks"][0]["status"], "native_incomplete");
    let java = &report["native_results"]["java_p3c"];
    assert_eq!(java["source_file_count"], 1);
    assert_eq!(java["observed_file_count"], 0);
    assert_eq!(java["files"][0]["configuration"], "missing");
    assert_eq!(
        java["files"][0]["reason"],
        "p3c_configuration_not_confirmed"
    );
    assert!(java["files"][0]["observation"].is_null());
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn check_java_runs_configured_static_dependency_tree_in_private_snapshot() {
    use std::io::Write;
    let project = Project::new(false);
    fs::write(project.0.join("pom.xml"), "<project><modelVersion>4.0.0</modelVersion><groupId>cg.test</groupId><artifactId>demo</artifactId><version>1</version><build><plugins><plugin><artifactId>maven-dependency-plugin</artifactId><version>3.8.1</version></plugin></plugins></build><dependencies><dependency><groupId>junit</groupId><artifactId>junit</artifactId><version>4.13.2</version><scope>test</scope></dependency></dependencies></project>").unwrap();
    let marker = project.0.join("native-cwd");
    let maven = project.0.join("mvn-dependency");
    fs::write(&maven, format!("#!/bin/sh\npwd > '{}'\nfor arg in \"$@\"; do\n case \"$arg\" in -DoutputFile=*) out=${{arg#-DoutputFile=}} ;; esac\ndone\ncat > \"$out\" <<'EOF'\n{{\"groupId\":\"cg.test\",\"artifactId\":\"demo\",\"version\":\"1\",\"type\":\"jar\",\"scope\":\"\",\"classifier\":\"\",\"optional\":\"false\",\"children\":[{{\"groupId\":\"junit\",\"artifactId\":\"junit\",\"version\":\"4.13.2\",\"type\":\"jar\",\"scope\":\"test\",\"classifier\":\"\",\"optional\":\"false\"}}]}}\nEOF\n", marker.display())).unwrap();
    fs::set_permissions(&maven, fs::Permissions::from_mode(0o700)).unwrap();
    writeln!(
        fs::OpenOptions::new().append(true).open(&maven).unwrap(),
        "printf '[INFO] --- dependency:3.8.1:tree (default-cli) @ demo ---\\n[INFO] BUILD SUCCESS\\n'"
    )
    .unwrap();
    let java_home = project.0.join("jdk");
    fs::create_dir_all(java_home.join("bin")).unwrap();
    fs::write(java_home.join("bin/java"), b"fake-java\n").unwrap();
    fs::write(java_home.join("release"), b"JAVA_VERSION=\"21.0.1\"\n").unwrap();
    let repo = project.0.join("repo");
    fs::create_dir(&repo).unwrap();
    fs::create_dir_all(repo.join("junit/junit/4.13.2")).unwrap();
    fs::write(
        repo.join("junit/junit/4.13.2/junit-4.13.2.jar"),
        b"fixture-junit-artifact",
    )
    .unwrap();
    let repo_sha = hash_bundle_tree(&repo).unwrap();
    let (exit, report) = project.check_java(&[
        "--maven-tool",
        maven.to_str().unwrap(),
        "--java-home",
        java_home.to_str().unwrap(),
        "--maven-repo",
        repo.to_str().unwrap(),
        "--repo-sha256",
        &repo_sha,
    ]);
    assert_eq!(exit, 3);
    let native = &report["native_results"]["java_dependencies"];
    assert_eq!(
        native["probes"][0]["observation"]["native_status"], "graph_observed_untrusted",
        "{report}"
    );
    assert_eq!(
        native["probes"][0]["observation"]["nodes"][1]["artifact_id"],
        "junit"
    );
    assert_eq!(
        native["probes"][0]["observation"]["nodes"][1]["artifact_sha256"],
        format!("{:x}", Sha256::digest(b"fixture-junit-artifact"))
    );
    assert_eq!(
        native["probes"][0]["observation"]["edges"][0],
        serde_json::json!([0, 1])
    );
    assert_ne!(
        fs::read_to_string(marker).unwrap().trim(),
        project.0.to_str().unwrap()
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
    let human = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "java",
            project.0.to_str().unwrap(),
            "--format=human",
            "--maven-tool",
            maven.to_str().unwrap(),
            "--java-home",
            java_home.to_str().unwrap(),
            "--maven-repo",
            repo.to_str().unwrap(),
            "--repo-sha256",
            &repo_sha,
        ])
        .output()
        .unwrap();
    assert_eq!(human.status.code(), Some(3));
    let shown = String::from_utf8(human.stdout).unwrap();
    assert!(shown.contains("Java/Maven 依赖图 .: graph_observed_untrusted"));
    assert!(shown.contains("junit:junit:4.13.2"));
    assert!(shown.contains("未检查 CVE/许可证"));
}

#[test]
#[ignore = "requires explicit Maven 3.9, JDK, and a pinned offline Maven repository with Dependency Plugin 3.8.1 and JUnit"]
fn real_maven_dependency_tree_reaches_check_java_without_cve_claim() {
    let project = Project::new(false);
    fs::write(project.0.join("pom.xml"), "<project><modelVersion>4.0.0</modelVersion><groupId>cg.test</groupId><artifactId>demo</artifactId><version>1</version><build><plugins><plugin><artifactId>maven-dependency-plugin</artifactId><version>3.8.1</version></plugin></plugins></build><dependencies><dependency><groupId>junit</groupId><artifactId>junit</artifactId><version>4.13.2</version><scope>test</scope></dependency></dependencies></project>").unwrap();
    let maven = std::env::var("CODEGUARD_MAVEN_BIN").unwrap();
    let java_home = std::env::var("CODEGUARD_JAVA_HOME").unwrap();
    let repo = std::env::var("CODEGUARD_DEP_MAVEN_REPO").unwrap();
    let digest = hash_bundle_tree(&PathBuf::from(&repo)).unwrap();
    let (exit, report) = project.check_java(&[
        "--maven-tool",
        &maven,
        "--java-home",
        &java_home,
        "--maven-repo",
        &repo,
        "--repo-sha256",
        &digest,
    ]);
    assert_eq!(exit, 3);
    let observation = &report["native_results"]["java_dependencies"]["probes"][0]["observation"];
    assert_eq!(
        observation["native_status"], "graph_observed_untrusted",
        "{report}"
    );
    assert_eq!(observation["nodes"][1]["artifact_id"], "junit");
    assert_eq!(
        observation["nodes"][2]["artifact_id"], "hamcrest-core",
        "{report}"
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
    let cve = report["category_candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["language"] == "java" && entry["category"] == "cve")
        .unwrap();
    assert_ne!(cve["status"], "observed_unverified");
}

#[test]
fn check_java_distinguishes_declared_dependency_and_cve_checkers_from_missing_security() {
    let project = Project::new(false);
    fs::write(
        project.0.join("pom.xml"),
        "<project><modelVersion>4.0.0</modelVersion><build><plugins><plugin><artifactId>maven-dependency-plugin</artifactId></plugin><plugin><groupId>org.owasp</groupId><artifactId>dependency-check-maven</artifactId></plugin></plugins></build></project>",
    )
    .unwrap();
    let (exit, report) = project.check_java(&[]);
    assert_eq!(exit, 3);
    for category in ["dependencies", "cve"] {
        let candidate = report["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["language"] == "java" && entry["category"] == category)
            .unwrap();
        assert_eq!(candidate["status"], "native_incomplete");
        assert_eq!(candidate["reason"], "native_scan_incomplete");
        assert_eq!(
            candidate["next_action"],
            if category == "dependencies" {
                "补齐 Maven/JDK/离线仓库身份或修正原生报告，再运行依赖图探针"
            } else {
                "补齐原生工具和离线漏洞库身份，核验数据库时效及漏洞归属后重新检查"
            }
        );
    }
    assert!(
        report["execution_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|task| task["id"] == "java.dependencies")
    );
    let security = report["category_candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["language"] == "java" && entry["category"] == "security")
        .unwrap();
    assert_eq!(security["status"], "not_configured");
    assert_eq!(security["reason"], "recognized_checker_not_configured");
    assert_eq!(security["checker_id"], "java.maven.findsecbugs");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    let human = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "java",
            project.0.to_str().unwrap(),
            "--format=human",
        ])
        .output()
        .unwrap();
    assert_eq!(human.status.code(), Some(3));
    let shown = String::from_utf8(human.stdout).unwrap();
    assert!(shown.contains(
        "候选 java / cve (java.maven.dependency_check): native_incomplete（native_scan_incomplete）"
    ));
    assert_eq!(
        report["native_results"]["java_cve"]["probes"][0]["observation"]["reason"],
        "project_pom_replay_ineligible"
    );
}

#[test]
fn dependency_only_maven_project_still_schedules_its_native_graph_probe() {
    let project = Project::new(false);
    fs::remove_file(project.0.join("src/main/java/Bad_Name.java")).unwrap();
    fs::write(project.0.join("pom.xml"), "<project><modelVersion>4.0.0</modelVersion><groupId>cg.test</groupId><artifactId>dependency-only</artifactId><version>1</version><build><plugins><plugin><artifactId>maven-dependency-plugin</artifactId><version>3.8.1</version></plugin></plugins></build><dependencies><dependency><groupId>junit</groupId><artifactId>junit</artifactId><version>4.13.2</version></dependency></dependencies></project>").unwrap();
    let (exit, report) = project.check_java(&[]);
    assert_eq!(exit, 3);
    assert!(
        report["execution_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|task| task["id"] == "java.dependencies")
    );
    let graph = &report["native_results"]["java_dependencies"];
    assert_eq!(graph["source_file_count"], 0);
    assert_eq!(graph["configured_build_root_count"], 1);
    assert_eq!(
        graph["probes"][0]["observation"]["reason"],
        "prerequisites_missing"
    );
    let category = report["category_candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["language"] == "java" && entry["category"] == "dependencies")
        .unwrap();
    assert_eq!(category["checker_id"], "java.maven.dependency");
    assert_eq!(category["status"], "native_incomplete");
    assert!(
        !report["execution_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|task| task["id"] == "java.p3c")
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
    let all = project.check(&[]);
    assert!(
        all["execution_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|task| task["id"] == "java.dependencies")
    );
    assert_eq!(all["delivery_decision"], "incomplete");
}

#[test]
fn source_less_nested_maven_dependency_root_is_not_hidden_by_parent_sources() {
    let project = Project::new(false);
    let configured_pom = "<project><modelVersion>4.0.0</modelVersion><groupId>cg.test</groupId><artifactId>demo</artifactId><version>1</version><build><plugins><plugin><artifactId>maven-dependency-plugin</artifactId><version>3.8.1</version></plugin></plugins></build></project>";
    fs::write(project.0.join("pom.xml"), configured_pom).unwrap();
    fs::create_dir(project.0.join("child")).unwrap();
    fs::write(project.0.join("child/pom.xml"), configured_pom).unwrap();
    let (exit, report) = project.check_java(&[]);
    assert_eq!(exit, 3);
    let graph = &report["native_results"]["java_dependencies"];
    assert_eq!(graph["configured_build_root_count"], 2);
    let roots: Vec<_> = graph["probes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|probe| probe["build_root"].as_str().unwrap())
        .collect();
    assert_eq!(roots, [".", "child"]);
}

#[test]
fn mixed_build_roots_do_not_claim_java_cve_checker_is_uniformly_configured() {
    let project = Project::new(false);
    fs::write(project.0.join("pom.xml"),
        "<project><modelVersion>4.0.0</modelVersion><build><plugins><plugin><groupId>org.owasp</groupId><artifactId>dependency-check-maven</artifactId></plugin></plugins></build></project>").unwrap();
    fs::create_dir_all(project.0.join("child/src/main/java")).unwrap();
    fs::write(
        project.0.join("child/pom.xml"),
        "<project><modelVersion>4.0.0</modelVersion></project>",
    )
    .unwrap();
    fs::write(
        project.0.join("child/src/main/java/Child.java"),
        "class Child {}\n",
    )
    .unwrap();
    let (exit, report) = project.check_java(&[]);
    assert_eq!(exit, 3);
    let cve = report["category_candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["language"] == "java" && entry["category"] == "cve")
        .unwrap();
    assert_eq!(cve["status"], "configuration_unresolved");
    assert_eq!(cve["reason"], "checker_configuration_mixed_across_sources");
    assert_eq!(
        cve["next_action"],
        "逐构建根核对缺失或未知配置，再运行原生检查"
    );
}

#[test]
fn skipped_cve_plugin_is_configuration_problem_not_clean_scan() {
    let project = Project::new(false);
    fs::write(project.0.join("pom.xml"),
        "<project><modelVersion>4.0.0</modelVersion><build><plugins><plugin><groupId>org.owasp</groupId><artifactId>dependency-check-maven</artifactId><configuration><skip>true</skip></configuration></plugin></plugins></build></project>").unwrap();
    let (exit, report) = project.check_java(&[]);
    assert_eq!(exit, 3);
    let cve = report["category_candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["language"] == "java" && entry["category"] == "cve")
        .unwrap();
    assert_eq!(cve["status"], "configuration_unresolved");
    assert_eq!(cve["reason"], "checker_configuration_unresolved");
    assert!(
        !report["execution_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|task| task["id"] == "java.cve")
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
fn gradle_project_does_not_claim_a_maven_cve_checker_identity() {
    let project = Project::new(false);
    fs::remove_file(project.0.join("pom.xml")).unwrap();
    fs::write(project.0.join("build.gradle.kts"), "plugins { java }\n").unwrap();
    let (exit, report) = project.check_java(&[]);
    assert_eq!(exit, 3);
    let cve = report["category_candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["language"] == "java" && entry["category"] == "cve")
        .unwrap();
    assert_eq!(cve["checker_id"], Value::Null);
    assert_eq!(cve["status"], "configuration_unresolved");
    assert_eq!(cve["reason"], "gradle_model_not_resolved");
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
fn mixed_maven_and_gradle_sources_keep_cve_build_system_unresolved() {
    let project = Project::new(false);
    fs::write(project.0.join("pom.xml"),
        "<project><modelVersion>4.0.0</modelVersion><build><plugins><plugin><groupId>org.owasp</groupId><artifactId>dependency-check-maven</artifactId></plugin></plugins></build></project>").unwrap();
    fs::create_dir_all(project.0.join("child/src/main/java")).unwrap();
    fs::write(
        project.0.join("child/build.gradle.kts"),
        "plugins { java }\n",
    )
    .unwrap();
    fs::write(
        project.0.join("child/src/main/java/Child.java"),
        "class Child {}\n",
    )
    .unwrap();
    let (exit, report) = project.check_java(&[]);
    assert_eq!(exit, 3);
    let cve = report["category_candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["language"] == "java" && entry["category"] == "cve")
        .unwrap();
    assert_eq!(cve["checker_id"], Value::Null);
    assert_eq!(cve["status"], "configuration_unresolved");
    assert_eq!(cve["reason"], "checker_build_systems_mixed");
}

#[test]
fn check_java_reports_disabled_javadoc_missing_checks_as_configuration_problem() {
    let project = Project::new(false);
    fs::write(
        project.0.join("pom.xml"),
        "<project><modelVersion>4.0.0</modelVersion><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId><configuration><doclint>all,-missing</doclint></configuration></plugin></plugins></build></project>",
    )
    .unwrap();
    let (exit, report) = project.check_java(&[]);
    assert_eq!(exit, 3);
    let javadoc = report["discovery"]["checker_configurations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["checker_id"] == "java.maven.javadoc")
        .unwrap();
    assert_eq!(javadoc["configuration"], "invalid");
    assert_eq!(javadoc["reason"], "javadoc_missing_check_disabled");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["native_results"]["java_javadoc"], Value::Null);
    let comments = report["category_candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|candidate| candidate["language"] == "java" && candidate["category"] == "comments")
        .unwrap();
    assert_eq!(comments["status"], "configuration_unresolved");
}

#[test]
fn check_java_observes_configured_javadoc_without_certifying_project_coverage() {
    let project = Project::new(false);
    fs::write(
        project.0.join("pom.xml"),
        "<project><modelVersion>4.0.0</modelVersion><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId><configuration><doclint>missing</doclint></configuration></plugin></plugins></build></project>",
    )
    .unwrap();
    let jdk = project.0.join("jdk");
    fs::create_dir_all(jdk.join("bin")).unwrap();
    fs::write(jdk.join("bin/java"), b"fixture java").unwrap();
    fs::write(jdk.join("release"), b"JAVA_VERSION=\"21.0.12\"\n").unwrap();
    let tool = jdk.join("bin/javadoc");
    fs::write(
        &tool,
        "#!/bin/sh\nmkdir -p docs\nprintf '<html></html>' > docs/index.html\nprintf '%s:1: warning: no comment\\nclass Bad_Name {}\\n^\\n1 warning\\n' \"$PWD/src/Bad_Name.java\" >&2\n",
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let (exit, report) = project.check_java(&["--java-home", jdk.to_str().unwrap()]);
    assert_eq!(exit, 3);
    assert_eq!(
        report["native_results"]["java_javadoc"]["checker_id"],
        "java.jdk.javadoc"
    );
    assert_eq!(
        report["native_results"]["java_javadoc"]["files"][0]["observation"]["findings"][0]["rule_id"],
        "JavadocMissingComment"
    );
    assert_eq!(
        report["native_results"]["java_javadoc"]["coverage_proven"],
        false
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["execution_budget"]["native_task_count"], 2);
    assert!(
        report["execution_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|task| task["id"] == "java.javadoc")
    );
}

#[test]
fn check_java_only_schedules_java_and_never_certifies_delivery() {
    let project = Project::new(false);
    fs::write(project.0.join("helper.py"), "import missing\n").unwrap();
    fs::write(
        project.0.join("Cargo.toml"),
        "[package]\nname='mixed'\nversion='0.1.0'\n",
    )
    .unwrap();
    fs::write(project.0.join("main.rs"), "fn main() {}\n").unwrap();
    let marker = project.0.join("mvn-launched");
    let maven = project.0.join("mvn");
    fs::write(&maven, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
    fs::set_permissions(&maven, fs::Permissions::from_mode(0o700)).unwrap();
    let (exit, report) = project.check_java(&["--maven-tool", maven.to_str().unwrap()]);
    assert_eq!(exit, 3);
    assert!(!marker.exists());
    assert_eq!(report["selection"], "java");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["native_results"]["python_lint"], Value::Null);
    assert_eq!(report["native_results"]["rust_lint"], Value::Null);
    assert_eq!(report["execution_budget"]["native_task_count"], 1);
    assert_eq!(report["execution_tasks"].as_array().unwrap().len(), 1);
    assert_eq!(report["execution_tasks"][0]["id"], "java.p3c");
    assert_eq!(report["native_results"]["java_javadoc"], Value::Null);
    let comments = report["category_candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|candidate| candidate["language"] == "java" && candidate["category"] == "comments")
        .unwrap();
    assert_eq!(comments["status"], "not_configured");
    assert!(
        report["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .all(|candidate| candidate["language"] == "java")
    );
}

#[test]
fn check_java_next_ignores_existing_python_tasks() {
    let project = Project::new(false);
    fs::write(project.0.join("helper.py"), "import missing\n").unwrap();
    project.init();
    let _ = project.check(&[]);
    let (exit, report) = project.check_java(&[]);
    assert_eq!(exit, 3);
    assert_eq!(
        report["next"]["repair_brief"]["checker_id"], "java.maven.p3c",
        "{report}"
    );
    assert_eq!(
        report["next"]["repair_brief"]["reason_code"],
        "p3c_configuration_not_confirmed"
    );
    assert!(
        report["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .all(|candidate| candidate["language"] == "java")
    );
}

#[test]
fn check_java_without_java_target_stays_incomplete() {
    let project = Project::new(false);
    fs::remove_file(project.0.join("src/main/java/Bad_Name.java")).unwrap();
    fs::write(project.0.join("helper.py"), "print('python')\n").unwrap();
    let (exit, report) = project.check_java(&[]);
    assert_eq!(exit, 3);
    assert_eq!(report["selection"], "java");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["execution_budget"]["native_task_count"], 0);
    assert!(
        report["unresolved_conditions"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("java_target_absent_or_unobserved"))
    );
}

#[test]
fn missing_p3c_configuration_creates_one_stable_environment_task() {
    let project = Project::new(false);
    project.init();
    let first = project.check(&[]);
    assert_eq!(
        first["native_results"]["java_p3c"]["backlog_status"], "synced_partial",
        "{first}"
    );
    let brief = &first["next"]["repair_brief"];
    assert_eq!(brief["kind"], "blocker");
    assert_eq!(brief["checker_id"], "java.maven.p3c");
    assert_eq!(brief["reason_code"], "p3c_configuration_not_confirmed");
    assert_eq!(brief["disposition"], "needs_decision");
    let id = brief["task_id"].as_str().unwrap();
    let second = project.check(&[]);
    assert_eq!(second["next"]["repair_brief"]["task_id"], id);
    assert_eq!(
        fs::read_dir(project.0.join(".codeguard/tasks"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn java_environment_task_requires_original_checker_recheck_after_configuration_repair() {
    let project = Project::new(false);
    project.init();
    let first = project.check(&[]);
    let id = first["next"]["repair_brief"]["task_id"].as_str().unwrap();
    fs::write(
        project.0.join("pom.xml"),
        include_str!("../../../tests/fixtures/p3c_native/pom.xml"),
    )
    .unwrap();
    let tool = project.0.join("mvn");
    fs::write(&tool, "#!/bin/sh\nmkdir -p target\nprintf '%s\\n' '<pmd xmlns=\"http://pmd.sourceforge.net/report/2.0.0\" version=\"6.15.0\"></pmd>' > target/pmd.xml\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let java_home = project.0.join("jdk");
    fs::create_dir_all(java_home.join("bin")).unwrap();
    fs::write(java_home.join("bin/java"), b"fixture java").unwrap();
    let repo = project.0.join("repo");
    fs::create_dir(&repo).unwrap();
    fs::write(repo.join("artifact.jar"), b"fixture").unwrap();
    let digest = hash_bundle_tree(&repo).unwrap();
    let verify = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            project.0.to_str().unwrap(),
            "--format=json",
            "--maven-tool",
            tool.to_str().unwrap(),
            "--java-home",
            java_home.to_str().unwrap(),
            "--maven-repo",
            repo.to_str().unwrap(),
            "--repo-sha256",
            &digest,
        ])
        .output()
        .unwrap();
    assert_eq!(verify.status.code(), Some(3));
    let verify: Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(
        verify["observation"], "environment_restored_unverified_policy",
        "{verify}"
    );
    assert_eq!(verify["event_persisted"], true);
    let fact: Value = serde_json::from_slice(
        &fs::read(
            project
                .0
                .join(".codeguard/findings")
                .join(id)
                .join("finding.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}

#[test]
fn configured_p3c_native_finding_is_visible_in_unified_feedback() {
    let project = Project::new(true);
    let tool = project.0.join("mvn");
    fs::write(
        &tool,
        "#!/bin/sh\nmkdir -p target\ncat > target/pmd.xml <<EOF\n<pmd xmlns=\"http://pmd.sourceforge.net/report/2.0.0\" version=\"6.15.0\"><file name=\"$PWD/src/main/java/Bad_Name.java\"><violation beginline=\"1\" endline=\"1\" begincolumn=\"7\" endcolumn=\"14\" rule=\"ClassNamingShouldBeCamelRule\" ruleset=\"AlibabaJavaNaming\" priority=\"2\">bad name</violation></file></pmd>\nEOF\n",
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let java_home = project.0.join("jdk");
    fs::create_dir_all(java_home.join("bin")).unwrap();
    fs::write(java_home.join("bin/java"), b"fixture java").unwrap();
    let repo = project.0.join("repo");
    fs::create_dir(&repo).unwrap();
    fs::write(repo.join("artifact.jar"), b"fixture").unwrap();
    let digest = hash_bundle_tree(&repo).unwrap();
    let report = project.check(&[
        "--maven-tool",
        tool.to_str().unwrap(),
        "--java-home",
        java_home.to_str().unwrap(),
        "--maven-repo",
        repo.to_str().unwrap(),
        "--repo-sha256",
        &digest,
    ]);
    let java = &report["native_results"]["java_p3c"];
    assert_eq!(java["local_observation_complete"], true, "{java}");
    assert_eq!(java["coverage_proven"], false);
    assert_eq!(java["files"][0]["observation"]["schema_version"], "0.2.0");
    assert_eq!(
        java["files"][0]["observation"]["declared_rulesets"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        java["files"][0]["observation"]["declared_rulesets"][0],
        "rulesets/java/ali-naming.xml"
    );
    assert_eq!(java["finding_count"], 1);
    assert_eq!(
        java["files"][0]["observation"]["findings"][0]["rule_id"],
        "ClassNamingShouldBeCamelRule"
    );
    assert_eq!(
        report["execution_tasks"][0]["status"],
        "native_observed_unverified"
    );
    assert!(
        report["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| {
                candidate["language"] == "java"
                    && candidate["category"] == "lint"
                    && candidate["status"] == "native_incomplete"
                    && candidate["reason"] == "p3c_declared_rulesets_unverified_coverage"
            })
    );
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn unknown_p3c_ruleset_never_starts_native_probe() {
    let project = Project::new(true);
    let pom = fs::read_to_string(project.0.join("pom.xml")).unwrap();
    fs::write(
        project.0.join("pom.xml"),
        pom.replace("rulesets/java/ali-naming.xml", "rulesets/java/unknown.xml"),
    )
    .unwrap();
    let marker = project.0.join("native-launched");
    let tool = project.0.join("mvn");
    fs::write(&tool, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let report = project.check(&["--maven-tool", tool.to_str().unwrap()]);
    assert!(!marker.exists());
    let java = &report["native_results"]["java_p3c"];
    assert_eq!(java["files"][0]["configuration"], "unknown");
    assert_eq!(
        java["files"][0]["reason"],
        "p3c_configuration_not_confirmed"
    );
    assert!(java["findings"].as_array().unwrap().is_empty());
}

#[test]
fn changing_project_pom_during_native_scan_drops_findings() {
    let project = Project::new(true);
    let tool = project.0.join("mvn");
    fs::write(
        &tool,
        format!(
            "#!/bin/sh\nprintf '\\n' >> '{}'\nmkdir -p target\ncat > target/pmd.xml <<EOF\n<pmd xmlns=\"http://pmd.sourceforge.net/report/2.0.0\" version=\"6.15.0\"><file name=\"$PWD/src/main/java/Bad_Name.java\"><violation beginline=\"1\" endline=\"1\" begincolumn=\"7\" endcolumn=\"14\" rule=\"ClassNamingShouldBeCamelRule\" ruleset=\"AlibabaJavaNaming\" priority=\"2\">bad name</violation></file></pmd>\nEOF\n",
            project.0.join("pom.xml").display()
        ),
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let java_home = project.0.join("jdk");
    fs::create_dir_all(java_home.join("bin")).unwrap();
    fs::write(java_home.join("bin/java"), b"fixture java").unwrap();
    let repo = project.0.join("repo");
    fs::create_dir(&repo).unwrap();
    fs::write(repo.join("artifact.jar"), b"fixture").unwrap();
    let digest = hash_bundle_tree(&repo).unwrap();
    let report = project.check(&[
        "--maven-tool",
        tool.to_str().unwrap(),
        "--java-home",
        java_home.to_str().unwrap(),
        "--maven-repo",
        repo.to_str().unwrap(),
        "--repo-sha256",
        &digest,
    ]);
    let java = &report["native_results"]["java_p3c"];
    assert_eq!(
        java["files"][0]["reason"],
        "p3c_configuration_changed_during_scan"
    );
    assert_eq!(java["observed_file_count"], 0);
    assert!(java["findings"].as_array().unwrap().is_empty());
}

#[test]
fn native_diagnostic_from_unselected_p3c_ruleset_is_not_a_project_finding() {
    let project = Project::new(true);
    let tool = project.0.join("mvn");
    fs::write(
        &tool,
        "#!/bin/sh\nmkdir -p target\ncat > target/pmd.xml <<EOF\n<pmd xmlns=\"http://pmd.sourceforge.net/report/2.0.0\" version=\"6.15.0\"><file name=\"$PWD/src/main/java/Bad_Name.java\"><violation beginline=\"1\" endline=\"1\" begincolumn=\"7\" endcolumn=\"14\" rule=\"ClassMustHaveAuthorRule\" ruleset=\"AlibabaJavaComments\" priority=\"3\">missing author</violation></file></pmd>\nEOF\n",
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let java_home = project.0.join("jdk");
    fs::create_dir_all(java_home.join("bin")).unwrap();
    fs::write(java_home.join("bin/java"), b"fixture java").unwrap();
    let repo = project.0.join("repo");
    fs::create_dir(&repo).unwrap();
    fs::write(repo.join("artifact.jar"), b"fixture").unwrap();
    let digest = hash_bundle_tree(&repo).unwrap();
    let report = project.check(&[
        "--maven-tool",
        tool.to_str().unwrap(),
        "--java-home",
        java_home.to_str().unwrap(),
        "--maven-repo",
        repo.to_str().unwrap(),
        "--repo-sha256",
        &digest,
    ]);
    let java = &report["native_results"]["java_p3c"];
    assert_eq!(
        java["files"][0]["observation"]["reason"],
        "native_rule_outside_selected_rulesets"
    );
    assert_eq!(java["observed_file_count"], 0);
    assert!(java["findings"].as_array().unwrap().is_empty());
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn work_sync_rejects_a_replayed_java_report_with_wrong_ruleset_identity() {
    let project = Project::new(true);
    project.init();
    let tool = project.0.join("mvn");
    fs::write(&tool, "#!/bin/sh\nmkdir -p target\ncat > target/pmd.xml <<EOF\n<pmd xmlns=\"http://pmd.sourceforge.net/report/2.0.0\" version=\"6.15.0\"><file name=\"$PWD/src/main/java/Bad_Name.java\"><violation beginline=\"1\" endline=\"1\" begincolumn=\"7\" endcolumn=\"14\" rule=\"ClassNamingShouldBeCamelRule\" ruleset=\"AlibabaJavaNaming\" priority=\"2\">bad name</violation></file></pmd>\nEOF\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let java_home = project.0.join("jdk");
    fs::create_dir_all(java_home.join("bin")).unwrap();
    fs::write(java_home.join("bin/java"), b"fixture java").unwrap();
    let repo = project.0.join("repo");
    fs::create_dir(&repo).unwrap();
    fs::write(repo.join("artifact.jar"), b"fixture").unwrap();
    let digest = hash_bundle_tree(&repo).unwrap();
    let first = project.check(&[
        "--maven-tool",
        tool.to_str().unwrap(),
        "--java-home",
        java_home.to_str().unwrap(),
        "--maven-repo",
        repo.to_str().unwrap(),
        "--repo-sha256",
        &digest,
    ]);
    let run_id = first["native_results"]["java_p3c"]["run_id"]
        .as_str()
        .unwrap();
    let saved = project.0.join(format!(".codeguard/reports/{run_id}.json"));
    let original: Value = serde_json::from_slice(&fs::read(saved).unwrap()).unwrap();
    let mut replay = original.clone();
    let replay_id = format!("{run_id}-replayed");
    replay["run_id"] = serde_json::json!(replay_id);
    replay["files"][0]["observation"]["findings"][0]["ruleset"] =
        serde_json::json!("AlibabaJavaComments");
    fs::write(
        project
            .0
            .join(format!(".codeguard/reports/{replay_id}.json")),
        serde_json::to_vec_pretty(&replay).unwrap(),
    )
    .unwrap();
    for (suffix, field, value) in [
        (
            "selection",
            "declared_rulesets",
            serde_json::json!(["rulesets/java/ali-comment.xml"]),
        ),
        (
            "plan",
            "native_plan_sha256",
            serde_json::json!("0".repeat(64)),
        ),
    ] {
        let mut altered = original.clone();
        let altered_id = format!("{run_id}-{suffix}");
        altered["run_id"] = serde_json::json!(altered_id);
        altered["files"][0]["observation"][field] = value;
        fs::write(
            project
                .0
                .join(format!(".codeguard/reports/{altered_id}.json")),
            serde_json::to_vec_pretty(&altered).unwrap(),
        )
        .unwrap();
    }
    let sync = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["work", "sync", project.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    assert_eq!(sync.status.code(), Some(3));
    let result: Value = serde_json::from_slice(&sync.stdout).unwrap();
    assert_eq!(result["failed_reports"], 3, "{result}");
    assert_eq!(result["new_findings"], 0);
}

#[test]
fn configured_p3c_finding_syncs_once_and_verifies_with_maven() {
    let project = Project::new(true);
    project.init();
    let tool = project.0.join("mvn");
    fs::write(
        &tool,
        "#!/bin/sh\nmkdir -p target\ncat > target/pmd.xml <<EOF\n<pmd xmlns=\"http://pmd.sourceforge.net/report/2.0.0\" version=\"6.15.0\"><file name=\"$PWD/src/main/java/Bad_Name.java\"><violation beginline=\"1\" endline=\"1\" begincolumn=\"7\" endcolumn=\"14\" rule=\"ClassNamingShouldBeCamelRule\" ruleset=\"AlibabaJavaNaming\" priority=\"2\">bad name</violation></file></pmd>\nEOF\n",
    ).unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let java_home = project.0.join("jdk");
    fs::create_dir_all(java_home.join("bin")).unwrap();
    fs::write(java_home.join("bin/java"), b"fixture java").unwrap();
    let repo = project.0.join("repo");
    fs::create_dir(&repo).unwrap();
    fs::write(repo.join("artifact.jar"), b"fixture").unwrap();
    let digest = hash_bundle_tree(&repo).unwrap();
    let args = [
        "--maven-tool",
        tool.to_str().unwrap(),
        "--java-home",
        java_home.to_str().unwrap(),
        "--maven-repo",
        repo.to_str().unwrap(),
        "--repo-sha256",
        digest.as_str(),
    ];
    let first = project.check(&args);
    assert_eq!(
        first["native_results"]["java_p3c"]["backlog_status"], "synced_partial",
        "{first}"
    );
    let brief = &first["next"]["repair_brief"];
    assert_eq!(brief["kind"], "finding");
    assert_eq!(brief["checker_id"], "java.maven.p3c");
    assert_eq!(brief["native_rule_id"], "ClassNamingShouldBeCamelRule");
    let id = brief["task_id"].as_str().unwrap();
    let second = project.check(&args);
    assert_eq!(second["next"]["repair_brief"]["task_id"], id);
    assert_eq!(
        fs::read_dir(project.0.join(".codeguard/tasks"))
            .unwrap()
            .count(),
        1
    );
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            project.0.to_str().unwrap(),
            "--format=json",
        ])
        .args(args)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let verify: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(verify["observation"], "still_present", "{verify}");
    assert_eq!(verify["event_persisted"], true, "{verify}");
    assert_eq!(verify["native_scan"]["checker_id"], "java.maven.p3c");
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", project.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    assert_eq!(
        next.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&next.stdout)
    );
    let next: Value = serde_json::from_slice(&next.stdout).unwrap();
    assert_eq!(
        next["repair_brief"]["verification_observation"], "still_present",
        "{next}"
    );
    fs::write(&tool, "#!/bin/sh\nmkdir -p target\nprintf '%s\\n' '<pmd xmlns=\"http://pmd.sourceforge.net/report/2.0.0\" version=\"6.15.0\"></pmd>' > target/pmd.xml\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let clean = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            project.0.to_str().unwrap(),
            "--format=json",
        ])
        .args(args)
        .output()
        .unwrap();
    assert_eq!(clean.status.code(), Some(3));
    let clean: Value = serde_json::from_slice(&clean.stdout).unwrap();
    assert_eq!(
        clean["observation"], "rule_coverage_requires_review",
        "{clean}"
    );
    assert_eq!(clean["event_persisted"], true);
    let fact: Value = serde_json::from_slice(
        &fs::read(
            project
                .0
                .join(".codeguard/findings")
                .join(id)
                .join("finding.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", project.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    assert_eq!(next.status.code(), Some(0));
    let next: Value = serde_json::from_slice(&next.stdout).unwrap();
    assert_eq!(
        next["repair_brief"]["verification_observation"],
        "rule_coverage_requires_review"
    );
}

#[test]
fn nested_maven_configuration_does_not_authorize_unconfigured_parent_sources() {
    let project = Project::new(false);
    fs::create_dir_all(project.0.join("module/src/main/java")).unwrap();
    fs::write(
        project.0.join("module/pom.xml"),
        include_str!("../../../tests/fixtures/p3c_native/pom.xml"),
    )
    .unwrap();
    fs::write(
        project.0.join("module/src/main/java/Bad_Name.java"),
        "class Bad_Name {}\n",
    )
    .unwrap();
    let marker = project.0.join("native-launched");
    let tool = project.0.join("mvn");
    fs::write(
        &tool,
        format!(
            "#!/bin/sh\nprintf x >> '{}'\nmkdir -p target\ncat > target/pmd.xml <<EOF\n<pmd xmlns=\"http://pmd.sourceforge.net/report/2.0.0\" version=\"6.15.0\"><file name=\"$PWD/src/main/java/Bad_Name.java\"><violation beginline=\"1\" endline=\"1\" begincolumn=\"7\" endcolumn=\"14\" rule=\"ClassNamingShouldBeCamelRule\" ruleset=\"AlibabaJavaNaming\" priority=\"2\">bad name</violation></file></pmd>\nEOF\n",
            marker.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let java_home = project.0.join("jdk");
    fs::create_dir_all(java_home.join("bin")).unwrap();
    fs::write(java_home.join("bin/java"), b"fixture java").unwrap();
    let repo = project.0.join("repo");
    fs::create_dir(&repo).unwrap();
    fs::write(repo.join("artifact.jar"), b"fixture").unwrap();
    let digest = hash_bundle_tree(&repo).unwrap();
    let report = project.check(&[
        "--maven-tool",
        tool.to_str().unwrap(),
        "--java-home",
        java_home.to_str().unwrap(),
        "--maven-repo",
        repo.to_str().unwrap(),
        "--repo-sha256",
        &digest,
    ]);
    let files = report["native_results"]["java_p3c"]["files"]
        .as_array()
        .unwrap();
    let parent = files
        .iter()
        .find(|file| file["path"] == "src/main/java/Bad_Name.java")
        .unwrap();
    let nested = files
        .iter()
        .find(|file| file["path"] == "module/src/main/java/Bad_Name.java")
        .unwrap();
    assert_eq!(parent["configuration"], "missing");
    assert!(parent["observation"].is_null());
    assert_eq!(nested["configuration"], "configured");
    assert_eq!(
        nested["observation"]["findings"].as_array().unwrap().len(),
        1
    );
    assert_eq!(fs::read(marker).unwrap(), b"x");
    assert_eq!(
        report["native_results"]["java_p3c"]["local_observation_complete"],
        false
    );
}

#[test]
#[ignore = "requires native Maven, JDK and pinned offline P3C dependency closure"]
fn real_native_p3c_finding_reaches_check_all_without_quality_allow() {
    let project = Project::new(true);
    fs::write(
        project.0.join("src/main/java/Bad_Name.java"),
        "public class Bad_Name {}\n",
    )
    .unwrap();
    let tool = std::env::var("CODEGUARD_MAVEN_BIN").unwrap();
    let java_home = std::env::var("CODEGUARD_JAVA_HOME").unwrap();
    let repo = std::env::var("CODEGUARD_P3C_MAVEN_REPO").unwrap();
    let digest = std::env::var("CODEGUARD_P3C_REPO_TREE_SHA256").unwrap();
    let report = project.check(&[
        "--maven-tool",
        &tool,
        "--java-home",
        &java_home,
        "--maven-repo",
        &repo,
        "--repo-sha256",
        &digest,
    ]);
    let java = &report["native_results"]["java_p3c"];
    assert_eq!(java["local_observation_complete"], true, "{java}");
    assert_eq!(java["coverage_proven"], false);
    assert!(
        java["files"][0]["observation"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["rule_id"] == "ClassNamingShouldBeCamelRule")
    );
    assert!(
        !java["files"][0]["observation"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["rule_id"] == "ClassMustHaveAuthorRule"),
        "{java}"
    );
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
#[ignore = "requires native Maven, JDK and pinned offline P3C dependency closure"]
fn real_native_p3c_finding_survives_task_verify() {
    let project = Project::new(true);
    project.init();
    let tool = std::env::var("CODEGUARD_MAVEN_BIN").unwrap();
    let java_home = std::env::var("CODEGUARD_JAVA_HOME").unwrap();
    let repo = std::env::var("CODEGUARD_P3C_MAVEN_REPO").unwrap();
    let digest = std::env::var("CODEGUARD_P3C_REPO_TREE_SHA256").unwrap();
    let args = [
        "--maven-tool",
        &tool,
        "--java-home",
        &java_home,
        "--maven-repo",
        &repo,
        "--repo-sha256",
        &digest,
    ];
    let first = project.check(&args);
    assert_eq!(
        first["native_results"]["java_p3c"]["backlog_status"], "synced_partial",
        "{first}"
    );
    assert!(
        first["native_results"]["java_p3c"]["files"][0]["observation"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|finding| finding["rule_id"] != "ClassMustHaveAuthorRule")
    );
    let id = first["next"]["repair_brief"]["task_id"].as_str().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            project.0.to_str().unwrap(),
            "--format=json",
        ])
        .args(args)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3), "{:?}", output);
    let verified: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(verified["observation"], "still_present", "{verified}");
    assert_eq!(verified["event_persisted"], true, "{verified}");
    let fact: Value = serde_json::from_slice(
        &fs::read(
            project
                .0
                .join(".codeguard/findings")
                .join(id)
                .join("finding.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    assert_eq!(verified["delivery_decision"], "not_evaluated");
}

#[test]
#[ignore = "requires native Maven, JDK and pinned offline P3C dependency closure"]
fn real_native_check_java_reports_two_rules_without_delivery_gate() {
    let project = Project::new(true);
    let pom = fs::read_to_string(project.0.join("pom.xml")).unwrap();
    fs::write(project.0.join("pom.xml"), pom.replace(
        "<rulesets><ruleset>rulesets/java/ali-naming.xml</ruleset></rulesets>",
        "<rulesets><ruleset>rulesets/java/ali-naming.xml</ruleset><ruleset>rulesets/java/ali-comment.xml</ruleset></rulesets>",
    )).unwrap();
    fs::write(
        project.0.join("src/main/java/Bad_Name.java"),
        "public class Bad_Name {}\n",
    )
    .unwrap();
    let tool = std::env::var("CODEGUARD_MAVEN_BIN").unwrap();
    let java_home = std::env::var("CODEGUARD_JAVA_HOME").unwrap();
    let repo = std::env::var("CODEGUARD_P3C_MAVEN_REPO").unwrap();
    let digest = std::env::var("CODEGUARD_P3C_REPO_TREE_SHA256").unwrap();
    let (exit, report) = project.check_java(&[
        "--maven-tool",
        &tool,
        "--java-home",
        &java_home,
        "--maven-repo",
        &repo,
        "--repo-sha256",
        &digest,
    ]);
    assert_eq!(exit, 3);
    assert_eq!(report["selection"], "java");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["execution_tasks"][0]["id"], "java.p3c");
    let findings = report["native_results"]["java_p3c"]["findings"]
        .as_array()
        .unwrap();
    for rule in ["ClassNamingShouldBeCamelRule", "ClassMustHaveAuthorRule"] {
        assert!(
            findings.iter().any(|finding| finding["rule_id"] == rule),
            "{report}"
        );
    }
    assert_eq!(report["native_results"]["python_lint"], Value::Null);
    assert_eq!(report["native_results"]["rust_lint"], Value::Null);
}

#[test]
#[ignore = "requires an explicit JDK 21 installation"]
fn real_jdk_check_java_reports_javadoc_comment_probe() {
    let project = Project::new(false);
    fs::write(
        project.0.join("pom.xml"),
        "<project><modelVersion>4.0.0</modelVersion><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId><configuration><doclint>missing</doclint></configuration></plugin></plugins></build></project>",
    )
    .unwrap();
    fs::write(
        project.0.join("src/main/java/Bad_Name.java"),
        "public class Bad_Name {}\n",
    )
    .unwrap();
    fs::create_dir_all(project.0.join("vendor/src/main/java")).unwrap();
    fs::write(
        project.0.join("vendor/src/main/java/Vendor.java"),
        "public class Vendor {}\n",
    )
    .unwrap();
    let java_home = std::env::var("CODEGUARD_JAVA_HOME").unwrap();
    let (exit, report) = project.check_java(&["--java-home", &java_home]);
    assert_eq!(exit, 3);
    let javadoc = &report["native_results"]["java_javadoc"];
    assert_eq!(javadoc["project_checker_attribution"], "unverified");
    assert_eq!(
        javadoc["files"][0]["observation"]["local_status"], "findings_observed_untrusted",
        "{report}"
    );
    assert!(
        javadoc["files"][0]["observation"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["rule_id"] == "JavadocMissingComment")
    );
    assert_eq!(javadoc["coverage_proven"], false);
    let vendor = javadoc["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|file| file["path"] == "vendor/src/main/java/Vendor.java")
        .unwrap();
    assert_eq!(vendor["reason"], "javadoc_source_scope_unverified");
    assert!(vendor["observation"].is_null());
    assert_eq!(javadoc["source_file_count"], 2);
    assert_eq!(javadoc["observed_file_count"], 1);
    assert_eq!(javadoc["local_probe_complete"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
fn explicit_maven_javadoc_missing_prerequisites_do_not_fall_back_to_single_file() {
    let project = Project::new(false);
    fs::write(project.0.join("pom.xml"),"<project><modelVersion>4.0.0</modelVersion><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId><configuration><doclint>missing</doclint></configuration></plugin></plugins></build></project>").unwrap();
    let jdk = project.0.join("jdk");
    fs::create_dir_all(jdk.join("bin")).unwrap();
    fs::write(jdk.join("release"), "JAVA_VERSION=\"21.0.1\"\n").unwrap();
    let marker = project.0.join("single-file-called");
    let tool = jdk.join("bin/javadoc");
    fs::write(
        &tool,
        format!(
            "#!/bin/sh\nprintf called > '{}'\nexit 1\n",
            marker.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
    let missing_maven = project.0.join("missing-maven");
    let (exit, report) = project.check_java(&[
        "--maven-tool",
        missing_maven.to_str().unwrap(),
        "--java-home",
        jdk.to_str().unwrap(),
    ]);
    assert_eq!(exit, 3);
    let javadoc = &report["native_results"]["java_javadoc"];
    assert_eq!(javadoc["schema_version"], "0.5.0");
    assert_eq!(javadoc["probe_mode"], "maven_multifile");
    assert_eq!(javadoc["checker_id"], "java.maven.javadoc");
    assert_eq!(
        javadoc["files"][0]["reason"],
        "maven_multifile_probe_selected"
    );
    assert!(javadoc["files"][0]["observation"].is_null());
    assert_eq!(
        javadoc["maven_multifile_probes"][0]["observation"]["native_status"],
        "incomplete"
    );
    assert!(!marker.exists());
}
