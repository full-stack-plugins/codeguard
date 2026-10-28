//! Native platform security primitives. Runs unchanged on Windows and Unix.
use codeguard_runtime::{TaskFileLock, platform_fs as fs};
use std::{io::Write, path::PathBuf};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "cg-platform-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_private_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn private_creation_and_validation() {
    let t = Temp::new();
    fs::verify_private_permissions(&t.0, 0o700).unwrap();
    let p = t.0.join("secret");
    let mut f = fs::secure_create_new(&p).unwrap();
    f.write_all(b"secret").unwrap();
    f.sync_all().unwrap();
    drop(f);
    fs::verify_private_permissions(&p, 0o600).unwrap();
    assert!(fs::secure_create_new(&p).is_err());
    let original = fs::file_identity(&p).unwrap();
    let alias = t.0.join("alias");
    std::fs::hard_link(&p, &alias).unwrap();
    assert_eq!(original, fs::file_identity(&alias).unwrap());
    assert!(!fs::single_link(&p));
}
#[test]
fn lock_exclusion_release_and_hardlink_rejection() {
    let t = Temp::new();
    let p = t.0.join("lock");
    let guard = TaskFileLock::acquire(&p).unwrap();
    assert!(TaskFileLock::acquire(&p).is_err());
    drop(guard);
    drop(TaskFileLock::acquire(&p).unwrap());
    std::fs::hard_link(&p, t.0.join("alias")).unwrap();
    assert!(TaskFileLock::acquire(&p).is_err());
}
#[test]
fn replacement_changes_identity_and_bytes() {
    let t = Temp::new();
    let p = t.0.join("state");
    let q = t.0.join("staging");
    drop(fs::secure_create_new(&p).unwrap());
    let old = fs::file_identity(&p).unwrap();
    let mut f = fs::secure_create_new(&q).unwrap();
    f.write_all(b"new").unwrap();
    f.sync_all().unwrap();
    drop(f);
    fs::atomic_replace(&q, &p).unwrap();
    assert_ne!(old, fs::file_identity(&p).unwrap());
    assert_eq!(std::fs::read(&p).unwrap(), b"new");
    fs::verify_private_permissions(&p, 0o600).unwrap();
}
#[test]
fn lock_child_fixture() {
    let Some(path) = std::env::var_os("CODEGUARD_LOCK_FIXTURE") else {
        return;
    };
    let _guard = TaskFileLock::acquire(std::path::Path::new(&path)).unwrap();
    println!("LOCK_READY");
    std::io::stdout().flush().unwrap();
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();
}
#[test]
fn cross_process_lock_and_crash_release() {
    use std::{
        io::{BufRead, BufReader},
        process::{Command, Stdio},
    };
    let t = Temp::new();
    let p = t.0.join("process-lock");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "lock_child_fixture", "--nocapture"])
        .env("CODEGUARD_LOCK_FIXTURE", &p)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let mut ready = false;
    for _ in 0..10 {
        let mut line = String::new();
        if output.read_line(&mut line).unwrap() == 0 {
            break;
        }
        if line.contains("LOCK_READY") {
            ready = true;
            break;
        }
    }
    assert!(ready, "child failed to acquire lock");
    assert!(TaskFileLock::acquire(&p).is_err());
    child.kill().unwrap();
    child.wait().unwrap();
    drop(TaskFileLock::acquire(&p).unwrap());
}
#[cfg(windows)]
#[test]
fn broader_dacl_is_rejected() {
    let t = Temp::new();
    let status = std::process::Command::new("icacls.exe")
        .arg(&t.0)
        .args(["/grant", "*S-1-1-0:(OI)(CI)F"])
        .status()
        .unwrap();
    assert!(status.success());
    assert!(fs::verify_private_permissions(&t.0, 0o700).is_err());
    fs::set_private_permissions(&t.0, 0o700).unwrap();
    fs::verify_private_permissions(&t.0, 0o700).unwrap();
}
#[cfg(windows)]
#[test]
fn junction_ancestors_and_locked_parent_rename_are_rejected() {
    let t = Temp::new();
    let real = t.0.join("real");
    fs::create_private_dir(&real).unwrap();
    let lock = real.join("lock");
    let guard = TaskFileLock::acquire(&lock).unwrap();
    assert!(std::fs::rename(&real, t.0.join("moved")).is_err());
    drop(guard);
    std::fs::rename(&real, t.0.join("moved")).unwrap();
    std::fs::rename(t.0.join("moved"), &real).unwrap();
    let alias = t.0.join("alias");
    let status = std::process::Command::new("cmd.exe")
        .args(["/D", "/C", "mklink", "/J"])
        .arg(&alias)
        .arg(&real)
        .status()
        .unwrap();
    assert!(status.success());
    assert!(fs::secure_create_new(&alias.join("secret")).is_err());
    assert!(TaskFileLock::acquire(&alias.join("lock")).is_err());
    std::fs::remove_dir(alias).unwrap();
}
#[cfg(unix)]
#[test]
fn secure_read_rejects_fifo_without_waiting_for_a_writer() {
    use std::os::unix::ffi::OsStrExt;
    let t = Temp::new();
    let path = t.0.join("fifo");
    let name = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let started = std::time::Instant::now();
    assert!(fs::secure_read(&path).is_err());
    assert!(started.elapsed() < std::time::Duration::from_secs(1));
}
