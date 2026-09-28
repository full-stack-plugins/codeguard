#![cfg(unix)]
use codeguard_runtime::publish_tool_bytes;
use ring::digest::{SHA256, digest};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Duration, Instant},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Cache(PathBuf);
impl Cache {
    fn new() -> Self {
        let path = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-install-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
    fn publish(&self, bytes: &[u8]) -> Result<codeguard_runtime::InstalledArtifact, &'static str> {
        publish_tool_bytes(
            &self.0,
            bytes,
            digest(&SHA256, bytes).as_ref().try_into().unwrap(),
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false),
        )
    }
}
impl Drop for Cache {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn publishes_verified_executable_without_running_it_and_reuses_exact_bytes() {
    let cache = Cache::new();
    let bytes = b"#!/bin/sh\ntouch marker\n";
    let first = cache.publish(bytes).unwrap();
    assert!(first.newly_published);
    assert_eq!(fs::read(cache.0.join(&first.relative_path)).unwrap(), bytes);
    assert_eq!(
        fs::metadata(cache.0.join(&first.relative_path))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    let second = cache.publish(bytes).unwrap();
    assert!(!second.newly_published);
    assert_eq!(first.relative_path, second.relative_path);
    assert_eq!(fs::read_dir(&cache.0).unwrap().count(), 1);
    assert!(!cache.0.join("marker").exists());
}
#[test]
fn digest_mismatch_cancel_and_expired_budget_publish_nothing() {
    let cache = Cache::new();
    let hash: [u8; 32] = digest(&SHA256, b"expected").as_ref().try_into().unwrap();
    assert_eq!(
        publish_tool_bytes(
            &cache.0,
            b"changed",
            hash,
            Instant::now() + Duration::from_secs(1),
            &AtomicBool::new(false)
        )
        .unwrap_err(),
        "install_digest_mismatch"
    );
    assert_eq!(
        publish_tool_bytes(
            &cache.0,
            b"expected",
            hash,
            Instant::now() + Duration::from_secs(1),
            &AtomicBool::new(true)
        )
        .unwrap_err(),
        "install_cancelled"
    );
    assert_eq!(
        publish_tool_bytes(
            &cache.0,
            b"expected",
            hash,
            Instant::now(),
            &AtomicBool::new(false)
        )
        .unwrap_err(),
        "install_deadline_exceeded"
    );
    assert_eq!(fs::read_dir(&cache.0).unwrap().count(), 0);
}
#[test]
fn existing_corrupt_artifact_is_not_overwritten() {
    let cache = Cache::new();
    let first = cache.publish(b"original").unwrap();
    let path = cache.0.join(first.relative_path);
    fs::write(&path, b"corrupt").unwrap();
    assert_eq!(
        cache.publish(b"original").unwrap_err(),
        "installed_artifact_invalid"
    );
    assert_eq!(fs::read(path).unwrap(), b"corrupt");
    assert_eq!(fs::read_dir(&cache.0).unwrap().count(), 1);
}
#[test]
fn cache_root_and_destination_links_are_not_followed() {
    let cache = Cache::new();
    let outside = Cache::new();
    let link = cache.0.join("linked");
    symlink(&outside.0, &link).unwrap();
    let bytes = b"tool";
    let hash = digest(&SHA256, bytes).as_ref().try_into().unwrap();
    assert_eq!(
        publish_tool_bytes(
            &link,
            bytes,
            hash,
            Instant::now() + Duration::from_secs(1),
            &AtomicBool::new(false)
        )
        .unwrap_err(),
        "install_cache_unavailable"
    );
    let first = cache.publish(bytes).unwrap();
    let target = cache.0.join(first.relative_path);
    fs::remove_file(&target).unwrap();
    symlink(outside.0.join("victim"), target).unwrap();
    assert_eq!(
        cache.publish(bytes).unwrap_err(),
        "installed_artifact_invalid"
    );
    assert_eq!(fs::read_dir(&outside.0).unwrap().count(), 0);
}
#[test]
fn writable_shared_cache_and_nonexecutable_existing_file_are_rejected() {
    let cache = Cache::new();
    fs::set_permissions(&cache.0, fs::Permissions::from_mode(0o777)).unwrap();
    assert_eq!(
        cache.publish(b"tool").unwrap_err(),
        "install_cache_permissions_invalid"
    );
    fs::set_permissions(&cache.0, fs::Permissions::from_mode(0o700)).unwrap();
    let first = cache.publish(b"tool").unwrap();
    fs::set_permissions(
        cache.0.join(first.relative_path),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    assert_eq!(
        cache.publish(b"tool").unwrap_err(),
        "installed_artifact_invalid"
    );
}

#[test]
fn hardlinked_destination_and_empty_input_do_not_get_reused() {
    let cache = Cache::new();
    assert_eq!(cache.publish(b"").unwrap_err(), "install_input_invalid");
    let first = cache.publish(b"tool").unwrap();
    fs::hard_link(
        cache.0.join(first.relative_path),
        cache.0.join("other-link"),
    )
    .unwrap();
    assert_eq!(
        cache.publish(b"tool").unwrap_err(),
        "installed_artifact_invalid"
    );
    assert_eq!(fs::read(cache.0.join("other-link")).unwrap(), b"tool");
}

#[test]
fn stale_stage_is_preserved_and_two_valid_artifacts_coexist() {
    let cache = Cache::new();
    let stale = cache.0.join(format!(".install-{}-0", std::process::id()));
    fs::write(&stale, b"previous interrupted bytes").unwrap();
    let first = cache.publish(b"first tool").unwrap();
    let second = cache.publish(b"second tool").unwrap();
    assert_ne!(first.relative_path, second.relative_path);
    assert_eq!(fs::read(&stale).unwrap(), b"previous interrupted bytes");
    assert_eq!(fs::read_dir(&cache.0).unwrap().count(), 3);
}

#[test]
fn concurrent_publication_never_overwrites_and_can_be_retried() {
    let cache = Cache::new();
    let mut handles = Vec::new();
    for _ in 0..4 {
        let path = cache.0.clone();
        handles.push(std::thread::spawn(move || {
            let bytes = b"concurrent tool";
            publish_tool_bytes(
                &path,
                bytes,
                digest(&SHA256, bytes).as_ref().try_into().unwrap(),
                Instant::now() + Duration::from_secs(5),
                &AtomicBool::new(false),
            )
        }));
    }
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert!(results.iter().any(Result::is_ok));
    // 同时出现的短暂双链接保守拒绝；重试必须核验已有最终字节。
    for reason in results.into_iter().filter_map(Result::err) {
        assert_eq!(reason, "installed_artifact_invalid");
    }
    let receipt = cache.publish(b"concurrent tool").unwrap();
    assert!(!receipt.newly_published);
    assert_eq!(
        fs::read(cache.0.join(receipt.relative_path)).unwrap(),
        b"concurrent tool"
    );
    assert_eq!(fs::read_dir(&cache.0).unwrap().count(), 1);
}

#[test]
fn oversized_or_placeholder_identity_is_rejected_before_writing() {
    let cache = Cache::new();
    let bytes = vec![0; 128 * 1024 * 1024 + 1];
    assert_eq!(
        publish_tool_bytes(
            &cache.0,
            &bytes,
            [1; 32],
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false)
        )
        .unwrap_err(),
        "install_input_invalid"
    );
    assert_eq!(
        publish_tool_bytes(
            &cache.0,
            b"tool",
            [0; 32],
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false)
        )
        .unwrap_err(),
        "install_input_invalid"
    );
    assert_eq!(fs::read_dir(&cache.0).unwrap().count(), 0);
}

#[test]
fn fifo_destination_is_rejected_without_waiting_for_a_writer() {
    let cache = Cache::new();
    let receipt = cache.publish(b"tool").unwrap();
    let path = cache.0.join(receipt.relative_path);
    fs::remove_file(&path).unwrap();
    let name = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    assert_eq!(
        cache.publish(b"tool").unwrap_err(),
        "installed_artifact_invalid"
    );
}

#[test]
#[ignore = "需显式指定 CODEGUARD_TEST_RUFF 并执行真实原生版本验收"]
fn real_ruff_publication_retains_native_version_without_policy_authority() {
    let source = PathBuf::from(std::env::var("CODEGUARD_TEST_RUFF").expect("显式原生 Ruff 路径"));
    let bytes = codeguard_runtime::read_bounded_regular_file(&source, 128 * 1024 * 1024).unwrap();
    let cache = Cache::new();
    let receipt = cache.publish(&bytes).unwrap();
    let expected = digest(&SHA256, &bytes).as_ref().try_into().unwrap();
    let observed = codeguard_runtime::observe_native_version(
        &codeguard_runtime::NativeVersionRequest {
            process: codeguard_runtime::ProcessSpec {
                executable: cache.0.join(receipt.relative_path),
                args: vec!["--version".into()],
                cwd: cache.0.clone(),
                env: std::collections::BTreeMap::new(),
                stdin: None,
                deadline: Instant::now() + Duration::from_secs(5),
                output_limit_bytes: 4096,
            },
            expected_tool_sha256: expected,
            expected_stdout: b"ruff 0.16.8\n".to_vec(),
            evidence_root: cache.0.clone(),
            log_name: "version.log".into(),
        },
        &AtomicBool::new(false),
    );
    assert!(observed.complete, "{:?}", observed.reason);
}
