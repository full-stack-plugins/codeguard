#![cfg(unix)]
use codeguard_cli::{gradle_model_probe::Request, gradle_javadoc_probe::observe};
use std::{
    collections::BTreeSet,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Duration, Instant},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Project(PathBuf);
impl Project {
    fn new(source: &str) -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-gradle-javadoc-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("src/main/java")).unwrap();
        fs::write(
            root.join("settings.gradle"),
            "rootProject.name = 'sample'\n",
        )
        .unwrap();
        fs::write(root.join("build.gradle"),"plugins { id 'java' }\ntasks.named('javadoc') { options.addBooleanOption('Xdoclint:missing', true); options.addBooleanOption('quiet', true); options.locale = 'en_US'; options.encoding = 'UTF-8' }\n").unwrap();
        fs::write(root.join("src/main/java/Sample.java"), source).unwrap();
        Self(root)
    }
    fn request(&self) -> Request {
        Request {
            project_root: self.0.clone(),
            project_files: BTreeSet::from([
                PathBuf::from("settings.gradle"),
                PathBuf::from("build.gradle"),
                PathBuf::from("src/main/java/Sample.java"),
            ]),
            gradle_bundle: PathBuf::from("/unavailable/gradle"),
            java_home: PathBuf::from("/unavailable/jdk"),
            deadline: Instant::now() + Duration::from_secs(90),
        }
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn missing_or_cancelled_native_model_cannot_become_comment_compliance() {
    let project = Project::new("public class Sample {}\n");
    let report = observe(&project.request(), &AtomicBool::new(false));
    assert_eq!(report["native_status"], "incomplete");
    assert!(report["findings"].as_array().unwrap().is_empty());
    assert_eq!(report["coverage_proven"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    let cancelled = observe(&project.request(), &AtomicBool::new(true));
    assert_eq!(cancelled["reason"], "request_cancelled");
    let mut expired = project.request();
    expired.deadline = Instant::now() - Duration::from_secs(1);
    let expired_report = observe(&expired, &AtomicBool::new(false));
    assert_eq!(expired_report["reason"], "request_deadline_exceeded");
    let mut incomplete = project.request();
    incomplete
        .project_files
        .remove(&PathBuf::from("settings.gradle"));
    let missing_root = observe(&incomplete, &AtomicBool::new(false));
    assert_eq!(missing_root["reason"], "root_build_inputs_missing");
    for report in [&cancelled, &expired_report, &missing_root] {
        assert_eq!(report["native_status"], "incomplete");
        assert_eq!(report["native_stdout_sha256"], serde_json::Value::Null);
        assert!(report["findings"].as_array().unwrap().is_empty());
    }
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_GRADLE_JAVADOC_INCOMPLETE_REPORT") {
        fs::write(
            path,
            serde_json::to_vec_pretty(&serde_json::json!([
                {"case":"missing_native","report":report},
                {"case":"cancelled","report":cancelled},
                {"case":"expired","report":expired_report},
                {"case":"missing_root","report":missing_root}
            ]))
            .unwrap(),
        )
        .unwrap();
    }
}

#[test]
#[ignore = "requires explicit existing Gradle8.10.2/JDK21; never installs"]
fn actual_gradle_javadoc_preserves_missing_comments_and_documented_counterexample() {
    let mut reports = Vec::new();
    for (name, source, expected_rules) in [
        (
            "missing",
            "public class Sample {\n    public Sample() {}\n    public int add(int value) { return value + 1; }\n}\n",
            &[
                "JavadocMissingComment",
                "JavadocMissingComment",
                "JavadocMissingComment",
            ][..],
        ),
        (
            "missing_tags",
            "/** Example. */\npublic class Sample {\n    /** Creates the example. */\n    public Sample() {}\n    /** Adds one. */\n    public int add(int value) { return value + 1; }\n}\n",
            &["JavadocMissingParam", "JavadocMissingReturn"][..],
        ),
        (
            "empty_tags",
            "/** Example. */\npublic class Sample {\n    /** Creates the example. */\n    public Sample() {}\n    /** Adds one.\n     * @param value\n     * @return\n     */\n    public int add(int value) { return value + 1; }\n}\n",
            &[
                "JavadocEmptyParamDescription",
                "JavadocEmptyReturnDescription",
            ][..],
        ),
        (
            "documented",
            "/** Provides an addition example. */\npublic class Sample {\n    /** Creates the example. */\n    public Sample() {}\n    /** Adds one.\n     * @param value input value\n     * @return incremented value\n     */\n    public int add(int value) { return value + 1; }\n}\n",
            &[][..],
        ),
    ] {
        let project = Project::new(source);
        let mut request = project.request();
        request.gradle_bundle = std::env::var_os("CODEGUARD_TEST_GRADLE_BUNDLE")
            .expect("existing Gradle")
            .into();
        request.java_home = std::env::var_os("CODEGUARD_TEST_JAVA_HOME")
            .expect("existing JDK")
            .into();
        let report = observe(&request, &AtomicBool::new(false));
        assert_eq!(
            report["native_status"],
            if !expected_rules.is_empty() {
                "findings_observed_unverified"
            } else {
                "empty_output_unverified"
            },
            "{report}"
        );
        let mut actual_rules = report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                assert_eq!(f["path"], "src/main/java/Sample.java");
                f["rule_id"].as_str().unwrap()
            })
            .collect::<Vec<_>>();
        actual_rules.sort_unstable();
        let mut expected_rules = expected_rules.to_vec();
        expected_rules.sort_unstable();
        assert_eq!(actual_rules, expected_rules, "{name}: {report}");
        assert_eq!(report["rule_configuration_complete"], false);
        assert_eq!(report["delivery_decision"], "not_evaluated");
        assert_eq!(report["task_paths"], serde_json::json!([":javadoc"]));
        assert_eq!(report["coverage_proven"], false);
        assert_eq!(
            fs::read_to_string(project.0.join("src/main/java/Sample.java")).unwrap(),
            source
        );
        reports.push(serde_json::json!({"case":name,"report":report}));
    }
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_GRADLE_JAVADOC_REPORT") {
        fs::write(path, serde_json::to_vec_pretty(&reports).unwrap()).unwrap();
    }
}

