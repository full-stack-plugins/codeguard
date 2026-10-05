#![cfg(all(feature = "wasm-precheck", unix))]

use codeguard_cli::syntax_worker_runner::run_syntax_worker_candidate;
use codeguard_core::SyntaxPrecheckStatus;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
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
fn go_whole_file_rule_stays_separate_and_forged_worker_frames_are_rejected() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let source = b"func f() {}\n";
    let executable = env!("CARGO_BIN_EXE_codeguard");
    let mut child = Command::new(executable)
        .args(["__syntax-worker", "go"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(source).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let original: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(original["schema_version"], "1.2.0");
    assert_eq!(original["recoveries"], serde_json::json!([]));
    assert_eq!(
        original["structural_observations"][0]["rule_id"],
        "codeguard.go.required_package"
    );
    for edit in [
        "old_version",
        "wrong_rule",
        "wrong_hash",
        "position",
        "duplicate",
        "unknown_field",
    ] {
        let mut value = original.clone();
        match edit {
            "old_version" => value["schema_version"] = serde_json::json!("1.1.0"),
            "wrong_rule" => {
                value["structural_observations"][0]["rule_id"] =
                    serde_json::json!("codeguard.python.required_suite")
            }
            "wrong_hash" => {
                value["structural_observations"][0]["rule_sha256"] =
                    serde_json::json!("0".repeat(64))
            }
            "position" => {
                value["structural_observations"][0]["start_byte"] = serde_json::json!(1);
                value["structural_observations"][0]["end_byte"] = serde_json::json!(1);
                value["structural_observations"][0]["start_column_byte"] = serde_json::json!(1);
                value["structural_observations"][0]["end_column_byte"] = serde_json::json!(1);
            }
            "duplicate" => {
                let duplicate = value["structural_observations"][0].clone();
                value["structural_observations"]
                    .as_array_mut()
                    .unwrap()
                    .push(duplicate);
            }
            _ => value["approved"] = serde_json::json!(true),
        }
        let quoted = value.to_string().replace('\'', "'\\''");
        let fake = fake_worker(&format!("printf '%s' '{quoted}'"));
        assert!(
            run_syntax_worker_candidate(
                &fake,
                "go",
                "main.go",
                source,
                deadline(),
                &AtomicBool::new(false)
            )
            .is_err(),
            "{edit}"
        );
        fs::remove_file(fake).unwrap();
    }
    let valid = run_syntax_worker_candidate(
        executable.as_ref(),
        "go",
        "main.go",
        source,
        deadline(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(valid.structural_observations.len(), 1);
    assert!(valid.recoveries.is_empty());
    assert_eq!(valid.precheck.status, SyntaxPrecheckStatus::Incomplete);
}

#[test]
fn adapted_zig_asset_preserves_candidate_status_and_empty_container_syntax() {
    let result = run_syntax_worker_candidate(
        env!("CARGO_BIN_EXE_codeguard").as_ref(),
        "zig",
        "src/main.zig",
        b"const Empty = struct {};\n",
        deadline(),
        &AtomicBool::new(false),
    )
    .expect("isolated Zig candidate");
    assert!(result.recoveries.is_empty());
    assert_eq!(result.precheck.status, SyntaxPrecheckStatus::Incomplete);
    assert!(!result.grammar_qualified);
}

#[test]
fn dependency_grammars_run_in_worker_without_claiming_clean() {
    for (language, path, source) in [
        (
            "objc",
            "src/Foo.m",
            b"@interface Foo : NSObject\n@end\n".as_slice(),
        ),
        (
            "solidity",
            "src/Vault.sol",
            b"pragma solidity ^0.8.20;\ncontract Vault {}\n".as_slice(),
        ),
    ] {
        let result = run_syntax_worker_candidate(
            env!("CARGO_BIN_EXE_codeguard").as_ref(),
            language,
            path,
            source,
            Instant::now() + Duration::from_secs(60),
            &AtomicBool::new(false),
        )
        .expect("dependency grammar worker candidate");
        assert!(result.recoveries.is_empty(), "{language}");
        assert_eq!(result.precheck.status, SyntaxPrecheckStatus::Incomplete);
        assert!(!result.grammar_qualified);
    }
}

#[test]
fn mainstream_grammars_run_in_worker_without_claiming_clean() {
    for (language, path, source) in [
        (
            "c",
            "src/main.c",
            b"int main(void) { return 0; }".as_slice(),
        ),
        (
            "go",
            "main.go",
            b"package main\nfunc main() {}\n".as_slice(),
        ),
        ("javascript", "src/main.js", b"const value = 1;".as_slice()),
        (
            "rust",
            "src/main.rs",
            b"fn main() { let x = 1; }".as_slice(),
        ),
    ] {
        let result = run_syntax_worker_candidate(
            env!("CARGO_BIN_EXE_codeguard").as_ref(),
            language,
            path,
            source,
            Instant::now() + Duration::from_secs(60),
            &AtomicBool::new(false),
        )
        .expect("mainstream grammar worker candidate");
        assert!(result.recoveries.is_empty(), "{language}");
        assert_eq!(result.precheck.status, SyntaxPrecheckStatus::Incomplete);
        assert!(!result.grammar_qualified);
    }
}

#[test]
fn cpp_csharp_lua_and_luau_run_in_worker_without_claiming_clean() {
    for (language, path, source) in [
        (
            "cpp",
            "src/main.cpp",
            b"int main() { return 0; }".as_slice(),
        ),
        (
            "csharp",
            "src/C.cs",
            b"class C { static int F() { return 1; } }".as_slice(),
        ),
        ("lua", "src/main.lua", b"local x = 1\n".as_slice()),
        ("luau", "src/main.luau", b"local x: number = 1\n".as_slice()),
    ] {
        let result = run_syntax_worker_candidate(
            env!("CARGO_BIN_EXE_codeguard").as_ref(),
            language,
            path,
            source,
            Instant::now() + Duration::from_secs(60),
            &AtomicBool::new(false),
        )
        .expect("grammar worker candidate");
        assert!(result.recoveries.is_empty(), "{language}");
        assert_eq!(result.precheck.status, SyntaxPrecheckStatus::Incomplete);
        assert!(!result.grammar_qualified);
    }
}

#[test]
fn arkts_nix_and_terraform_run_in_worker_without_claiming_clean() {
    for (language, path, source) in [
        (
            "arkts",
            "src/Main.ets",
            b"@Component struct C { build() { Text('hi') } }".as_slice(),
        ),
        ("nix", "flake.nix", b"let x = 1; in x".as_slice()),
        (
            "terraform",
            "main.tf",
            b"resource \"x\" \"y\" { foo = \"bar\" }".as_slice(),
        ),
    ] {
        let result = run_syntax_worker_candidate(
            env!("CARGO_BIN_EXE_codeguard").as_ref(),
            language,
            path,
            source,
            Instant::now() + Duration::from_secs(60),
            &AtomicBool::new(false),
        )
        .expect("grammar worker candidate");
        assert!(result.recoveries.is_empty(), "{language}");
        assert_eq!(result.precheck.status, SyntaxPrecheckStatus::Incomplete);
        assert!(!result.grammar_qualified);
    }
}

#[test]
fn r_ruby_php_and_kotlin_run_in_worker_without_claiming_clean() {
    for (language, path, source) in [
        ("r", "analysis.R", b"x <- 1\n".as_slice()),
        ("ruby", "app.rb", b"def f; 1; end\n".as_slice()),
        (
            "php",
            "index.php",
            b"<?php function f() { return 1; }".as_slice(),
        ),
        ("kotlin", "Main.kt", b"fun main() { val x = 1 }".as_slice()),
    ] {
        let result = run_syntax_worker_candidate(
            env!("CARGO_BIN_EXE_codeguard").as_ref(),
            language,
            path,
            source,
            Instant::now() + Duration::from_secs(60),
            &AtomicBool::new(false),
        )
        .expect("isolated grammar candidate");
        assert!(result.recoveries.is_empty(), "{language}");
        assert_eq!(result.precheck.status, SyntaxPrecheckStatus::Incomplete);
        assert!(!result.grammar_qualified);
    }
}

#[test]
fn rebuilt_dart_runs_in_worker_without_claiming_clean() {
    let result = run_syntax_worker_candidate(
        env!("CARGO_BIN_EXE_codeguard").as_ref(),
        "dart",
        "lib/main.dart",
        b"/// doc\nString greet(String name) => 'Hello $name';",
        Instant::now() + Duration::from_secs(60),
        &AtomicBool::new(false),
    )
    .expect("Dart grammar candidate with real scanner");
    assert!(result.recoveries.is_empty());
    assert_eq!(result.precheck.status, SyntaxPrecheckStatus::Incomplete);
    assert!(!result.grammar_qualified);
}

#[test]
fn erlang_candidate_runs_in_worker_without_claiming_clean() {
    let result = run_syntax_worker_candidate(
        env!("CARGO_BIN_EXE_codeguard").as_ref(),
        "erlang",
        "src/hello.erl",
        b"-module(hello).\nhello() -> ok.\n",
        Instant::now() + Duration::from_secs(60),
        &AtomicBool::new(false),
    )
    .expect("Erlang grammar candidate");
    assert!(result.recoveries.is_empty());
    assert_eq!(result.precheck.status, SyntaxPrecheckStatus::Incomplete);
    assert!(!result.grammar_qualified);
}

#[test]
fn pascal_candidate_runs_in_worker_without_claiming_clean() {
    let result = run_syntax_worker_candidate(
        env!("CARGO_BIN_EXE_codeguard").as_ref(),
        "pascal",
        "src/hello.pas",
        b"program Hello;\nbegin\n  writeln('Hi');\nend.\n",
        Instant::now() + Duration::from_secs(60),
        &AtomicBool::new(false),
    )
    .expect("Pascal grammar candidate");
    assert!(result.recoveries.is_empty());
    assert_eq!(result.precheck.status, SyntaxPrecheckStatus::Incomplete);
    assert!(!result.grammar_qualified);
}

#[test]
fn remaining_source_grammars_run_in_worker_without_claiming_clean() {
    for (language, path, source) in [
        ("cfml", "web/index.cfm", b"<cfset x = 1>".as_slice()),
        ("cfquery", "web/query.sql", b"SELECT #x# FROM users".as_slice()),
        ("cfscript", "web/component.cfs", b"component { function f() { return 1; } }".as_slice()),
        ("scala", "src/Main.scala", b"object Main { def f(x: Int): Int = x + 1 }".as_slice()),
        ("swift", "Sources/Main.swift", b"func f() -> Int { return 1 }".as_slice()),
        ("vbnet", "src/C.vb", b"Public Class C\n    Public Function F() As Integer\n        Return 1\n    End Function\nEnd Class\n".as_slice()),
    ] {
        let result = run_syntax_worker_candidate(
            env!("CARGO_BIN_EXE_codeguard").as_ref(),
            language,
            path,
            source,
            Instant::now() + Duration::from_secs(60),
            &AtomicBool::new(false),
        )
        .unwrap_or_else(|reason| panic!("{language} candidate: {reason}"));
        assert!(result.recoveries.is_empty(), "{language}");
        assert_eq!(result.precheck.status, SyntaxPrecheckStatus::Incomplete);
        assert!(!result.grammar_qualified);
    }
}

#[test]
fn cobol_candidate_runs_in_worker_without_claiming_clean() {
    let result = run_syntax_worker_candidate(
        env!("CARGO_BIN_EXE_codeguard").as_ref(),
        "cobol",
        "src/HELLO.cbl",
        b"       IDENTIFICATION DIVISION.\n       PROGRAM-ID. HELLO.\n",
        Instant::now() + Duration::from_secs(90),
        &AtomicBool::new(false),
    )
    .expect("COBOL candidate");
    assert!(result.recoveries.is_empty());
    assert_eq!(result.precheck.status, SyntaxPrecheckStatus::Incomplete);
    assert!(!result.grammar_qualified);
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

#[cfg(unix)]
#[test]
fn crashed_worker_reaps_its_descendant_and_preserves_the_next_observation() {
    let marker = std::env::temp_dir().join(format!(
        "codeguard-crashed-worker-marker-{}",
        std::process::id()
    ));
    assert!(!marker.exists());
    let worker = fake_worker(&format!(
        "/bin/cat >/dev/null; (sleep 1; printf orphan > '{}') & exit 9",
        marker.display()
    ));
    let crashed = run_syntax_worker_candidate(
        &worker,
        "java",
        "src/A.java",
        b"class A {}",
        deadline(),
        &AtomicBool::new(false),
    );
    fs::remove_file(&worker).unwrap();
    assert!(
        crashed
            .as_ref()
            .is_err_and(|reason| reason.contains("Exited(9)")),
        "异常退出必须保持失败语义：{crashed:?}"
    );
    thread::sleep(Duration::from_millis(1200));
    assert!(!marker.exists(), "worker 退出后不能遗留写入的后代进程");
    assert!(
        run_syntax_worker_candidate(
            env!("CARGO_BIN_EXE_codeguard").as_ref(),
            "java",
            "src/A.java",
            b"class A {}",
            deadline(),
            &AtomicBool::new(false),
        )
        .is_ok()
    );
}

#[cfg(unix)]
#[test]
fn output_flood_and_midflight_cancel_do_not_poison_later_observation() {
    let flood = fake_worker("while :; do printf '0123456789abcdef'; done");
    let limited = run_syntax_worker_candidate(
        &flood,
        "java",
        "src/A.java",
        b"class A {}",
        deadline(),
        &AtomicBool::new(false),
    );
    fs::remove_file(&flood).unwrap();
    assert!(
        limited
            .as_ref()
            .is_err_and(|reason| reason.contains("OutputLimit")),
        "输出洪泛必须保持超限语义：{limited:?}"
    );

    let busy = fake_worker("while :; do :; done");
    let cancelled = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&cancelled);
    let trigger = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        signal.store(true, std::sync::atomic::Ordering::Relaxed);
    });
    let interrupted = run_syntax_worker_candidate(
        &busy,
        "java",
        "src/A.java",
        b"class A {}",
        deadline(),
        &cancelled,
    );
    trigger.join().unwrap();
    fs::remove_file(&busy).unwrap();
    assert!(
        interrupted
            .as_ref()
            .is_err_and(|reason| reason.contains("Cancelled")),
        "执行中取消必须保持取消语义：{interrupted:?}"
    );
    assert!(
        run_syntax_worker_candidate(
            env!("CARGO_BIN_EXE_codeguard").as_ref(),
            "java",
            "src/A.java",
            b"class A {}",
            deadline(),
            &AtomicBool::new(false),
        )
        .is_ok()
    );
}
