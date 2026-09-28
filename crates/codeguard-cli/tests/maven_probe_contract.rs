#![cfg(unix)]

use codeguard_cli::maven_probe::{
    MavenProbeContext, MavenProbeRequest, MavenProbeState, run_maven_probe,
};
use codeguard_cli::tool_identity::{current_platform_id, hash_bundle_tree};
use codeguard_cli::tool_lock::{LockedBundle, LockedRuntime, LockedTool};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .canonicalize()
            .expect("规范化临时目录")
            .join(format!("codeguard-maven-probe-{}-{id}", std::process::id()));
        fs::create_dir(&path).expect("project root");
        fs::create_dir(path.join("evidence")).expect("evidence root");
        fs::set_permissions(path.join("evidence"), fs::Permissions::from_mode(0o700))
            .expect("private evidence");
        fs::write(path.join("pom.xml"), "<project/>").expect("pom");
        Self(path)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove fixture");
    }
}

fn digest(path: &Path) -> [u8; 32] {
    Sha256::digest(fs::read(path).expect("tool bytes")).into()
}

fn request(fixture: &Fixture, tool: PathBuf) -> MavenProbeRequest {
    MavenProbeRequest {
        project_root: fixture.0.clone(),
        expected_tool_sha256: digest(&tool),
        tool,
        expected_maven_line: "Apache Maven 3.9.16".into(),
        expected_java_prefix: "Java version: 26.0.1".into(),
        runtime_identity: None,
        bundle_identity: None,
        environment: BTreeMap::from([
            (
                OsString::from("PATH"),
                OsString::from("/opt/homebrew/bin:/usr/bin:/bin"),
            ),
            (OsString::from("MAVEN_SKIP_RC"), OsString::from("1")),
        ]),
        evidence_dir: fixture.0.join("evidence"),
        run_id: "maven-probe".into(),
        deadline: Instant::now() + Duration::from_secs(20),
    }
}

fn locked_tool(tool: &Path, java: &Path, bundle_root: &Path) -> LockedTool {
    let tool_digest = format!("{:x}", Sha256::digest(fs::read(tool).expect("tool bytes")));
    let runtime_digest = format!("{:x}", Sha256::digest(fs::read(java).expect("java bytes")));
    LockedTool {
        id: "maven".into(),
        version: "3.9.16".into(),
        binary_sha256: tool_digest.clone(),
        platform: current_platform_id().expect("platform").into(),
        adapter_id: "java.maven".into(),
        adapter_version: "0.1.0".into(),
        rule_source_id: "maven-native".into(),
        rule_source_kind: "native_builtin".into(),
        rule_source_sha256: tool_digest,
        origin_kind: "system".into(),
        origin_ref: tool.to_str().expect("utf8 tool").into(),
        runtime: Some(LockedRuntime {
            id: "jdk".into(),
            version: "26.0.1".into(),
            binary_sha256: runtime_digest,
        }),
        bundle: Some(LockedBundle {
            root: bundle_root.to_str().expect("utf8 bundle").into(),
            tree_sha256: hash_bundle_tree(bundle_root).expect("bundle digest"),
        }),
    }
}

#[test]
fn locked_builder_requires_matching_jdk_bytes_and_java_home() {
    let fixture = Fixture::new();
    let tool = fixture.0.join("mvn");
    fs::write(&tool, b"#!/bin/sh\nexit 0\n").expect("fake Maven");
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).expect("executable");
    let java_home = fixture.0.join("jdk");
    fs::create_dir_all(java_home.join("bin")).expect("java bin");
    let java = java_home.join("bin/java");
    fs::write(&java, b"jdk launcher").expect("java bytes");
    fs::set_permissions(&java, fs::Permissions::from_mode(0o700)).expect("executable");
    let bundle_root = fixture.0.join("maven-home");
    fs::create_dir(&bundle_root).expect("bundle root");
    fs::write(bundle_root.join("maven-core.jar"), b"jar").expect("bundle jar");
    let lock = locked_tool(&tool, &java, &bundle_root);
    let environment = BTreeMap::from([(
        OsString::from("JAVA_HOME"),
        java_home.as_os_str().to_os_string(),
    )]);
    let built = MavenProbeRequest::from_locked_artifacts(
        &lock,
        MavenProbeContext {
            project_root: fixture.0.clone(),
            managed_cache_root: fixture.0.join("cache"),
            runtime_path: java.clone(),
            environment: environment.clone(),
            evidence_dir: fixture.0.join("evidence"),
            run_id: "locked".into(),
            deadline: Instant::now() + Duration::from_secs(2),
        },
    )
    .expect("matching artifacts");
    assert_eq!(built.tool, fs::canonicalize(&tool).expect("canonical tool"));
    fs::write(&java, b"changed").expect("tamper Java");
    assert!(
        MavenProbeRequest::from_locked_artifacts(
            &lock,
            MavenProbeContext {
                project_root: fixture.0.clone(),
                managed_cache_root: fixture.0.join("cache"),
                runtime_path: java,
                environment,
                evidence_dir: fixture.0.join("evidence"),
                run_id: "locked".into(),
                deadline: Instant::now() + Duration::from_secs(2),
            },
        )
        .is_err()
    );
}

