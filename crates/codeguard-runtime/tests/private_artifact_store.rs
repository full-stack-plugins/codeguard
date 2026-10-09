#![cfg(target_os = "linux")]
use codeguard_runtime::private_artifact_store::{ArtifactInput, PrivateArtifactStore, StoreLimits};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "cg-audit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&p).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self(p)
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn limits() -> StoreLimits {
    StoreLimits {
        max_entries: 8,
        max_total_bytes: 1_048_576,
    }
}
#[test]
fn explicit_store_stages_without_publish_and_never_overwrites() {
    let root = Root::new();
    let store = PrivateArtifactStore::open(&root.0, limits()).unwrap();
    let key = "a".repeat(64);
    let inputs = [ArtifactInput {
        name: "native.json",
        bytes: b"original findings",
    }];
    let stage = store.stage(&key, &inputs).unwrap();
    assert!(!root.0.join(&key).exists());
    drop(stage);
    assert_eq!(std::fs::read_dir(&root.0).unwrap().count(), 0);
    store.stage(&key, &inputs).unwrap().publish().unwrap();
    assert_eq!(
        store.read(&key).unwrap()["native.json"],
        b"original findings"
    );
    assert!(
        store
            .stage(
                &key,
                &[ArtifactInput {
                    name: "native.json",
                    bytes: b"replacement"
                }]
            )
            .unwrap()
            .publish()
            .is_err()
    );
    assert_eq!(
        store.read(&key).unwrap()["native.json"],
        b"original findings"
    );
}

