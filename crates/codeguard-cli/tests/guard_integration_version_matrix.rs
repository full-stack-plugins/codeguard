//! Exact supported readers, not a semver router or an implicit N/N-1 promise.
use codeguard_cli::guard_integration::{
    profile::InvocationDescriptor,
    projection::ProtectedMapping,
    reader::read_native,
    ruff_profile::{RuffF401Policy, read_ruff_feedback},
    scope::{FrozenObligations, ScopeAssessment, assess_scope},
};
use guardengine::Completeness;
use serde_json::{Value, json};
#[path = "support/guard_integration.rs"]
mod fixture;
fn raw(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/../../tests/fixtures/guard-integration/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}
#[test]
fn native_feedback_schema_is_exact_not_adjacent_version_compatibility() {
    let bytes = raw("ruff-f401/clean.json");
    let original: Value = serde_json::from_slice(&bytes).unwrap();
    let manifest: Value = serde_json::from_slice(&raw("ruff-f401/capture.json")).unwrap();
    let producer = manifest["codeguardSha256"].as_str().unwrap();
    let policy = RuffF401Policy::new(
        "app.py",
        manifest["cases"]["clean"]["sourceSha256"].as_str().unwrap(),
        producer,
    )
    .unwrap();
    let run = original["run_id"].as_str().unwrap();
    assert_eq!(original["schema_version"], "0.13.0");
    let accepted = read_ruff_feedback(&bytes, &policy, run, 3, producer).unwrap();
    assert_eq!(accepted.completeness(), Completeness::Complete);
    assert_eq!(accepted.raw_bytes(), bytes);
    for schema in ["0.12.0", "0.14.0", "1.0", "0.13.1", "unknown"] {
        let mut changed = original.clone();
        changed["schema_version"] = json!(schema);
        assert!(
            read_ruff_feedback(
                &serde_json::to_vec(&changed).unwrap(),
                &policy,
                run,
                3,
                producer
            )
            .is_err(),
            "{schema}"
        );
    }
    // A different tool is at best partial, never complete merely by version label.
    let mut changed = original.clone();
    changed["files"][0]["tool_sha256"] = json!("0".repeat(64));
    match read_ruff_feedback(
        &serde_json::to_vec(&changed).unwrap(),
        &policy,
        run,
        3,
        producer,
    ) {
        Err(_) => {}
        Ok(evidence) => assert_eq!(evidence.completeness(), Completeness::Partial),
    }
}
#[test]
fn legacy_reader_stays_unqualified_and_rejects_adjacent_package_and_schema() {
    // Protocol fixture; actual guard-project process behavior is covered by the CLI suite.
    let original = fixture::invocation();
    let evidence = read_native(&serde_json::to_vec(&fixture::valid()).unwrap(), &original).unwrap();
    assert!(!original.capability().unwrap().qualified);
    let obligations = FrozenObligations::new(std::collections::BTreeMap::from([(
        "python/app/lint/ruff".into(),
        vec!["src/main.py".into()],
    )]))
    .unwrap();
    assert!(matches!(
        assess_scope(&evidence, &obligations),
        ScopeAssessment::Partial { .. }
    ));
    for (key, values) in [
        ("package_version", ["0.1.3", "0.1.5", "unknown"]),
        ("native_schema", ["0.13.0", "1.1", "unknown"]),
        (
            "version",
            [
                "codeguard.invocation/v1alpha0",
                "codeguard.invocation/v1alpha2",
                "unknown",
            ],
        ),
    ] {
        for value in values {
            let mut changed = serde_json::to_value(&original).unwrap();
            changed[key] = json!(value);
            assert!(
                InvocationDescriptor::parse(&serde_json::to_vec(&changed).unwrap()).is_err(),
                "{key}={value}"
            );
        }
    }
}
#[test]
fn mapping_version_never_inherits_package_or_native_version() {
    let fixture: Value = serde_json::from_slice(&raw("relations.json")).unwrap();
    let original = fixture["mapping"].clone();
    assert!(ProtectedMapping::parse(&serde_json::to_vec(&original).unwrap()).is_ok());
    for version in [
        "codeguard.mapping/v1alpha0",
        "codeguard.mapping/v1alpha2",
        "0.1.4",
        "0.13.0",
    ] {
        let mut changed = original.clone();
        changed["version"] = json!(version);
        assert!(
            ProtectedMapping::parse(&serde_json::to_vec(&changed).unwrap()).is_err(),
            "{version}"
        );
    }
}
