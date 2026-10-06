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
