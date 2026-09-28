use codeguard_runtime::read_verified_package;
use ring::digest::{SHA256, digest};
use std::io::{self, Cursor, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
fn sha(bytes: &[u8]) -> [u8; 32] {
    digest(&SHA256, bytes).as_ref().try_into().unwrap()
}
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}
#[test]
fn exact_stream_yields_frozen_bytes() {
    let bytes = b"verified native package";
    let result = read_verified_package(
        &mut Cursor::new(bytes),
        bytes.len() as u64,
        sha(bytes),
        deadline(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(result, bytes);
}
#[test]
fn short_long_and_wrong_digest_are_not_valid_packages() {
    for bytes in [b"too".as_slice(), b"tool!".as_slice(), b"evil".as_slice()] {
        assert!(
            read_verified_package(
                &mut Cursor::new(bytes),
                4,
                sha(b"tool"),
                deadline(),
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
}
struct NeverRead;
impl Read for NeverRead {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        panic!("invalid/cancelled input must not read")
    }
}
#[test]
fn invalid_size_digest_cancel_or_deadline_do_not_read() {
    let no = AtomicBool::new(false);
    for (size, hash) in [(0, sha(b"tool")), (134217729, sha(b"tool")), (4, [0; 32])] {
        assert_eq!(
            read_verified_package(&mut NeverRead, size, hash, deadline(), &no).unwrap_err(),
            "package_input_invalid"
        );
    }
    assert_eq!(
        read_verified_package(
            &mut NeverRead,
            4,
            sha(b"tool"),
            deadline(),
            &AtomicBool::new(true)
        )
        .unwrap_err(),
        "package_cancelled"
    );
    assert_eq!(
        read_verified_package(&mut NeverRead, 4, sha(b"tool"), Instant::now(), &no).unwrap_err(),
        "package_deadline_exceeded"
    );
}
struct CancelDuringRead<'a>(&'a AtomicBool);
impl Read for CancelDuringRead<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        out[0] = b't';
        self.0.store(true, Ordering::Relaxed);
        Ok(1)
    }
}
#[test]
fn cancellation_after_a_chunk_does_not_return_bytes() {
    let cancel = AtomicBool::new(false);
    assert_eq!(
        read_verified_package(
            &mut CancelDuringRead(&cancel),
            4,
            sha(b"tool"),
            deadline(),
            &cancel
        )
        .unwrap_err(),
        "package_cancelled"
    );
}
struct Broken;
impl Read for Broken {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("PRIVATE transport address"))
    }
}
#[test]
fn transport_error_is_fixed_and_redacted() {
    assert_eq!(
        read_verified_package(
            &mut Broken,
            4,
            sha(b"tool"),
            deadline(),
            &AtomicBool::new(false)
        )
        .unwrap_err(),
        "package_read_failed"
    );
}
struct InterruptOnce(bool, Cursor<Vec<u8>>);
impl Read for InterruptOnce {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if !self.0 {
            self.0 = true;
            Err(io::ErrorKind::Interrupted.into())
        } else {
            self.1.read(out)
        }
    }
}
#[test]
fn interrupted_read_retries_with_the_same_budget() {
    let mut reader = InterruptOnce(false, Cursor::new(b"tool".to_vec()));
    assert_eq!(
        read_verified_package(
            &mut reader,
            4,
            sha(b"tool"),
            deadline(),
            &AtomicBool::new(false)
        )
        .unwrap(),
        b"tool"
    );
}

#[test]
fn oversized_stream_consumes_at_most_declared_length_plus_one() {
    let mut reader = Cursor::new(vec![b't'; 1024]);
    assert_eq!(
        read_verified_package(
            &mut reader,
            4,
            sha(b"tttt"),
            deadline(),
            &AtomicBool::new(false)
        )
        .unwrap_err(),
        "package_size_mismatch"
    );
    assert_eq!(reader.position(), 5);
}
struct SlowRead;
impl Read for SlowRead {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        std::thread::sleep(Duration::from_millis(30));
        out[0] = b't';
        Ok(1)
    }
}
#[test]
fn deadline_crossed_inside_read_is_rejected_after_return() {
    assert_eq!(
        read_verified_package(
            &mut SlowRead,
            4,
            sha(b"tool"),
            Instant::now() + Duration::from_millis(10),
            &AtomicBool::new(false)
        )
        .unwrap_err(),
        "package_deadline_exceeded"
    );
}

#[cfg(unix)]
#[test]
fn verified_raw_package_publishes_and_corrupt_package_never_reaches_cache() {
    use codeguard_runtime::publish_tool_bytes;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::AtomicU64;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "cg-package-chain-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let cancel = AtomicBool::new(false);
    let budget = deadline();
    let corrupted =
        read_verified_package(&mut Cursor::new(b"evil"), 4, sha(b"tool"), budget, &cancel);
    assert!(corrupted.is_err());
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    let bytes =
        read_verified_package(&mut Cursor::new(b"tool"), 4, sha(b"tool"), budget, &cancel).unwrap();
    let receipt = publish_tool_bytes(&root, &bytes, sha(b"tool"), budget, &cancel).unwrap();
    assert!(receipt.newly_published);
    assert_eq!(
        fs::read(root.join(&receipt.relative_path)).unwrap(),
        b"tool"
    );
    assert!(
        !publish_tool_bytes(&root, &bytes, sha(b"tool"), budget, &cancel)
            .unwrap()
            .newly_published
    );
    fs::remove_dir_all(root).unwrap();
}
