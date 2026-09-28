use codeguard_cli::distribution_raw::inspect_distribution_raw;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}
fn inputs(platform: &str) -> (Vec<u8>, Value) {
    let digest = sha(b"tool");
    let reference = format!("{digest}.bin");
    let lock=serde_json::to_vec(&json!({"schema_version":"1.0","lock_id":"candidate","tools":[{"id":"tool","version":"1","platform":platform,"binary_sha256":digest,"adapter":{"id":"test","version":"1"},"rule_source":{"kind":"native_builtin","id":"test","sha256":"b".repeat(64)},"origin":{"kind":"managed_cache","ref":reference}}]})).unwrap();
    let manifest = json!({"schema_version":"1.2","manifest_id":"test","lock_sha256":sha(&lock),"artifacts":[{"tool_id":"tool","version":"1","platform":platform,"binary_sha256":digest,"origin_ref_sha256":sha(reference.as_bytes()),"download_url":"https://example.org/tool.bin","package_sha256":digest,"package_size_bytes":4,"format":"raw","unpacked_size_limit_bytes":4}]});
    (lock, manifest)
}
#[test]
fn raw_bytes_bind_exact_content_addressed_locator_without_copy_or_approval() {
    let (lock, m) = inputs("linux_x86_64");
    let bytes = b"tool";
    let observed = inspect_distribution_raw(
        &serde_json::to_vec(&m).unwrap(),
        &lock,
        "tool",
        "linux_x86_64",
        bytes,
        deadline(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(observed.bytes().as_ptr(), bytes.as_ptr());
    assert_eq!(observed.origin_ref(), format!("{}.bin", sha(bytes)));
    assert_eq!(observed.binary_sha256(), Sha256::digest(bytes)[..]);
    assert_eq!(observed.lock_sha256(), sha(&lock));
}
#[test]
fn invalid_bytes_missing_mapping_and_budget_never_return_success() {
    let (lock, m) = inputs("linux_x86_64");
    let mb = serde_json::to_vec(&m).unwrap();
    for bytes in [
        b"bad".as_slice(),
        b"evil".as_slice(),
        b"tool-more".as_slice(),
    ] {
        assert!(
            inspect_distribution_raw(
                &mb,
                &lock,
                "tool",
                "linux_x86_64",
                bytes,
                deadline(),
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
    for version in ["1.0", "1.1"] {
        let mut old = m.clone();
        old["schema_version"] = version.into();
        assert!(
            inspect_distribution_raw(
                &serde_json::to_vec(&old).unwrap(),
                &lock,
                "tool",
                "linux_x86_64",
                b"tool",
                deadline(),
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
    assert!(
        inspect_distribution_raw(
            &mb,
            &lock,
            "other",
            "linux_x86_64",
            b"tool",
            deadline(),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    assert!(
        inspect_distribution_raw(
            &mb,
            &lock,
            "tool",
            "linux_x86_64",
            b"tool",
            deadline(),
            &AtomicBool::new(true)
        )
        .is_err()
    );
    assert!(
        inspect_distribution_raw(
            &mb,
            &lock,
            "tool",
            "linux_x86_64",
            b"tool",
            Instant::now(),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    let mut changed = lock.clone();
    changed.push(b' ');
    assert!(
        inspect_distribution_raw(
            &mb,
            &changed,
            "tool",
            "linux_x86_64",
            b"tool",
            deadline(),
            &AtomicBool::new(false)
        )
        .is_err()
    );
}
#[test]
fn even_a_matching_source_ref_digest_cannot_authorize_a_different_raw_locator() {
    let (lock, mut m) = inputs("linux_x86_64");
    let mut wrong: Value = serde_json::from_slice(&lock).unwrap();
    wrong["tools"][0]["origin"]["ref"] = "other/tool".into();
    let lb = serde_json::to_vec(&wrong).unwrap();
    m["lock_sha256"] = sha(&lb).into();
    m["artifacts"][0]["origin_ref_sha256"] = sha(b"other/tool").into();
    assert_eq!(
        inspect_distribution_raw(
            &serde_json::to_vec(&m).unwrap(),
            &lb,
            "tool",
            "linux_x86_64",
            b"tool",
            deadline(),
            &AtomicBool::new(false)
        )
        .unwrap_err(),
        "distribution_raw_locator_mismatch"
    );
}
#[cfg(unix)]
#[test]
fn verified_raw_bytes_publish_and_match_the_same_original_lock() {
    use std::os::unix::fs::PermissionsExt;
    let platform = codeguard_cli::tool_identity::current_platform_id().unwrap();
    let (lock, m) = inputs(platform);
    let mb = serde_json::to_vec(&m).unwrap();
    let observed = inspect_distribution_raw(
        &mb,
        &lock,
        "tool",
        platform,
        b"tool",
        deadline(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-raw-bound-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
    let receipt = codeguard_runtime::publish_tool_bytes(
        &root,
        observed.bytes(),
        observed.binary_sha256(),
        deadline(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(receipt.relative_path, observed.origin_ref());
    let parsed = codeguard_cli::tool_lock::parse_tool_lock_document(&lock).unwrap();
    let identity = codeguard_cli::tool_identity::verify_locked_artifacts(
        parsed.find("tool", platform).unwrap(),
        &root,
        &root,
        None,
    );
    assert!(identity.tool.digest_matched && identity.tool.executable_bit);
    assert!(identity.runtime.is_none() && identity.bundle.is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
#[ignore = "需显式指定 CODEGUARD_TEST_RUFF，进行真实 raw 关联/发布/版本观察"]
fn real_ruff_binding_publication_and_native_version_without_authority() {
    use std::os::unix::fs::PermissionsExt;
    let source =
        std::path::PathBuf::from(std::env::var("CODEGUARD_TEST_RUFF").expect("显式 Ruff 原生路径"));
    let bytes = codeguard_runtime::read_bounded_regular_file(&source, 128 * 1024 * 1024).unwrap();
    let digest = sha(&bytes);
    let reference = format!("{digest}.bin");
    let platform = codeguard_cli::tool_identity::current_platform_id().unwrap();
    let (old_lock, mut m) = inputs(platform);
    let mut lock: Value = serde_json::from_slice(&old_lock).unwrap();
    lock["tools"][0]["id"] = "ruff".into();
    lock["tools"][0]["version"] = "0.16.8".into();
    lock["tools"][0]["binary_sha256"] = digest.clone().into();
    lock["tools"][0]["origin"]["ref"] = reference.clone().into();
    let lb = serde_json::to_vec(&lock).unwrap();
    m["lock_sha256"] = sha(&lb).into();
    let a = &mut m["artifacts"][0];
    a["tool_id"] = "ruff".into();
    a["version"] = "0.16.8".into();
    a["binary_sha256"] = digest.clone().into();
    a["package_sha256"] = digest.into();
    a["origin_ref_sha256"] = sha(reference.as_bytes()).into();
    a["package_size_bytes"] = bytes.len().into();
    a["unpacked_size_limit_bytes"] = bytes.len().into();
    let observed = inspect_distribution_raw(
        &serde_json::to_vec(&m).unwrap(),
        &lb,
        "ruff",
        platform,
        &bytes,
        deadline(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(observed.bytes().as_ptr(), bytes.as_ptr());
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-real-raw-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
    let receipt = codeguard_runtime::publish_tool_bytes(
        &root,
        observed.bytes(),
        observed.binary_sha256(),
        deadline(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(receipt.relative_path, observed.origin_ref());
    let version = codeguard_runtime::observe_native_version(
        &codeguard_runtime::NativeVersionRequest {
            process: codeguard_runtime::ProcessSpec {
                executable: root.join(receipt.relative_path),
                args: vec!["--version".into()],
                cwd: root.clone(),
                env: std::collections::BTreeMap::new(),
                stdin: None,
                deadline: deadline(),
                output_limit_bytes: 4096,
            },
            expected_tool_sha256: observed.binary_sha256(),
            expected_stdout: b"ruff 0.16.8\n".to_vec(),
            evidence_root: root.clone(),
            log_name: "version.log".into(),
        },
        &AtomicBool::new(false),
    );
    assert!(version.complete, "{:?}", version.reason);
    std::fs::remove_dir_all(root).unwrap();
}
