use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[test]
fn detect_keeps_shared_m_and_sc_suffixes_ambiguous_without_false_languages() {
    let project = TempProject::new();
    fs::write(
        project.0.join("plot.m"),
        "title('@interface Foo');\n%{\n#import <NotObjectiveC.h>\n@interface Fake\n%}\nplot(1:3);\n",
    )
    .unwrap();
    fs::write(project.0.join("synth.sc"), "{ SinOsc.ar(440) }.play;\n").unwrap();
    fs::write(
        project.0.join("model.m"),
        "@interface Model : NSObject\n@end\n",
    )
    .unwrap();
    fs::write(project.0.join("main.scala"), "object Main {}\n").unwrap();
    let report = detect_json(&project);
    let languages = report["languages"].as_array().unwrap();
    let objc = languages.iter().find(|item| item["id"] == "objc").unwrap();
    let scala = languages.iter().find(|item| item["id"] == "scala").unwrap();
    assert_eq!(objc["source_files"], serde_json::json!(["model.m"]));
    assert_eq!(scala["source_files"], serde_json::json!(["main.scala"]));
    assert!(report["observation_complete"].as_bool().unwrap());
    assert!(
        report["unknown_conditions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "ambiguous_language_suffix:plot.m")
    );
    assert!(
        report["unknown_conditions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "ambiguous_language_suffix:synth.sc")
    );
}

struct TempProject(PathBuf);

impl TempProject {
    fn new() -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("codeguard-detect-{}-{id}", std::process::id()));
        fs::create_dir(&path).expect("create fixture root");
        Self(path)
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove fixture");
    }
}

#[test]
fn detect_reports_evidence_and_does_not_create_project_state() {
    let project = TempProject::new();
    fs::write(project.0.join("pom.xml"), "<project/>").expect("manifest");
    fs::create_dir_all(project.0.join("module/src/main/java")).expect("source dir");
    fs::write(
        project.0.join("module/src/main/java/App.java"),
        "class App {}",
    )
    .expect("source");
    let before: Vec<_> = fs::read_dir(&project.0)
        .expect("fixture listing")
        .map(|entry| entry.expect("entry").file_name())
        .collect();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "detect",
            project.0.to_str().expect("utf8 path"),
            "--format",
            "json",
        ])
        .output()
        .expect("run detect");
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let report: Value = serde_json::from_slice(&output.stdout).expect("JSON report");
    assert_eq!(report["report_type"], "discovery");
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/discovery-report.schema.json"
    ))
    .expect("published discovery schema");
    let previous: Value = serde_json::from_str(include_str!(
        "../../../schemas/discovery-report-v0.3.schema.json"
    ))
    .expect("retained discovery schema");
    assert_eq!(previous["properties"]["schema_version"]["const"], "0.3.0");
    assert_eq!(schema["properties"]["schema_version"]["const"], "0.4.0");
    let states =
        schema["properties"]["native_tool_candidates"]["items"]["properties"]["state"]["enum"]
            .as_array()
            .unwrap();
    assert!(
        states
            .iter()
            .all(|state| state != "ready" && state != "clean")
    );
    let required: BTreeSet<_> = schema["required"]
        .as_array()
        .expect("required fields")
        .iter()
        .map(|field| field.as_str().expect("name"))
        .collect();
    let actual: BTreeSet<_> = report
        .as_object()
        .expect("report object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(required, actual);
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(report["observation_complete"], true);
    assert!(
        report["unknown_conditions"]
            .as_array()
            .expect("unknown list")
            .iter()
            .any(|item| item == "build_model_not_evaluated")
    );
    assert_eq!(report["build_roots"][0]["manifests"][0], "pom.xml");
    assert_eq!(
        report["source_set_candidates"][0]["path"],
        "module/src/main/java"
    );
    assert_eq!(report["source_set_candidates"][0]["role"], "main");
    let after: Vec<_> = fs::read_dir(&project.0)
        .expect("fixture listing")
        .map(|entry| entry.expect("entry").file_name())
        .collect();
    assert_eq!(before, after);
}

#[test]
fn missing_root_is_incomplete_and_invalid_format_is_usage_error() {
    let project = TempProject::new();
    let missing = project.0.join("missing");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "detect",
            missing.to_str().expect("utf8 path"),
            "--format=json",
        ])
        .output()
        .expect("run detect");
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).expect("partial JSON");
    assert_eq!(report["observation_complete"], false);
    assert_eq!(report["blocked_paths"][0], ".");

    let invalid = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "detect",
            missing.to_str().expect("utf8 path"),
            "--format",
            "yaml",
        ])
        .output()
        .expect("run invalid detect");
    assert_eq!(invalid.status.code(), Some(2));
    assert!(invalid.stdout.is_empty());
}

