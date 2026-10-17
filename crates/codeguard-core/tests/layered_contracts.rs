use codeguard_core::finding_identity::IdentityGenerator;
use codeguard_core::run_config::{ApprovedPolicy, ConfigResolver, RunConfig};
use codeguard_core::typed_module_graph::{DependencyType, TypedModuleGraphGenerator};

#[test]
fn stable_identity_is_available_without_cli_or_runtime() {
    let first = IdentityGenerator::generate("old.rs", "R1", "same content");
    let renamed = IdentityGenerator::generate("new.rs", "R1", "same content");
    assert!(IdentityGenerator::match_identity(&first, &renamed));
}

#[test]
fn pure_policy_and_graph_contracts_live_below_cli() {
    let policy = ApprovedPolicy {
        required: vec!["architecture".into()],
        min_threshold: 0.9,
        exclude: vec![],
        version: "1".into(),
    };
    let run = RunConfig {
        version: "1".into(),
        timeout_ms: 1000,
        jobs: 1,
    };
    let resolved = ConfigResolver::resolve(&policy, &run, &run);
    assert_eq!(resolved.policy, policy);
    assert!(ConfigResolver::validate(&resolved).is_ok());
    let graph = TypedModuleGraphGenerator::generate(
        &["domain".into(), "adapter".into()],
        &[(
            "domain".into(),
            "adapter".into(),
            DependencyType::DynamicUnknown,
        )],
    );
    assert!(!graph.complete);
    assert!(!graph.edges[0].resolved);
}