#[test]
fn runtime_mutation_during_native_validate_invalidates_local_evidence() {
    let fixture = Fixture::new();
    let java_home = fixture.0.join("jdk");
    fs::create_dir_all(java_home.join("bin")).expect("java bin");
    let java = java_home.join("bin/java");
    fs::write(&java, b"jdk launcher").expect("java bytes");
    fs::set_permissions(&java, fs::Permissions::from_mode(0o700)).expect("executable");
    let tool = fixture.0.join("mvn");
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  printf 'Apache Maven 3.9.16\\nJava version: 26.0.1\\n'\n  exit 0\nfi\nprintf changed > '{}'\nexit 0\n",
        java.display()
    );
    fs::write(&tool, script).expect("fake Maven");
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).expect("executable");
    let bundle_root = fixture.0.join("maven-home");
    fs::create_dir(&bundle_root).expect("bundle root");
    fs::write(bundle_root.join("maven-core.jar"), b"jar").expect("bundle jar");
    let lock = locked_tool(&tool, &java, &bundle_root);
    let environment = BTreeMap::from([
        (
            OsString::from("JAVA_HOME"),
            java_home.as_os_str().to_os_string(),
        ),
        (OsString::from("PATH"), OsString::from("/usr/bin:/bin")),
    ]);
    let request = MavenProbeRequest::from_locked_artifacts(
        &lock,
        MavenProbeContext {
            project_root: fixture.0.clone(),
            managed_cache_root: fixture.0.join("cache"),
            runtime_path: java,
            environment,
            evidence_dir: fixture.0.join("evidence"),
            run_id: "mutating-runtime".into(),
            deadline: Instant::now() + Duration::from_secs(2),
        },
    )
    .expect("matching artifacts before execution");
    let result = run_maven_probe(&request, &AtomicBool::new(false));
    assert_eq!(result.state, MavenProbeState::Incomplete);
    assert_eq!(result.reason, Some("runtime_identity_changed"));
    assert_eq!(result.native_exit, Some(0));
}

#[test]
fn delegated_bundle_mutation_during_validate_invalidates_local_evidence() {
    let fixture = Fixture::new();
    let java_home = fixture.0.join("jdk");
    fs::create_dir_all(java_home.join("bin")).expect("java bin");
    let java = java_home.join("bin/java");
    fs::write(&java, b"jdk launcher").expect("java bytes");
    fs::set_permissions(&java, fs::Permissions::from_mode(0o700)).expect("executable");
    let bundle_root = fixture.0.join("maven-home");
    fs::create_dir(&bundle_root).expect("bundle root");
    let delegated = bundle_root.join("maven-core.jar");
    fs::write(&delegated, b"jar-v1").expect("bundle jar");
    let tool = fixture.0.join("mvn");
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  printf 'Apache Maven 3.9.16\\nJava version: 26.0.1\\n'\n  exit 0\nfi\nprintf jar-v2 > '{}'\nexit 0\n",
        delegated.display()
    );
    fs::write(&tool, script).expect("fake Maven");
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).expect("executable");
    let lock = locked_tool(&tool, &java, &bundle_root);
    let environment = BTreeMap::from([
        (
            OsString::from("JAVA_HOME"),
            java_home.as_os_str().to_os_string(),
        ),
        (OsString::from("PATH"), OsString::from("/usr/bin:/bin")),
    ]);
    let request = MavenProbeRequest::from_locked_artifacts(
        &lock,
        MavenProbeContext {
            project_root: fixture.0.clone(),
            managed_cache_root: fixture.0.join("cache"),
            runtime_path: java,
            environment,
            evidence_dir: fixture.0.join("evidence"),
            run_id: "mutating-bundle".into(),
            deadline: Instant::now() + Duration::from_secs(2),
        },
    )
    .expect("matching artifacts before execution");
    let result = run_maven_probe(&request, &AtomicBool::new(false));
    assert_eq!(result.state, MavenProbeState::Incomplete);
    assert_eq!(result.reason, Some("bundle_identity_changed"));
    assert_eq!(result.native_exit, Some(0));
}

