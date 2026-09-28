use codeguard_cli::tool_identity::hash_bundle_tree;
use codeguard_runtime::{
    UnpackedArchive, UnpackedFile, hash_unpacked_bundle_tree, verify_unpacked_bundle_tree,
};
use std::fs;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};
fn archive() -> UnpackedArchive {
    UnpackedArchive {
        files: vec![
            UnpackedFile {
                relative_path: "a/lib/tool.jar".into(),
                bytes: b"library".to_vec(),
            },
            UnpackedFile {
                relative_path: "z/bin/tool".into(),
                bytes: b"tool".to_vec(),
            },
            UnpackedFile {
                relative_path: "README".into(),
                bytes: b"readme".to_vec(),
            },
        ],
        directories: vec![
            "a".into(),
            "a/lib".into(),
            "z".into(),
            "z/bin".into(),
            "empty".into(),
            "z/empty".into(),
        ],
    }
}
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}
#[test]
fn memory_tree_matches_existing_native_directory_digest_including_empty_dirs() {
    let tree = archive();
    let root = std::env::temp_dir().join(format!("cg-memory-tree-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    for dir in &tree.directories {
        fs::create_dir_all(root.join(dir)).unwrap();
    }
    for file in &tree.files {
        fs::write(root.join(&file.relative_path), &file.bytes).unwrap();
    }
    let disk = hash_bundle_tree(&root).unwrap();
    let actual = hash_unpacked_bundle_tree(&tree, deadline(), &AtomicBool::new(false)).unwrap();
    assert_eq!(actual, disk);
    verify_unpacked_bundle_tree(&tree, &disk, deadline(), &AtomicBool::new(false)).unwrap();
    fs::remove_dir_all(root).unwrap();
    let mut changed = archive();
    changed.files.reverse();
    changed.directories.reverse();
    assert_eq!(
        hash_unpacked_bundle_tree(&changed, deadline(), &AtomicBool::new(false)).unwrap(),
        actual
    );
}
#[test]
fn file_tampering_and_missing_empty_directory_do_not_match_the_locked_tree() {
    let expected =
        hash_unpacked_bundle_tree(&archive(), deadline(), &AtomicBool::new(false)).unwrap();
    let mut changed = archive();
    changed.files[0].bytes = b"tampered".to_vec();
    assert!(
        verify_unpacked_bundle_tree(&changed, &expected, deadline(), &AtomicBool::new(false))
            .is_err()
    );
    let mut changed = archive();
    changed.directories.retain(|d| d != "empty");
    assert!(
        verify_unpacked_bundle_tree(&changed, &expected, deadline(), &AtomicBool::new(false))
            .is_err()
    );
}
#[test]
fn forged_tree_shapes_and_invalid_expected_hashes_are_rejected() {
    for path in ["../escape", "a//escape", "CON.exe"] {
        let mut tree = archive();
        tree.files[0].relative_path = path.into();
        assert!(hash_unpacked_bundle_tree(&tree, deadline(), &AtomicBool::new(false)).is_err());
    }
    let mut tree = archive();
    tree.directories.retain(|d| d != "a/lib");
    assert!(hash_unpacked_bundle_tree(&tree, deadline(), &AtomicBool::new(false)).is_err());
    let mut tree = archive();
    tree.directories.push("A".into());
    assert!(hash_unpacked_bundle_tree(&tree, deadline(), &AtomicBool::new(false)).is_err());
    for expected in ["0".repeat(64), "not_a_digest".into()] {
        assert!(
            verify_unpacked_bundle_tree(&archive(), &expected, deadline(), &AtomicBool::new(false))
                .is_err()
        );
    }
    assert!(
        hash_unpacked_bundle_tree(&archive(), Instant::now(), &AtomicBool::new(false)).is_err()
    );
    assert!(hash_unpacked_bundle_tree(&archive(), deadline(), &AtomicBool::new(true)).is_err());
}

#[test]
fn explicit_bundle_root_mapping_matches_real_directory_identity() {
    use codeguard_runtime::project_unpacked_bundle;
    let mut source = archive();
    for file in &mut source.files {
        file.relative_path = format!("release/libexec/{}", file.relative_path);
    }
    for dir in &mut source.directories {
        *dir = format!("release/libexec/{dir}");
    }
    source.directories.extend([
        "release".into(),
        "release/libexec".into(),
        "release/bin".into(),
    ]);
    source.files.push(UnpackedFile {
        relative_path: "release/bin/wrapper".into(),
        bytes: b"wrapper".to_vec(),
    });
    let expected =
        hash_unpacked_bundle_tree(&archive(), deadline(), &AtomicBool::new(false)).unwrap();
    let result = project_unpacked_bundle(
        source,
        "release/libexec",
        &expected,
        deadline(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(result.remainder().files.len(), 1);
    assert_eq!(
        result.remainder().files[0].relative_path,
        "release/bin/wrapper"
    );
    let root = std::env::temp_dir().join(format!("cg-projection-disk-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    for dir in &result.tree().directories {
        fs::create_dir_all(root.join(dir)).unwrap();
    }
    for file in &result.tree().files {
        fs::write(root.join(&file.relative_path), &file.bytes).unwrap();
    }
    assert_eq!(hash_bundle_tree(&root).unwrap(), expected);
    fs::remove_dir_all(root).unwrap();
}
