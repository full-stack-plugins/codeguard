use codeguard_adapters::{bundled_ruby_candidate_profile, parse_ruby_candidate_profile};
use codeguard_core::{CANDIDATE_PLATFORMS, CHECK_CATEGORIES};

#[test]
fn ruby_profile_covers_six_categories_without_granting_native_or_release_capabilities() {
    let profile = bundled_ruby_candidate_profile().unwrap();
    assert_eq!(profile.language, "ruby");
    assert_eq!(profile.platforms, CANDIDATE_PLATFORMS);
    assert_eq!(profile.categories.len(), CHECK_CATEGORIES.len());
    for name in CHECK_CATEGORIES {
        let slot = profile
            .categories
            .iter()
            .find(|s| s.category == name)
            .unwrap();
        assert_eq!(slot.status, "gap");
        assert!(!slot.tools.is_empty());
        assert!(!slot.coverage_gaps.is_empty());
        for tool in &slot.tools {
            assert!(!tool.sources.is_empty());
            assert_eq!(tool.version_status, "unvalidated_project_lock_required");
            assert_eq!(tool.candidate_version, "project_locked");
        }
    }
    assert_eq!(profile.runtime_dialects, ["mri", "jruby", "truffleruby"]);
    let registry = codeguard_adapters::legacy_registry().unwrap();
    let ruby = registry
        .languages
        .iter()
        .find(|entry| entry.id == "ruby")
        .unwrap();
    let row = codeguard_adapters::capability_row(ruby);
    assert_eq!(row.legacy_status, "stable");
    assert!(
        row.platforms
            .values()
            .all(|platform| platform.values().all(|cell| cell.status == "gap"))
    );

    assert!(
        profile
            .categories
            .iter()
            .find(|s| s.category == "cve")
            .unwrap()
            .requires_database_freshness
    );
    assert!(
        profile
            .categories
            .iter()
            .find(|s| s.category == "security")
            .unwrap()
            .tools
            .iter()
            .any(|t| t.tool_id == "brakeman" && t.scope == "rails_project")
    );
    assert_eq!(
        profile
            .categories
            .iter()
            .find(|s| s.category == "build")
            .unwrap()
            .applicability,
        "project_dependent"
    );
}

#[test]
fn ruby_profile_rejects_incomplete_categories_false_support_and_unsupported_runtime_claims() {
    let base: serde_json::Value = serde_json::from_str(include_str!(
        "../../../rulepacks/ruby_static_candidate_v1.json"
    ))
    .unwrap();
    for (key, value) in [
        ("schema_version", serde_json::json!("9.0.0")),
        ("language", serde_json::json!("go")),
        ("platform_validation", serde_json::json!("verified")),
        ("runtime_dialects", serde_json::json!(["mri"])),
    ] {
        let mut bad = base.clone();
        bad[key] = value;
        assert!(parse_ruby_candidate_profile(&bad.to_string()).is_err());
    }
    let mut missing = base.clone();
    missing["categories"].as_array_mut().unwrap().pop();
    assert!(parse_ruby_candidate_profile(&missing.to_string()).is_err());
    for (key, value) in [
        ("category", serde_json::json!("lint")),
        ("status", serde_json::json!("implemented")),
        ("applicability", serde_json::json!("not_applicable")),
    ] {
        let mut bad = base.clone();
        bad["categories"][1][key] = value;
        assert!(parse_ruby_candidate_profile(&bad.to_string()).is_err());
    }
    let mut bad = base.clone();
    bad["categories"][3]["requires_database_freshness"] = serde_json::json!(false);
    assert!(parse_ruby_candidate_profile(&bad.to_string()).is_err());
    let duplicate = base.to_string().replacen(
        "\"schema_version\":\"1.0.0\"",
        "\"schema_version\":\"1.0.0\",\"schema_version\":\"1.0.0\"",
        1,
    );
    assert!(parse_ruby_candidate_profile(&duplicate).is_err());
}

#[test]
fn ruby_profile_does_not_accept_formatter_as_documentation_or_gem_build_as_script_build() {
    let base: serde_json::Value = serde_json::from_str(include_str!(
        "../../../rulepacks/ruby_static_candidate_v1.json"
    ))
    .unwrap();
    for (key, value) in [
        ("tool_id", serde_json::json!("ruby_syntax")),
        (
            "candidate_argv",
            serde_json::json!(["ruby", "-c", "app.rb"]),
        ),
        ("candidate_version", serde_json::json!("latest")),
        ("sources", serde_json::json!([])),
        ("scope", serde_json::json!("all_ruby_projects")),
    ] {
        let mut bad = base.clone();
        bad["categories"][1]["tools"][0][key] = value;
        assert!(
            parse_ruby_candidate_profile(&bad.to_string()).is_err(),
            "{key}"
        );
    }
    let mut bad = base.clone();
    bad["categories"][5]["tools"][0]["scope"] = serde_json::json!("all_ruby_projects");
    assert!(parse_ruby_candidate_profile(&bad.to_string()).is_err());
    let mut bad = base;
    bad["categories"][4]["tools"][1]["scope"] = serde_json::json!("all_ruby_projects");
    assert!(parse_ruby_candidate_profile(&bad.to_string()).is_err());
}

#[test]
fn ruby_profile_rejects_unsafe_sources_unknown_fields_and_missing_version_constraints() {
    let base: serde_json::Value = serde_json::from_str(include_str!(
        "../../../rulepacks/ruby_static_candidate_v1.json"
    ))
    .unwrap();
    let mut bad = base.clone();
    bad["target_version_inputs"] = serde_json::json!([".ruby-version"]);
    assert!(parse_ruby_candidate_profile(&bad.to_string()).is_err());
    let mut bad = base.clone();
    bad["approval"] = serde_json::json!(true);
    assert!(parse_ruby_candidate_profile(&bad.to_string()).is_err());
    let mut bad = base.clone();
    bad["categories"][4]["tools"][1]["sources"] =
        serde_json::json!(["https://example.invalid/\n--ignore"]);
    assert!(parse_ruby_candidate_profile(&bad.to_string()).is_err());
    let mut bad = base.clone();
    bad["categories"][3]["tools"][0]["candidate_argv"] =
        serde_json::json!(["bundle-audit", "check", "--update"]);
    assert!(parse_ruby_candidate_profile(&bad.to_string()).is_err());
    let mut bad = base;
    bad["platforms"][1] = bad["platforms"][0].clone();
    assert!(parse_ruby_candidate_profile(&bad.to_string()).is_err());
    assert!(parse_ruby_candidate_profile(&" ".repeat(65537)).is_err());
}
