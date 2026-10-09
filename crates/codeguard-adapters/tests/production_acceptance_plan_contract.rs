use codeguard_adapters::parse_production_acceptance_plan;
use serde_json::Value;

fn plan() -> Value {
    serde_json::from_slice(include_bytes!(
        "../../../rulepacks/production_acceptance_plan_v1.json"
    ))
    .unwrap()
}

#[test]
fn full_mapping_preserves_unqualified_four_core_obligations() {
    let raw = include_bytes!("../../../rulepacks/production_acceptance_plan_v1.json");
    let parsed = parse_production_acceptance_plan(raw).unwrap();
    assert_eq!(parsed["languages"].as_array().unwrap().len(), 57);
    assert_eq!(parsed["qualification"], "not_granted");
}

#[test]
fn missing_dimensions_and_forged_qualification_fail_closed() {
    for mutation in 0..8 {
        let mut doc = plan();
        match mutation {
            0 => {
                doc["languages"].as_array_mut().unwrap().pop();
            }
            1 => {
                doc["platform_targets"].as_array_mut().unwrap().pop();
            }
            2 => {
                doc["languages"][0]["capabilities"]
                    .as_object_mut()
                    .unwrap()
                    .remove("syntax");
            }
            3 => {
                doc["qualification"] = "production_ready".into();
            }
            4 => {
                doc["languages"][0]["capabilities"]["syntax"]["qualification"] = "passed".into();
            }
            5 => {
                let row = doc["languages"][0].clone();
                doc["languages"][1] = row;
            }
            6 => {
                doc["source_hashes"]
                    .as_object_mut()
                    .unwrap()
                    .insert("../outside".into(), "a".repeat(64).into());
            }
            _ => {
                doc["languages"][0]["capabilities"]["syntax"]["build_paths"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
        }
        assert!(
            parse_production_acceptance_plan(&serde_json::to_vec(&doc).unwrap()).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn duplicate_keys_and_unknown_fields_are_rejected() {
    let mut doc = plan();
    doc["approval"] = true.into();
    assert!(parse_production_acceptance_plan(&serde_json::to_vec(&doc).unwrap()).is_err());
    assert!(
        parse_production_acceptance_plan(
            br#"{"qualification":"not_granted","qualification":"passed"}"#
        )
        .is_err()
    );
}

#[test]
fn every_language_and_core_rejects_missing_obligations_and_forged_readiness() {
    let original = plan();
    let languages = original["languages"].as_array().unwrap();
    let cores = ["syntax", "documentation", "conventions", "vulnerabilities"];
    let mut checked = 0;
    for (index, language) in languages.iter().enumerate() {
        for core in cores {
            for mutation in 0..3 {
                let mut doc = original.clone();
                match mutation {
                    0 => {
                        doc["languages"][index]["capabilities"]
                            .as_object_mut()
                            .unwrap()
                            .remove(core);
                    }
                    1 => {
                        doc["languages"][index]["capabilities"][core]["qualification"] =
                            "production_ready".into();
                    }
                    _ => {
                        doc["languages"][index]["capabilities"][core]["build_paths"][0]["implementation_status"] =
                            "production_ready".into();
                    }
                }
                assert!(
                    parse_production_acceptance_plan(&serde_json::to_vec(&doc).unwrap()).is_err(),
                    "language {} core {core} mutation {mutation} must not bypass acceptance",
                    language["language"]
                );
                checked += 1;
            }
        }
    }
    // 对全部228项义务逐项检验；不能只验证第一种语言或把局部状态升级为生产资格。
    assert_eq!(checked, 57 * 4 * 3);
}

#[test]
fn repository_plan_cannot_grant_qualification_with_unresolved_blockers() {
    for level in 0..3 {
        let mut doc = plan();
        doc["qualification"] = "not_granted".into();
        match level {
            0 => doc["qualification"] = "v1_granted".into(),
            1 => doc["languages"][0]["version_scope"]["qualification"] = "v1_qualified".into(),
            _ => {
                doc["languages"][0]["capabilities"]["syntax"]["qualification"] =
                    "v1_qualified".into()
            }
        }
        assert!(
            parse_production_acceptance_plan(&serde_json::to_vec(&doc).unwrap()).is_err(),
            "repository plan granted authority at level {level}"
        );
    }
}
