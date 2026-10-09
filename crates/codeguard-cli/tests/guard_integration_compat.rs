//! Command-aware real native captures. No fabricated native report or cross-family exit inference.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "cg-native-parity-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn base() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/guard-integration")
}
fn matrix() -> Value {
    serde_json::from_slice(&fs::read(base().join("native-matrix.json")).unwrap()).unwrap()
}
fn captures(m: &Value) -> &[Value] {
    m["captures"]
        .as_array()
        .expect("actual command captures must replace source-only pending matrix")
}
fn raw(c: &Value, stream: &str) -> Vec<u8> {
    fs::read(base().join(c[stream]["path"].as_str().unwrap())).unwrap()
}
fn capture<'a>(m: &'a Value, name: &str) -> &'a Value {
    captures(m).iter().find(|c| c["id"] == name).unwrap()
}
#[test]
fn real_streams_and_source_side_effects_are_digest_bound() {
    let m = matrix();
    assert_eq!(m["status"], "actual-command-captures");
    assert_eq!(captures(&m).len(), 23);
    for c in captures(&m) {
        assert_eq!(c["evidence_kind"], "actual-command");
        assert!(c["argv"].as_array().unwrap().len() > 1);
        for stream in ["stdout", "stderr"] {
            assert_eq!(
                format!("{:x}", Sha256::digest(raw(c, stream))),
                c[stream]["sha256"]
            );
        }
        let before = c["before"].as_object().unwrap();
        let after = c["after"].as_object().unwrap();
        for key in before.keys().chain(after.keys()) {
            if !key.starts_with(".ruff_cache/") {
                assert_eq!(
                    before.get(key),
                    after.get(key),
                    "{} changed source {}",
                    c["id"],
                    key
                );
            }
        }
    }
}
#[test]
fn historical_mixed_priorities_come_from_actual_tools_not_numeric_aliases() {
    use codeguard_cli::legacy_v1_protocol::{
        LegacyV1Entry as E, LegacyV1Signal as S, project_legacy_v1,
    };
    let m = matrix();
    let c = capture(&m, "legacy-check-mixed");
    let text = String::from_utf8(raw(c, "stdout")).unwrap();
    assert!(text.contains("UNVERIFIED") && text.contains("FAILED"));
    assert_eq!(c["exit"], 2);
    for (id, entry, exit) in [
        ("legacy-cve-mixed", E::Cve, 2),
        ("legacy-docker-mixed", E::Dockerfile, 1),
    ] {
        let c = capture(&m, id);
        let v: Value = serde_json::from_slice(&raw(c, "stdout")).unwrap();
        assert_eq!(c["exit"], exit);
        if entry == E::Cve {
            let r = v["results"].as_array().unwrap();
            assert!(r.iter().any(|x| x["status"] == "FAIL"));
            assert!(r.iter().any(|x| x["status"] == "UNVERIFIED"));
        } else {
            assert_eq!(v["hadolint"]["status"], "FAIL");
            assert_eq!(v["trivy"]["status"], "UNVERIFIED");
            assert!(!v["hadolint"]["records"].as_array().unwrap().is_empty());
        }
        let p = project_legacy_v1(entry, &[S::Fail, S::Unverified]).unwrap();
        assert_eq!(p.legacy_exit_code, exit);
        assert_eq!(p.new_delivery_decision, "not_evaluated");
    }
    assert_eq!(
        project_legacy_v1(E::Check, &[S::Fail, S::Unverified])
            .unwrap()
            .legacy_exit_code,
        2
    );
}
#[test]
fn native_exit_families_and_exact_explained_differences_stay_separate() {
    let m = matrix();
    for label in ["old", "new"] {
        for (case, exit) in [
            ("format-clean", 0),
            ("format-bad", 1),
            ("query", 0),
            ("usage", 2),
            ("lint-clean", 3),
            ("lint-bad", 3),
            ("lint-missing", 3),
            ("lint-error", 3),
            ("io-error", 4),
            ("cancel", 130),
        ] {
            let c = capture(&m, &format!("{label}-{case}"));
            assert_eq!(c["exit"], exit);
            if case == "usage" || case == "io-error" {
                assert!(raw(c, "stdout").is_empty());
                assert!(!raw(c, "stderr").is_empty());
                continue;
            }
            let v: Value = serde_json::from_slice(&raw(c, "stdout")).unwrap();
            match case {
                "format-clean" | "format-bad" => {
                    assert_eq!(v["report_type"], "format_feedback");
                    assert_eq!(v["authority"], "local_unverified");
                    assert_eq!(v["mutates_sources"], false);
                    assert_eq!(
                        v["delivery_decision"],
                        if exit == 0 { "allow" } else { "deny" }
                    );
                }
                "query" => {
                    assert_eq!(v["report_type"], "version");
                    assert!(v.get("delivery_decision").is_none());
                }
                _ => {
                    assert_eq!(v["report_type"], "python_lint_feedback");
                    assert_eq!(v["exit_code"], exit);
                    assert_eq!(v["delivery_decision"], "not_evaluated");
                    if exit == 130 {
                        assert_eq!(v["command_status"], "cancelled");
                    }
                }
            }
        }
    }
    for case in m["comparisons"].as_array().unwrap() {
        let old = capture(&m, case["old"].as_str().unwrap());
        let new = capture(&m, case["new"].as_str().unwrap());
        assert_eq!(old["exit"], new["exit"]);
        assert_eq!(raw(old, "stderr"), raw(new, "stderr"));
        if case["json"] == false {
            assert_eq!(raw(old, "stdout"), raw(new, "stdout"));
            continue;
        }
        let mut a: Value = serde_json::from_slice(&raw(old, "stdout")).unwrap();
        let mut b: Value = serde_json::from_slice(&raw(new, "stdout")).unwrap();
        for key in case["variable_fields"].as_array().unwrap() {
            let key = key.as_str().unwrap();
            assert!(matches!(key, "run_id" | "elapsed_ms" | "adapter_sha256"));
            assert_ne!(a[key], b[key], "do not silently normalize unchanged fields");
            a.as_object_mut().unwrap().remove(key);
            b.as_object_mut().unwrap().remove(key);
        }
        assert_eq!(a, b, "unexplained native change in {}", case["old"]);
    }
}
#[test]
#[ignore = "explicit pinned real Ruff path required; run with GUARD_PARITY_RUFF"]
fn actual_formatter_zero_one_query_usage_and_io_failure_keep_native_contracts() {
    use std::process::Command;
    let ruff = std::env::var("GUARD_PARITY_RUFF").expect("explicit actual Ruff");
    let root = Temp::new();
    let source = root.path().join("app.py");
    let bin = env!("CARGO_BIN_EXE_codeguard");
    for (input, expected) in [("x = 1\n", 0), ("x=1\n", 1)] {
        fs::write(&source, input).unwrap();
        let o = Command::new(bin)
            .args(["format", "check", "python"])
            .arg(root.path())
            .args(["--tool", &format!("ruff={ruff}"), "--format=json"])
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(expected));
        let v: Value = serde_json::from_slice(&o.stdout).unwrap();
        assert_eq!(v["authority"], "local_unverified");
        assert_eq!(fs::read_to_string(&source).unwrap(), input);
    }
    assert!(
        Command::new(bin)
            .args(["--version", "--format=json"])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert_eq!(
        Command::new(bin)
            .args(["lint", "python", "--unsupported-parity-flag"])
            .output()
            .unwrap()
            .status
            .code(),
        Some(2)
    );
    #[cfg(unix)]
    {
        let dir = fs::File::open(root.path()).unwrap();
        let o = Command::new(bin)
            .args(["hook", "execute"])
            .arg(root.path())
            .args(["--format=json", "--timeout", "1s"])
            .stdin(dir)
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(4));
        assert!(o.stdout.is_empty());
        assert!(String::from_utf8_lossy(&o.stderr).contains("读取 hook 请求失败"));
    }
}
