use codeguard_cli::distribution_bundle::inspect_distribution_bundle;
use codeguard_runtime::{UnpackedArchive, UnpackedFile, hash_unpacked_bundle_tree};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};
const PACKAGE: &[u8] = include_bytes!("../../../tests/fixtures/distribution/bundle-projection.zip");
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}
fn inputs() -> (Vec<u8>, Value) {
    let tree = UnpackedArchive {
        files: vec![UnpackedFile {
            relative_path: "lib/tool.jar".into(),
            bytes: b"library".to_vec(),
        }],
        directories: vec!["lib".into(), "empty".into()],
    };
    let hash = hash_unpacked_bundle_tree(&tree, deadline(), &AtomicBool::new(false)).unwrap();
    let lock = serde_json::to_vec(&json!({"schema_version":"1.0","lock_id":"candidate","tools":[{"id":"tool","version":"1","platform":"linux_x86_64","binary_sha256":sha(b"tool"),"adapter":{"id":"tool","version":"1"},"rule_source":{"kind":"native_builtin","id":"tool","sha256":"b".repeat(64)},"origin":{"kind":"managed_cache","ref":"tool/bin/tool"},"bundle":{"root":"tool/libexec","tree_sha256":hash}}]})).unwrap();
    let manifest = json!({"schema_version":"1.1","manifest_id":"test","lock_sha256":sha(&lock),"artifacts":[{"tool_id":"tool","version":"1","platform":"linux_x86_64","binary_sha256":sha(b"tool"),"origin_ref_sha256":sha(b"tool/bin/tool"),"download_url":"https://example.org/tool.zip","package_sha256":sha(PACKAGE),"package_size_bytes":PACKAGE.len(),"format":"zip","entrypoint":"release/bin/tool","unpacked_size_limit_bytes":1024,"bundle_tree_sha256":hash,"bundle_archive_root":"release/libexec"}]});
    (lock, manifest)
}
fn inspect(
    lock: &[u8],
    manifest: &Value,
    package: &[u8],
) -> Result<codeguard_runtime::ProjectedBundle, &'static str> {
    inspect_distribution_bundle(
        &serde_json::to_vec(manifest).unwrap(),
        lock,
        "tool",
        "linux_x86_64",
        package,
        deadline(),
        &AtomicBool::new(false),
    )
}
#[test]
fn exact_manifest_lock_package_entry_and_bundle_are_bound_without_installation() {
    let (lock, m) = inputs();
    let result = inspect(&lock, &m, PACKAGE).unwrap();
    assert_eq!(result.archive_root(), "release/libexec");
    assert_eq!(result.tree().files[0].bytes, b"library");
    assert_eq!(result.tree().directories, vec!["empty", "lib"]);
    assert_eq!(result.remainder().files.len(), 2);
}
#[test]
fn package_or_mapping_mismatch_never_produces_a_bundle() {
    let (lock, original) = inputs();
    for (field, value) in [
        ("package_size_bytes", json!(PACKAGE.len() + 1)),
        ("package_sha256", json!("c".repeat(64))),
        ("entrypoint", json!("release/missing")),
        ("bundle_archive_root", json!("release/libexec-other")),
    ] {
        let mut m = original.clone();
        m["artifacts"][0][field] = value;
        assert!(inspect(&lock, &m, PACKAGE).is_err(), "{field}");
    }
    let mut bad = PACKAGE.to_vec();
    bad[0] ^= 1;
    assert!(inspect(&lock, &original, &bad).is_err());
    let mut changed = lock.clone();
    changed.push(b' ');
    assert!(inspect(&changed, &original, PACKAGE).is_err());
}
#[test]
fn legacy_or_absent_mapping_and_cancelled_budget_do_not_fall_back() {
    let (lock, mut m) = inputs();
    m["schema_version"] = "1.0".into();
    m["artifacts"][0]
        .as_object_mut()
        .unwrap()
        .remove("bundle_archive_root");
    assert!(inspect(&lock, &m, PACKAGE).is_err());
    let (_, m) = inputs();
    let bytes = serde_json::to_vec(&m).unwrap();
    assert!(
        inspect_distribution_bundle(
            &bytes,
            &lock,
            "tool",
            "linux_x86_64",
            PACKAGE,
            deadline(),
            &AtomicBool::new(true)
        )
        .is_err()
    );
    assert!(
        inspect_distribution_bundle(
            &bytes,
            &lock,
            "tool",
            "linux_x86_64",
            PACKAGE,
            Instant::now(),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    assert!(
        inspect_distribution_bundle(
            &bytes,
            &lock,
            "other",
            "linux_x86_64",
            PACKAGE,
            deadline(),
            &AtomicBool::new(false)
        )
        .is_err()
    );
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn bound_package_publishes_a_tree_matching_the_existing_disk_identity() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-bound-bundle-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
    {
        let (lock, m) = inputs();
        let bound = inspect(&lock, &m, PACKAGE).unwrap();
        let receipt = codeguard_runtime::publish_tool_bundle(
            &root,
            bound.tree(),
            bound.tree_sha256(),
            deadline(),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(
            codeguard_cli::tool_identity::hash_bundle_tree(&root.join(receipt.relative_path))
                .unwrap(),
            bound.tree_sha256()
        );
        assert_eq!(bound.remainder().files.len(), 2);
    }
    std::fs::remove_dir_all(&root).unwrap()
}

#[test]
fn full_layout_preserves_wrapper_relationships_and_binds_lock_paths() {
    use codeguard_cli::distribution_layout::inspect_distribution_layout;
    let (old_lock, mut manifest) = inputs();
    let mut full = UnpackedArchive {
        files: vec![
            UnpackedFile {
                relative_path: "release/bin/tool".into(),
                bytes: b"tool".to_vec(),
            },
            UnpackedFile {
                relative_path: "release/libexec/lib/tool.jar".into(),
                bytes: b"library".to_vec(),
            },
            UnpackedFile {
                relative_path: "release/libexec-other/README".into(),
                bytes: b"other".to_vec(),
            },
        ],
        directories: vec![
            "release".into(),
            "release/bin".into(),
            "release/libexec".into(),
            "release/libexec/lib".into(),
            "release/libexec/empty".into(),
            "release/libexec-other".into(),
        ],
    };
    let tree_sha = hash_unpacked_bundle_tree(&full, deadline(), &AtomicBool::new(false)).unwrap();
    let mut lock: Value = serde_json::from_slice(&old_lock).unwrap();
    lock["tools"][0]["origin"]["ref"] = format!("{tree_sha}.bundle/release/bin/tool").into();
    lock["tools"][0]["bundle"]["root"] = format!("{tree_sha}.bundle/release/libexec").into();
    let lock_bytes = serde_json::to_vec(&lock).unwrap();
    manifest["schema_version"] = "1.2".into();
    manifest["lock_sha256"] = sha(&lock_bytes).into();
    manifest["artifacts"][0]["origin_ref_sha256"] = sha(lock["tools"][0]["origin"]["ref"]
        .as_str()
        .unwrap()
        .as_bytes())
    .into();
    manifest["artifacts"][0]["install_tree_sha256"] = tree_sha.clone().into();
    let bytes = serde_json::to_vec(&manifest).unwrap();
    let plan = inspect_distribution_layout(
        &bytes,
        &lock_bytes,
        "tool",
        "linux_x86_64",
        PACKAGE,
        deadline(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(plan.install_tree_sha256(), tree_sha);
    assert_eq!(plan.origin_ref(), lock["tools"][0]["origin"]["ref"]);
    assert_eq!(plan.tree().files.len(), 3);
    assert_eq!(
        plan.bundle_root(),
        lock["tools"][0]["bundle"]["root"].as_str()
    );
    assert_eq!(
        hash_unpacked_bundle_tree(plan.tree(), deadline(), &AtomicBool::new(false)).unwrap(),
        hash_unpacked_bundle_tree(&full, deadline(), &AtomicBool::new(false)).unwrap()
    );

    // 缓存定位由原锁声明，不在安装时重写锁；发布后使用同一只读制品检查器。
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        use std::os::unix::fs::PermissionsExt;
        let platform = codeguard_cli::tool_identity::current_platform_id().unwrap();
        let mut local_lock = lock.clone();
        local_lock["tools"][0]["platform"] = platform.into();
        let lb = serde_json::to_vec(&local_lock).unwrap();
        let mut local_manifest = manifest.clone();
        local_manifest["lock_sha256"] = sha(&lb).into();
        local_manifest["artifacts"][0]["platform"] = platform.into();
        let mb = serde_json::to_vec(&local_manifest).unwrap();
        let layout = inspect_distribution_layout(
            &mb,
            &lb,
            "tool",
            platform,
            PACKAGE,
            deadline(),
            &AtomicBool::new(false),
        )
        .unwrap();
        let cache = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-layout-{}", std::process::id()));
        std::fs::create_dir(&cache).unwrap();
        std::fs::set_permissions(&cache, std::fs::Permissions::from_mode(0o700)).unwrap();
        codeguard_runtime::publish_tool_bundle(
            &cache,
            layout.tree(),
            layout.install_tree_sha256(),
            deadline(),
            &AtomicBool::new(false),
        )
        .unwrap();
        let frozen_lock = codeguard_cli::tool_lock::parse_tool_lock_document(&lb).unwrap();
        let identity = codeguard_cli::tool_identity::verify_locked_artifacts(
            frozen_lock.find("tool", platform).unwrap(),
            &cache,
            &cache,
            None,
        );
        assert!(identity.tool.digest_matched && identity.tool.executable_bit);
        assert!(identity.tool.issue.is_none());
        assert!(identity.bundle.unwrap().digest_matched);
        assert!(identity.runtime.is_none());
        assert_eq!(layout.lock_sha256(), sha(&lb));
        assert_eq!(layout.manifest_sha256(), sha(&mb));
        std::fs::remove_dir_all(cache).unwrap();
    }
    let mut whole_lock = lock.clone();
    whole_lock["tools"][0]["bundle"]["root"] = format!("{tree_sha}.bundle").into();
    let wl = serde_json::to_vec(&whole_lock).unwrap();
    let mut whole = manifest.clone();
    whole["lock_sha256"] = sha(&wl).into();
    whole["artifacts"][0]["bundle_archive_root"] = "".into();
    whole["artifacts"][0]["bundle_tree_sha256"] = tree_sha.clone().into();
    whole_lock["tools"][0]["bundle"]["tree_sha256"] = tree_sha.clone().into();
    let wl = serde_json::to_vec(&whole_lock).unwrap();
    whole["lock_sha256"] = sha(&wl).into();
    let whole_plan = inspect_distribution_layout(
        &serde_json::to_vec(&whole).unwrap(),
        &wl,
        "tool",
        "linux_x86_64",
        PACKAGE,
        deadline(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(whole_plan.tree().files.len(), 3);
    assert_eq!(
        whole_plan.bundle_root(),
        Some(format!("{tree_sha}.bundle").as_str())
    );
    let mut plain_lock = lock.clone();
    plain_lock["tools"][0]
        .as_object_mut()
        .unwrap()
        .remove("bundle");
    let pl = serde_json::to_vec(&plain_lock).unwrap();
    let mut plain = manifest.clone();
    plain["lock_sha256"] = sha(&pl).into();
    plain["artifacts"][0]["bundle_tree_sha256"] = Value::Null;
    plain["artifacts"][0]["bundle_archive_root"] = Value::Null;
    let plain_plan = inspect_distribution_layout(
        &serde_json::to_vec(&plain).unwrap(),
        &pl,
        "tool",
        "linux_x86_64",
        PACKAGE,
        deadline(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(plain_plan.bundle_root().is_none());
    assert_eq!(plain_plan.tree().files.len(), 3);
    full.files[2].bytes = b"changed remainder".to_vec();
    let wrong_tree = hash_unpacked_bundle_tree(&full, deadline(), &AtomicBool::new(false)).unwrap();
    let mut wrong = manifest.clone();
    wrong["artifacts"][0]["install_tree_sha256"] = wrong_tree.clone().into();
    let mut wrong_lock = lock.clone();
    wrong_lock["tools"][0]["origin"]["ref"] =
        format!("{wrong_tree}.bundle/release/bin/tool").into();
    wrong_lock["tools"][0]["bundle"]["root"] =
        format!("{wrong_tree}.bundle/release/libexec").into();
    let wrong_lock_bytes = serde_json::to_vec(&wrong_lock).unwrap();
    wrong["lock_sha256"] = sha(&wrong_lock_bytes).into();
    wrong["artifacts"][0]["origin_ref_sha256"] = sha(wrong_lock["tools"][0]["origin"]["ref"]
        .as_str()
        .unwrap()
        .as_bytes())
    .into();
    assert_eq!(
        inspect_distribution_layout(
            &serde_json::to_vec(&wrong).unwrap(),
            &wrong_lock_bytes,
            "tool",
            "linux_x86_64",
            PACKAGE,
            deadline(),
            &AtomicBool::new(false)
        )
        .unwrap_err(),
        "bundle_digest_mismatch"
    );
    for field in ["ref", "root"] {
        let mut wrong = lock.clone();
        if field == "ref" {
            wrong["tools"][0]["origin"][field] = "other/bin/tool".into();
        } else {
            wrong["tools"][0]["bundle"][field] = "other/libexec".into();
        }
        let lb = serde_json::to_vec(&wrong).unwrap();
        let mut m = manifest.clone();
        m["lock_sha256"] = sha(&lb).into();
        m["artifacts"][0]["origin_ref_sha256"] = sha(wrong["tools"][0]["origin"]["ref"]
            .as_str()
            .unwrap()
            .as_bytes())
        .into();
        assert!(
            inspect_distribution_layout(
                &serde_json::to_vec(&m).unwrap(),
                &lb,
                "tool",
                "linux_x86_64",
                PACKAGE,
                deadline(),
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
}
