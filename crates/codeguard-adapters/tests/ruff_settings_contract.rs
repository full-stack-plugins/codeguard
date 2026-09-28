use codeguard_adapters::{is_ruff_pydocstyle_rule, parse_ruff_settings};

#[test]
fn only_exact_pydocstyle_rule_codes_are_classified_as_comments() {
    for code in ["D100", "D101", "D417"] {
        assert!(is_ruff_pydocstyle_rule(code));
    }
    for code in ["D", "D10", "D1000", "D1A1", "DOC201", "F401"] {
        assert!(!is_ruff_pydocstyle_rule(code));
    }
}

#[test]
fn native_settings_distinguish_enabled_rules_and_possible_suppression() {
    let raw = b"# Linter Settings\nlinter.rules.enabled = [\n\tunused-import (F401),\n]\nlinter.per_file_ignores = {}\n";
    let observed = parse_ruff_settings(raw).unwrap();
    assert_eq!(observed.globally_enabled_mapped_rules, ["F401"]);
    assert!(!observed.per_file_ignores_present);
    assert!(!observed.coverage_proven);
    assert_eq!(observed.settings_sha256.len(), 64);
    assert!(observed.native_rule_enabled("F401"));
    assert!(!observed.native_rule_enabled("D100"));

    let raw = b"linter.rules.enabled = [\n\tline-too-long (E501),\n\tunused-import (F401),\n]\nlinter.per_file_ignores = {\n\tdata = [\n\tunused-import (F401),\n]\n}\n";
    let observed = parse_ruff_settings(raw).unwrap();
    assert_eq!(observed.globally_enabled_mapped_rules, ["E501", "F401"]);
    assert!(observed.per_file_ignores_present);
    assert!(!observed.coverage_proven);
}

#[test]
fn d100_can_be_validated_without_claiming_an_approved_rule_mapping() {
    let raw = b"linter.rules.enabled = [\n\tundocumented-public-module (D100),\n]\nlinter.per_file_ignores = {}\n";
    let observed = parse_ruff_settings(raw).unwrap();
    assert!(observed.native_rule_enabled("D100"));
    assert!(observed.globally_enabled_mapped_rules.is_empty());
    assert!(
        serde_json::to_value(&observed)
            .unwrap()
            .get("enabled_native_rules")
            .is_none()
    );
}

#[test]
fn malformed_or_missing_settings_never_look_like_clean_coverage() {
    for raw in [
        &b"[]"[..],
        &b"linter.rules.enabled = [\n\tbad\n]\nlinter.per_file_ignores = {}\n"[..],
        &b"linter.rules.enabled = [\n\tunused-import (F401),\n"[..],
        &b"linter.rules.enabled = [\n]\n"[..],
        &b"linter.rules.enabled = [\n]\nlinter.per_file_ignores = unknown\n"[..],
        &b"linter.rules.enabled = [\n]\nlinter.per_file_ignores = {\n"[..],
    ] {
        assert!(parse_ruff_settings(raw).is_err(), "{raw:?}");
    }
}
