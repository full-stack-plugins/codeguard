#![cfg(unix)]
use codeguard_runtime::{NativeVersionRequest, ProcessSpec, Termination, observe_native_version};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-version-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        Self(root)
    }
    fn request(&self, body: &str) -> NativeVersionRequest {
        let tool = self.0.join("tool");
        let bytes = format!("#!/bin/sh\n{body}\n");
        fs::write(&tool, bytes.as_bytes()).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        NativeVersionRequest {
            process: ProcessSpec {
                executable: tool,
                args: vec!["--version".into()],
                cwd: self.0.clone(),
                env: BTreeMap::new(),
                stdin: None,
                deadline: Instant::now() + Duration::from_secs(5),
                output_limit_bytes: 4096,
            },
            expected_tool_sha256: Sha256::digest(bytes.as_bytes()).into(),
            expected_stdout: b"fixture 1\n".to_vec(),
            evidence_root: self.0.clone(),
            log_name: "version.log".into(),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn exact_native_version_is_local_evidence_and_raw_output_is_private() {
    let fixture = Fixture::new();
    let request = fixture.request("printf 'fixture 1\\n'");
    let result = observe_native_version(&request, &AtomicBool::new(false));
    assert!(result.complete);
    assert_eq!(result.reason, None);
    assert_eq!(result.termination, Some(Termination::Exited(0)));
    assert!(fixture.0.join("version.log").exists());
}

#[test]
fn unsupported_version_stderr_and_nonzero_exit_are_distinct() {
    for (body, reason) in [
        ("printf 'fixture 2\\n'", "tool_version_mismatch"),
        (
            "printf 'fixture 1\\n'; printf 'private message\\n' >&2",
            "version_stderr_unexpected",
        ),
        ("exit 7", "version_native_exit_nonzero"),
    ] {
        let fixture = Fixture::new();
        let result = observe_native_version(&fixture.request(body), &AtomicBool::new(false));
        assert!(!result.complete);
        assert_eq!(result.reason, Some(reason));
    }
}

#[test]
fn timeout_cancellation_and_exhausted_budget_keep_native_failure_kind() {
    for variant in 0..3 {
        let fixture = Fixture::new();
        let mut request = fixture.request("exec /bin/sleep 1");
        let cancelled = AtomicBool::new(variant == 1);
        request.process.deadline = if variant == 2 {
            Instant::now()
        } else {
            Instant::now() + Duration::from_millis(30)
        };
        let result = observe_native_version(&request, &cancelled);
        let (termination, reason) = match variant {
            0 => (Termination::TimedOut, "version_timed_out"),
            1 => (Termination::Cancelled, "request_cancelled"),
            _ => (Termination::DeadlineBeforeStart, "deadline_exhausted"),
        };
        assert_eq!(result.termination, Some(termination));
        assert_eq!(result.reason, Some(reason));
        assert!(!result.complete);
    }
}

#[test]
fn wrong_identity_and_nonversion_invocation_never_start_tool() {
    for variant in 0..2 {
        let fixture = Fixture::new();
        let mut request = fixture.request("touch marker; printf 'fixture 1\\n'");
        if variant == 0 {
            request.expected_tool_sha256 = [1; 32];
        } else {
            request.process.args = vec!["check".into()];
        }
        let result = observe_native_version(&request, &AtomicBool::new(false));
        assert!(!result.complete);
        assert!(!fixture.0.join("marker").exists());
    }
}

#[test]
fn evidence_failure_does_not_turn_native_exit_zero_into_complete() {
    let fixture = Fixture::new();
    let mut request = fixture.request("printf 'fixture 1\\n'");
    request.evidence_root = fixture.0.join("missing");
    let result = observe_native_version(&request, &AtomicBool::new(false));
    assert!(!result.complete);
    assert_eq!(result.reason, Some("evidence_write_failed"));
    assert_eq!(result.termination, Some(Termination::Exited(0)));
}

#[test]
fn spawn_signal_output_limit_and_changed_tool_have_specific_diagnostics() {
    for variant in 0..4 {
        let fixture = Fixture::new();
        let body = match variant {
            0 => "printf 'fixture 1\\n'",
            1 => "kill -TERM $$",
            2 => "while :; do printf 'fixture 1\\n'; done",
            _ => "printf '\\n# changed\\n' >> \"$0\"; printf 'fixture 1\\n'",
        };
        let mut request = fixture.request(body);
        if variant == 0 {
            fs::set_permissions(
                &request.process.executable,
                fs::Permissions::from_mode(0o600),
            )
            .unwrap();
        }
        if variant == 2 {
            request.process.output_limit_bytes = 32;
        }
        let result = observe_native_version(&request, &AtomicBool::new(false));
        let reason = [
            "version_spawn_failed",
            "version_signaled",
            "version_output_limit",
            "tool_identity_changed",
        ][variant];
        assert_eq!(result.reason, Some(reason));
        assert!(!result.complete);
    }
}

#[test]
fn selected_real_ruff_version_is_observed_without_running_lint() {
    let Some(tool) = std::env::var_os("CODEGUARD_TEST_RUFF") else {
        return;
    };
    let tool = fs::canonicalize(tool).unwrap();
    let fixture = Fixture::new();
    let mut request = fixture.request("exit 1");
    request.expected_tool_sha256 = Sha256::digest(fs::read(&tool).unwrap()).into();
    request.process.executable = tool;
    request.expected_stdout = b"ruff 0.16.8\n".to_vec();
    let result = observe_native_version(&request, &AtomicBool::new(false));
    assert!(result.complete, "{:?}", result.reason);
    assert_eq!(result.termination, Some(Termination::Exited(0)));
    assert!(fixture.0.join("version.log").exists());
}
