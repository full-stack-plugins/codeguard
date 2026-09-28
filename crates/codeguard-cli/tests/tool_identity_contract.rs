#![cfg(unix)]

use codeguard_cli::tool_identity::{
    current_platform_id, hash_bundle_tree, verify_locked_artifacts,
};
use codeguard_cli::tool_lock::{LockedBundle, LockedRuntime, LockedTool};
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "codeguard-tool-identity-{}-{id}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("fixture root");
        fs::create_dir(root.join("project")).expect("project");
        fs::create_dir_all(root.join("cache/maven/bin")).expect("cache");
        Self(root)
    }

    fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, bytes).expect("fixture binary");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("executable");
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove fixture");
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn locked(origin_kind: &str, origin_ref: &str, expected: &str) -> LockedTool {
    LockedTool {
        id: "maven".into(),
        version: "3.9.16".into(),
        binary_sha256: expected.into(),
        platform: current_platform_id()
            .expect("supported test platform")
            .into(),
        adapter_id: "java.maven".into(),
        adapter_version: "0.1.0".into(),
        rule_source_id: "maven-native".into(),
        rule_source_kind: "native_builtin".into(),
        rule_source_sha256: expected.into(),
        origin_kind: origin_kind.into(),
        origin_ref: origin_ref.into(),
        runtime: None,
        bundle: None,
    }
}

fn verify(
    fixture: &Fixture,
    locked: &LockedTool,
    runtime: Option<&Path>,
) -> codeguard_cli::tool_identity::LockedArtifactVerification {
    verify_locked_artifacts(
        locked,
        &fixture.0.join("project"),
        &fixture.0.join("cache"),
        runtime,
    )
}

#[test]
fn system_wrapper_and_managed_cache_sources_match_only_their_locked_bytes() {
    let fixture = Fixture::new();
    let bytes = b"#!/bin/sh\nexit 0\n";
    let system = fixture.file("mvn-system", bytes);
    let wrapper = fixture.file("project/mvnw", bytes);
    let cache = fixture.file("cache/maven/bin/mvn", bytes);
    for (kind, reference, expected_path) in [
        ("system", system.to_str().expect("utf8"), system.clone()),
        ("project_wrapper", "mvnw", wrapper),
        ("managed_cache", "maven/bin/mvn", cache),
    ] {
        let result = verify(&fixture, &locked(kind, reference, &digest(bytes)), None);
        assert!(result.tool.digest_matched);
        assert!(result.tool.executable_bit);
        assert_eq!(
            result.tool.path,
            Some(fs::canonicalize(expected_path).expect("canonical tool"))
        );
        assert_eq!(result.tool.issue, None);
    }
}

#[test]
fn digest_mismatch_and_missing_runtime_remain_visible() {
    let fixture = Fixture::new();
    let path = fixture.file("project/mvnw", b"#!/bin/sh\nexit 0\n");
    let mut tool = locked("project_wrapper", "mvnw", &digest(b"different"));
    tool.runtime = Some(LockedRuntime {
        id: "jdk".into(),
        version: "26.0.1".into(),
        binary_sha256: digest(b"java binary"),
    });
    let result = verify(&fixture, &tool, None);
    assert_eq!(
        result.tool.path,
        Some(fs::canonicalize(path).expect("canonical tool"))
    );
    assert_eq!(result.tool.issue, Some("artifact_digest_mismatch"));
    assert_eq!(
        result.runtime.expect("runtime").issue,
        Some("runtime_path_missing")
    );
}

#[test]
fn locked_runtime_bytes_are_checked_separately_from_the_tool() {
    let fixture = Fixture::new();
    let tool_bytes = b"#!/bin/sh\nexit 0\n";
    fixture.file("project/mvnw", tool_bytes);
    let runtime_bytes = b"jdk launcher";
    let runtime = fixture.file("java", runtime_bytes);
    let mut tool = locked("project_wrapper", "mvnw", &digest(tool_bytes));
    tool.runtime = Some(LockedRuntime {
        id: "jdk".into(),
        version: "26.0.1".into(),
        binary_sha256: digest(runtime_bytes),
    });
    let matched = verify(&fixture, &tool, Some(&runtime));
    assert_eq!(matched.tool.issue, None);
    assert_eq!(matched.runtime.expect("runtime").issue, None);
    fs::write(&runtime, b"different launcher").expect("tamper runtime");
    let changed = verify(&fixture, &tool, Some(&runtime));
    assert_eq!(changed.tool.issue, None);
    assert_eq!(
        changed.runtime.expect("runtime").issue,
        Some("artifact_digest_mismatch")
    );
}