fn public_command(project: &Project, selection: &str) -> std::process::Command {
    public_command_with_tools(
        project,
        selection,
        std::path::Path::new("/unavailable/gradle"),
        std::path::Path::new("/unavailable/jdk"),
    )
}
fn public_command_with_tools(
    project: &Project,
    selection: &str,
    bundle: &std::path::Path,
    jdk: &std::path::Path,
) -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_codeguard"));
    command
        .args(["check", selection])
        .arg(&project.0)
        .args(["--gradle-javadoc", "--gradle-bundle"])
        .arg(bundle)
        .arg("--java-home")
        .arg(jdk)
        .args([
            "--gradle-project-file",
            "settings.gradle",
            "--gradle-project-file",
            "build.gradle",
            "--gradle-project-file",
            "src/main/java/Sample.java",
            "--format=json",
        ])
        .env("PATH", "/no/tools");
    command
}
#[test]
fn public_check_schedules_one_documentation_job_and_preserves_incomplete_feedback() {
    let project = Project::new("public class Sample {}\n");
    for selection in ["java", "all"] {
        let output = public_command(&project, selection).output().unwrap();
        assert_eq!(output.status.code(), Some(3), "{output:?}");
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["schema_version"], "0.67.0");
        let category = report["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["language"] == "java" && c["category"] == "comments")
            .unwrap();
        assert_eq!(category["checker_id"], "java.gradle.javadoc");
        assert_eq!(category["status"], "native_incomplete");
        assert_eq!(category["reason"], "gradle_javadoc_native_incomplete");
        assert!(category["next_action"].as_str().unwrap().contains("Gradle"));
        assert!(report["native_results"].get("java_gradle_model").is_none());
        assert_eq!(
            report["native_results"]["java_gradle_javadoc"]["native_status"],
            "incomplete"
        );
        let gradle_jobs = report["execution_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|j| j["id"].as_str().unwrap().starts_with("java.gradle."))
            .collect::<Vec<_>>();
        assert_eq!(gradle_jobs.len(), 1);
        assert_eq!(gradle_jobs[0]["id"], "java.gradle.javadoc");
        assert_ne!(report["delivery_decision"], "allow");
        if let Some(dir) = std::env::var_os("CODEGUARD_TEST_GRADLE_JAVADOC_PUBLIC_REPORTS") {
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                PathBuf::from(dir).join(format!("{selection}-incomplete.json")),
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
        }
    }
}

#[test]
fn public_javadoc_selection_rejects_missing_sources_duplicate_and_lint_scope() {
    let project = Project::new("public class Sample {}\n");
    for mut command in [
        {
            let mut c = public_command(&project, "java");
            c.arg("--gradle-javadoc");
            c
        },
        public_command(&project, "rust"),
        {
            let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_codeguard"));
            c.args(["check", "java"])
                .arg(&project.0)
                .arg("--gradle-javadoc");
            c
        },
        {
            let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_codeguard"));
            c.args(["lint", "all"])
                .arg(&project.0)
                .arg("--gradle-javadoc");
            c
        },
        {
            let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_codeguard"));
            c.args(["check", "java"]).arg(&project.0).args([
                "--gradle-javadoc",
                "--gradle-bundle",
                "/unavailable/gradle",
                "--java-home",
                "/unavailable/jdk",
                "--gradle-project-file",
                "settings.gradle",
                "--gradle-project-file",
                "build.gradle",
            ]);
            c
        },
    ] {
        let output = command.env("PATH", "/no/tools").output().unwrap();
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(output.stdout.is_empty());
    }
}
#[test]
#[ignore = "requires explicit existing Gradle8.10.2/JDK21; never installs"]
fn actual_public_check_preserves_native_documentation_findings_without_coverage_upgrade() {
    let bundle =
        PathBuf::from(std::env::var_os("CODEGUARD_TEST_GRADLE_BUNDLE").expect("existing Gradle"));
    let jdk = PathBuf::from(std::env::var_os("CODEGUARD_TEST_JAVA_HOME").expect("existing JDK"));
    for (name, source, expected_count, status) in [
        (
            "missing",
            "public class Sample {\n    public Sample() {}\n    public int add(int value) { return value + 1; }\n}\n",
            3,
            "findings_observed_unverified",
        ),
        (
            "documented",
            "/** Provides an example. */\npublic class Sample {\n    /** Creates the example. */\n    public Sample() {}\n    /** Adds one.\n     * @param value input value\n     * @return incremented value\n     */\n    public int add(int value) { return value + 1; }\n}\n",
            0,
            "empty_output_unverified",
        ),
    ] {
        let project = Project::new(source);
        let output = public_command_with_tools(&project, "java", &bundle, &jdk)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3), "{output:?}");
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["schema_version"], "0.67.0");
        let category = report["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["language"] == "java" && c["category"] == "comments")
            .unwrap();
        assert_eq!(category["checker_id"], "java.gradle.javadoc");
        assert_eq!(category["status"], "observed_unverified");
        assert_eq!(
            category["reason"],
            "gradle_javadoc_selected_inputs_and_rules_unverified"
        );
        assert!(category["next_action"].as_str().unwrap().contains("Gradle"));
        let native = &report["native_results"]["java_gradle_javadoc"];
        assert_eq!(native["native_status"], status, "{report}");
        assert_eq!(native["findings"].as_array().unwrap().len(), expected_count);
        for finding in native["findings"].as_array().unwrap() {
            assert_eq!(finding["rule_id"], "JavadocMissingComment");
            assert_eq!(finding["path"], "src/main/java/Sample.java");
        }
        assert_eq!(native["coverage_proven"], false);
        assert_eq!(native["rule_configuration_complete"], false);
        assert_ne!(report["delivery_decision"], "allow");
        assert!(report["native_results"].get("java_gradle_model").is_none());
        assert_eq!(
            report["execution_tasks"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["id"] == "java.gradle.javadoc")
                .count(),
            1
        );
        if let Some(dir) = std::env::var_os("CODEGUARD_TEST_GRADLE_JAVADOC_PUBLIC_REPORTS") {
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                PathBuf::from(dir).join(format!("{name}-native.json")),
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
        }
    }
}