#[test]
fn tool_mismatch_blocks_before_any_native_execution() {
    let fixture = Fixture::new();
    let tool = fixture.0.join("mvn");
    fs::write(&tool, b"#!/bin/sh\nexit 0\n").expect("fake tool");
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).expect("executable");
    let mut request = request(&fixture, tool);
    request.expected_tool_sha256 = [0; 32];
    let result = run_maven_probe(&request, &AtomicBool::new(false));
    assert_eq!(result.state, MavenProbeState::Incomplete);
    assert_eq!(result.reason, Some("tool_identity_mismatch"));
    assert!(
        fs::read_dir(fixture.0.join("evidence"))
            .expect("evidence entries")
            .next()
            .is_none()
    );
}

#[test]
fn maven_rc_loading_must_be_disabled_before_any_native_execution() {
    let fixture = Fixture::new();
    let tool = fixture.0.join("mvn");
    fs::write(&tool, b"#!/bin/sh\nexit 0\n").expect("fake tool");
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).expect("executable");
    let mut request = request(&fixture, tool);
    request.environment.remove(&OsString::from("MAVEN_SKIP_RC"));
    let result = run_maven_probe(&request, &AtomicBool::new(false));
    assert_eq!(result.state, MavenProbeState::Incomplete);
    assert_eq!(result.reason, Some("invalid_probe_request"));
    assert!(
        fs::read_dir(fixture.0.join("evidence"))
            .expect("evidence entries")
            .next()
            .is_none()
    );
}

#[test]
fn native_nonzero_is_incomplete_even_when_maven_version_matches() {
    let fixture = Fixture::new();
    let tool = fixture.0.join("mvn");
    fs::write(&tool, b"#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  printf 'Apache Maven 3.9.16\\nJava version: 26.0.1\\n'\n  exit 0\nfi\nexit 7\n").expect("fake tool");
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).expect("executable");
    let result = run_maven_probe(&request(&fixture, tool), &AtomicBool::new(false));
    assert_eq!(result.state, MavenProbeState::Incomplete);
    assert_eq!(result.reason, Some("native_validate_failed"));
    assert_eq!(result.native_exit, Some(7));
    assert!(
        fixture
            .0
            .join("evidence/maven-probe-maven-version.log")
            .exists()
    );
    assert!(
        fixture
            .0
            .join("evidence/maven-probe-maven-validate.log")
            .exists()
    );
}

#[test]
fn wrong_version_cannot_advance_to_validate() {
    let fixture = Fixture::new();
    let tool = fixture.0.join("mvn");
    fs::write(
        &tool,
        b"#!/bin/sh\nprintf 'Apache Maven 3.8.0\\nJava version: 26.0.1\\n'\nexit 0\n",
    )
    .expect("fake tool");
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).expect("executable");
    let result = run_maven_probe(&request(&fixture, tool), &AtomicBool::new(false));
    assert_eq!(result.state, MavenProbeState::Incomplete);
    assert_eq!(result.reason, Some("tool_version_mismatch"));
    assert!(
        !fixture
            .0
            .join("evidence/maven-probe-maven-validate.log")
            .exists()
    );
}

