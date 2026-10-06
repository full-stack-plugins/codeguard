#![cfg(unix)]
use codeguard_cli::gradle_model_probe::{Request, observe as native_observe};
use std::{
    collections::BTreeSet,
    fs,
    path::PathBuf,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

fn observe(request: &Request, cancelled: &AtomicBool) -> serde_json::Value {
    let report = native_observe(request, cancelled);
    if let Some(directory) = std::env::var_os("CODEGUARD_TEST_GRADLE_ALL_REPORTS") {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            PathBuf::from(directory).join(format!(
                "{}-{}.json",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }
    report
}

struct TempDir(PathBuf);
impl TempDir {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-gradle-probe-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct Fixture {
    directory: TempDir,
}
impl Fixture {
    fn new(script: &str) -> Self {
        let directory = TempDir::new();
        for path in ["project", "gradle/bin", "jdk/bin"] {
            fs::create_dir_all(directory.path().join(path)).unwrap();
        }
        fs::write(
            directory.path().join("project/settings.gradle"),
            "rootProject.name = 'sample'\n",
        )
        .unwrap();
        fs::write(
            directory.path().join("project/build.gradle"),
            "plugins { id 'java' }\n",
        )
        .unwrap();
        fs::write(directory.path().join("gradle/bin/gradle"), script).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            directory.path().join("gradle/bin/gradle"),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        fs::write(directory.path().join("jdk/bin/java"), "fixture").unwrap();
        fs::write(
            directory.path().join("jdk/release"),
            "JAVA_VERSION=\"21\"\n",
        )
        .unwrap();
        Self { directory }
    }
    fn request(&self) -> Request {
        Request {
            project_root: self.directory.path().join("project"),
            project_files: BTreeSet::from([
                PathBuf::from("settings.gradle"),
                PathBuf::from("build.gradle"),
            ]),
            gradle_bundle: self.directory.path().join("gradle"),
            java_home: self.directory.path().join("jdk"),
            deadline: Instant::now() + Duration::from_secs(30),
        }
    }
}

#[test]
fn cancelled_expired_or_missing_root_inputs_do_not_launch_native_tool() {
    let fixture = Fixture::new("#!/bin/sh\nprintf launched > marker\nexit 1\n");
    let cancelled = AtomicBool::new(true);
    assert_eq!(
        observe(&fixture.request(), &cancelled)["reason"],
        "request_cancelled"
    );
    let cancelled = AtomicBool::new(false);
    let mut request = fixture.request();
    request.deadline = Instant::now();
    assert_eq!(
        observe(&request, &cancelled)["reason"],
        "request_deadline_exceeded"
    );
    request = fixture.request();
    request
        .project_files
        .remove(&PathBuf::from("settings.gradle"));
    assert_eq!(
        observe(&request, &cancelled)["reason"],
        "root_build_inputs_missing"
    );
    assert!(!fixture.directory.path().join("project/marker").exists());
}

#[test]
fn failed_execution_is_environment_incomplete_without_source_findings() {
    let fixture = Fixture::new("#!/bin/sh\nprintf secret >&2\nexit 1\n");
    let report = observe(&fixture.request(), &AtomicBool::new(false));
    assert_eq!(report["native_status"], "incomplete");
    assert_eq!(report["reason"], "native_execution_incomplete");
    assert_eq!(report["model"], serde_json::Value::Null);
    assert!(!report.to_string().contains("secret"));
    assert_eq!(report["coverage_proven"], false);
}

#[test]
#[ignore = "requires explicitly supplied existing Gradle and JDK; never installs"]
fn existing_gradle_observes_actual_subprojects_and_rejects_same_named_default_task() {
    let fixture = Fixture::new("#!/bin/sh\nexit 1\n");
    let root = fixture.directory.path().join("project");
    fs::create_dir(root.join("app")).unwrap();
    fs::write(
        root.join("settings.gradle"),
        "rootProject.name = 'sample'\ninclude 'app'\n",
    )
    .unwrap();
    fs::write(
        root.join("build.gradle"),
        "plugins { id 'java' }\ntasks.register('dependencyCheckAnalyze')\n",
    )
    .unwrap();
    fs::write(root.join("app/build.gradle"), "plugins { id 'java'; id 'checkstyle'; id 'pmd' }\ntasks.named('pmdMain') { enabled = false }\n").unwrap();
    let mut request = fixture.request();
    request
        .project_files
        .insert(PathBuf::from("app/build.gradle"));
    request.gradle_bundle = std::env::var_os("CODEGUARD_TEST_GRADLE_BUNDLE")
        .expect("explicit existing distribution")
        .into();
    request.java_home = std::env::var_os("CODEGUARD_TEST_JAVA_HOME")
        .expect("explicit existing JDK")
        .into();
    request.deadline = Instant::now() + Duration::from_secs(90);
    let report = observe(&request, &AtomicBool::new(false));
    assert_eq!(
        report["native_status"], "model_observed_unverified",
        "{report}"
    );
    let model = codeguard_adapters::parse_gradle_checker_model(
        &serde_json::to_vec(&report["model"]).unwrap(),
    )
    .unwrap();
    assert_eq!(model.gradle_version, "8.10.2");
    assert_eq!(model.projects.len(), 2);
    assert!(model.projects[0].dependency_check_tasks().is_empty());
    assert!(model.projects[1].plugins.contains(&"checkstyle".into()));
    assert!(
        model.projects[1]
            .tasks
            .iter()
            .any(|t| t.implementation == "org.gradle.api.plugins.quality.Pmd" && !t.enabled)
    );
    assert_eq!(report["coverage_proven"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_GRADLE_REPORT") {
        fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}

#[test]
fn controlled_model_is_untrusted_and_changes_or_incomplete_scope_are_rejected() {
    let model = r#"{"schema_version":"0.1.0","report_type":"gradle_checker_model","gradle_version":"8.10.2","included_build_count":0,"projects":[{"path":":","project_dir":".","build_dir":"build","plugins":["java"],"tasks":[]}]}"#;
    for (body, why) in [
        (
            model.to_owned(),
            "selected_inputs_and_jdk_closure_unverified",
        ),
        (
            model.replace("\"included_build_count\":0", "\"included_build_count\":1"),
            "native_model_invalid_or_incomplete",
        ),
        (
            model.replace(
                "\"plugins\":[\"java\"]",
                "\"plugins\":[\"java\"],\"production_qualified\":true",
            ),
            "native_model_invalid_or_incomplete",
        ),
    ] {
        let script = format!(
            "#!/bin/sh\nfor arg in \"$@\"; do\ncase \"$arg\" in -Dcodeguard.model.output=*) report=${{arg#-Dcodeguard.model.output=}};; esac\ndone\ncat > \"$report\" <<'JSON'\n{body}\nJSON\n"
        );
        let fixture = Fixture::new(&script);
        let report = observe(&fixture.request(), &AtomicBool::new(false));
        assert_eq!(report["reason"], why, "{report}");
        assert_eq!(report["coverage_proven"], false);
        assert_eq!(report["authority"], "local_unverified");
        assert_eq!(report["delivery_decision"], "not_evaluated");
        if why == "selected_inputs_and_jdk_closure_unverified" {
            assert_eq!(report["native_status"], "model_observed_unverified");
        } else {
            assert_eq!(report["model"], serde_json::Value::Null);
        }
    }
    let fixture = Fixture::new("#!/bin/sh\nprintf changed > build.gradle\nexit 0\n");
    let report = observe(&fixture.request(), &AtomicBool::new(false));
    assert_eq!(report["reason"], "native_inputs_changed_during_probe");
    assert_eq!(
        fs::read_to_string(fixture.directory.path().join("project/build.gradle")).unwrap(),
        "plugins { id 'java' }\n"
    );
}

#[test]
fn missing_output_and_native_timeout_never_become_empty_models() {
    let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
    let report = observe(&fixture.request(), &AtomicBool::new(false));
    assert_eq!(report["reason"], "native_model_missing");
    let fixture = Fixture::new("#!/bin/sh\nsleep 5\n");
    let mut request = fixture.request();
    request.deadline = Instant::now() + Duration::from_millis(350);
    let report = observe(&request, &AtomicBool::new(false));
    assert_eq!(report["reason"], "request_deadline_exceeded", "{report}");
    assert_eq!(report["model"], serde_json::Value::Null);
    assert_eq!(report["native_status"], "incomplete");
}

#[test]
#[ignore = "requires explicitly supplied existing Gradle and JDK; never installs"]
fn existing_gradle_observes_kotlin_dsl_without_executing_quality_tasks() {
    let fixture = Fixture::new("#!/bin/sh\nexit 1\n");
    let root = fixture.directory.path().join("project");
    fs::remove_file(root.join("settings.gradle")).unwrap();
    fs::remove_file(root.join("build.gradle")).unwrap();
    fs::write(
        root.join("settings.gradle.kts"),
        "rootProject.name = \"sample\"\n",
    )
    .unwrap();
    fs::write(
        root.join("build.gradle.kts"),
        "plugins { java; checkstyle; pmd }\ntasks.named(\"pmdMain\") { enabled = false }\n",
    )
    .unwrap();
    let mut request = fixture.request();
    request.project_files = BTreeSet::from([
        PathBuf::from("settings.gradle.kts"),
        PathBuf::from("build.gradle.kts"),
    ]);
    request.gradle_bundle = std::env::var_os("CODEGUARD_TEST_GRADLE_BUNDLE")
        .expect("explicit existing distribution")
        .into();
    request.java_home = std::env::var_os("CODEGUARD_TEST_JAVA_HOME")
        .expect("explicit existing JDK")
        .into();
    request.deadline = Instant::now() + Duration::from_secs(90);
    let report = observe(&request, &AtomicBool::new(false));
    assert_eq!(
        report["native_status"], "model_observed_unverified",
        "{report}"
    );
    assert_eq!(
        report["model"]["projects"][0]["plugins"],
        serde_json::json!(["java", "checkstyle", "pmd"])
    );
    assert_eq!(report["coverage_proven"], false);
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_GRADLE_REPORT") {
        let mut output = PathBuf::from(path);
        output.set_extension("kotlin.json");
        fs::write(output, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}
