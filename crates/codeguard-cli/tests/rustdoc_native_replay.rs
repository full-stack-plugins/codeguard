#![cfg(unix)]

use codeguard_adapters::parse_cargo_rustdoc_json;
use codeguard_runtime::{ProcessSpec, Termination, run_process};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

struct Fixture(PathBuf);

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "requires explicit existing CODEGUARD_CARGO_BIN; no tool installation"]
fn actual_rustdoc_comments_links_compilation_failure_and_suppression_are_distinct() {
    let cargo = PathBuf::from(std::env::var_os("CODEGUARD_CARGO_BIN").expect("显式Cargo路径"));
    assert!(cargo.is_absolute());
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-rustdoc-replay-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fixture = Fixture(root);
    fs::create_dir(fixture.0.join("src")).unwrap();
    fs::write(
        fixture.0.join("Cargo.toml"),
        "[package]\nname='cg-rustdoc-replay'\nversion='0.1.0'\nedition='2021'\n",
    )
    .unwrap();
    let cases = [
        (
            "pub fn answer() -> i32 { 42 }\n",
            Some("missing_docs"),
            false,
        ),
        (
            "//! Sample crate.\n/// Computes an answer.\npub fn answer() -> i32 { 42 }\n",
            None,
            false,
        ),
        (
            "//! Sample crate.\n/// See [NoSuchType].\npub fn answer() -> i32 { 42 }\n",
            Some("rustdoc::broken_intra_doc_links"),
            false,
        ),
        (
            "//! Sample crate.\n/// Computes an answer.\npub fn answer() -> NoSuchType { 42 }\n",
            None,
            true,
        ),
        (
            "#![allow(missing_docs)]\npub fn answer() -> i32 { 42 }\n",
            None,
            false,
        ),
    ];
    for (index, (source, expected_rule, compiler_failure)) in cases.into_iter().enumerate() {
        fs::write(fixture.0.join("src/lib.rs"), source).unwrap();
        let mut environment = BTreeMap::new();
        for name in ["PATH", "HOME", "CARGO_HOME", "RUSTUP_HOME"] {
            if let Some(value) = std::env::var_os(name) {
                environment.insert(OsString::from(name), value);
            }
        }
        environment.insert(
            OsString::from("CARGO_TARGET_DIR"),
            fixture.0.join(format!("target-{index}")).into_os_string(),
        );
        environment.insert(OsString::from("CARGO_NET_OFFLINE"), OsString::from("true"));
        let result = run_process(
            &ProcessSpec {
                executable: cargo.clone(),
                args: [
                    "rustdoc",
                    "--lib",
                    "--offline",
                    "--message-format=json",
                    "--",
                    "--warn",
                    "missing_docs",
                    "--warn",
                    "rustdoc::broken_intra_doc_links",
                ]
                .into_iter()
                .map(OsString::from)
                .collect(),
                cwd: fixture.0.clone(),
                env: environment,
                stdin: None,
                deadline: Instant::now() + Duration::from_secs(30),
                output_limit_bytes: 4 * 1024 * 1024,
            },
            &AtomicBool::new(false),
        );
        let parsed = parse_cargo_rustdoc_json(&result.stdout);
        if compiler_failure {
            assert!(matches!(result.termination,Termination::Exited(code) if code != 0));
            assert_eq!(parsed.issue, Some("non_rustdoc_compilation_error"));
            assert!(parsed.findings.is_empty());
        } else {
            assert_eq!(result.termination, Termination::Exited(0));
            assert_eq!(parsed.issue, None, "case {index}");
            assert!(parsed.build_finished);
            if let Some(rule) = expected_rule {
                assert!(!parsed.findings.is_empty());
                for finding in parsed.findings {
                    assert_eq!(finding.rule_id, rule);
                    assert_eq!(finding.path, "src/lib.rs");
                    assert_eq!(
                        finding.manifest_path,
                        fixture.0.join("Cargo.toml").to_str().unwrap()
                    );
                    assert_eq!(
                        finding.target_source,
                        fixture.0.join("src/lib.rs").to_str().unwrap()
                    );
                    assert_eq!(finding.target_kinds, vec!["lib"]);
                }
            } else {
                assert!(parsed.findings.is_empty());
            }
        }
        assert_eq!(
            fs::read_to_string(fixture.0.join("src/lib.rs")).unwrap(),
            source
        );
    }
}