#[test]
#[ignore = "requires pinned Maven/JDK; run with CODEGUARD_MAVEN_BIN"]
fn real_maven_model_failure_stays_environment_incomplete() {
    let fixture = Fixture::new();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/corpus/maven_pom_4_1/pom.xml");
    fs::copy(source, fixture.0.join("pom.xml")).expect("copy fixture pom");
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_MAVEN_BIN").expect("Maven path"));
    let result = run_maven_probe(&request(&fixture, tool), &AtomicBool::new(false));
    assert_eq!(result.state, MavenProbeState::Incomplete);
    assert_eq!(result.reason, Some("native_validate_failed"));
    assert!(result.native_exit.is_some_and(|code| code != 0));
}

#[test]
#[ignore = "requires pinned Maven/JDK; run with CODEGUARD_MAVEN_BIN"]
fn real_maven_validate_success_remains_only_local_model_evidence() {
    let fixture = Fixture::new();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/corpus/maven_verify_without_quality/pom.xml");
    fs::copy(source, fixture.0.join("pom.xml")).expect("copy fixture pom");
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_MAVEN_BIN").expect("Maven path"));
    let result = run_maven_probe(&request(&fixture, tool), &AtomicBool::new(false));
    assert_eq!(result.state, MavenProbeState::NativeValidateComplete);
    assert_eq!(result.native_exit, Some(0));
    assert!(result.reason.is_none());
}

#[test]
#[ignore = "requires pinned Maven/JDK; run with CODEGUARD_MAVEN_BIN and CODEGUARD_JAVA_HOME"]
fn real_maven_probe_can_be_built_from_matching_locked_artifacts() {
    let fixture = Fixture::new();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/corpus/maven_pom_4_1/pom.xml");
    fs::copy(source, fixture.0.join("pom.xml")).expect("copy fixture pom");
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_MAVEN_BIN").expect("Maven path"));
    let java_home = PathBuf::from(std::env::var_os("CODEGUARD_JAVA_HOME").expect("JDK home"));
    let java = java_home.join("bin/java");
    let bundle_root = PathBuf::from(std::env::var_os("CODEGUARD_MAVEN_HOME").expect("Maven home"));
    let lock = locked_tool(&tool, &java, &bundle_root);
    let environment = BTreeMap::from([
        (
            OsString::from("JAVA_HOME"),
            java_home.as_os_str().to_os_string(),
        ),
        (
            OsString::from("PATH"),
            OsString::from("/opt/homebrew/bin:/usr/bin:/bin"),
        ),
    ]);
    let request = MavenProbeRequest::from_locked_artifacts(
        &lock,
        MavenProbeContext {
            project_root: fixture.0.clone(),
            managed_cache_root: fixture.0.join("cache"),
            runtime_path: java,
            environment,
            evidence_dir: fixture.0.join("evidence"),
            run_id: "locked-maven".into(),
            deadline: Instant::now() + Duration::from_secs(20),
        },
    )
    .expect("matching local artifacts");
    let result = run_maven_probe(&request, &AtomicBool::new(false));
    assert_eq!(result.state, MavenProbeState::Incomplete);
    assert_eq!(result.reason, Some("native_validate_failed"));
}

#[test]
fn version_timeout_preserves_runtime_failure_kind() {
    let fixture = Fixture::new();
    let tool = fixture.0.join("mvn");
    fs::write(&tool, b"#!/bin/sh\n/bin/sleep 5\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let mut request = request(&fixture, tool);
    request.deadline = Instant::now() + Duration::from_millis(100);
    let result = run_maven_probe(&request, &AtomicBool::new(false));
    assert_eq!(result.state, MavenProbeState::Incomplete);
    assert!(
        matches!(
            result.reason,
            Some("version_timed_out" | "version_deadline_before_start")
        ),
        "reason: {:?}",
        result.reason
    );
    assert_eq!(result.native_exit, None);
    assert!(
        !fixture
            .0
            .join("evidence/maven-probe-maven-validate.log")
            .exists()
    );
}

#[test]
fn version_spawn_failure_is_distinct_from_timeout() {
    let fixture = Fixture::new();
    let tool = fixture.0.join("mvn");
    fs::write(&tool, b"#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o600)).unwrap();
    let result = run_maven_probe(&request(&fixture, tool), &AtomicBool::new(false));
    assert_eq!(result.state, MavenProbeState::Incomplete);
    assert_eq!(result.reason, Some("version_spawn_failure"));
    assert_eq!(result.native_exit, None);
}