#[test]
fn detect_reads_literal_package_version_without_running_package_scripts() {
    let project = TempProject::new();
    fs::write(
        project.0.join("package.json"),
        r#"{"name":"demo","version":"2.4.6","scripts":{"postinstall":"exit 99"}}"#,
    )
    .expect("manifest");
    fs::write(project.0.join("index.ts"), "export const x = 1;").expect("source");
    fs::write(project.0.join("package-lock.json"), "{}").expect("lockfile");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "detect",
            project.0.to_str().expect("utf8 path"),
            "--format=json",
        ])
        .output()
        .expect("run detect");
    assert_eq!(output.status.code(), Some(0));
    let report: Value = serde_json::from_slice(&output.stdout).expect("JSON report");
    assert_eq!(report["declared_versions"]["package.json"], "2.4.6");
    assert_eq!(
        report["manifest_sha256"]["package.json"]
            .as_str()
            .expect("digest")
            .len(),
        64
    );
    assert_eq!(report["observation_complete"], true);
    assert_eq!(report["lockfiles"][0], "package-lock.json");
    assert!(
        report["unknown_conditions"]
            .as_array()
            .expect("unknown list")
            .iter()
            .any(|item| item == "lock_contents_not_evaluated")
    );
    assert!(!project.0.join("node_modules").exists());
}

#[test]
fn detect_observes_local_eslint_and_maven_wrapper_without_scanning_dependencies() {
    let project = TempProject::new();
    fs::write(
        project.0.join("package.json"),
        r#"{"name":"demo","version":"1.0.0","devDependencies":{"eslint":"^10.0.0"}}"#,
    )
    .unwrap();
    fs::write(project.0.join("eslint.config.js"), "export default [];").unwrap();
    fs::write(project.0.join("index.ts"), "export const x = 1;").unwrap();
    fs::create_dir_all(project.0.join("node_modules/eslint/bin")).unwrap();
    fs::write(
        project.0.join("node_modules/eslint/package.json"),
        r#"{"name":"eslint","version":"10.1.0","bin":"bin/eslint.js"}"#,
    )
    .unwrap();
    fs::write(
        project.0.join("node_modules/eslint/bin/eslint.js"),
        "process.exit(99);",
    )
    .unwrap();
    fs::write(project.0.join("pom.xml"), "<project/>").unwrap();
    fs::write(project.0.join("mvnw"), "exit 99\n").unwrap();
    fs::create_dir_all(project.0.join(".mvn/wrapper")).unwrap();
    fs::write(
        project.0.join(".mvn/wrapper/maven-wrapper.properties"),
        "distributionUrl=https\\://repo.maven.apache.org/maven2/org/apache/maven/apache-maven/3.9.9/apache-maven-3.9.9-bin.zip\n",
    )
    .unwrap();

    let report = detect_json(&project);
    assert_eq!(report["schema_version"], "0.4.0");
    assert_eq!(report["observation_complete"], true);
    assert_eq!(report["scope_summary"]["dependency_roots_excluded"], 1);
    let candidates = report["native_tool_candidates"].as_array().unwrap();
    assert!(candidates.iter().any(|candidate| {
        candidate["checker_id"] == "node.eslint"
            && candidate["state"] == "local_candidate_requires_native_probe"
            && candidate["observed_version"] == "10.1.0"
    }));
    assert!(candidates.iter().any(|candidate| {
        candidate["checker_id"] == "java.maven"
            && candidate["state"] == "wrapper_candidate_requires_native_probe"
            && candidate["observed_version"] == "3.9.9"
    }));
    assert!(report["languages"].to_string().contains("index.ts"));
    assert!(!report["languages"].to_string().contains("node_modules"));
    assert!(!project.0.join(".codeguard").exists());
}

#[cfg(unix)]
#[test]
fn detect_keeps_linked_local_tools_and_config_as_blockers() {
    use std::os::unix::fs::symlink;

    let project = TempProject::new();
    fs::write(
        project.0.join("package.json"),
        r#"{"devDependencies":{"eslint":"^10.0.0"}}"#,
    )
    .unwrap();
    fs::create_dir_all(project.0.join("node_modules")).unwrap();
    fs::create_dir_all(project.0.join("outside-eslint/bin")).unwrap();
    fs::write(
        project.0.join("outside-eslint/package.json"),
        r#"{"name":"eslint","version":"10.1.0","bin":"bin/eslint.js"}"#,
    )
    .unwrap();
    fs::write(project.0.join("outside-eslint/bin/eslint.js"), "exit 99").unwrap();
    symlink("../outside-eslint", project.0.join("node_modules/eslint")).unwrap();
    fs::write(project.0.join("eslint-source.js"), "export default [];").unwrap();
    symlink("eslint-source.js", project.0.join("eslint.config.js")).unwrap();
    fs::write(project.0.join("pom.xml"), "<project/>").unwrap();
    fs::write(project.0.join("mvnw"), "exit 99\n").unwrap();
    fs::create_dir_all(project.0.join("outside-wrapper/wrapper")).unwrap();
    fs::write(
        project
            .0
            .join("outside-wrapper/wrapper/maven-wrapper.properties"),
        "distributionUrl=https\\://example.test/apache-maven-3.9.9-bin.zip\n",
    )
    .unwrap();
    symlink("outside-wrapper", project.0.join(".mvn")).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["detect", project.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["observation_complete"], false);
    let candidates = report["native_tool_candidates"].as_array().unwrap();
    assert!(candidates.iter().any(|candidate| {
        candidate["checker_id"] == "node.eslint"
            && candidate["state"] == "local_package_path_untrusted"
            && candidate["declaration_observed"] == true
    }));
    assert!(candidates.iter().any(|candidate| {
        candidate["checker_id"] == "java.maven"
            && candidate["state"] == "wrapper_configuration_untrusted"
    }));
    assert!(
        report["blocked_paths"]
            .to_string()
            .contains("node_modules/eslint")
    );
    assert!(report["blocked_paths"].to_string().contains(".mvn"));
}

