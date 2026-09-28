#![cfg(unix)]

use codeguard_cli::checkstyle_probe::run_checkstyle_probe;
use codeguard_cli::checkstyle_probe_request::CheckstyleProbeRequest;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture(tamper: bool) -> (Fixture, CheckstyleProbeRequest) {
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "cg-checkstyle-probe-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let source = root.join("Foo.java");
    let config = root.join("checks.xml");
    let java = root.join("java");
    let jar = root.join("checks.jar");
    fs::write(&source, "public class Foo {}\n").unwrap();
    fs::write(&config, "<module name=\"Checker\"/>\n").unwrap();
    fs::write(&jar, b"fixture jar").unwrap();
    let mutation = if tamper {
        format!("printf changed > '{}'\n", config.display())
    } else {
        String::new()
    };
    fs::write(&java,format!("#!/bin/sh\nwhile [ \"$#\" -gt 0 ]; do if [ \"$1\" = -o ]; then shift; report=$1; fi; shift; done\n{mutation}printf '%s' '<checkstyle version=\"10.21.4\"><file name=\"{}\"><error line=\"1\" severity=\"error\" message=\"missing doc\" source=\"publicType\"/></file></checkstyle>' > \"$report\"\nexit 1\n",source.display())).unwrap();
    fs::set_permissions(&java, fs::Permissions::from_mode(0o700)).unwrap();
    let expected_sha256 = BTreeMap::from_iter(
        [&source, &config, &java, &jar]
            .into_iter()
            .map(|p| (p.clone(), Sha256::digest(fs::read(p).unwrap()).into())),
    );
    let req = CheckstyleProbeRequest {
        source,
        config,
        java,
        jar,
        expected_sha256,
        report_dir: root.clone(),
        evidence_dir: root.clone(),
        run_id: "run".into(),
        expected_version: "10.21.4".into(),
        deadline: Instant::now() + Duration::from_secs(5),
    };
    (Fixture(root), req)
}

#[test]
fn controlled_report_keeps_diagnostic_without_project_or_runtime_authority() {
    let (_fixture, req) = fixture(false);
    let result = run_checkstyle_probe(&req, &AtomicBool::new(false));
    assert!(result.local_coherent, "{:?}", result.reason);
    assert_eq!(result.parsed.unwrap().diagnostics.len(), 1);
}

#[test]
fn input_mutation_keeps_evidence_but_prevents_coherence() {
    let (_fixture, req) = fixture(true);
    let result = run_checkstyle_probe(&req, &AtomicBool::new(false));
    assert!(!result.local_coherent);
    assert_eq!(result.reason, Some("checkstyle_input_changed"));
    assert_eq!(result.parsed.unwrap().diagnostics.len(), 1);
}

#[test]
fn stale_report_bad_identity_cancel_and_deadline_never_return_success() {
    for mode in 0..4 {
        let (fixture, mut req) = fixture(false);
        let cancelled = AtomicBool::new(mode == 2);
        match mode {
            0 => fs::write(fixture.0.join("run-checkstyle.xml"), b"old").unwrap(),
            1 => {
                req.expected_sha256.insert(req.jar.clone(), [1; 32]);
            }
            3 => req.deadline = Instant::now() - Duration::from_millis(1),
            _ => {}
        }
        let result = run_checkstyle_probe(&req, &cancelled);
        assert!(!result.local_coherent);
        assert!(result.parsed.is_none());
    }
}

#[test]
fn incomplete_extra_or_aliased_identity_map_never_starts_java() {
    for mode in 0..3 {
        let (fixture, mut req) = fixture(false);
        match mode {
            0 => {
                req.expected_sha256.remove(&req.config);
            }
            1 => {
                req.expected_sha256.insert(fixture.0.join("extra"), [1; 32]);
            }
            _ => {
                req.java = req.source.clone();
            }
        }
        let result = run_checkstyle_probe(&req, &AtomicBool::new(false));
        assert!(!result.local_coherent);
        assert_eq!(result.reason, Some("checkstyle_input_identity_missing"));
        assert!(!fixture.0.join("run-checkstyle.xml").exists());
    }
}

