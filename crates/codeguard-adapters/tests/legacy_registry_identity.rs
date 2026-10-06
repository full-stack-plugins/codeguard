use codeguard_adapters::parse_legacy_registry;
use serde_json::Value;

fn baseline() -> Value {
    serde_json::from_str(include_str!("../../../rulepacks/legacy_languages.json")).unwrap()
}

#[test]
fn equal_counts_cannot_replace_a_canonical_language() {
    let mut value = baseline();
    value["languages"][0]["id"] = "unregistered_language".into();
    assert!(parse_legacy_registry(&value.to_string()).is_err());
}

#[test]
fn stable_and_planned_identities_cannot_be_swapped() {
    let mut value = baseline();
    let rows = value["languages"].as_array_mut().unwrap();
    rows.iter_mut().find(|r| r["id"] == "java").unwrap()["status"] = "planned".into();
    rows.iter_mut().find(|r| r["id"] == "metal").unwrap()["status"] = "stable".into();
    assert!(parse_legacy_registry(&value.to_string()).is_err());
}

#[test]
fn duplicate_unknown_status_row_cannot_hide_behind_unique_counts() {
    let mut value = baseline();
    let mut extra = value["languages"][0].clone();
    extra["status"] = "unknown".into();
    value["languages"].as_array_mut().unwrap().push(extra);
    assert!(parse_legacy_registry(&value.to_string()).is_err());
}

#[test]
fn duplicate_ignored_metadata_keys_are_rejected_before_projection() {
    let raw = include_str!("../../../rulepacks/legacy_languages.json")
        .replace("\"version\": 1,", "\"version\": 1, \"version\": 2,");
    assert!(parse_legacy_registry(&raw).is_err());
    let nested = include_str!("../../../rulepacks/legacy_languages.json").replace(
        "\"append_files\": false,",
        "\"append_files\": false, \"append_files\": true,",
    );
    assert!(parse_legacy_registry(&nested).is_err());
}

#[test]
fn latest_plugin_snapshot_and_rust_registry_preserve_all_57_identities() {
    let latest = include_str!("../../../tests/fixtures/legacy_languages_plugin_2026_10_06.json");
    let before = parse_legacy_registry(latest).unwrap();
    let after =
        parse_legacy_registry(include_str!("../../../rulepacks/legacy_languages.json")).unwrap();
    let ids =
        |r: codeguard_adapters::LegacyRegistry| -> std::collections::BTreeMap<String, String> {
            r.languages
                .into_iter()
                .map(|row| (row.id, row.status))
                .collect()
        };
    assert_eq!(ids(before), ids(after));
}

#[test]
fn an_unknown_registry_version_cannot_reuse_the_v1_baseline() {
    let mut value = baseline();
    value["version"] = 99.into();
    assert!(parse_legacy_registry(&value.to_string()).is_err());
}