#[cfg(unix)]
#[test]
fn linked_eslint_config_is_a_configuration_blocker_not_an_install_request() {
    use std::os::unix::fs::symlink;

    let project = TempProject::new();
    fs::write(
        project.0.join("package.json"),
        r#"{"devDependencies":{"eslint":"10.1.0"}}"#,
    )
    .unwrap();
    fs::create_dir_all(project.0.join("node_modules/eslint/bin")).unwrap();
    fs::write(
        project.0.join("node_modules/eslint/package.json"),
        r#"{"name":"eslint","version":"10.1.0","bin":"bin/eslint.js"}"#,
    )
    .unwrap();
    fs::write(
        project.0.join("node_modules/eslint/bin/eslint.js"),
        "exit 99",
    )
    .unwrap();
    fs::write(project.0.join("config-source.js"), "export default [];").unwrap();
    symlink("config-source.js", project.0.join("eslint.config.js")).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["detect", project.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let eslint = report["native_tool_candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["checker_id"] == "node.eslint")
        .unwrap();
    assert_eq!(eslint["state"], "configuration_invalid");
    assert!(!eslint["next_action"].as_str().unwrap().contains("安装"));
}

#[test]
fn dot_prefix_exclusions_are_summarized_without_per_path_alerts() {
    let project = TempProject::new();
    fs::write(project.0.join("app.py"), "print('ok')\n").unwrap();
    fs::write(project.0.join(".hidden.py"), "bad source\n").unwrap();
    fs::write(project.0.join(".env"), "TOKEN=fixture\n").unwrap();
    fs::create_dir(project.0.join(".hidden_dir")).unwrap();
    fs::write(project.0.join(".hidden_dir/hidden.py"), "bad source\n").unwrap();
    fs::write(project.0.join(".ruff.toml"), "[lint]\nselect = ['F']\n").unwrap();
    fs::write(project.0.join(".pre-commit-config.yaml"), "repos: []\n").unwrap();
    let report = detect_json(&project);
    assert_eq!(report["schema_version"], "0.4.0");
    assert_eq!(
        report["scope_summary"]["ordinary_scan_policy"],
        "project_sources_default_v2"
    );
    assert_eq!(report["scope_summary"]["dot_prefix_roots_excluded"], 3);
    assert_eq!(
        report["scope_summary"]["configuration_exception_files_observed"],
        2
    );
    assert_eq!(
        report["scope_summary"]["git_safety_status"],
        "not_evaluated"
    );
    let rendered = report.to_string();
    assert!(!rendered.contains(".hidden.py"));
    assert!(!rendered.contains(".hidden_dir"));
    assert!(!rendered.contains(".env"));
    assert!(rendered.contains(".ruff.toml"));
    assert!(
        !report["languages"][0]["source_files"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn malformed_package_manifest_remains_unknown_without_a_false_version() {
    let project = TempProject::new();
    fs::write(project.0.join("package.json"), "{oops").expect("manifest");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "detect",
            project.0.to_str().expect("utf8 path"),
            "--format=json",
        ])
        .output()
        .expect("run detect");
    assert_eq!(output.status.code(), Some(0));
    let report: Value = serde_json::from_slice(&output.stdout).expect("JSON report");
    assert!(
        report["declared_versions"]
            .as_object()
            .expect("object")
            .is_empty()
    );
    assert!(
        report["unknown_conditions"]
            .as_array()
            .expect("array")
            .iter()
            .any(|item| item == "manifest_parse_failed:package.json")
    );
}

fn detect_json(project: &TempProject) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["detect", project.0.to_str().unwrap(), "--format=json"])
        .output()
        .expect("run detect");
    assert_eq!(output.status.code(), Some(0));
    serde_json::from_slice(&output.stdout).expect("discovery JSON")
}

fn checker<'a>(report: &'a Value, id: &str) -> &'a Value {
    report["checker_configurations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["checker_id"] == id)
        .expect("checker entry")
}

fn checker_in<'a>(report: &'a Value, id: &str, build_root: &str) -> &'a Value {
    report["checker_configurations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["checker_id"] == id && item["build_root"] == build_root)
        .expect("checker at build root")
}

