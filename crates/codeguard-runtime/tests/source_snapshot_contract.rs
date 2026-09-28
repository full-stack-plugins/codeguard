use codeguard_runtime::SourceSnapshot;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "codeguard-source-snapshot-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn capture(root: &Path, paths: &[&str]) -> std::io::Result<SourceSnapshot> {
    SourceSnapshot::capture(root, paths.iter().map(PathBuf::from), 2, 32, 64)
}

#[test]
fn bounded_snapshot_rechecks_original_and_private_copy() {
    let fixture = Fixture::new();
    let source = fixture.path().join("src/main/java/demo/Good.java");
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    fs::write(&source, b"class Good {}").unwrap();
    let snapshot = capture(fixture.path(), &["src/main/java/demo/Good.java"]).unwrap();
    let copy = fixture.path().join("private-copy");
    snapshot.materialize_new(&copy).unwrap();
    assert!(snapshot.verify_unchanged(&copy).unwrap());
    assert!(snapshot.materialize_new(&copy).is_err());

    fs::write(&source, b"class Bad {}").unwrap();
    assert!(!snapshot.verify_unchanged(&copy).unwrap());
    fs::write(&source, b"class Good {}").unwrap();
    fs::write(copy.join("src/main/java/demo/Good.java"), b"class Bad {}").unwrap();
    assert!(!snapshot.verify_unchanged(&copy).unwrap());
}

#[test]
fn source_only_recheck_detects_same_path_byte_changes() {
    let fixture = Fixture::new();
    let source = fixture.path().join("app.py");
    fs::write(&source, b"import os\n").unwrap();
    let snapshot = capture(fixture.path(), &["app.py"]).unwrap();
    assert!(snapshot.verify_source_unchanged().unwrap());
    fs::write(&source, b"import io\n").unwrap();
    assert!(!snapshot.verify_source_unchanged().unwrap());
}

#[test]
fn snapshot_rejects_escape_duplicate_and_budget_overflow() {
    let fixture = Fixture::new();
    fs::write(fixture.path().join("pom.xml"), b"1234").unwrap();
    for path in ["../pom.xml", "./pom.xml", "/pom.xml", "", "a//pom.xml"] {
        assert!(capture(fixture.path(), &[path]).is_err(), "{path}");
    }
    assert!(capture(fixture.path(), &["pom.xml", "pom.xml"]).is_err());
    assert!(SourceSnapshot::capture(fixture.path(), [PathBuf::from("pom.xml")], 1, 3, 64).is_err());
    assert!(SourceSnapshot::capture(fixture.path(), [PathBuf::from("pom.xml")], 1, 32, 3).is_err());
    assert!(capture(fixture.path(), &[]).is_err());
}

#[cfg(unix)]
#[test]
fn snapshot_rejects_linked_file_and_linked_ancestor() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    let real = fixture.path().join("real");
    fs::create_dir(&real).unwrap();
    fs::write(real.join("Good.java"), b"class Good {}").unwrap();
    symlink(real.join("Good.java"), fixture.path().join("linked.java")).unwrap();
    symlink(&real, fixture.path().join("linked-dir")).unwrap();
    assert!(capture(fixture.path(), &["linked.java"]).is_err());
    assert!(capture(fixture.path(), &["linked-dir/Good.java"]).is_err());
    let snapshot = capture(fixture.path(), &["real/Good.java"]).unwrap();
    let copy = fixture.path().join("copy");
    snapshot.materialize_new(&copy).unwrap();
    fs::remove_file(copy.join("real/Good.java")).unwrap();
    symlink(real.join("Good.java"), copy.join("real/Good.java")).unwrap();
    assert!(snapshot.verify_unchanged(&copy).is_err());
}

#[cfg(unix)]
#[test]
fn directory_swap_cannot_capture_bytes_outside_the_pinned_root() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    let root = fixture.path().join("root");
    let outside = fixture.path().join("outside");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&outside).unwrap();
    fs::create_dir(root.join("inside")).unwrap();
    fs::write(root.join("inside/Good.java"), b"inside").unwrap();
    fs::write(outside.join("Good.java"), b"outside").unwrap();
    let alias = root.join("inside");
    let held = root.join("held");
    let attacker = std::thread::spawn(move || {
        for _ in 0..1_000 {
            fs::rename(&alias, &held).unwrap();
            symlink(&outside, &alias).unwrap();
            fs::remove_file(&alias).unwrap();
            fs::rename(&held, &alias).unwrap();
        }
    });
    for _ in 0..1_000 {
        if let Ok(snapshot) = capture(&root, &["inside/Good.java"]) {
            assert_eq!(snapshot.files()[Path::new("inside/Good.java")], b"inside");
        }
    }
    attacker.join().unwrap();
}
