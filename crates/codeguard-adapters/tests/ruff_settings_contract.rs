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

#[test]
fn explicit_native_python_target_is_observed_without_changing_public_settings_shape() {
    for (version, expected) in [("3.9", "py39"), ("3.10", "py310"), ("3.15", "py315")] {
        let raw = format!(
            "linter.rules.enabled = [\n]\nlinter.per_file_ignores = {{}}\nlinter.unresolved_target_version = {version}\nlinter.per_file_target_version = {{}}\n"
        );
        let observed = parse_ruff_settings(raw.as_bytes()).unwrap();
        assert_eq!(observed.explicit_python_target(), Ok(expected));
        let value = serde_json::to_value(observed).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 4);
        assert!(value.get("explicit_python_target").is_none());
    }
}

#[test]
fn unknown_implicit_duplicate_and_per_file_targets_do_not_supply_a_closure_target() {
    let base = "linter.rules.enabled = [\n]\nlinter.per_file_ignores = {}\n";
    for (suffix, reason) in [
        ("", "ruff_target_settings_missing"),
        (
            "formatter.unresolved_target_version = 3.12\nanalyze.target_version = 3.12\n",
            "ruff_target_settings_missing",
        ),
        (
            "linter.unresolved_target_version = 3.10\nlinter.per_file_target_version = {}\nlinter.per_file_target_version = {}\n",
            "ruff_target_settings_duplicate",
        ),
        (
            "linter.unresolved_target_version = none\nlinter.per_file_target_version = {}\n",
            "ruff_target_not_explicit",
        ),
        (
            "linter.unresolved_target_version = 3.16\nlinter.per_file_target_version = {}\n",
            "ruff_target_settings_unvalidated",
        ),
        (
            "linter.unresolved_target_version = 3.9\nlinter.unresolved_target_version = 3.10\nlinter.per_file_target_version = {}\n",
            "ruff_target_settings_duplicate",
        ),
        (
            "linter.unresolved_target_version = 3.10\nlinter.per_file_target_version = {\n}\n",
            "ruff_per_file_target_unresolved",
        ),
        (
            "linter.unresolved_target_version = 3.10\n",
            "ruff_target_settings_missing",
        ),
    ] {
        let observed = parse_ruff_settings(format!("{base}{suffix}").as_bytes()).unwrap();
        assert_eq!(observed.explicit_python_target(), Err(reason), "{suffix}");
        assert!(!observed.coverage_proven);
    }
}

#[test]
fn pydoclint_exact_catalog_does_not_expand_pydocstyle_or_approved_mappings() {
    for code in [
        "DOC102", "DOC201", "DOC202", "DOC402", "DOC403", "DOC501", "DOC502",
    ] {
        assert!(codeguard_adapters::is_ruff_documentation_rule(code));
        assert!(!is_ruff_pydocstyle_rule(code));
        let rule = codeguard_adapters::RuffDocumentationRule::from_code(code).unwrap();
        assert_eq!(rule.investigation_required, code == "DOC502");
        assert!(rule.step.contains("实际"));
        let raw = format!(
            "linter.rules.enabled = [\n\tdocumentation ({code}),\n]\nlinter.per_file_ignores = {{}}\n"
        );
        let settings = parse_ruff_settings(raw.as_bytes()).unwrap();
        assert!(settings.native_rule_enabled(code));
        assert!(settings.globally_enabled_mapped_rules.is_empty());
    }
    for code in [
        "DOC", "DOC20", "DOC2010", "DOC101", "DOC999", "doc201", "DOC2A1", "F401",
    ] {
        assert!(!codeguard_adapters::is_ruff_documentation_rule(code));
        assert!(codeguard_adapters::RuffDocumentationRule::from_code(code).is_none());
    }
    assert!(codeguard_adapters::is_ruff_documentation_rule("D417"));
}

#[test]
fn documentation_selection_is_exact_sorted_and_does_not_change_serialized_settings() {
    let raw = b"linter.rules.enabled = [\n\tx (DOC201),\n\tx (F401),\n\tx (D101),\n\tx (DOC999),\n\tx (D1000),\n]\nlinter.per_file_ignores = {}\n";
    let settings = parse_ruff_settings(raw).unwrap();
    assert_eq!(
        settings.globally_enabled_documentation_rules(),
        ["D101", "DOC201"]
    );
    let old_shape = serde_json::to_value(&settings).unwrap();
    assert_eq!(old_shape.as_object().unwrap().len(), 4);
    assert_eq!(
        old_shape["globally_enabled_mapped_rules"],
        serde_json::json!(["F401"])
    );
    assert!(!settings.coverage_proven);
}