#[test]
fn python_dependency_inputs_distinguish_pins_lock_and_missing_graph() {
    let project = TempProject::new();
    fs::write(project.0.join("app.py"), "print('hello')\n").unwrap();
    fs::write(project.0.join("requirements.txt"), "requests==2.31.0\n").unwrap();
    let pinned = detect_json(&project);
    assert_eq!(
        checker(&pinned, "python.pip_audit")["reason"],
        "pinned_requirements_graph_unverified"
    );
    assert_eq!(
        checker(&pinned, "python.pip_audit")["configuration"],
        "unknown"
    );

    fs::write(project.0.join("requirements.txt"), "requests>=2.31\n").unwrap();
    let dynamic = detect_json(&project);
    assert_eq!(
        checker(&dynamic, "python.pip_audit")["reason"],
        "requirements_dynamic_or_unpinned"
    );

    fs::write(
        project.0.join("uv.lock"),
        "version = 1\n[[package]]\nname = 'requests'\nversion = '2.31.0'\n",
    )
    .unwrap();
    let locked = detect_json(&project);
    assert_eq!(
        checker(&locked, "python.pip_audit")["reason"],
        "python_lock_model_unparsed"
    );
    assert_eq!(
        checker(&locked, "python.pip_audit")["configuration_ref"],
        "uv.lock"
    );

    fs::remove_file(project.0.join("uv.lock")).unwrap();
    fs::remove_file(project.0.join("requirements.txt")).unwrap();
    let missing = detect_json(&project);
    assert_eq!(
        checker(&missing, "python.pip_audit")["reason"],
        "python_dependency_input_not_found"
    );
}

#[test]
fn python_cve_plan_keeps_dependency_input_as_unresolved_candidate() {
    let project = TempProject::new();
    fs::write(project.0.join("app.py"), "print('hello')\n").unwrap();
    fs::write(project.0.join("requirements.txt"), "requests==2.31.0\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "plan",
            "cve",
            "python",
            project.0.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let plan: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        plan["observed_checkers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|checker| checker["checker_id"] == "python.pip_audit"
                && checker["reason"] == "pinned_requirements_graph_unverified")
    );
    assert_eq!(plan["candidate_tasks"], serde_json::json!([]));
    assert_eq!(plan["quality_decision"], "not_evaluated");
}

#[test]
fn python_dependency_lock_is_attributed_to_its_own_build_root() {
    let project = TempProject::new();
    fs::write(project.0.join("app.py"), "print('root')\n").unwrap();
    fs::write(project.0.join("requirements.txt"), "requests==2.31.0\n").unwrap();
    fs::create_dir(project.0.join("service")).unwrap();
    fs::write(project.0.join("service/app.py"), "print('service')\n").unwrap();
    fs::write(
        project.0.join("service/pyproject.toml"),
        "[project]\nname = 'service'\nversion = '0.1.0'\n",
    )
    .unwrap();
    fs::write(
        project.0.join("service/uv.lock"),
        "version = 1\n[[package]]\nname = 'requests'\nversion = '2.31.0'\n",
    )
    .unwrap();
    let report = detect_json(&project);
    assert_eq!(
        checker_in(&report, "python.pip_audit", ".")["reason"],
        "pinned_requirements_graph_unverified"
    );
    assert_eq!(
        checker_in(&report, "python.pip_audit", "service")["reason"],
        "python_lock_model_unparsed"
    );
    assert_eq!(
        checker_in(&report, "python.pip_audit", "service")["configuration_ref"],
        "service/uv.lock"
    );
}

#[test]
fn standard_pylock_is_discovered_without_treating_uv_lock_as_pip_audit_input() {
    let project = TempProject::new();
    fs::write(project.0.join("app.py"), "print('hello')\n").unwrap();
    fs::write(
        project.0.join("pyproject.toml"),
        "[project]\nname = 'app'\nversion = '0.1.0'\n",
    )
    .unwrap();
    fs::write(project.0.join("pylock.toml"), "lock-version = '1.0'\n").unwrap();
    let standard = detect_json(&project);
    assert_eq!(
        checker(&standard, "python.pip_audit")["reason"],
        "python_standard_lock_model_unparsed"
    );
    assert_eq!(
        checker(&standard, "python.pip_audit")["configuration_ref"],
        "pylock.toml"
    );

    fs::rename(
        project.0.join("pylock.toml"),
        project.0.join("pylock.ci.toml"),
    )
    .unwrap();
    let named = detect_json(&project);
    assert_eq!(
        checker(&named, "python.pip_audit")["reason"],
        "python_standard_lock_model_unparsed"
    );
    assert_eq!(
        checker(&named, "python.pip_audit")["configuration_ref"],
        "pylock.ci.toml"
    );

    fs::write(project.0.join("uv.lock"), "version = 1\n").unwrap();
    let ambiguous = detect_json(&project);
    assert_eq!(
        checker(&ambiguous, "python.pip_audit")["reason"],
        "multiple_python_lock_inputs_unresolved"
    );
    fs::remove_file(project.0.join("pylock.ci.toml")).unwrap();
    let uv_only = detect_json(&project);
    assert_eq!(
        checker(&uv_only, "python.pip_audit")["reason"],
        "python_lock_model_unparsed"
    );
    assert!(
        checker(&uv_only, "python.pip_audit")["next_action"]
            .as_str()
            .unwrap()
            .contains("不能直接交给 pip-audit --locked")
    );
}

