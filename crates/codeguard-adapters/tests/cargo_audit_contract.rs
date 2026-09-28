use codeguard_adapters::{bind_cargo_audit_lockfile, parse_cargo_audit_json};

fn report(found: bool) -> Vec<u8> {
    let findings = if found {
        r#"[{"advisory":{"id":"RUSTSEC-2020-0071","package":"time","aliases":["CVE-2020-26235"],"cvss":"CVSS:3.1/AV:L/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:H"},"package":{"name":"time","version":"0.1.40","source":"registry+https://github.com/rust-lang/crates.io-index","checksum":"0000000000000000000000000000000000000000000000000000000000000000"}}]"#
    } else {
        "[]"
    };
    format!(r#"{{"database":{{"advisory-count":1251,"last-commit":null,"last-updated":null}},"lockfile":{{"dependency-count":1}},"settings":{{"target_arch":[],"target_os":[],"severity":null,"ignore":[],"informational_warnings":[]}},"vulnerabilities":{{"found":{found},"count":{},"list":{findings}}},"warnings":{{}}}}"#, usize::from(found)).into_bytes()
}

#[test]
fn native_vulnerability_preserves_resolved_identity_without_granting_database_freshness() {
    let parsed = parse_cargo_audit_json(&report(true), Some(1)).expect("native finding");
    assert_eq!(parsed.dependency_count, 1);
    assert_eq!(parsed.database_commit, None);
    assert_eq!(parsed.findings[0].advisory_id, "RUSTSEC-2020-0071");
    assert_eq!(parsed.findings[0].package_version, "0.1.40");
    assert_eq!(
        parsed.findings[0].package_source,
        "registry+https://github.com/rust-lang/crates.io-index"
    );
    assert_eq!(parsed.findings[0].cve_aliases, ["CVE-2020-26235"]);
    assert_eq!(
        parsed.findings[0].cvss.as_deref(),
        Some("CVSS:3.1/AV:L/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:H")
    );
}

#[test]
fn empty_native_result_is_observation_not_a_fresh_database_proof() {
    let parsed = parse_cargo_audit_json(&report(false), Some(0)).expect("native empty report");
    assert!(parsed.findings.is_empty());
    assert_eq!(parsed.database_updated, None);
}

#[test]
fn native_warnings_do_not_erase_vulnerability_facts() {
    let warning = String::from_utf8(report(true))
        .unwrap()
        .replace("\"warnings\":{}", "\"warnings\":{\"unmaintained\":[]}");
    let parsed = parse_cargo_audit_json(warning.as_bytes(), Some(1)).unwrap();
    assert!(parsed.warnings_present);
    assert_eq!(parsed.findings.len(), 1);
}

#[test]
fn malformed_or_suppressed_reports_cannot_be_consumed() {
    let mut ignored = report(false);
    let needle = br#""ignore":[]"#;
    let start = ignored
        .windows(needle.len())
        .position(|v| v == needle)
        .unwrap();
    ignored.splice(
        start..start + needle.len(),
        br#""ignore":["RUSTSEC-2020-0071"]"#.iter().copied(),
    );
    assert!(parse_cargo_audit_json(&ignored, Some(0)).is_err());
    assert!(parse_cargo_audit_json(&report(true), Some(0)).is_err());
    assert!(parse_cargo_audit_json(&report(false), Some(1)).is_err());
    let duplicate = String::from_utf8(report(false))
        .unwrap()
        .replace("\"count\":0", "\"count\":0,\"count\":0");
    assert!(parse_cargo_audit_json(duplicate.as_bytes(), Some(0)).is_err());
    let native_error = String::from_utf8(report(false))
        .unwrap()
        .replace("\"warnings\":{}", "\"warnings\":{},\"error\":\"fatal\"");
    assert!(parse_cargo_audit_json(native_error.as_bytes(), Some(0)).is_err());
    let wrong_component = String::from_utf8(report(true))
        .unwrap()
        .replace("\"package\":\"time\"", "\"package\":\"other\"");
    assert!(parse_cargo_audit_json(wrong_component.as_bytes(), Some(1)).is_err());
}

#[test]
fn native_advisory_must_bind_the_exact_resolved_lockfile_package() {
    let parsed = parse_cargo_audit_json(&report(true), Some(1)).unwrap();
    let lock = b"version = 3\n[[package]]\nname = 'time'\nversion = '0.1.40'\nsource = 'registry+https://github.com/rust-lang/crates.io-index'\nchecksum = '0000000000000000000000000000000000000000000000000000000000000000'\n";
    assert!(bind_cargo_audit_lockfile(&parsed, lock).is_ok());
    assert!(bind_cargo_audit_lockfile(&parsed, &lock.replace_bytes(b"0.1.40", b"0.1.41")).is_err());
    assert!(
        bind_cargo_audit_lockfile(
            &parsed,
            &lock.replace_bytes(b"crates.io-index", b"other-index")
        )
        .is_err()
    );
}

trait ReplaceBytes {
    fn replace_bytes(&self, from: &[u8], to: &[u8]) -> Vec<u8>;
}
impl ReplaceBytes for [u8] {
    fn replace_bytes(&self, from: &[u8], to: &[u8]) -> Vec<u8> {
        String::from_utf8(self.to_vec())
            .unwrap()
            .replace(
                std::str::from_utf8(from).unwrap(),
                std::str::from_utf8(to).unwrap(),
            )
            .into_bytes()
    }
}