#[test]
fn project_wrapper_cannot_escape_root_via_parent_or_symlink() {
    let fixture = Fixture::new();
    let outside = fixture.file("outside-mvn", b"#!/bin/sh\nexit 0\n");
    symlink(&outside, fixture.0.join("project/mvnw")).expect("escaping link");
    for reference in ["../outside-mvn", "mvnw"] {
        let result = verify(
            &fixture,
            &locked(
                "project_wrapper",
                reference,
                &digest(b"#!/bin/sh\nexit 0\n"),
            ),
            None,
        );
        assert_eq!(result.tool.issue, Some("origin_outside_root"));
        assert!(!result.tool.digest_matched);
    }
}

#[test]
fn non_executable_identity_is_not_a_startup_proof() {
    let fixture = Fixture::new();
    let bytes = b"binary";
    let tool = fixture.file("project/mvnw", bytes);
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o600)).expect("remove execute bit");
    let result = verify(
        &fixture,
        &locked("project_wrapper", "mvnw", &digest(bytes)),
        None,
    );
    assert!(result.tool.digest_matched);
    assert!(!result.tool.executable_bit);
    assert_eq!(result.tool.issue, Some("artifact_not_executable"));
}

#[test]
fn lock_for_another_platform_is_not_resolved() {
    let fixture = Fixture::new();
    let bytes = b"#!/bin/sh\nexit 0\n";
    fixture.file("project/mvnw", bytes);
    let mut tool = locked("project_wrapper", "mvnw", &digest(bytes));
    tool.platform = if current_platform_id() == Some("macos_arm64") {
        "linux_x86_64".into()
    } else {
        "macos_arm64".into()
    };
    let result = verify(&fixture, &tool, None);
    assert_eq!(result.tool.issue, Some("platform_mismatch"));
    assert!(result.tool.path.is_none());
}

#[test]
fn bundle_tree_hash_detects_delegated_artifact_changes() {
    let fixture = Fixture::new();
    let tool_bytes = b"#!/bin/sh\nexit 0\n";
    let tool = fixture.file("mvn-system", tool_bytes);
    let bundle = fixture.0.join("maven-home");
    fs::create_dir_all(bundle.join("bin")).expect("bundle bin");
    fs::create_dir_all(bundle.join("lib")).expect("bundle lib");
    fs::write(bundle.join("bin/mvn"), b"delegate").expect("delegate script");
    fs::write(bundle.join("lib/maven-core.jar"), b"jar-v1").expect("core jar");
    let expected = hash_bundle_tree(&bundle).expect("tree digest");
    assert_eq!(hash_bundle_tree(&bundle).expect("stable digest"), expected);
    let mut lock = locked("system", tool.to_str().expect("utf8"), &digest(tool_bytes));
    lock.bundle = Some(LockedBundle {
        root: bundle.to_str().expect("utf8").into(),
        tree_sha256: expected,
    });
    let matched = verify(&fixture, &lock, None);
    assert_eq!(matched.tool.issue, None);
    assert_eq!(matched.bundle.expect("bundle").issue, None);
    fs::write(bundle.join("lib/maven-core.jar"), b"jar-v2").expect("change delegated jar");
    let changed = verify(&fixture, &lock, None);
    assert_eq!(changed.tool.issue, None);
    assert_eq!(
        changed.bundle.expect("bundle").issue,
        Some("bundle_digest_mismatch")
    );
}

#[test]
fn symlink_inside_bundle_cannot_be_accepted_as_a_complete_tree() {
    let fixture = Fixture::new();
    let bundle = fixture.0.join("maven-home");
    fs::create_dir(&bundle).expect("bundle");
    let outside = fixture.file("outside", b"jar");
    symlink(outside, bundle.join("core.jar")).expect("bundle symlink");
    assert_eq!(hash_bundle_tree(&bundle), Err("bundle_special_or_symlink"));
}
