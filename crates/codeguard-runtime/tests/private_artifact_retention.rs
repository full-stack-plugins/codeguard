#![cfg(target_os = "linux")]
use codeguard_runtime::private_artifact_store::{
    ArtifactInput, PrivateArtifactStore, StoreError, StoreLimits,
};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "cg-retention-{}-{}",
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
    fn open(&self) -> PrivateArtifactStore {
        PrivateArtifactStore::open(
            &self.0,
            StoreLimits {
                max_entries: 16,
                max_total_bytes: 1024 * 1024,
            },
        )
        .unwrap()
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn lease_serializes_control_and_purge_across_handles() {
    let root = Root::new();
    let store = root.open();
    let other = root.open();
    let key = "a".repeat(64);
    store
        .stage(
            &key,
            &[ArtifactInput {
                name: "data",
                bytes: b"original",
            }],
        )
        .unwrap()
        .publish()
        .unwrap();
    let lease = store.lease().unwrap();
    assert!(matches!(other.lease(), Err(StoreError::Busy)));
    assert!(lease.read_control(".retention", 64).unwrap().is_none());
    lease
        .replace_control(".retention", b"time=100;tombstone=true", 64)
        .unwrap();
    assert_eq!(
        lease.read_control(".retention", 64).unwrap().unwrap(),
        b"time=100;tombstone=true"
    );
    lease.purge(&key).unwrap();
    assert!(store.read(&key).is_err());
    drop(lease);
    assert_eq!(
        other
            .lease()
            .unwrap()
            .read_control(".retention", 64)
            .unwrap()
            .unwrap(),
        b"time=100;tombstone=true"
    );
}
#[test]
fn control_and_purge_refuse_symlinks_without_touching_external_file() {
    let root = Root::new();
    let external = Root::new();
    std::fs::write(external.0.join("sentinel"), b"private").unwrap();
    std::os::unix::fs::symlink(external.0.join("sentinel"), root.0.join(".retention")).unwrap();
    let store = root.open();
    let lease = store.lease().unwrap();
    assert!(lease.read_control(".retention", 64).is_err());
    assert!(
        lease
            .replace_control(".retention", b"overwrite", 64)
            .is_err()
    );
    assert_eq!(
        std::fs::read(external.0.join("sentinel")).unwrap(),
        b"private"
    );
}
#[test]
fn control_replacement_is_bounded_and_purge_validates_all_leaves_first() {
    let root = Root::new();
    let store = root.open();
    let key = "b".repeat(64);
    store
        .stage(
            &key,
            &[
                ArtifactInput {
                    name: "a",
                    bytes: b"keep",
                },
                ArtifactInput {
                    name: "z",
                    bytes: b"old",
                },
            ],
        )
        .unwrap()
        .publish()
        .unwrap();
    let lease = store.lease().unwrap();
    lease.replace_control(".retention", b"old", 16).unwrap();
    assert!(lease.replace_control(".retention", &[0; 17], 16).is_err());
    assert_eq!(
        lease.read_control(".retention", 16).unwrap().unwrap(),
        b"old"
    );
    let external = Root::new();
    std::fs::write(external.0.join("data"), b"external").unwrap();
    std::fs::remove_file(root.0.join(&key).join("z")).unwrap();
    std::os::unix::fs::symlink(external.0.join("data"), root.0.join(&key).join("z")).unwrap();
    assert!(lease.purge(&key).is_err());
    assert_eq!(std::fs::read(root.0.join(&key).join("a")).unwrap(), b"keep");
    assert_eq!(std::fs::read(external.0.join("data")).unwrap(), b"external");
}
#[test]
fn real_concurrent_handle_cannot_enter_while_checkpoint_transaction_is_held() {
    let root = Root::new();
    let first = root.open();
    let second = root.open();
    let lease = first.lease().unwrap();
    let other = std::thread::spawn(move || matches!(second.lease(), Err(StoreError::Busy)));
    assert!(other.join().unwrap());
    lease
        .replace_control(".retention", b"committed", 64)
        .unwrap();
    drop(lease);
    assert_eq!(
        root.open()
            .lease()
            .unwrap()
            .read_control(".retention", 64)
            .unwrap()
            .unwrap(),
        b"committed"
    );
}
