use codeguard_adapters::{
    bundled_go_candidate_profile, capability_row, legacy_registry, parse_go_candidate_profile,
};
use codeguard_core::{CANDIDATE_PLATFORMS, CHECK_CATEGORIES};

#[test]
fn every_go_category_has_an_applicable_but_unvalidated_native_candidate() {
    let profile = bundled_go_candidate_profile().expect("候选档案必须可解析");
    assert_eq!(profile.language, "go");
    assert_eq!(profile.platforms, CANDIDATE_PLATFORMS);
    assert_eq!(profile.categories.len(), CHECK_CATEGORIES.len());
    for category in CHECK_CATEGORIES {
        let slot = profile
            .categories
            .iter()
            .find(|slot| slot.category == category)
            .expect("六类别不得缺项");
        assert_eq!(slot.applicability, "applicable");
        assert_eq!(slot.status, "gap");
        assert!(!slot.candidate_argv.is_empty());
        assert!(!slot.coverage_gaps.is_empty());
        assert!(!slot.sources.is_empty());
    }
    let comments = profile
        .categories
        .iter()
        .find(|slot| slot.category == "comments")
        .unwrap();
    assert_ne!(comments.candidate_argv[0], "gofmt");
    let registry = legacy_registry().unwrap();
    let go = registry
        .languages
        .iter()
        .find(|item| item.id == "go")
        .unwrap();
    let row = capability_row(go);
    assert_eq!(row.legacy_status, "stable");
    for platform in row.platforms.values() {
        assert!(platform.values().all(|cell| cell.status == "gap"));
    }
}

#[test]
fn missing_duplicate_or_falsely_implemented_slots_are_rejected() {
    let baseline: serde_json::Value = serde_json::from_str(include_str!(
        "../../../rulepacks/go_static_candidate_v1.json"
    ))
    .unwrap();
    let mut missing = baseline.clone();
    missing["categories"].as_array_mut().unwrap().pop();
    assert!(parse_go_candidate_profile(&missing.to_string()).is_err());

    let mut duplicate = baseline.clone();
    duplicate["categories"][1]["category"] = "lint".into();
    assert!(parse_go_candidate_profile(&duplicate.to_string()).is_err());

    let mut implemented = baseline.clone();
    implemented["categories"][0]["status"] = "implemented".into();
    assert!(parse_go_candidate_profile(&implemented.to_string()).is_err());

    let mut not_applicable = baseline;
    not_applicable["categories"][0]["applicability"] = "not_applicable".into();
    assert!(parse_go_candidate_profile(&not_applicable.to_string()).is_err());
}

#[test]
fn candidate_cannot_replace_comment_rules_with_formatter_or_claim_unverified_cve_freshness() {
    let baseline: serde_json::Value = serde_json::from_str(include_str!(
        "../../../rulepacks/go_static_candidate_v1.json"
    ))
    .unwrap();
    let mut formatter = baseline.clone();
    formatter["categories"][1]["candidate_argv"] = serde_json::json!(["gofmt", "-l", "."]);
    assert!(parse_go_candidate_profile(&formatter.to_string()).is_err());

    let mut cve_without_db_gap = baseline.clone();
    cve_without_db_gap["categories"][3]["requires_database_freshness"] = false.into();
    assert!(parse_go_candidate_profile(&cve_without_db_gap.to_string()).is_err());

    let mut floating_version = baseline;
    floating_version["categories"][4]["candidate_version"] = "latest".into();
    assert!(parse_go_candidate_profile(&floating_version.to_string()).is_err());

    let baseline: serde_json::Value = serde_json::from_str(include_str!(
        "../../../rulepacks/go_static_candidate_v1.json"
    ))
    .unwrap();
    let mut wrong_tool = baseline;
    wrong_tool["categories"][3]["candidate_argv"] = serde_json::json!(["go", "vet", "./..."]);
    assert!(parse_go_candidate_profile(&wrong_tool.to_string()).is_err());
}