#[test]
fn maven_configuration_detection_separates_declared_missing_and_unknown() {
    let project = TempProject::new();
    fs::write(
        project.0.join("pom.xml"),
        r#"<project><build><plugins>
          <plugin><artifactId>maven-javadoc-plugin</artifactId></plugin>
          <plugin><groupId>org.owasp</groupId><artifactId>dependency-check-maven</artifactId></plugin>
          <plugin><groupId>com.github.spotbugs</groupId><artifactId>spotbugs-maven-plugin</artifactId></plugin>
        </plugins><pluginManagement><plugins>
          <plugin><artifactId>maven-pmd-plugin</artifactId></plugin>
        </plugins></pluginManagement></build></project>"#,
    )
    .expect("pom");
    let report = detect_json(&project);
    assert_eq!(report["schema_version"], "0.4.0");
    assert_eq!(
        checker(&report, "java.maven.javadoc")["configuration"],
        "configured"
    );
    assert_eq!(
        checker(&report, "java.maven.dependency_check")["configuration"],
        "configured"
    );
    assert_eq!(
        checker(&report, "java.maven.dependency")["configuration"],
        "missing"
    );
    assert_eq!(
        checker(&report, "java.maven.pmd")["configuration"],
        "unknown"
    );
    assert_eq!(
        checker(&report, "java.maven.p3c")["configuration"],
        "unknown"
    );
    assert_eq!(
        checker(&report, "java.maven.findsecbugs")["configuration"],
        "unknown"
    );
    assert_eq!(
        checker(&report, "java.maven.javadoc")["configuration_ref"],
        "pom.xml"
    );
}

#[test]
fn p3c_configuration_requires_explicit_artifact_ruleset_and_error_policy() {
    let configured = TempProject::new();
    fs::write(
        configured.0.join("pom.xml"),
        include_str!("../../../tests/fixtures/p3c_native/pom.xml"),
    )
    .expect("p3c fixture");
    let report = detect_json(&configured);
    assert_eq!(
        checker(&report, "java.maven.pmd")["configuration"],
        "configured"
    );
    assert_eq!(
        checker(&report, "java.maven.p3c")["configuration"],
        "configured"
    );
    assert_eq!(
        checker(&report, "java.maven.p3c")["configuration_ref"],
        "pom.xml"
    );

    let no_dependency = TempProject::new();
    let pom = include_str!("../../../tests/fixtures/p3c_native/pom.xml");
    let pom = pom.replace("<groupId>com.alibaba.p3c</groupId><artifactId>p3c-pmd</artifactId><version>2.1.1</version>", "<groupId>example</groupId><artifactId>lookalike</artifactId><version>2.1.1</version>");
    fs::write(no_dependency.0.join("pom.xml"), pom).expect("lookalike");
    assert_ne!(
        checker(&detect_json(&no_dependency), "java.maven.p3c")["configuration"],
        "configured"
    );

    let no_rule = TempProject::new();
    let pom = include_str!("../../../tests/fixtures/p3c_native/pom.xml").replace(
        "rulesets/java/ali-naming.xml",
        "rulesets/java/maven-pmd-plugin-default.xml",
    );
    fs::write(no_rule.0.join("pom.xml"), pom).expect("wrong rule");
    assert_ne!(
        checker(&detect_json(&no_rule), "java.maven.p3c")["configuration"],
        "configured"
    );

    let default_error_policy = TempProject::new();
    let pom = include_str!("../../../tests/fixtures/p3c_native/pom.xml")
        .replace("<skipPmdError>false</skipPmdError>", "");
    fs::write(default_error_policy.0.join("pom.xml"), pom).expect("default policy");
    assert_ne!(
        checker(&detect_json(&default_error_policy), "java.maven.p3c")["configuration"],
        "configured"
    );

    let skipped_errors = TempProject::new();
    let pom = include_str!("../../../tests/fixtures/p3c_native/pom.xml").replace(
        "<skipPmdError>false</skipPmdError>",
        "<skipPmdError>true</skipPmdError>",
    );
    fs::write(skipped_errors.0.join("pom.xml"), pom).expect("skipped processing errors");
    assert_eq!(
        checker(&detect_json(&skipped_errors), "java.maven.p3c")["configuration"],
        "invalid"
    );

    let wrong_version = TempProject::new();
    let pom = include_str!("../../../tests/fixtures/p3c_native/pom.xml").replace(
        "<artifactId>p3c-pmd</artifactId><version>2.1.1</version>",
        "<artifactId>p3c-pmd</artifactId><version>${p3c.version}</version>",
    );
    fs::write(wrong_version.0.join("pom.xml"), pom).expect("dynamic version");
    assert_eq!(
        checker(&detect_json(&wrong_version), "java.maven.p3c")["configuration"],
        "unknown"
    );
}

#[test]
fn inherited_p3c_inputs_are_unknown_not_missing_when_direct_pmd_is_partial() {
    let project = TempProject::new();
    fs::write(
        project.0.join("pom.xml"),
        r#"<project><parent><groupId>example</groupId><artifactId>parent</artifactId><version>1</version></parent>
          <build><plugins><plugin><artifactId>maven-pmd-plugin</artifactId></plugin></plugins></build>
        </project>"#,
    )
    .expect("child pom");
    let report = detect_json(&project);
    assert_eq!(
        checker(&report, "java.maven.p3c")["configuration"],
        "unknown"
    );
    assert_eq!(
        checker(&report, "java.maven.p3c")["reason"],
        "p3c_effective_model_not_resolved"
    );
}

