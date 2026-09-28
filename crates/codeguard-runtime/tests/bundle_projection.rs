use codeguard_runtime::{
    UnpackedArchive, UnpackedFile, hash_unpacked_bundle_tree, project_unpacked_bundle,
};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}
fn tree() -> UnpackedArchive {
    UnpackedArchive {
        files: vec![
            UnpackedFile {
                relative_path: "pkg/libexec/lib/tool.jar".into(),
                bytes: b"library".to_vec(),
            },
            UnpackedFile {
                relative_path: "pkg/bin/tool".into(),
                bytes: b"wrapper".to_vec(),
            },
            UnpackedFile {
                relative_path: "pkg/libexec-other/README".into(),
                bytes: b"other".to_vec(),
            },
        ],
        directories: vec![
            "pkg".into(),
            "pkg/libexec".into(),
            "pkg/libexec/lib".into(),
            "pkg/libexec/empty".into(),
            "pkg/bin".into(),
            "pkg/libexec-other".into(),
        ],
    }
}
fn expected() -> String {
    hash_unpacked_bundle_tree(
        &UnpackedArchive {
            files: vec![UnpackedFile {
                relative_path: "lib/tool.jar".into(),
                bytes: b"library".to_vec(),
            }],
            directories: vec!["lib".into(), "empty".into()],
        },
        deadline(),
        &AtomicBool::new(false),
    )
    .unwrap()
}
#[test]
fn exact_prefix_projection_preserves_empty_dirs_and_retains_outside_members() {
    let source = tree();
    let pointer = source.files[0].bytes.as_ptr();
    let result = project_unpacked_bundle(
        source,
        "pkg/libexec",
        &expected(),
        deadline(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(result.archive_root(), "pkg/libexec");
    assert_eq!(result.tree_sha256(), expected());
    assert_eq!(result.tree().files[0].relative_path, "lib/tool.jar");
    assert_eq!(result.tree().files[0].bytes.as_ptr(), pointer);
    assert_eq!(result.tree().directories, vec!["empty", "lib"]);
    assert_eq!(result.remainder().files.len(), 2);
    assert!(
        result
            .remainder()
            .files
            .iter()
            .any(|f| f.relative_path == "pkg/libexec-other/README")
    );
    assert!(
        result
            .remainder()
            .files
            .iter()
            .any(|f| f.relative_path == "pkg/bin/tool")
    );
}
#[test]
fn projection_of_whole_tree_is_explicit_and_does_not_guess_a_root() {
    let source = tree();
    let expected = hash_unpacked_bundle_tree(&source, deadline(), &AtomicBool::new(false)).unwrap();
    let result =
        project_unpacked_bundle(source, "", &expected, deadline(), &AtomicBool::new(false))
            .unwrap();
    assert_eq!(result.tree().files.len(), 3);
    assert!(result.remainder().files.is_empty());
    assert!(result.remainder().directories.is_empty());
}
#[test]
fn unsafe_missing_file_or_case_mismatched_roots_are_rejected() {
    for root in [
        "../pkg",
        "pkg//libexec",
        "pkg/libexec/",
        "PKG/libexec",
        "pkg/missing",
        "pkg/bin/tool",
    ] {
        assert!(
            project_unpacked_bundle(
                tree(),
                root,
                &expected(),
                deadline(),
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
}
#[test]
fn original_tree_and_selected_digest_must_both_validate() {
    let mut source = tree();
    source.files[1].relative_path = "../wrapper".into();
    assert!(
        project_unpacked_bundle(
            source,
            "pkg/libexec",
            &expected(),
            deadline(),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    let mut source = tree();
    source.files[0].bytes = b"tampered".to_vec();
    assert_eq!(
        project_unpacked_bundle(
            source,
            "pkg/libexec",
            &expected(),
            deadline(),
            &AtomicBool::new(false)
        )
        .unwrap_err(),
        "bundle_digest_mismatch"
    );
    assert!(
        project_unpacked_bundle(
            tree(),
            "pkg/libexec",
            &"0".repeat(64),
            deadline(),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    assert!(
        project_unpacked_bundle(
            tree(),
            "pkg/libexec",
            &expected(),
            Instant::now(),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    assert!(
        project_unpacked_bundle(
            tree(),
            "pkg/libexec",
            &expected(),
            deadline(),
            &AtomicBool::new(true)
        )
        .is_err()
    );
}
