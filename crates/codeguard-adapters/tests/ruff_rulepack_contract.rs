use codeguard_adapters::{bundled_ruff_rulepack, parse_ruff_rulepack};
use serde_json::{Value, json};

#[test]
fn bundled_mapping_is_versioned_source_attributed_and_only_observed() {
    let pack = bundled_ruff_rulepack().unwrap();
    assert_eq!(pack.id, "python.ruff.lint.preview");
    assert_eq!(pack.version, "0.1.0");
    assert_eq!(pack.status, "candidate_unapproved");
    assert_eq!(pack.license, "MIT");
    assert_eq!(pack.compatible_tool_versions, ["ruff 0.16.8"]);
    assert_eq!(pack.mappings.len(), 2);
    assert!(pack.mapping("F401").is_some());
    assert!(pack.mapping("E501").is_some());
    assert!(pack.mapping("E902").is_none());
    assert!(pack.mapping("F999").is_none());
    assert_eq!(pack.sha256.len(), 64);
    assert!(!pack.approved());
}

#[test]
fn broad_duplicate_or_unattributed_mappings_are_rejected() {
    let raw = include_bytes!("../../../rulepacks/ruff_lint_preview_v1.json");
    let original: Value = serde_json::from_slice(raw).unwrap();
    for variant in [
        {
            let mut value = original.clone();
            value["mappings"][0]["native_rule_id"] = json!("F*");
            value
        },
        {
            let mut value = original.clone();
            value["mappings"][1]["native_rule_id"] = json!("F401");
            value
        },
        {
            let mut value = original.clone();
            value["mappings"][0]["source_ref"] = json!("https://example.invalid/rule");
            value
        },
        {
            let mut value = original.clone();
            value["license"] = json!("unknown");
            value
        },
        {
            let mut value = original.clone();
            value["status"] = json!("approved");
            value
        },
    ] {
        assert!(parse_ruff_rulepack(&serde_json::to_vec(&variant).unwrap()).is_err());
    }
}

#[test]
fn contents_and_tool_compatibility_are_part_of_the_identity() {
    let raw = include_bytes!("../../../rulepacks/ruff_lint_preview_v1.json");
    let first = parse_ruff_rulepack(raw).unwrap();
    let mut changed: Value = serde_json::from_slice(raw).unwrap();
    changed["version"] = json!("0.1.1");
    let second = parse_ruff_rulepack(&serde_json::to_vec(&changed).unwrap()).unwrap();
    assert_ne!(first.sha256, second.sha256);
    assert!(first.supports_tool_version("ruff 0.16.8"));
    assert!(!first.supports_tool_version("ruff 0.16.9"));
}