#[test]
fn roots_paths_and_nonprivate_roots_are_rejected() {
    let root = Root::new();
    let link = root.0.join("link");
    std::os::unix::fs::symlink(&root.0, &link).unwrap();
    assert!(PrivateArtifactStore::open(&link, limits()).is_err());
    assert!(PrivateArtifactStore::open(std::path::Path::new("relative"), limits()).is_err());
    std::fs::remove_file(&link).unwrap();
    std::fs::set_permissions(&root.0, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(PrivateArtifactStore::open(&root.0, limits()).is_err());
    std::fs::set_permissions(&root.0, std::fs::Permissions::from_mode(0o700)).unwrap();
    let store = PrivateArtifactStore::open(&root.0, limits()).unwrap();
    for key in ["../outside", "artifact://secret", "", "uppercase"] {
        assert!(store.read(key).is_err());
    }
    for name in ["../native", "/tmp/native", "a/b", ".", "..", "with\0nul"] {
        assert!(
            store
                .stage(&"a".repeat(64), &[ArtifactInput { name, bytes: b"x" }])
                .is_err()
        );
    }
}
#[test]
fn linked_special_wrong_owner_and_nonprivate_payloads_are_refused() {
    use std::os::unix::ffi::OsStrExt;
    for mutation in 0..5 {
        let root = Root::new();
        let store = PrivateArtifactStore::open(&root.0, limits()).unwrap();
        let key = "a".repeat(64);
        let file = root.0.join(&key).join("native.json");
        store
            .stage(
                &key,
                &[ArtifactInput {
                    name: "native.json",
                    bytes: b"private",
                }],
            )
            .unwrap()
            .publish()
            .unwrap();
        match mutation {
            0 => {
                std::fs::hard_link(&file, root.0.join("alias")).unwrap();
            }
            1 => {
                std::fs::remove_file(&file).unwrap();
                std::os::unix::fs::symlink("/etc/passwd", &file).unwrap();
            }
            2 => {
                std::fs::remove_file(&file).unwrap();
                let c = std::ffi::CString::new(file.as_os_str().as_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
            }
            3 => std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap(),
            _ => {
                if unsafe { libc::geteuid() } != 0 {
                    continue;
                }
                let c = std::ffi::CString::new(file.as_os_str().as_bytes()).unwrap();
                assert_eq!(unsafe { libc::chown(c.as_ptr(), 1, 1) }, 0);
            }
        }
        assert!(store.read(&key).is_err(), "mutation {mutation}");
    }
}
#[test]
fn cross_handle_global_quota_reopen_and_same_handle_lock_are_enforced() {
    let root = Root::new();
    let policy = StoreLimits {
        max_entries: 2,
        max_total_bytes: 8,
    };
    let a = PrivateArtifactStore::open(&root.0, policy).unwrap();
    let b = PrivateArtifactStore::open(&root.0, policy).unwrap();
    let stage = a
        .stage(
            &"a".repeat(64),
            &[ArtifactInput {
                name: "data",
                bytes: b"12345",
            }],
        )
        .unwrap();
    assert!(
        a.stage(
            &"b".repeat(64),
            &[ArtifactInput {
                name: "data",
                bytes: b"x"
            }]
        )
        .is_err()
    );
    assert!(
        b.stage(
            &"b".repeat(64),
            &[ArtifactInput {
                name: "data",
                bytes: b"x"
            }]
        )
        .is_err()
    );
    stage.publish().unwrap();
    let reopened = PrivateArtifactStore::open(&root.0, policy).unwrap();
    assert!(
        reopened
            .stage(
                &"b".repeat(64),
                &[ArtifactInput {
                    name: "data",
                    bytes: b"6789"
                }]
            )
            .is_err()
    );
    reopened
        .stage(
            &"b".repeat(64),
            &[ArtifactInput {
                name: "data",
                bytes: b"678",
            }],
        )
        .unwrap()
        .publish()
        .unwrap();
    assert!(
        a.stage(
            &"c".repeat(64),
            &[ArtifactInput {
                name: "data",
                bytes: b""
            }]
        )
        .is_err()
    );
}
#[test]
fn concurrent_quota_one_has_one_actual_publication() {
    let root = Root::new();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let mut threads = vec![];
    for letter in ['a', 'b'] {
        let path = root.0.clone();
        let barrier = barrier.clone();
        threads.push(std::thread::spawn(move || {
            let store = PrivateArtifactStore::open(
                &path,
                StoreLimits {
                    max_entries: 1,
                    max_total_bytes: 100,
                },
            )
            .unwrap();
            barrier.wait();
            store
                .stage(
                    &letter.to_string().repeat(64),
                    &[ArtifactInput {
                        name: "data",
                        bytes: b"value",
                    }],
                )
                .and_then(|stage| stage.publish())
                .is_ok()
        }));
    }
    assert_eq!(
        threads
            .into_iter()
            .map(|t| usize::from(t.join().unwrap()))
            .sum::<usize>(),
        1
    );
    assert_eq!(std::fs::read_dir(&root.0).unwrap().count(), 1);
}
#[test]
fn pinned_directory_and_drop_cleanup_ignore_replacement_path_and_stage_slot() {
    let root = Root::new();
    let store = PrivateArtifactStore::open(&root.0, limits()).unwrap();
    let moved = root.0.with_extension("pinned");
    let key = "a".repeat(64);
    std::fs::rename(&root.0, &moved).unwrap();
    std::fs::create_dir(&root.0).unwrap();
    std::fs::set_permissions(&root.0, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(root.0.join("keep"), b"user").unwrap();
    store
        .stage(
            &key,
            &[ArtifactInput {
                name: "data",
                bytes: b"pinned",
            }],
        )
        .unwrap()
        .publish()
        .unwrap();
    assert_eq!(store.read(&key).unwrap()["data"], b"pinned");
    assert!(!root.0.join(&key).exists());
    let stage = store
        .stage(
            &"b".repeat(64),
            &[ArtifactInput {
                name: "data",
                bytes: b"stage",
            }],
        )
        .unwrap();
    let stage_path = std::fs::read_dir(&moved)
        .unwrap()
        .map(Result::unwrap)
        .find(|e| e.file_name().to_string_lossy().starts_with(".stage-"))
        .unwrap()
        .path();
    let retained = moved.join("retained");
    std::fs::rename(&stage_path, &retained).unwrap();
    std::fs::create_dir(&stage_path).unwrap();
    std::fs::write(stage_path.join("keep"), b"replacement").unwrap();
    drop(stage);
    assert_eq!(
        std::fs::read(stage_path.join("keep")).unwrap(),
        b"replacement"
    );
    assert_eq!(std::fs::read(root.0.join("keep")).unwrap(), b"user");
    drop(store);
    std::fs::remove_dir_all(moved).unwrap();
}
#[test]
fn existing_user_file_or_orphan_stage_is_never_replaced_or_ignored_for_quota() {
    let root = Root::new();
    let key = "a".repeat(64);
    let path = root.0.join(&key);
    std::fs::write(&path, b"user-original").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let store = PrivateArtifactStore::open(&root.0, limits()).unwrap();
    assert!(
        store
            .stage(
                &key,
                &[ArtifactInput {
                    name: "data",
                    bytes: b"new"
                }]
            )
            .unwrap()
            .publish()
            .is_err()
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"user-original");
    let orphan = root.0.join(".stage-orphan");
    std::fs::create_dir(&orphan).unwrap();
    std::fs::set_permissions(&orphan, std::fs::Permissions::from_mode(0o700)).unwrap();
    let old = orphan.join("data");
    std::fs::write(&old, b"orphan").unwrap();
    std::fs::set_permissions(&old, std::fs::Permissions::from_mode(0o600)).unwrap();
    let bounded = PrivateArtifactStore::open(
        &root.0,
        StoreLimits {
            max_entries: 2,
            max_total_bytes: 100,
        },
    )
    .unwrap();
    assert!(
        bounded
            .stage(
                &"b".repeat(64),
                &[ArtifactInput {
                    name: "data",
                    bytes: b"x"
                }]
            )
            .is_err()
    );
    assert_eq!(std::fs::read(&old).unwrap(), b"orphan");
}
