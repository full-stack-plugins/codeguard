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
