use codeguard_adapters::{PmdParseState, parse_pmd_xml};

const REPORT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<pmd xmlns="http://pmd.sourceforge.net/report/2.0.0" version="6.15.0" timestamp="2026-09-24T00:00:00.000">
  <file name="/fixture/src/main/java/Bad.java">
    <violation beginline="4" endline="4" begincolumn="7" endcolumn="10" rule="FixtureRule" ruleset="Fixture Rules" priority="2">Fixture diagnostic</violation>
  </file>
</pmd>"#;

#[test]
fn native_violation_preserves_rule_path_position_and_priority() {
    let parsed = parse_pmd_xml(REPORT.as_bytes(), "6.15.0");
    assert_eq!(parsed.state, PmdParseState::ValidReport);
    assert_eq!(parsed.diagnostics.len(), 1);
    assert_eq!(parsed.diagnostics[0].rule, "FixtureRule");
    assert_eq!(
        parsed.diagnostics[0].filename,
        "/fixture/src/main/java/Bad.java"
    );
    assert_eq!(parsed.diagnostics[0].beginline, 4);
    assert_eq!(parsed.diagnostics[0].priority, 2);
}

#[test]
fn empty_report_is_only_structurally_valid_and_does_not_prove_rules_loaded() {
    let empty = REPORT.replace("    <violation beginline=\"4\" endline=\"4\" begincolumn=\"7\" endcolumn=\"10\" rule=\"FixtureRule\" ruleset=\"Fixture Rules\" priority=\"2\">Fixture diagnostic</violation>\n", "");
    let parsed = parse_pmd_xml(empty.as_bytes(), "6.15.0");
    assert_eq!(parsed.state, PmdParseState::ValidReport);
    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn processing_error_preserves_real_violation_but_blocks_completion() {
    let mixed = REPORT.replace(
        "  </file>",
        "  </file>\n  <error filename=\"/fixture/Broken.java\" msg=\"Parse failed\">stack trace</error>",
    );
    let parsed = parse_pmd_xml(mixed.as_bytes(), "6.15.0");
    assert_eq!(parsed.state, PmdParseState::Incomplete);
    assert_eq!(parsed.reason, Some("native_processing_error"));
    assert_eq!(parsed.diagnostics.len(), 1);
}

#[test]
fn configuration_error_with_no_violations_is_not_a_clean_scan() {
    let report = REPORT.replace(
        "  <file name=\"/fixture/src/main/java/Bad.java\">\n    <violation beginline=\"4\" endline=\"4\" begincolumn=\"7\" endcolumn=\"10\" rule=\"FixtureRule\" ruleset=\"Fixture Rules\" priority=\"2\">Fixture diagnostic</violation>\n  </file>",
        "  <configerror rule=\"FixtureRule\" msg=\"Rule could not load\"/>",
    );
    let parsed = parse_pmd_xml(report.as_bytes(), "6.15.0");
    assert_eq!(parsed.state, PmdParseState::Incomplete);
    assert_eq!(parsed.reason, Some("native_processing_error"));
    assert_eq!(parsed.processing_errors, 1);
    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn reported_suppression_cannot_silently_erase_a_required_finding() {
    let suppressed = REPORT.replace(
        "</pmd>",
        "  <suppressedviolation filename=\"/fixture/Hidden.java\" suppressiontype=\"nopmd\" msg=\"Fixture diagnostic\"/>\n</pmd>",
    );
    let parsed = parse_pmd_xml(suppressed.as_bytes(), "6.15.0");
    assert_eq!(parsed.state, PmdParseState::Incomplete);
    assert_eq!(
        parsed.reason,
        Some("native_suppression_requires_policy_review")
    );
    assert_eq!(parsed.suppressed_count, 1);
    assert_eq!(parsed.diagnostics.len(), 1);
}

#[test]
fn invalid_sibling_keeps_valid_native_finding_but_blocks_completion() {
    let mixed = REPORT.replace(
        "  </file>",
        "    <violation beginline=\"0\" endline=\"1\" begincolumn=\"1\" endcolumn=\"2\" rule=\"Bad\" ruleset=\"Fixture Rules\" priority=\"2\">bad</violation>\n  </file>",
    );
    let parsed = parse_pmd_xml(mixed.as_bytes(), "6.15.0");
    assert_eq!(parsed.state, PmdParseState::Incomplete);
    assert_eq!(parsed.reason, Some("invalid_report_entry"));
    assert_eq!(parsed.diagnostics.len(), 1);
}

#[test]
fn wrong_version_bad_xml_and_dtd_are_not_empty_success() {
    for report in [
        REPORT.replace("version=\"6.15.0\"", "version=\"7.0.0\""),
        REPORT.replace("</pmd>", ""),
        REPORT.replace(
            "<pmd ",
            "<!DOCTYPE pmd [<!ENTITY x SYSTEM 'file:///etc/passwd'>]><pmd ",
        ),
        REPORT.replace("beginline=\"4\"", "beginline=\"0\""),
        REPORT.replace(
            "xmlns=\"http://pmd.sourceforge.net/report/2.0.0\"",
            "xmlns=\"urn:other\"",
        ),
    ] {
        let parsed = parse_pmd_xml(report.as_bytes(), "6.15.0");
        assert_eq!(parsed.state, PmdParseState::Incomplete);
    }
}
