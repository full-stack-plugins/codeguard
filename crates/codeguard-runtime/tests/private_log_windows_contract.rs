#![cfg(windows)]

use codeguard_runtime::{
    ProcessOutcome, Termination, platform_fs, prepare_fresh_report, write_private_log,
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let path = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-private-windows-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        platform_fs::create_private_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn outcome() -> ProcessOutcome {
    ProcessOutcome {
        termination: Termination::Exited(0),
        spawn_os_error: None,
        stdout: vec![0xff, 0, b'a'],
        stderr: vec![],
        elapsed: Duration::from_millis(1),
    }
}
#[test]
fn private_record_preserves_bytes_and_never_replaces_destination() {
    let root = Root::new();
    let path = write_private_log(&root.0, "run.log", &outcome()).unwrap();
    let bytes = fs::read(&path).unwrap();
    assert!(bytes.starts_with(b"CGLOG1\0"));
    assert!(bytes.ends_with(&[0xff, 0, b'a']));
    assert!(write_private_log(&root.0, "run.log", &outcome()).is_err());
    assert_eq!(fs::read(path).unwrap(), bytes);
}
#[test]
fn rejects_windows_aliases_streams_and_traversal() {
    let root = Root::new();
    for name in [
        "../escape",
        "sub\\file",
        "file:stream",
        "NUL",
        "CON.txt",
        "COM¹.log",
        "LPT²",
        "trailing.",
        "trailing ",
        "",
        ".",
        "..",
    ] {
        assert!(
            write_private_log(&root.0, name, &outcome()).is_err(),
            "{name:?}"
        );
    }
    assert_eq!(fs::read_dir(&root.0).unwrap().count(), 0);
}
#[test]
fn fresh_report_rejects_old_files_and_hardlinks_and_pins_root() {
    let root = Root::new();
    fs::write(root.0.join("old.json"), b"old").unwrap();
    assert!(prepare_fresh_report(&root.0, "old.json", 32).is_err());
    let slot = prepare_fresh_report(&root.0, "report.json", 32).unwrap();
    let moved = root.0.with_extension("moved");
    assert!(fs::rename(&root.0, &moved).is_err());
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS;
    assert!(
        fs::OpenOptions::new()
            .write(true)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(&root.0)
            .is_err(),
        "pinned directory must refuse write handles used for reparse mutation"
    );
    fs::hard_link(root.0.join("old.json"), root.0.join("report.json")).unwrap();
    assert!(slot.read().is_err());
}
#[test]
fn fresh_report_reads_bounded_real_file() {
    let root = Root::new();
    let slot = prepare_fresh_report(&root.0, "report.json", 3).unwrap();
    fs::write(root.0.join("report.json"), b"abc").unwrap();
    assert_eq!(slot.read().unwrap(), b"abc");
    fs::write(root.0.join("report.json"), b"abcd").unwrap();
    assert!(slot.read().is_err());
}
#[test]
fn junction_ancestor_is_rejected_without_writing_target() {
    let root = Root::new();
    let outside = Root::new();
    let link = root.0.join("junction");
    let status = std::process::Command::new("cmd.exe")
        .args(["/d", "/c", "mklink", "/J"])
        .arg(&link)
        .arg(&outside.0)
        .status()
        .unwrap();
    assert!(status.success(), "fixture junction creation must succeed");
    assert!(write_private_log(&link, "run.log", &outcome()).is_err());
    assert!(!outside.0.join("run.log").exists());
    fs::remove_dir(link).unwrap();
}
#[test]
fn rejects_public_dacl_before_creating_any_evidence() {
    let root = Root::new();
    let status = std::process::Command::new("icacls.exe")
        .arg(&root.0)
        .args(["/grant", "*S-1-1-0:(OI)(CI)R"])
        .status()
        .unwrap();
    assert!(status.success(), "fixture ACL broadening must succeed");
    assert!(write_private_log(&root.0, "run.log", &outcome()).is_err());
    assert!(prepare_fresh_report(&root.0, "report.json", 16).is_err());
    assert_eq!(fs::read_dir(&root.0).unwrap().count(), 0);
}