#[test]
#[ignore = "requires explicit Java and Checkstyle 10.21.4 all.jar"]
fn real_checkstyle_probe_binds_inputs_and_rechecks_documented_source() {
    let (_fixture, mut req) = fixture(false);
    req.java = PathBuf::from(std::env::var_os("CODEGUARD_JAVA_BIN").unwrap())
        .canonicalize()
        .unwrap();
    req.jar = PathBuf::from(std::env::var_os("CODEGUARD_CHECKSTYLE_JAR").unwrap())
        .canonicalize()
        .unwrap();
    fs::write(&req.config,"<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\"><module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"MissingJavadocType\"><property name=\"id\" value=\"publicType\"/></module></module></module>").unwrap();
    for (index, source) in [
        "public class Foo {}\n",
        "/** Documented API. */\npublic class Foo {}\n",
    ]
    .into_iter()
    .enumerate()
    {
        fs::write(&req.source, source).unwrap();
        req.run_id = format!("native-{index}");
        req.expected_sha256 = BTreeMap::from_iter(
            [&req.source, &req.config, &req.java, &req.jar]
                .into_iter()
                .map(|p| (p.clone(), Sha256::digest(fs::read(p).unwrap()).into())),
        );
        req.deadline = Instant::now() + Duration::from_secs(10);
        let result = run_checkstyle_probe(&req, &AtomicBool::new(false));
        assert!(result.local_coherent, "{:?}", result.reason);
        let parsed = result.parsed.unwrap();
        if index == 0 {
            assert_eq!(parsed.diagnostics.len(), 1);
            assert_eq!(parsed.diagnostics[0].source, "publicType");
        } else {
            assert!(parsed.diagnostics.is_empty());
        }
    }
}

#[test]
fn native_regex_diagnostic_cannot_override_coherent_reports_or_changed_inputs() {
    let stderr = "Caused by: com.puppycrawl.tools.checkstyle.api.CheckstyleException: illegal value '[' for property 'authorFormat'\nCaused by: java.util.regex.PatternSyntaxException: Unclosed character class\n";
    for mode in 0..6 {
        let (fixture, mut req) = fixture(false);
        let report = if matches!(mode, 0 | 5) {
            String::new()
        } else if mode == 3 {
            format!(
                "printf '%s' '<checkstyle version=\"10.21.4\"><file name=\"{}\"><error line=\"1\" severity=\"error\" message=\"missing doc\" source=\"publicType\"/></file></checkstyle>' > \"$report\"\n",
                req.source.display()
            )
        } else {
            ": > \"$report\"\n".to_owned()
        };
        let mutation = if matches!(mode, 4 | 5) {
            format!("printf changed > '{}'\n", req.config.display())
        } else {
            String::new()
        };
        let exit = if matches!(mode, 2 | 3) { 1 } else { 254 };
        fs::write(&req.java, format!("#!/bin/sh\nwhile [ \"$#\" -gt 0 ]; do if [ \"$1\" = -o ]; then shift; report=$1; fi; shift; done\n{report}{mutation}cat >&2 <<'CG_REGEX_FAILURE'\n{stderr}CG_REGEX_FAILURE\nexit {exit}\n")).unwrap();
        req.expected_sha256.insert(
            req.java.clone(),
            Sha256::digest(fs::read(&req.java).unwrap()).into(),
        );
        let result = run_checkstyle_probe(&req, &AtomicBool::new(false));
        match mode {
            0 | 1 => {
                assert!(!result.local_coherent);
                assert_eq!(
                    result.reason,
                    Some("checkstyle_configuration_regex_invalid")
                );
            }
            2 => {
                assert!(!result.local_coherent);
                assert_ne!(
                    result.reason,
                    Some("checkstyle_configuration_regex_invalid")
                );
            }
            3 => {
                assert!(result.local_coherent, "{:?}", result.reason);
                assert_eq!(result.reason, None);
            }
            4 | 5 => {
                assert!(!result.local_coherent);
                assert_eq!(result.reason, Some("checkstyle_input_changed"));
            }
            _ => unreachable!(),
        }
        drop(fixture);
    }
}
