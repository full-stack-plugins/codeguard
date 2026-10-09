use codeguard_cli::guard_integration::{
    projection::ProtectedMapping,
    ruff_profile::{RuffF401Policy, project_ruff, read_ruff_feedback},
};
use guardengine::{Completeness, Decision, GuardContract, GuardSubject};
use serde_json::{Value, json};
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/../../tests/fixtures/guard-integration/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}
fn dictionary() -> Value {
    serde_json::from_slice(&fixture("relations.json")).unwrap()
}
fn projection(
    name: &str,
    mapping: &Value,
    contract: &Value,
) -> Result<codeguard_cli::guard_integration::projection::Projection, &'static str> {
    let raw = fixture(&format!("ruff-f401/{name}.json"));
    let document: Value = serde_json::from_slice(&raw).unwrap();
    let capture: Value = serde_json::from_slice(&fixture("ruff-f401/capture.json")).unwrap();
    let producer = capture["codeguardSha256"].as_str().unwrap();
    let policy = RuffF401Policy::new(
        "app.py",
        capture["cases"][name]["sourceSha256"].as_str().unwrap(),
        producer,
    )
    .unwrap();
    let evidence = read_ruff_feedback(
        &raw,
        &policy,
        document["run_id"].as_str().unwrap(),
        3,
        producer,
    )?;
    let mapping = ProtectedMapping::parse(&serde_json::to_vec(mapping).unwrap())?;
    let contract: GuardContract =
        serde_json::from_value(contract.clone()).map_err(|_| "unsupported contract")?;
    project_ruff(
        &evidence,
        &mapping,
        &contract,
        GuardSubject {
            id: "repo".into(),
            snapshot_digest: policy.source_digest(),
        },
    )
}
#[test]
fn each_registered_finding_and_gap_has_positive_and_missing_mapping_counterexample() {
    let vocab = dictionary();
    assert_eq!(vocab["version"], "codeguard.relations/v1alpha1");
    assert_eq!(vocab["mapping"]["entries"].as_array().unwrap().len(), 2);
    for case in vocab["cases"].as_array().unwrap() {
        let name = case["nativeFixture"].as_str().unwrap();
        let p = projection(name, &vocab["mapping"], &vocab["contract"]).unwrap();
        assert_eq!(p.report().decision, Decision::Block);
        assert!(
            p.facts()
                .facts
                .iter()
                .any(|f| f.source.starts_with(case["sourcePrefix"].as_str().unwrap()))
        );
        assert_eq!(
            p.facts().completeness,
            if case["completeness"] == "complete" {
                Completeness::Complete
            } else {
                Completeness::Partial
            }
        );
        assert_eq!(p.domain_bytes(), fixture(&format!("ruff-f401/{name}.json")));
        let mut missing = vocab["mapping"].clone();
        missing["entries"]
            .as_array_mut()
            .unwrap()
            .remove(case["entry"].as_u64().unwrap() as usize);
        assert!(projection(name, &missing, &vocab["contract"]).is_err());
        let mut wrong = vocab["mapping"].clone();
        wrong["entries"][case["entry"].as_u64().unwrap() as usize]["rule_id"] =
            json!("unregistered");
        assert!(projection(name, &wrong, &vocab["contract"]).is_err());
    }
}
#[test]
fn no_count_unknown_source_duplicate_or_wildcard_vocabulary_is_accepted() {
    let vocab = dictionary();
    for kind in vocab["unsupportedSources"].as_array().unwrap() {
        let mut mapping = vocab["mapping"].clone();
        mapping["entries"][0]["source"]["kind"] = kind.clone();
        assert!(projection("bad", &mapping, &vocab["contract"]).is_err());
    }
    for assertion in vocab["unsupportedAssertions"].as_array().unwrap() {
        let mut contract = vocab["contract"].clone();
        contract["spec"]["rules"][0]["assertion"]["type"] = assertion.clone();
        assert!(projection("bad", &vocab["mapping"], &contract).is_err());
    }
    for field in ["subject", "predicate", "object"] {
        for wildcard in ["*", "?", "[x]"] {
            let mut contract = vocab["contract"].clone();
            contract["spec"]["rules"][0]["assertion"][field] = json!(wildcard);
            assert!(projection("bad", &vocab["mapping"], &contract).is_err());
        }
    }
    let mut duplicate = vocab["mapping"].clone();
    let first = duplicate["entries"][0].clone();
    duplicate["entries"].as_array_mut().unwrap().push(first);
    assert!(projection("bad", &duplicate, &vocab["contract"]).is_err());
    let mut unseen = vocab["mapping"].clone();
    unseen["entries"][0]["source"]["native_rule_id"] = json!("F821");
    assert!(projection("bad", &unseen, &vocab["contract"]).is_err());
}

#[test]
fn ten_shadow_comparisons_preserve_facts_report_and_original_native_bytes() {
    use codeguard_cli::guard_integration::shadow::compare;
    let vocab = dictionary();
    for name in ["bad", "clean", "missing-tool"] {
        let original = projection(name, &vocab["mapping"], &vocab["contract"]).unwrap();
        let facts = serde_json::to_value(original.facts()).unwrap();
        let report = serde_json::to_value(original.report()).unwrap();
        let raw = original.domain_bytes().to_vec();
        for _ in 0..10 {
            let repeated = projection(name, &vocab["mapping"], &vocab["contract"]).unwrap();
            assert_eq!(serde_json::to_value(repeated.facts()).unwrap(), facts);
            assert_eq!(serde_json::to_value(repeated.report()).unwrap(), report);
            let shadow = compare(&raw, Some(&repeated)).unwrap();
            assert_eq!(
                serde_json::to_value(shadow.engine_report().unwrap()).unwrap(),
                report
            );
            assert_eq!(repeated.domain_bytes(), raw);
        }
        assert!(compare(b"{}", Some(&original)).is_err());
        assert!(compare(&raw, None).unwrap().engine_report().is_err());
    }
}
