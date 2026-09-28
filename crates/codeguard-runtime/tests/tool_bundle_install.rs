#![cfg(any(target_os = "macos", target_os = "linux"))]
use codeguard_runtime::{
    UnpackedArchive, UnpackedFile, hash_unpacked_bundle_tree, publish_tool_bundle,
};
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Cache(PathBuf);
impl Cache {
    fn new() -> Self {
        let p = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-bundle-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        Self(p)
    }
}
impl Drop for Cache {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn tree() -> UnpackedArchive {
    UnpackedArchive {
        files: vec![
            UnpackedFile {
                relative_path: "bin/tool".into(),
                bytes: b"#!/bin/sh\nexit 0\n".to_vec(),
            },
            UnpackedFile {
                relative_path: "lib/tool.jar".into(),
                bytes: b"library".to_vec(),
            },
        ],
        directories: vec![
            "bin".into(),
            "lib".into(),
            "empty".into(),
            "empty/nested".into(),
        ],
    }
}
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}
fn hash(t: &UnpackedArchive) -> String {
    hash_unpacked_bundle_tree(t, deadline(), &AtomicBool::new(false)).unwrap()
}
#[test]
fn complete_tree_publishes_once_with_empty_dirs_and_exact_reuse() {
    let c = Cache::new();
    let t = tree();
    let h = hash(&t);
    let first = publish_tool_bundle(&c.0, &t, &h, deadline(), &AtomicBool::new(false)).unwrap();
    assert!(first.newly_published);
    assert_eq!(first.relative_path, format!("{h}.bundle"));
    assert_eq!(
        fs::read(c.0.join(&first.relative_path).join("lib/tool.jar")).unwrap(),
        b"library"
    );
    assert!(c.0.join(&first.relative_path).join("empty/nested").is_dir());
    assert_eq!(
        fs::metadata(c.0.join(&first.relative_path).join("bin/tool"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    let second = publish_tool_bundle(&c.0, &t, &h, deadline(), &AtomicBool::new(false)).unwrap();
    assert!(!second.newly_published);
    assert_eq!(fs::read_dir(&c.0).unwrap().count(), 1);
}
#[test]
fn stale_tree_extra_members_and_unsafe_entries_are_never_overwritten() {
    for attack in 0..8 {
        let c = Cache::new();
        let t = tree();
        let h = hash(&t);
        let first = publish_tool_bundle(&c.0, &t, &h, deadline(), &AtomicBool::new(false)).unwrap();
        let root = c.0.join(first.relative_path);
        match attack {
            0 => {
                fs::write(root.join("lib/tool.jar"), b"bad").unwrap();
            }
            1 => {
                fs::create_dir(root.join("unexpected")).unwrap();
            }
            2 => {
                fs::remove_file(root.join("lib/tool.jar")).unwrap();
                symlink("/etc/passwd", root.join("lib/tool.jar")).unwrap();
            }
            3 => {
                fs::hard_link(root.join("lib/tool.jar"), root.join("copy.jar")).unwrap();
            }
            4 => {
                fs::set_permissions(root.join("lib"), fs::Permissions::from_mode(0o777)).unwrap();
            }
            5 => {
                fs::remove_file(root.join("lib/tool.jar")).unwrap();
                let name = std::ffi::CString::new(
                    root.join("lib/tool.jar").as_os_str().as_encoded_bytes(),
                )
                .unwrap();
                assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o700) }, 0);
            }
            6 => {
                fs::set_permissions(&root, fs::Permissions::from_mode(0o777)).unwrap();
            }
            _ => {
                fs::remove_dir(root.join("empty/nested")).unwrap();
            }
        }
        assert!(publish_tool_bundle(&c.0, &t, &h, deadline(), &AtomicBool::new(false)).is_err());
        assert_eq!(fs::read_dir(&c.0).unwrap().count(), 1);
    }
}
#[test]
fn invalid_input_budget_and_unsafe_cache_publish_nothing() {
    let c = Cache::new();
    let t = tree();
    let h = hash(&t);
    assert!(
        publish_tool_bundle(
            &c.0,
            &t,
            &"a".repeat(64),
            deadline(),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    assert!(publish_tool_bundle(&c.0, &t, &h, deadline(), &AtomicBool::new(true)).is_err());
    assert!(publish_tool_bundle(&c.0, &t, &h, Instant::now(), &AtomicBool::new(false)).is_err());
    let mut bad = tree();
    bad.files[0].relative_path = "../escape".into();
    assert!(publish_tool_bundle(&c.0, &bad, &h, deadline(), &AtomicBool::new(false)).is_err());
    fs::set_permissions(&c.0, fs::Permissions::from_mode(0o777)).unwrap();
    assert!(publish_tool_bundle(&c.0, &t, &h, deadline(), &AtomicBool::new(false)).is_err());
    assert_eq!(fs::read_dir(&c.0).unwrap().count(), 0);
}
#[test]
fn destination_links_and_empty_directory_collisions_cannot_be_replaced() {
    for link in [false, true] {
        let c = Cache::new();
        let t = tree();
        let h = hash(&t);
        let target = c.0.join(format!("{h}.bundle"));
        if link {
            symlink("/tmp", &target).unwrap();
        } else {
            fs::create_dir(&target).unwrap();
        }
        assert!(publish_tool_bundle(&c.0, &t, &h, deadline(), &AtomicBool::new(false)).is_err());
        assert_eq!(fs::read_dir(&c.0).unwrap().count(), 1);
    }
}
#[test]
fn concurrent_publishers_share_only_one_complete_tree() {
    let c = Cache::new();
    let root = c.0.clone();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let root = root.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let t = tree();
                let h = hash(&t);
                barrier.wait();
                publish_tool_bundle(&root, &t, &h, deadline(), &AtomicBool::new(false)).unwrap()
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.newly_published).count(), 1);
    assert_eq!(fs::read_dir(&c.0).unwrap().count(), 1);
}

#[test]
fn cancellation_after_stage_creation_cleans_partial_members() {
    let c = Cache::new();
    let t = UnpackedArchive {
        files: vec![UnpackedFile {
            relative_path: "lib/large".into(),
            bytes: vec![42; 32 * 1024 * 1024],
        }],
        directories: vec!["lib".into()],
    };
    let h = hash(&t);
    let cancelled = std::sync::Arc::new(AtomicBool::new(false));
    let finished = std::sync::Arc::new(AtomicBool::new(false));
    let observer = {
        let root = c.0.clone();
        let cancelled = cancelled.clone();
        let finished = finished.clone();
        std::thread::spawn(move || {
            let end = Instant::now() + Duration::from_secs(10);
            while !finished.load(Ordering::Relaxed) && Instant::now() < end {
                if fs::read_dir(&root).unwrap().any(|e| {
                    e.unwrap()
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".bundle-install-")
                }) {
                    cancelled.store(true, Ordering::Relaxed);
                    return true;
                }
                std::thread::yield_now();
            }
            false
        })
    };
    let result = publish_tool_bundle(
        &c.0,
        &t,
        &h,
        Instant::now() + Duration::from_secs(10),
        &cancelled,
    );
    finished.store(true, Ordering::Relaxed);
    assert!(
        observer.join().unwrap(),
        "must observe a real temporary directory"
    );
    assert_eq!(result.unwrap_err(), "bundle_install_cancelled");
    assert_eq!(fs::read_dir(&c.0).unwrap().count(), 0);
}
