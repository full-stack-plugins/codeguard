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
    // Ordinary child namespace operations need directory write sharing.
    // Actual metadata mutation/redirect safety is tested separately below.
    fs::hard_link(root.0.join("old.json"), root.0.join("report.json")).unwrap();
    assert!(slot.read().is_err());
    drop(slot);
    fs::rename(&root.0, &moved).expect("dropping all guard handles releases rename protection");
    fs::rename(&moved, &root.0).unwrap();
}
#[test]
fn fresh_report_pins_ancestor_not_just_leaf_directory() {
    let root = Root::new();
    let child = root.0.join("evidence");
    platform_fs::create_private_dir(&child).unwrap();
    let slot = prepare_fresh_report(&child, "report.json", 32).unwrap();
    let moved = root.0.with_extension("moved");
    assert!(
        fs::rename(&root.0, &moved).is_err(),
        "ancestor replacement must be blocked too"
    );
    fs::write(child.join("report.json"), b"ok").unwrap();
    assert_eq!(slot.read().unwrap(), b"ok");
    drop(slot);
    fs::rename(&root.0, &moved).expect("ancestor guard handles must be released");
    fs::rename(&moved, &root.0).unwrap();
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

#[test]
fn mutation_of_open_directory_into_junction_cannot_redirect_report() {
    use std::os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, io::AsRawHandle};
    use windows_sys::Win32::{
        Storage::FileSystem::{FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT},
        System::IO::DeviceIoControl,
    };
    let root = Root::new();
    let outside = Root::new();
    fs::write(
        outside.0.join("report.json"),
        b"outside evidence must never be accepted",
    )
    .unwrap();
    let slot = prepare_fresh_report(&root.0, "report.json", 128).unwrap();
    let writable = fs::OpenOptions::new()
        .write(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(&root.0)
        .unwrap();
    let target = outside.0.canonicalize().unwrap();
    let target = target.to_string_lossy();
    let target = target.strip_prefix(r"\\?\").unwrap_or(&target);
    let substitute = format!(r"\??\{}", target);
    let substitute: Vec<u16> = std::ffi::OsStr::new(&substitute).encode_wide().collect();
    let printable: Vec<u16> = std::ffi::OsStr::new(target).encode_wide().collect();
    // REPARSE_DATA_BUFFER MountPointReparseBuffer, documented NTFS junction form.
    let data_len = 8 + (substitute.len() + printable.len() + 2) * 2;
    let mut buffer = Vec::new();
    buffer.extend_from_slice(&0xA0000003u32.to_le_bytes()); // IO_REPARSE_TAG_MOUNT_POINT
    buffer.extend_from_slice(&(data_len as u16).to_le_bytes());
    buffer.extend_from_slice(&0u16.to_le_bytes());
    for field in [
        0u16,
        (substitute.len() * 2) as u16,
        ((substitute.len() + 1) * 2) as u16,
        (printable.len() * 2) as u16,
    ] {
        buffer.extend_from_slice(&field.to_le_bytes());
    }
    for character in substitute
        .into_iter()
        .chain([0])
        .chain(printable)
        .chain([0])
    {
        buffer.extend_from_slice(&character.to_le_bytes());
    }
    let mut returned = 0;
    let ok = unsafe {
        DeviceIoControl(
            writable.as_raw_handle(),
            0x000900A4,
            buffer.as_ptr().cast(),
            buffer.len() as u32,
            std::ptr::null_mut(),
            0,
            &mut returned,
            std::ptr::null_mut(),
        )
    }; // FSCTL_SET_REPARSE_POINT
    assert_ne!(
        ok,
        0,
        "fixture must perform actual metadata mutation without enabling privileges: {}",
        std::io::Error::last_os_error()
    );
    assert!(
        slot.read().is_err(),
        "mutated root must not read target evidence"
    );
    assert!(write_private_log(&root.0, "redirect.log", &outcome()).is_err());
    assert!(!outside.0.join("redirect.log").exists());
    assert_eq!(
        fs::read(outside.0.join("report.json")).unwrap(),
        b"outside evidence must never be accepted"
    );
    drop(writable);
    drop(slot);
    fs::remove_dir(&root.0).unwrap(); // remove junction, never traverse its target
}