#[test]
fn malformed_pom_is_invalid_and_gradle_is_unknown() {
    let malformed = TempProject::new();
    fs::write(malformed.0.join("pom.xml"), "<!DOCTYPE project><project/>").expect("pom");
    let report = detect_json(&malformed);
    assert_eq!(
        checker(&report, "java.maven.javadoc")["configuration"],
        "invalid"
    );
    assert_ne!(
        checker(&report, "java.maven.javadoc")["configuration"],
        "configured"
    );

    let gradle = TempProject::new();
    fs::write(gradle.0.join("build.gradle.kts"), "plugins { java }").expect("gradle");
    let report = detect_json(&gradle);
    assert_eq!(
        checker(&report, "java.gradle.javadoc")["configuration"],
        "unknown"
    );
}

#[test]
fn namespaced_child_pom_does_not_turn_inherited_configuration_into_missing() {
    let project = TempProject::new();
    fs::write(
        project.0.join("pom.xml"),
        r#"<project xmlns="http://maven.apache.org/POM/4.0.0">
          <parent><groupId>example</groupId><artifactId>parent</artifactId><version>1</version></parent>
          <build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId></plugin></plugins></build>
        </project>"#,
    )
    .expect("pom");
    let report = detect_json(&project);
    assert_eq!(
        checker(&report, "java.maven.javadoc")["configuration"],
        "configured"
    );
    assert_eq!(
        checker(&report, "java.maven.dependency_check")["configuration"],
        "unknown"
    );
}

#[test]
fn findsecbugs_requires_its_plugin_dependency() {
    let project = TempProject::new();
    fs::write(
        project.0.join("pom.xml"),
        r#"<project><build><plugins><plugin>
          <groupId>com.github.spotbugs</groupId><artifactId>spotbugs-maven-plugin</artifactId>
          <dependencies><dependency><groupId>com.h3xstream.findsecbugs</groupId>
            <artifactId>findsecbugs-plugin</artifactId></dependency></dependencies>
        </plugin></plugins></build></project>"#,
    )
    .expect("pom");
    let report = detect_json(&project);
    assert_eq!(
        checker(&report, "java.maven.findsecbugs")["configuration"],
        "configured"
    );
}

#[test]
fn similarly_named_findsecbugs_artifact_from_another_group_is_not_configured() {
    let project = TempProject::new();
    fs::write(project.0.join("pom.xml"),
        "<project><build><plugins><plugin><groupId>com.github.spotbugs</groupId><artifactId>spotbugs-maven-plugin</artifactId><dependencies><dependency><groupId>example.unrelated</groupId><artifactId>findsecbugs-plugin</artifactId></dependency></dependencies></plugin></plugins></build></project>").unwrap();
    let report = detect_json(&project);
    assert_ne!(
        checker(&report, "java.maven.findsecbugs")["configuration"],
        "configured"
    );
}

#[test]
fn skipped_and_dynamic_plugin_configuration_never_claims_configured() {
    let skipped = TempProject::new();
    fs::write(
        skipped.0.join("pom.xml"),
        r#"<project><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId>
          <configuration><skip>true</skip></configuration>
        </plugin></plugins></build></project>"#,
    )
    .expect("pom");
    let report = detect_json(&skipped);
    assert_eq!(
        checker(&report, "java.maven.javadoc")["configuration"],
        "invalid"
    );
    assert_eq!(
        checker(&report, "java.maven.javadoc")["reason"],
        "plugin_explicitly_skipped"
    );

    let dynamic = TempProject::new();
    fs::write(
        dynamic.0.join("pom.xml"),
        r#"<project><build><plugins>
          <plugin><artifactId>maven-javadoc-plugin</artifactId>
            <configuration><skip>${javadoc.skip}</skip></configuration></plugin>
          <plugin><artifactId>${quality.plugin}</artifactId></plugin>
        </plugins></build></project>"#,
    )
    .expect("pom");
    let report = detect_json(&dynamic);
    assert_eq!(
        checker(&report, "java.maven.javadoc")["configuration"],
        "unknown"
    );
    assert_eq!(
        checker(&report, "java.maven.dependency_check")["configuration"],
        "unknown"
    );
}

#[test]
fn javadoc_missing_comment_rule_must_not_be_claimed_when_disabled() {
    for (doclint, expected, reason) in [
        ("none", "invalid", "javadoc_missing_check_disabled"),
        ("all,-missing", "invalid", "javadoc_missing_check_disabled"),
        ("syntax", "invalid", "javadoc_missing_check_disabled"),
        ("${javadoc.doclint}", "unknown", "javadoc_doclint_dynamic"),
        ("missing", "configured", "plugin_declared_in_build_plugins"),
    ] {
        let project = TempProject::new();
        fs::write(
            project.0.join("pom.xml"),
            format!(
                "<project><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId><configuration><doclint>{doclint}</doclint></configuration></plugin></plugins></build></project>"
            ),
        )
        .expect("pom");
        let report = detect_json(&project);
        let config = checker(&report, "java.maven.javadoc");
        assert_eq!(config["configuration"], expected, "{doclint}: {config}");
        assert_eq!(config["reason"], reason, "{doclint}: {config}");
    }
}

