#![cfg(unix)]

use codeguard_adapters::{PmdParseState, PmdParsed, parse_pmd_xml};
use codeguard_cli::tool_identity::hash_bundle_tree;
use codeguard_runtime::{
    ProcessOutcome, ProcessSpec, Termination, read_bounded_regular_file, run_process_recorded,
};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// 原生验收共用的 Maven、JDK 与离线依赖仓库身份。
struct NativeTestContext {
    maven: PathBuf,
    java_home: PathBuf,
    maven_repo: PathBuf,
    expected_repo_sha256: String,
}

fn run_case(
    name: &str,
    source_name: &str,
    source: &[u8],
    ruleset: &str,
    context: &NativeTestContext,
) -> (ProcessOutcome, Option<PmdParsed>) {
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir()
        .canonicalize()
        .expect("physical temp root")
        .join(format!(
            "codeguard-p3c-native-{}-{id}-{name}",
            std::process::id()
        ));
    let sources = root.join("src/main/java");
    let evidence = root.join("evidence");
    fs::create_dir_all(&sources).expect("source directory");
    fs::create_dir(&evidence).expect("evidence directory");
    fs::set_permissions(&evidence, fs::Permissions::from_mode(0o700)).expect("private evidence");
    let pom = include_str!("../../../tests/fixtures/p3c_native/pom.xml")
        .replace("rulesets/java/ali-naming.xml", ruleset);
    fs::write(root.join("pom.xml"), pom).expect("fixture POM");
    let settings = root.join("settings.xml");
    fs::write(
        &settings,
        include_str!("../../../tests/fixtures/p3c_native/settings.xml"),
    )
    .expect("isolated settings");
    let source_path = sources.join(source_name);
    fs::write(&source_path, source).expect("fixture source");
    let mut env = BTreeMap::new();
    env.insert(
        OsString::from("JAVA_HOME"),
        context.java_home.as_os_str().to_os_string(),
    );
    env.insert(OsString::from("MAVEN_SKIP_RC"), OsString::from("1"));
    env.insert(
        OsString::from("PATH"),
        OsString::from(format!(
            "{}:/opt/homebrew/bin:/usr/bin:/bin",
            context.java_home.join("bin").display()
        )),
    );
    let spec = ProcessSpec {
        executable: context.maven.clone(),
        args: vec![
            "-B".into(),
            "-ntp".into(),
            "-q".into(),
            "-o".into(),
            "-s".into(),
            settings.as_os_str().to_os_string(),
            "-gs".into(),
            settings.as_os_str().to_os_string(),
            format!("-Dmaven.repo.local={}", context.maven_repo.display()).into(),
            "org.apache.maven.plugins:maven-pmd-plugin:3.11.0:pmd".into(),
        ],
        cwd: root.clone(),
        env,
        stdin: None,
        deadline: Instant::now() + Duration::from_secs(120),
        output_limit_bytes: 1024 * 1024,
    };
    let outcome = run_process_recorded(&spec, &AtomicBool::new(false), &evidence, "maven-pmd.log")
        .expect("private native evidence");
    assert_eq!(
        hash_bundle_tree(&context.maven_repo).expect("native dependency closure"),
        context.expected_repo_sha256,
        "offline Maven execution must not modify the dependency closure"
    );
    let report = read_bounded_regular_file(&root.join("target/pmd.xml"), 16 * 1024 * 1024)
        .ok()
        .map(|bytes| parse_pmd_xml(&bytes, "6.15.0"));
    if name == "violating" {
        assert_eq!(
            report.as_ref().expect("native report").files,
            vec![source_path.to_string_lossy().into_owned()]
        );
    }
    fs::remove_dir_all(root).expect("remove isolated fixture");
    (outcome, report)
}

#[test]
#[ignore = "requires native Maven, JDK and the pinned P3C/PMD Maven artifacts"]
fn native_p3c_2_1_1_rule_loading_and_failure_matrix() {
    let context = NativeTestContext {
        maven: PathBuf::from(std::env::var_os("CODEGUARD_MAVEN_BIN").expect("Maven path")),
        java_home: PathBuf::from(std::env::var_os("CODEGUARD_JAVA_HOME").expect("JDK home")),
        maven_repo: PathBuf::from(
            std::env::var_os("CODEGUARD_P3C_MAVEN_REPO").expect("isolated Maven repository"),
        ),
        expected_repo_sha256: std::env::var("CODEGUARD_P3C_REPO_TREE_SHA256")
            .expect("locked Maven repository digest"),
    };
    assert!(context.maven.is_absolute() && context.java_home.is_absolute());
    assert!(context.maven_repo.is_absolute());
    let actual_repo_sha256 =
        hash_bundle_tree(&context.maven_repo).expect("native dependency closure");
    println!("P3C_MAVEN_REPO_TREE_SHA256={actual_repo_sha256}");
    assert_eq!(actual_repo_sha256, context.expected_repo_sha256);
    let violating = include_bytes!("../../../tests/fixtures/p3c_native/violating/Bad_Name.java");
    let clean = include_bytes!("../../../tests/fixtures/p3c_native/clean/GoodName.java");
    let (bad_outcome, bad_report) = run_case(
        "violating",
        "Bad_Name.java",
        violating,
        "rulesets/java/ali-naming.xml",
        &context,
    );
    assert_eq!(bad_outcome.termination, Termination::Exited(0));
    let bad = bad_report.expect("violating native report");
    assert_eq!(bad.state, PmdParseState::ValidReport);
    assert!(bad.diagnostics.iter().any(|finding| {
        finding.rule == "ClassNamingShouldBeCamelRule" && finding.ruleset == "AlibabaJavaNaming"
    }));

    let (clean_outcome, clean_report) = run_case(
        "clean",
        "GoodName.java",
        clean,
        "rulesets/java/ali-naming.xml",
        &context,
    );
    assert_eq!(clean_outcome.termination, Termination::Exited(0));
    let clean = clean_report.expect("clean native report");
    assert_eq!(clean.state, PmdParseState::ValidReport);
    assert!(clean.diagnostics.is_empty());
    assert!(
        clean.files.is_empty(),
        "PMD XML does not attest clean-file coverage"
    );

    let (missing_outcome, _) = run_case(
        "missing",
        "Bad_Name.java",
        violating,
        "rulesets/java/does-not-exist.xml",
        &context,
    );
    assert_ne!(missing_outcome.termination, Termination::Exited(0));
}