#[test]
fn public_javadoc_sigint_preserves_cancelled_native_observation() {
    use std::os::unix::fs::PermissionsExt;
    let project = Project::new("public class Sample {}\n");
    let bundle = project.0.join("native-gradle");
    let jdk = project.0.join("native-jdk");
    fs::create_dir_all(bundle.join("bin")).unwrap();
    fs::create_dir_all(jdk.join("bin")).unwrap();
    fs::write(jdk.join("bin/java"), "controlled fixture").unwrap();
    fs::write(jdk.join("release"), "JAVA_VERSION=\"21\"\n").unwrap();
    let marker = project.0.join("native-started");
    fs::write(
        bundle.join("bin/gradle"),
        format!(
            "#!/bin/sh\nprintf started > '{}'\nexec /bin/sleep 30\n",
            marker.display()
        ),
    )
    .unwrap();
    fs::set_permissions(bundle.join("bin/gradle"), fs::Permissions::from_mode(0o700)).unwrap();
    let child = public_command_with_tools(&project, "java", &bundle, &jdk)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while !marker.exists() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(marker.exists(), "native process did not start");
    assert!(
        std::process::Command::new("/bin/kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(130), "{output:?}");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema_version"], "0.67.0");
    assert_eq!(
        report["native_results"]["java_gradle_javadoc"]["reason"],
        "request_cancelled"
    );
    assert!(
        report["native_results"]["java_gradle_javadoc"]["findings"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        report["execution_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == "java.gradle.javadoc" && r["status"] == "cancelled")
    );
    if let Some(dir) = std::env::var_os("CODEGUARD_TEST_GRADLE_JAVADOC_PUBLIC_REPORTS") {
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            PathBuf::from(dir).join("sigint.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn unexecuted_gradle_documentation_configuration_remains_unknown_not_missing() {
    let project = Project::new("public class Sample {}\n");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "java"])
        .arg(&project.0)
        .arg("--format=json")
        .env("PATH", "/no/tools")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let category = report["category_candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["language"] == "java" && c["category"] == "comments")
        .unwrap();
    assert_eq!(category["status"], "configuration_unresolved");
    assert_eq!(category["reason"], "javadoc_configuration_unresolved");
    assert!(category["next_action"].as_str().unwrap().contains("Gradle"));
    assert!(
        report["native_results"]
            .get("java_gradle_javadoc")
            .is_none()
    );
    if let Some(dir) = std::env::var_os("CODEGUARD_TEST_GRADLE_JAVADOC_PUBLIC_REPORTS") {
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            PathBuf::from(dir).join("static-unknown.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }
}

#[test]
#[ignore = "requires explicit existing Gradle8.10.2/JDK21; never installs"]
fn actual_public_detailed_descriptions_create_recheckable_original_tasks() {
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};
    let binary = env!("CARGO_BIN_EXE_codeguard");
    let binary_sha256 = format!("{:x}", Sha256::digest(fs::read(binary).unwrap()));
    let bundle =
        PathBuf::from(std::env::var_os("CODEGUARD_TEST_GRADLE_BUNDLE").expect("existing Gradle"));
    let jdk = PathBuf::from(std::env::var_os("CODEGUARD_TEST_JAVA_HOME").expect("existing JDK"));
    let documented = "/** 提供数值计算示例。 */\npublic class Sample {\n /** 创建计算器。 */ public Sample() {}\n /** 输出结果的初始值。 */ public int value;\n /** 返回输入数值。\n  * @param input 待返回的输入数值\n  * @return {@code input} 的原值\n  * @throws IllegalArgumentException 输入为负数时抛出\n  */\n public int run(int input) throws IllegalArgumentException { if (input < 0) { throw new IllegalArgumentException(); } return input; }\n /** {@inheritDoc} */\n @Override public String toString() { return \"sample\"; }\n}\n";
    let mut reports = Vec::new();
    for (name, source, expected) in [
        (
            "empty_declarations",
            "/** */\npublic class Sample {\n /** */ public Sample() {}\n /** */ public int value;\n /** */ public void run() {}\n}\n",
            vec!["JavadocEmptyComment"; 4],
        ),
        (
            "bare_tags",
            "/** Sample API. */\npublic class Sample { /** Creates sample. */ public Sample() {}\n/** Computes value.\n * @param value\n * @return\n * @throws IllegalArgumentException\n */\npublic int run(int value) throws IllegalArgumentException { return value; } }\n",
            vec![
                "JavadocEmptyParamDescription",
                "JavadocEmptyReturnDescription",
                "JavadocEmptyThrowsDescription",
            ],
        ),
        (
            "tags_without_purpose",
            "/** Sample API. */\npublic class Sample { /** Creates sample. */ public Sample() {}\n/** @param value the input\n * @return the value\n */\npublic int run(int value) { return value; } }\n",
            vec!["JavadocMissingMainDescription"],
        ),
        ("documented_inheritance", documented, vec![]),
    ] {
        let project = Project::new(source);
        let init = std::process::Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("init")
            .arg(&project.0)
            .arg("--apply")
            .output()
            .unwrap();
        assert_eq!(init.status.code(), Some(3));
        let out = public_command_with_tools(&project, "java", &bundle, &jdk)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
        let check: Value = serde_json::from_slice(&out.stdout).unwrap();
        let native = &check["native_results"]["java_gradle_javadoc"];
        assert_eq!(
            native["native_status"],
            if expected.is_empty() {
                "empty_output_unverified"
            } else {
                "findings_observed_unverified"
            },
            "{name}: {check}"
        );
        let mut actual = native["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["rule_id"].as_str().unwrap())
            .collect::<Vec<_>>();
        actual.sort_unstable();
        let mut expected = expected;
        expected.sort_unstable();
        assert_eq!(actual, expected, "{name}");
        assert_eq!(
            check["gradle_javadoc_tasks"]["status"], "synced_partial",
            "{check}"
        );
        let mut scans = Vec::new();
        if !expected.is_empty() {
            let fact_id = check["next"]["repair_brief"]["task_id"].as_str().unwrap();
            assert_eq!(check["next"]["repair_brief"]["kind"], "finding");
            let verify = || {
                let out = std::process::Command::new(env!("CARGO_BIN_EXE_codeguard"))
                    .args(["task", "verify", fact_id])
                    .arg(&project.0)
                    .arg("--gradle-bundle")
                    .arg(&bundle)
                    .arg("--java-home")
                    .arg(&jdk)
                    .arg("--format=json")
                    .output()
                    .unwrap();
                assert_eq!(out.status.code(), Some(3));
                serde_json::from_slice::<Value>(&out.stdout).unwrap()
            };
            let present = verify();
            assert_eq!(present["observation"], "still_present", "{present}");
            assert_eq!(present["event_persisted"], true);
            scans.push(present);
            if name == "empty_declarations" {
                fs::write(project.0.join("src/main/java/Sample.java"), documented).unwrap();
                let repaired = verify();
                assert_eq!(
                    repaired["observation"], "candidate_absent_unverified_policy",
                    "{repaired}"
                );
                assert_eq!(repaired["event_persisted"], true);
                scans.push(repaired);
                let fact: Value = serde_json::from_slice(
                    &fs::read(
                        project
                            .0
                            .join(format!(".codeguard/findings/{fact_id}/finding.json")),
                    )
                    .unwrap(),
                )
                .unwrap();
                assert_eq!(fact["state"], "open");
            }
        }
        let raw = fs::read_dir(project.0.join(".codeguard/reports"))
            .unwrap()
            .map(|e| {
                serde_json::from_slice::<Value>(&fs::read(e.unwrap().path()).unwrap()).unwrap()
            })
            .collect::<Vec<_>>();
        reports.push(json!({"case":name,"source":source,"check":check,"rechecks":scans,"stored_reports":raw}));
    }
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_GRADLE_DETAILED_REPORT") {
        assert_eq!(
            format!("{:x}", Sha256::digest(fs::read(binary).unwrap())),
            binary_sha256
        );
        fs::write(path, serde_json::to_vec_pretty(&json!({"evidence_kind":"actual_existing_gradle_jdk_public_detailed_descriptions","codeguard_binary_sha256":binary_sha256,"reports":reports})).unwrap()).unwrap();
    }
}