#[test]
fn javadoc_fail_on_error_false_must_not_claim_reliable_configuration() {
    let project = TempProject::new();
    fs::write(
        project.0.join("pom.xml"),
        "<project><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId><configuration><failOnError>false</failOnError></configuration></plugin></plugins></build></project>",
    )
    .expect("pom");
    let config = checker(&detect_json(&project), "java.maven.javadoc").clone();
    assert_eq!(config["configuration"], "invalid");
    assert_eq!(config["reason"], "javadoc_errors_ignored");
}

#[test]
fn javadoc_project_property_skip_prevents_configured_claim() {
    let project = TempProject::new();
    fs::write(
        project.0.join("pom.xml"),
        "<project><properties><maven.javadoc.skip>true</maven.javadoc.skip></properties><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId></plugin></plugins></build></project>",
    )
    .expect("pom");
    let config = checker(&detect_json(&project), "java.maven.javadoc").clone();
    assert_eq!(config["configuration"], "invalid");
    assert_eq!(config["reason"], "plugin_explicitly_skipped");

    fs::write(
        project.0.join("pom.xml"),
        "<project><properties><maven.javadoc.skip>true</maven.javadoc.skip></properties><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId><configuration><skip>false</skip></configuration></plugin></plugins></build></project>",
    )
    .expect("explicit override");
    let overridden = checker(&detect_json(&project), "java.maven.javadoc").clone();
    assert_eq!(overridden["configuration"], "configured");
}

#[test]
fn ruff_lint_config_is_distinct_from_format_only_or_absent_config() {
    let configured = TempProject::new();
    fs::write(configured.0.join("app.py"), "import os\n").expect("source");
    fs::write(
        configured.0.join("pyproject.toml"),
        "[tool.ruff.lint]\nselect = ['F']\n",
    )
    .expect("config");
    let report = detect_json(&configured);
    assert_eq!(
        checker(&report, "python.ruff")["configuration"],
        "configured"
    );
    assert_eq!(
        checker(&report, "python.ruff")["configuration_ref"],
        "pyproject.toml"
    );

    let format_only = TempProject::new();
    fs::write(format_only.0.join("app.py"), "pass\n").expect("source");
    fs::write(
        format_only.0.join("pyproject.toml"),
        "[tool.ruff.format]\nquote-style = 'single'\n",
    )
    .expect("config");
    let report = detect_json(&format_only);
    assert_eq!(checker(&report, "python.ruff")["configuration"], "unknown");
    assert_eq!(
        checker(&report, "python.ruff")["reason"],
        "ruff_lint_intent_not_explicit"
    );

    let absent = TempProject::new();
    fs::write(absent.0.join("app.py"), "pass\n").expect("source");
    fs::write(
        absent.0.join("pyproject.toml"),
        "[project]\nname = 'example'\n",
    )
    .expect("manifest");
    let report = detect_json(&absent);
    assert_eq!(checker(&report, "python.ruff")["configuration"], "missing");
}

#[test]
fn ruff_hierarchy_keeps_root_and_nested_configuration_separate() {
    let project = TempProject::new();
    fs::create_dir(project.0.join("pkg")).expect("package");
    fs::write(project.0.join("root.py"), "pass\n").expect("source");
    fs::write(project.0.join("pkg/nested.py"), "pass\n").expect("source");
    fs::write(
        project.0.join("pyproject.toml"),
        "[tool.ruff.lint]\nselect = ['F']\n",
    )
    .expect("config");
    fs::write(project.0.join("pkg/.ruff.toml"), "[lint]\nselect = ['E']\n").expect("nested config");
    let report = detect_json(&project);
    assert_eq!(
        checker_in(&report, "python.ruff", ".")["configuration"],
        "configured"
    );
    assert_eq!(
        checker_in(&report, "python.ruff", "pkg")["configuration"],
        "configured"
    );
    assert_eq!(
        checker_in(&report, "python.ruff", "pkg")["configuration_ref"],
        "pkg/.ruff.toml"
    );
}

#[test]
fn bad_ruff_config_and_unresolved_extension_never_claim_configured() {
    let invalid = TempProject::new();
    fs::write(invalid.0.join("app.py"), "pass\n").expect("source");
    fs::write(invalid.0.join("ruff.toml"), "[lint\nselect = ['F']\n").expect("broken config");
    let report = detect_json(&invalid);
    assert_eq!(checker(&report, "python.ruff")["configuration"], "invalid");

    let extension = TempProject::new();
    fs::write(extension.0.join("app.py"), "pass\n").expect("source");
    fs::write(
        extension.0.join("ruff.toml"),
        "extend = '../shared.toml'\n[lint]\nselect = ['F']\n",
    )
    .expect("dynamic config");
    let report = detect_json(&extension);
    assert_eq!(checker(&report, "python.ruff")["configuration"], "unknown");
    assert_eq!(
        checker(&report, "python.ruff")["reason"],
        "ruff_extend_not_resolved"
    );

    let malformed_section = TempProject::new();
    fs::write(malformed_section.0.join("app.py"), "pass\n").expect("source");
    fs::write(
        malformed_section.0.join("pyproject.toml"),
        "[tool]\nruff = 'yes'\n",
    )
    .expect("config");
    let report = detect_json(&malformed_section);
    assert_eq!(checker(&report, "python.ruff")["configuration"], "invalid");
    assert_eq!(
        checker(&report, "python.ruff")["reason"],
        "ruff_section_invalid"
    );
}

