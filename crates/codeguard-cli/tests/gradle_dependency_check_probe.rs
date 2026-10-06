#![cfg(unix)]
use codeguard_cli::{
    gradle_dependency_check_probe::observe as native_observe,
    gradle_dependency_check_request::Request, gradle_model_probe_request::Request as NativeRequest,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Duration, Instant},
};
fn observe(request: &Request, cancelled: &AtomicBool) -> Value {
    let value = native_observe(request, cancelled);
    if let Some(directory) = std::env::var_os("CODEGUARD_TEST_GRADLE_CVE_ALL_REPORTS") {
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            PathBuf::from(directory).join(format!(
                "{}-{}.json",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )),
            serde_json::to_vec_pretty(&value).unwrap(),
        )
        .unwrap();
    }
    value
}
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new(script: &str) -> Self {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-gradle-cve-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for path in ["project", "gradle/bin", "jdk/bin"] {
            fs::create_dir_all(root.join(path)).unwrap();
        }
        fs::write(
            root.join("project/settings.gradle"),
            "rootProject.name='sample'\n",
        )
        .unwrap();
        fs::write(root.join("project/build.gradle"), "plugins { id 'java' }\n").unwrap();
        fs::write(root.join("gradle/bin/gradle"), script).unwrap();
        fs::set_permissions(
            root.join("gradle/bin/gradle"),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        fs::write(root.join("jdk/bin/java"), "controlled executable bytes").unwrap();
        fs::write(root.join("jdk/release"), "JAVA_VERSION=\"21.0.12\"\n").unwrap();
        Self(root)
    }
    fn request(&self) -> Request {
        Request {
            module_cache: None,
            native: NativeRequest {
                project_root: self.0.join("project"),
                project_files: BTreeSet::from(["settings.gradle".into(), "build.gradle".into()]),
                gradle_bundle: self.0.join("gradle"),
                java_home: self.0.join("jdk"),
                deadline: Instant::now() + Duration::from_secs(30),
            },
            task_paths: vec![":dependencyCheckAnalyze".into()],
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn model() -> Value {
    json!({"schema_version":"0.1.0","report_type":"gradle_checker_model","gradle_version":"8.10.2","included_build_count":0,"projects":[{"path":":","project_dir":".","build_dir":"build","plugins":["org.owasp.dependencycheck"],"tasks":[{"name":"dependencyCheckAnalyze","implementation":"org.owasp.dependencycheck.gradle.tasks.Analyze","enabled":true}]}]})
}
fn ownership() -> Value {
    json!({"schema_version":"0.1.0","report_type":"gradle_owasp_report_ownership","tasks":[{"project_path":":","task_path":":dependencyCheckAnalyze","implementation":"org.owasp.dependencycheck.gradle.tasks.Analyze","report_project_name":"root project 'sample'","report_path":"build/reports/dependency-check-report.json"}]})
}
fn report() -> Value {
    json!({"reportSchema":"1.1","scanInfo":{"engineVersion":"12.1.0","dataSource":[]},"projectInfo":{"name":"root project 'sample'","reportDate":"2026-10-06T12:00:00"},"dependencies":[{"fileName":"sample.jar","isVirtual":false,"sha256":"a".repeat(64),"packages":[{"id":"pkg:maven/example/sample@1"}],"vulnerabilities":[{"source":"NVD","name":"CVE-2020-0001"}],"suppressedVulnerabilities":[{"source":"NVD","name":"CVE-2020-0002"}]}]})
}
fn script(model: &Value, owner: &Value, report: &Value, tail: &str) -> String {
    format!(
        r#"#!/bin/sh
for arg in "$@"; do
case "$arg" in
-Dcodeguard.model.output=*) model=${{arg#*=}} ;;
-Dcodeguard.owasp.ownership=*) owner=${{arg#*=}} ;;
esac
done
cat > "$model" <<'JSON'
{model}
JSON
cat > "$owner" <<'JSON'
{owner}
JSON
mkdir -p build/reports
cat > build/reports/dependency-check-report.json <<'JSON'
{report}
JSON
{tail}
"#
    )
}
fn assert_no_acceptance(value: &Value) {
    assert_eq!(value["coverage_proven"], false);
    assert_eq!(value["database_freshness_verified"], false);
    assert_eq!(value["dependency_attribution_verified"], false);
    assert_eq!(value["authority"], "local_unverified");
    assert_eq!(value["delivery_decision"], "not_evaluated");
}
#[test]
fn current_reports_preserve_active_and_suppressed_observations_even_on_failed_exit() {
    for exit in [0, 1] {
        let fixture = Fixture::new(&script(
            &model(),
            &ownership(),
            &report(),
            &format!("exit {exit}"),
        ));
        let out = observe(&fixture.request(), &AtomicBool::new(false));
        assert_eq!(out["native_status"], "reports_observed_unverified", "{out}");
        assert_eq!(out["native_exit_code"], exit);
        let items = out["reports"][0]["advisories"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["suppressed_by_native_tool"], false);
        assert_eq!(items[1]["suppressed_by_native_tool"], true);
        assert_no_acceptance(&out);
    }
}
#[test]
fn cancelled_expired_invalid_selection_and_missing_inputs_do_not_launch() {
    let fixture = Fixture::new("#!/bin/sh\nexit 77\n");
    let cancelled = observe(&fixture.request(), &AtomicBool::new(true));
    assert_eq!(cancelled["reason"], "request_cancelled");
    let mut request = fixture.request();
    request.native.deadline = Instant::now();
    assert_eq!(
        observe(&request, &AtomicBool::new(false))["reason"],
        "request_deadline_exceeded"
    );
    for paths in [
        vec![],
        vec!["x".into()],
        vec![":scan,other".into()],
        vec![":scan".into(), ":scan".into()],
    ] {
        let mut request = fixture.request();
        request.task_paths = paths;
        let out = observe(&request, &AtomicBool::new(false));
        assert_eq!(out["reason"], "gradle_owasp_task_selection_invalid");
        assert_eq!(out["native_exit_code"], Value::Null);
        assert_no_acceptance(&out);
    }
    let mut request = fixture.request();
    request
        .native
        .project_files
        .remove(&PathBuf::from("settings.gradle"));
    assert_eq!(
        observe(&request, &AtomicBool::new(false))["reason"],
        "root_build_inputs_missing"
    );
}
#[test]
fn unknown_task_bad_reports_and_changed_private_inputs_never_create_advisories() {
    for mutation in 0..8 {
        let mut native_model = model();
        let mut owner = ownership();
        let mut native_report = report();
        let mut tail = "exit 0".to_owned();
        match mutation {
            0 => {
                native_model["projects"][0]["tasks"][0]["implementation"] =
                    "org.gradle.api.DefaultTask".into()
            }
            1 => owner["tasks"][0]["report_path"] = "../dependency-check-report.json".into(),
            2 => native_report["projectInfo"]["name"] = "different project".into(),
            3 => native_report["scanInfo"]["engineVersion"] = "unknown".into(),
            4 => native_report["scanInfo"]["analysisExceptions"] = json!([{"message":"failed"}]),
            5 => tail = "printf changed >> build.gradle\nexit 0".into(),
            6 => tail = "rm build/reports/dependency-check-report.json\nexit 0".into(),
            _ => tail = "exit 2".into(),
        }
        let fixture = Fixture::new(&script(&native_model, &owner, &native_report, &tail));
        let out = observe(&fixture.request(), &AtomicBool::new(false));
        assert_eq!(out["native_status"], "incomplete", "{mutation}: {out}");
        assert!(out["reports"].as_array().unwrap().is_empty());
        assert_no_acceptance(&out);
    }
}
#[test]
fn original_inputs_changed_during_native_execution_are_rejected() {
    let fixture = Fixture::new("");
    let source = fixture.0.join("project/build.gradle");
    let tail = format!("printf changed >> '{}'\nexit 0", source.display());
    fs::write(
        fixture.0.join("gradle/bin/gradle"),
        script(&model(), &ownership(), &report(), &tail),
    )
    .unwrap();
    let out = observe(&fixture.request(), &AtomicBool::new(false));
    assert_eq!(out["native_status"], "incomplete", "{out}");
    assert_eq!(out["reason"], "selected_inputs_changed_during_task");
}

#[test]
fn empty_valid_report_does_not_become_vulnerability_free_acceptance() {
    let mut native_report = report();
    native_report["dependencies"] = json!([]);
    let fixture = Fixture::new(&script(&model(), &ownership(), &native_report, "exit 0"));
    let out = observe(&fixture.request(), &AtomicBool::new(false));
    assert_eq!(out["native_status"], "reports_observed_unverified");
    assert_eq!(out["reports"][0]["dependency_count"], 0);
    assert_no_acceptance(&out);
}

#[test]
fn preexisting_snapshot_reports_and_changed_tool_bytes_are_rejected() {
    let fixture = Fixture::new(&script(&model(), &ownership(), &report(), "exit 0"));
    fs::create_dir_all(fixture.0.join("project/build/reports")).unwrap();
    fs::write(
        fixture
            .0
            .join("project/build/reports/dependency-check-report.json"),
        format!("{}\n", report()),
    )
    .unwrap();
    let mut request = fixture.request();
    request
        .native
        .project_files
        .insert("build/reports/dependency-check-report.json".into());
    let out = observe(&request, &AtomicBool::new(false));
    assert_eq!(out["native_status"], "incomplete");
    assert!(out["reports"].as_array().unwrap().is_empty());
    assert_eq!(out["reason"], "gradle_owasp_report_preexisting");
    let fixture = Fixture::new("");
    let tail = format!(
        "printf changed >> '{}'\nexit 0",
        fixture.0.join("jdk/bin/java").display()
    );
    fs::write(
        fixture.0.join("gradle/bin/gradle"),
        script(&model(), &ownership(), &report(), &tail),
    )
    .unwrap();
    let out = observe(&fixture.request(), &AtomicBool::new(false));
    assert_eq!(out["reason"], "native_identity_changed_during_task");
    assert!(out["reports"].as_array().unwrap().is_empty());
}

#[test]
#[ignore = "requires explicit existing Gradle8.10.2/JDK21; never installs OWASP"]
fn actual_gradle_rejects_imitation_task_and_missing_offline_plugin_without_advisories() {
    let mut reports = Vec::new();
    for (name, build) in [
        (
            "ordinary_imitation",
            "plugins { id 'java' }\ntasks.register('dependencyCheckAnalyze') { doLast { throw new GradleException('ordinary_task_must_not_run') } }\n",
        ),
        (
            "missing_offline_plugin",
            "plugins { id 'org.owasp.dependencycheck' version '12.1.0' }\n",
        ),
    ] {
        let fixture = Fixture::new("");
        fs::write(fixture.0.join("project/build.gradle"), build).unwrap();
        let mut request = fixture.request();
        request.native.gradle_bundle = std::env::var_os("CODEGUARD_TEST_GRADLE_BUNDLE")
            .expect("existing Gradle")
            .into();
        request.native.java_home = std::env::var_os("CODEGUARD_TEST_JAVA_HOME")
            .expect("existing JDK")
            .into();
        request.native.deadline = Instant::now() + Duration::from_secs(90);
        let out = observe(&request, &AtomicBool::new(false));
        assert_eq!(out["native_status"], "incomplete", "{name}: {out}");
        assert_eq!(out["native_exit_code"], 1, "{name}: {out}");
        assert!(out["reports"].as_array().unwrap().is_empty());
        assert_no_acceptance(&out);
        let public = std::process::Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["cve", "java"])
            .arg(&request.native.project_root)
            .arg("--gradle-bundle")
            .arg(&request.native.gradle_bundle)
            .arg("--java-home")
            .arg(&request.native.java_home)
            .args([
                "--gradle-owasp-task",
                ":dependencyCheckAnalyze",
                "--gradle-project-file",
                "settings.gradle",
                "--gradle-project-file",
                "build.gradle",
                "--timeout",
                "90s",
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(
            public.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&public.stderr)
        );
        let public_feedback: Value = serde_json::from_slice(&public.stdout).unwrap();
        assert_eq!(public_feedback["native"]["native_status"], "incomplete");
        assert_eq!(public_feedback["native"]["native_exit_code"], 1);
        assert_eq!(public_feedback["task_sync"], "not_initialized");
        assert_no_acceptance(&public_feedback["native"]);
        assert!(!request.native.project_root.join(".codeguard").exists());
        reports.push(json!({"case":name,"source_settings":"rootProject.name='sample'\n","source_build":build,"report":out,"public_feedback":public_feedback}));
    }
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_GRADLE_CVE_NATIVE_EVIDENCE") {
        fs::write(path, serde_json::to_vec_pretty(&reports).unwrap()).unwrap();
    }
}

#[test]
fn native_managed_blocker_codes_are_preserved_without_raw_error_text() {
    for (marker, reason) in [
        (
            "codeguard_owasp_json_not_configured",
            "gradle_owasp_json_not_configured",
        ),
        ("codeguard_owasp_scan_skipped", "gradle_owasp_scan_skipped"),
        (
            "codeguard_owasp_report_ownership_duplicate",
            "gradle_owasp_report_ownership_duplicate",
        ),
    ] {
        let fixture = Fixture::new(&format!(
            "#!/bin/sh\nprintf '%s\\n' '{marker}' 'sensitive_native_detail' >&2\nexit 1\n"
        ));
        let out = observe(&fixture.request(), &AtomicBool::new(false));
        assert_eq!(out["reason"], reason);
        assert!(!out.to_string().contains("sensitive_native_detail"));
        assert_no_acceptance(&out);
    }
}

#[test]
fn public_java_cve_keeps_original_native_reports_and_rejects_ambiguous_options() {
    use std::process::Command;
    let fixture = Fixture::new(&script(&model(), &ownership(), &report(), "exit 0"));
    let args = vec![
        "cve".to_string(),
        "java".into(),
        fixture.0.join("project").display().to_string(),
        "--gradle-bundle".into(),
        fixture.0.join("gradle").display().to_string(),
        "--java-home".into(),
        fixture.0.join("jdk").display().to_string(),
        "--gradle-owasp-task".into(),
        ":dependencyCheckAnalyze".into(),
        "--gradle-project-file".into(),
        "settings.gradle".into(),
        "--gradle-project-file".into(),
        "build.gradle".into(),
        "--format=json".into(),
    ];
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(&args)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let feedback: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(feedback["report_type"], "java_gradle_cve_feedback");
    assert_eq!(
        feedback["native"]["reports"][0]["advisories"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(feedback["delivery_decision"], "not_evaluated");
    assert_eq!(feedback["task_sync"], "not_initialized");
    assert!(!fixture.0.join("project/.codeguard").exists());
    for extra in [
        vec!["--gradle-owasp-task", ":dependencyCheckAnalyze"],
        vec!["--gradle-project-file", "build.gradle"],
        vec!["--gradle-bundle", "/duplicate"],
        vec!["--format=xml"],
        vec!["--timeout", "0"],
        vec!["--unknown"],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(&args)
            .args(extra)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
    }
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_GRADLE_CVE_PUBLIC_EVIDENCE") {
        fs::write(path, serde_json::to_vec_pretty(&feedback).unwrap()).unwrap();
    }
}

#[test]
fn selected_existing_module_cache_is_private_and_original_cache_changes_are_rejected() {
    let fixture = Fixture::new(&script(
        &model(),
        &ownership(),
        &report(),
        "test -f \"$HOME/.gradle/caches/modules-2/files-2.1/example/sample/1/hash/sample.jar\" || exit 9\nexit 0",
    ));
    let cache = fixture.0.join("modules-2");
    fs::create_dir_all(cache.join("files-2.1/example/sample/1/hash")).unwrap();
    let jar = cache.join("files-2.1/example/sample/1/hash/sample.jar");
    fs::write(&jar, b"controlled cache bytes").unwrap();
    let mut request = fixture.request();
    request.module_cache = Some(cache.clone());
    let out = observe(&request, &AtomicBool::new(false));
    assert_eq!(out["native_status"], "reports_observed_unverified", "{out}");
    assert!(out["dependency_cache_sha256"].as_str().is_some());
    assert_eq!(fs::read(&jar).unwrap(), b"controlled cache bytes");
    let tail = format!("printf changed >> '{}'\nexit 0", jar.display());
    fs::write(
        fixture.0.join("gradle/bin/gradle"),
        script(&model(), &ownership(), &report(), &tail),
    )
    .unwrap();
    let out = observe(&request, &AtomicBool::new(false));
    assert_eq!(out["reason"], "gradle_dependency_cache_changed_during_task");
    assert!(out["reports"].as_array().unwrap().is_empty());
}

#[test]
fn module_cache_cannot_load_user_properties_or_follow_artifact_symlinks() {
    for forbidden in ["properties", "symlink", "wrong_root"] {
        let fixture = Fixture::new("#!/bin/sh\nexit 77\n");
        let cache = fixture.0.join(if forbidden == "wrong_root" {
            "user-home"
        } else {
            "modules-2"
        });
        fs::create_dir_all(cache.join("files-2.1")).unwrap();
        if forbidden == "properties" {
            fs::write(cache.join("gradle.properties"), "credential=must_not_load").unwrap();
        }
        if forbidden == "symlink" {
            fs::write(fixture.0.join("outside"), "must_not_load").unwrap();
            std::os::unix::fs::symlink(fixture.0.join("outside"), cache.join("files-2.1/outside"))
                .unwrap();
        }
        let mut request = fixture.request();
        request.module_cache = Some(cache);
        let out = observe(&request, &AtomicBool::new(false));
        assert_eq!(out["reason"], "gradle_dependency_cache_scope_invalid");
        assert_eq!(out["native_exit_code"], Value::Null);
        assert!(out["reports"].as_array().unwrap().is_empty());
        assert!(!out.to_string().contains("must_not_load"));
    }
}

#[test]
fn public_sigint_returns_cancelled_feedback_without_quality_acceptance() {
    use std::{
        process::{Command, Stdio},
        thread,
    };
    let fixture = Fixture::new("");
    let marker = fixture.0.join("native-started");
    fs::write(
        fixture.0.join("gradle/bin/gradle"),
        format!(
            "#!/bin/sh\nprintf started > '{}'\nwhile :; do sleep 1; done\n",
            marker.display()
        ),
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["cve", "java"])
        .arg(fixture.0.join("project"))
        .arg("--gradle-bundle")
        .arg(fixture.0.join("gradle"))
        .arg("--java-home")
        .arg(fixture.0.join("jdk"))
        .args([
            "--gradle-owasp-task",
            ":dependencyCheckAnalyze",
            "--gradle-project-file",
            "settings.gradle",
            "--gradle-project-file",
            "build.gradle",
            "--timeout",
            "30s",
            "--format=json",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !marker.exists() && Instant::now() < deadline {
        assert!(child.try_wait().unwrap().is_none());
        thread::sleep(Duration::from_millis(10));
    }
    if !marker.exists() {
        let _ = child.kill();
        panic!("native process did not start");
    }
    assert!(
        Command::new("/bin/kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(130),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let feedback: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(feedback["native"]["reason"], "request_cancelled");
    assert_no_acceptance(&feedback["native"]);
    assert_eq!(feedback["task_sync"], "incomplete");
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_GRADLE_CVE_CANCEL_EVIDENCE") {
        fs::write(path, serde_json::to_vec_pretty(&feedback).unwrap()).unwrap();
    }
}

fn multi_report_fixture(native_reports: &[Value]) -> (Fixture, Request) {
    let mut native_model = model();
    native_model["projects"][0]["tasks"] = json!([]);
    let mut owner = ownership();
    owner["tasks"] = json!([]);
    let mut writes = String::new();
    let mut tasks = Vec::new();
    for (index, native_report) in native_reports.iter().enumerate() {
        let task = format!(":scan{index}");
        let path = format!("build/scan{index}/dependency-check-report.json");
        native_model["projects"][0]["tasks"].as_array_mut().unwrap().push(json!({"name":format!("scan{index}"),"implementation":"org.owasp.dependencycheck.gradle.tasks.Analyze","enabled":true}));
        owner["tasks"].as_array_mut().unwrap().push(json!({"project_path":":","task_path":task,"implementation":"org.owasp.dependencycheck.gradle.tasks.Analyze","report_project_name":"root project 'sample'","report_path":path}));
        writes +=
            &format!("mkdir -p build/scan{index}\ncat > {path} <<'JSON'\n{native_report}\nJSON\n");
        tasks.push(task);
    }
    let fixture = Fixture::new(&script(
        &native_model,
        &owner,
        &report(),
        &(writes + "exit 0"),
    ));
    let mut request = fixture.request();
    request.task_paths = tasks;
    (fixture, request)
}

#[test]
fn aggregate_report_bytes_and_advisories_cannot_escape_per_file_limits() {
    for (name, native_reports) in [
        ("bytes", {
            let mut sample = report();
            sample["padding"] = "x".repeat(6 * 1024 * 1024).into();
            vec![sample; 3]
        }),
        ("advisories", {
            let mut sample = report();
            sample["dependencies"][0]["suppressedVulnerabilities"] = json!([]);
            sample["dependencies"][0]["vulnerabilities"] = json!(
                (0..501)
                    .map(|index| json!({"source":"NVD","name":format!("CVE-2020-{index:04}")}))
                    .collect::<Vec<_>>()
            );
            vec![sample; 2]
        }),
    ] {
        let (_fixture, request) = multi_report_fixture(&native_reports);
        let out = observe(&request, &AtomicBool::new(false));
        assert_eq!(out["native_status"], "incomplete", "{name}: {out}");
        assert_eq!(out["reason"], "gradle_owasp_report_budget_exceeded");
        assert!(out["reports"].as_array().unwrap().is_empty());
        assert_no_acceptance(&out);
    }
}

#[test]
fn repeated_package_ids_cannot_expand_feedback_beyond_aggregate_budget() {
    let mut sample = report();
    sample["dependencies"][0]["packages"] = json!(
        (0..32)
            .map(|index| json!({"id":format!("pkg:maven/{index}/{}@1","x".repeat(480))}))
            .collect::<Vec<_>>()
    );
    sample["dependencies"][0]["suppressedVulnerabilities"] = json!([]);
    sample["dependencies"][0]["vulnerabilities"] = json!(
        (0..200)
            .map(|index| json!({"source":"NVD","name":format!("CVE-2020-{index:04}")}))
            .collect::<Vec<_>>()
    );
    let (_fixture, request) = multi_report_fixture(&[sample]);
    let out = observe(&request, &AtomicBool::new(false));
    assert_eq!(out["reason"], "gradle_owasp_report_budget_exceeded");
    assert_no_acceptance(&out);
}

#[test]
fn below_budget_multiple_reports_keep_every_task_and_suppressed_observation() {
    let (_fixture, request) = multi_report_fixture(&[report(), report()]);
    let out = observe(&request, &AtomicBool::new(false));
    assert_eq!(out["native_status"], "reports_observed_unverified");
    let reports = out["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    for (index, row) in reports.iter().enumerate() {
        assert_eq!(row["task_path"], format!(":scan{index}"));
        assert_eq!(row["advisories"].as_array().unwrap().len(), 2);
        assert_eq!(row["advisories"][1]["suppressed_by_native_tool"], true);
    }
    assert_no_acceptance(&out);
}

#[test]
fn initialized_gradle_cve_syncs_one_stable_preparation_task_and_rejects_tampering() {
    use std::process::Command;
    let fixture = Fixture::new(&script(&model(), &ownership(), &report(), "exit 0"));
    let root = fixture.0.join("project");
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["init", root.to_str().unwrap(), "--apply"])
        .output()
        .unwrap();
    assert_eq!(
        init.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    let scan = || {
        Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "cve",
                "java",
                root.to_str().unwrap(),
                "--gradle-bundle",
                fixture.0.join("gradle").to_str().unwrap(),
                "--java-home",
                fixture.0.join("jdk").to_str().unwrap(),
                "--gradle-project-file",
                "settings.gradle",
                "--gradle-project-file",
                "build.gradle",
                "--gradle-owasp-task",
                ":dependencyCheckAnalyze",
                "--format=json",
            ])
            .output()
            .unwrap()
    };
    for _ in 0..2 {
        let result = scan();
        assert_eq!(result.status.code(), Some(3));
        let value: Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(value["task_sync"], "synced", "{value}");
    }
    fs::write(fixture.0.join("gradle/bin/gradle"), "#!/bin/sh\nexit 1\n").unwrap();
    let blocked = scan();
    let blocked_value: Value = serde_json::from_slice(&blocked.stdout).unwrap();
    assert_eq!(blocked_value["native"]["native_status"], "incomplete");
    assert_eq!(blocked_value["task_sync"], "synced");
    let mut empty = report();
    empty["dependencies"] = json!([]);
    fs::write(
        fixture.0.join("gradle/bin/gradle"),
        script(&model(), &ownership(), &empty, "exit 0"),
    )
    .unwrap();
    let zero = scan();
    let value: Value = serde_json::from_slice(&zero.stdout).unwrap();
    assert_eq!(value["task_sync"], "synced");
    assert_eq!(value["native"]["reports"][0]["dependency_count"], 0);
    if let Some(directory) = std::env::var_os("CODEGUARD_TEST_GRADLE_CVE_WORKBENCH_EVIDENCE") {
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            PathBuf::from(directory).join("public-empty.json"),
            serde_json::to_vec_pretty(&value).unwrap(),
        )
        .unwrap();
    }
    let tasks: Vec<_> = fs::read_dir(root.join(".codeguard/tasks"))
        .unwrap()
        .collect();
    assert_eq!(tasks.len(), 1);
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", root.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    assert!(
        next.status.success(),
        "{}",
        String::from_utf8_lossy(&next.stderr)
    );
    let text = String::from_utf8(next.stdout).unwrap();
    if let Some(directory) = std::env::var_os("CODEGUARD_TEST_GRADLE_CVE_WORKBENCH_EVIDENCE") {
        fs::write(PathBuf::from(directory).join("next.json"), &text).unwrap();
    }
    assert!(text.contains("java.gradle.dependency_check"), "{text}");
    assert!(text.contains(":dependencyCheckAnalyze"), "{text}");
    assert!(!text.contains("pkg:maven"));
    let reports: Vec<_> = fs::read_dir(root.join(".codeguard/reports"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("cve-gradle-")
        })
        .collect();
    assert_eq!(reports.len(), 4);
    let id = tasks[0]
        .as_ref()
        .unwrap()
        .path()
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let verify = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            &id,
            root.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(verify.status.code(), Some(3));
    assert!(
        String::from_utf8_lossy(&verify.stdout).contains("gradle_cve_task_verify_not_integrated")
    );
    let mut value: Value = serde_json::from_slice(&fs::read(&reports[0]).unwrap()).unwrap();
    if let Some(directory) = std::env::var_os("CODEGUARD_TEST_GRADLE_CVE_WORKBENCH_EVIDENCE") {
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            PathBuf::from(directory).join("workbench.json"),
            serde_json::to_vec_pretty(&value).unwrap(),
        )
        .unwrap();
    }
    let mut stale = value.clone();
    stale["run_id"] = json!("cve-gradle-unconsumed-01");
    fs::write(
        root.join(".codeguard/reports/cve-gradle-unconsumed-01.json"),
        serde_json::to_vec_pretty(&stale).unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("build.gradle"),
        "plugins { id 'java' } // changed after scan\n",
    )
    .unwrap();
    let stale_sync = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["work", "sync", root.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    assert!(!stale_sync.status.success());
    fs::remove_file(root.join(".codeguard/reports/cve-gradle-unconsumed-01.json")).unwrap();
    value["coverage_proven"] = json!(true);
    fs::write(&reports[0], serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    let sync = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["work", "sync", root.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    assert!(
        !sync.status.success(),
        "{}",
        String::from_utf8_lossy(&sync.stdout)
    );
}
