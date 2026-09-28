#![cfg(all(feature = "wasm-precheck", unix))]

use codeguard_cli::syntax_worker_runner::run_syntax_worker_candidate;
use codeguard_core::SyntaxPrecheckStatus;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};
#[cfg(unix)]
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(30)
}

#[test]
fn isolated_java_worker_preserves_missing_anchor_and_candidate_status() {
    let source = b"class A { int x = 1;";
    let result = run_syntax_worker_candidate(
        env!("CARGO_BIN_EXE_codeguard").as_ref(),
        "java",
        "src/A.java",
        source,
        deadline(),
        &AtomicBool::new(false),
    )
    .expect("isolated candidate observation");
    assert_eq!(result.precheck.status, SyntaxPrecheckStatus::Incomplete);
    assert!(!result.grammar_qualified);
    assert_eq!(result.source_sha256.len(), 64);
    assert!(result.recoveries.iter().any(|item| item.kind == "MISSING"));
    assert!(
        result
            .recoveries
            .iter()
            .all(|item| item.end_byte <= source.len())
    );
}

#[test]
fn clean_candidate_cannot_be_promoted_to_clean() {
    let result = run_syntax_worker_candidate(
        env!("CARGO_BIN_EXE_codeguard").as_ref(),
        "typescript",
        "src/a.ts",
        b"const value: number = 1;",
        deadline(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(result.recoveries.is_empty());
    assert_eq!(result.precheck.status, SyntaxPrecheckStatus::Incomplete);
    assert_eq!(result.precheck.unqualified_files, 1);
}

#[test]
fn invalid_input_and_cancellation_cannot_yield_observations() {
    let exe = env!("CARGO_BIN_EXE_codeguard").as_ref();
    assert!(
        run_syntax_worker_candidate(
            exe,
            "unknown",
            "src/a",
            b"x",
            deadline(),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    assert!(
        run_syntax_worker_candidate(
            exe,
            "java",
            "src/A.java",
            &vec![b'x'; 1024 * 1024 + 1],
            deadline(),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    assert!(
        run_syntax_worker_candidate(
            exe,
            "java",
            "src/A.java",
            b"class A {}",
            deadline(),
            &AtomicBool::new(true)
        )
        .is_err()
    );
}

#[cfg(unix)]
fn fake_worker(body: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "codeguard-fake-syntax-worker-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("unnamed")
    ));
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}

#[cfg(unix)]
#[test]
fn timeout_or_forged_worker_output_does_not_poison_later_observation() {
    let source = b"class A {}";
    let busy = fake_worker("while :; do :; done");
    let started = Instant::now();
    let timed_out = run_syntax_worker_candidate(
        &busy,
        "java",
        "src/A.java",
        source,
        Instant::now() + Duration::from_millis(500),
        &AtomicBool::new(false),
    );
    fs::remove_file(&busy).unwrap();
    assert!(timed_out.is_err());
    assert!(started.elapsed() < Duration::from_secs(4));

    let forged = fake_worker("printf '%s' '{}'");
    let invalid = run_syntax_worker_candidate(
        &forged,
        "java",
        "src/A.java",
        source,
        deadline(),
        &AtomicBool::new(false),
    );
    fs::remove_file(&forged).unwrap();
    assert!(invalid.is_err());

    let good = run_syntax_worker_candidate(
        env!("CARGO_BIN_EXE_codeguard").as_ref(),
        "java",
        "src/A.java",
        source,
        deadline(),
        &AtomicBool::new(false),
    );
    assert!(good.is_ok());
}