#[test]
fn dot_directories_are_not_python_sources_but_dot_ruff_config_is_discovered() {
    let project = TempProject::new();
    fs::create_dir(project.0.join(".venv")).expect("virtual environment");
    fs::write(project.0.join(".venv/third_party.py"), "import os\n").expect("external source");
    fs::write(project.0.join("app.py"), "pass\n").expect("project source");
    fs::write(project.0.join(".ruff.toml"), "[lint]\nselect = ['F']\n").expect("dot config");
    let report = detect_json(&project);
    assert_eq!(report["languages"][0]["id"], "python");
    assert_eq!(
        report["languages"][0]["source_files"],
        serde_json::json!(["app.py"])
    );
    assert_eq!(
        checker(&report, "python.ruff")["configuration"],
        "configured"
    );
    assert_eq!(
        checker(&report, "python.ruff")["configuration_ref"],
        ".ruff.toml"
    );
}

#[test]
fn node_flat_configs_are_observed_without_executing_or_claiming_effective_rules() {
    let project = TempProject::new();
    fs::write(
        project.0.join("package.json"),
        r#"{"name":"fixture","version":"1.0.0"}"#,
    )
    .unwrap();
    for extension in ["js", "mjs", "cjs", "ts", "mts", "cts"] {
        fs::write(
            project.0.join(format!("eslint.config.{extension}")),
            "throw new Error('must not execute');",
        )
        .unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["detect", project.0.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let configs: Vec<_> = report["checker_configurations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["checker_id"] == "node.eslint")
        .collect();
    assert_eq!(configs.len(), 6);
    for entry in configs {
        assert_eq!(entry["configuration"], "unknown");
        assert_eq!(entry["reason"], "eslint_config_selection_not_resolved");
    }
    assert!(!project.0.join(".codeguard").exists());
}

#[test]
fn node_config_not_observed_and_legacy_config_do_not_trigger_default_rules() {
    for legacy in [false, true] {
        let project = TempProject::new();
        fs::write(
            project.0.join("package.json"),
            if legacy {
                r#"{"version":"1.0.0","eslintConfig":{"rules":{"semi":"error"}}}"#
            } else {
                r#"{"version":"1.0.0"}"#
            },
        )
        .unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["detect", project.0.to_str().unwrap(), "--format", "json"])
            .output()
            .unwrap();
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        let entry = report["checker_configurations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["checker_id"] == "node.eslint")
            .unwrap();
        assert_eq!(entry["configuration"], "unknown");
        assert_eq!(
            entry["reason"],
            if legacy {
                "eslint_legacy_configuration_requires_tool_context"
            } else {
                "eslint_configuration_not_observed"
            }
        );
        assert!(!project.0.join("eslint.config.js").exists());
    }
}

#[test]
fn node_nested_config_roots_and_typescript_loader_require_native_confirmation() {
    let project = TempProject::new();
    fs::write(project.0.join("package.json"), r#"{"version":"1.0.0"}"#).unwrap();
    fs::write(project.0.join("eslint.config.js"), "export default []").unwrap();
    fs::create_dir_all(project.0.join("packages/web")).unwrap();
    fs::write(
        project.0.join("packages/web/package.json"),
        r#"{"version":"1.0.0"}"#,
    )
    .unwrap();
    fs::write(
        project.0.join("packages/web/eslint.config.cts"),
        "export default []",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["detect", project.0.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let entries: Vec<_> = report["checker_configurations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["checker_id"] == "node.eslint")
        .collect();
    assert_eq!(entries.len(), 2);
    let root = entries
        .iter()
        .find(|entry| entry["build_root"] == ".")
        .unwrap();
    let nested = entries
        .iter()
        .find(|entry| entry["build_root"] == "packages/web")
        .unwrap();
    assert_eq!(root["reason"], "eslint_dynamic_configuration_not_evaluated");
    assert_eq!(
        nested["reason"],
        "eslint_typescript_config_loader_not_verified"
    );
    assert!(
        entries
            .iter()
            .all(|entry| entry["configuration"] == "unknown")
    );
}

#[test]
fn standalone_node_source_without_package_returns_configuration_preparation() {
    let project = TempProject::new();
    fs::write(project.0.join("app.js"), "console.log(1)").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["detect", project.0.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let entry = report["checker_configurations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["checker_id"] == "node.eslint")
        .unwrap();
    assert_eq!(entry["configuration"], "unknown");
    assert_eq!(entry["reason"], "eslint_configuration_not_observed");
    assert_eq!(entry["configuration_ref"], "app.js");
    assert!(!project.0.join("package.json").exists());
}
