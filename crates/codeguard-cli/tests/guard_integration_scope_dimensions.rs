use codeguard_cli::guard_integration::{
    reader::read_native,
    scope::{
        FrozenObligations, RequiredConfiguration, RequiredScope, RequiredTool, ScopeAssessment,
        assess_scope,
    },
};
use std::collections::BTreeMap;
#[path = "support/guard_integration.rs"]
mod fixture;

fn required() -> BTreeMap<String, RequiredScope> {
    BTreeMap::from([(
        "python/app/lint/ruff".into(),
        RequiredScope {
            targets: vec!["src/main.py".into()],
            rules: vec!["F401".into()],
            tools: vec![RequiredTool {
                id: "ruff".into(),
                version: "0.16.8".into(),
                sha256: "c".repeat(64),
            }],
            configurations: vec![RequiredConfiguration {
                reference: "pyproject.toml".into(),
                sha256: "b".repeat(64),
            }],
        },
    )])
}
fn gaps(document: &serde_json::Value) -> Vec<String> {
    let bytes = serde_json::to_vec(document).unwrap();
    let evidence = read_native(&bytes, &fixture::invocation()).unwrap();
    let scope = FrozenObligations::with_requirements(required()).unwrap();
    let ScopeAssessment::Partial { gaps } = assess_scope(&evidence, &scope);
    gaps
}
#[test]
fn matching_findings_and_policy_digest_do_not_prove_rule_or_configuration_coverage() {
    let gaps = gaps(&fixture::valid());
    assert!(
        gaps.iter()
            .any(|gap| gap == "missing native rule coverage python/app/lint/ruff:F401")
    );
    assert!(
        gaps.iter().any(|gap| gap
            == "missing native configuration coverage python/app/lint/ruff:pyproject.toml")
    );
    assert!(
        !gaps
            .iter()
            .any(|gap| gap.starts_with("tool identity mismatch"))
    );
    assert!(gaps.iter().any(|gap| gap == "native profile unqualified"));
}
#[test]
fn changed_tool_version_or_digest_does_not_match_frozen_identity() {
    for field in ["version", "binary_sha256"] {
        let mut document = fixture::valid();
        document["identities"]["tools"][0][field] = if field == "version" {
            "0.16.9".into()
        } else {
            "d".repeat(64).into()
        };
        assert!(
            gaps(&document)
                .iter()
                .any(|gap| gap == "tool identity mismatch python/app/lint/ruff:ruff")
        );
    }
}
#[test]
fn absent_required_tool_and_unrelated_tool_success_do_not_discharge_scope() {
    let mut document = fixture::valid();
    document["identities"]["tools"][0]["id"] = "other".into();
    document["results"][0]["findings"][0]["tool_id"] = "other".into();
    assert!(
        gaps(&document)
            .iter()
            .any(|gap| gap == "tool identity mismatch python/app/lint/ruff:ruff")
    );
}
#[test]
fn required_dimensions_are_nonempty_unique_and_identity_checked() {
    for field in 0..4 {
        let mut scope = required();
        let entry = scope.values_mut().next().unwrap();
        match field {
            0 => entry.targets.clear(),
            1 => entry.rules.clear(),
            2 => entry.tools.clear(),
            _ => entry.configurations.clear(),
        }
        assert!(FrozenObligations::with_requirements(scope).is_err());
    }
    let mut scope = required();
    scope.values_mut().next().unwrap().rules.push("F401".into());
    assert!(FrozenObligations::with_requirements(scope).is_err());
    let mut scope = required();
    scope.values_mut().next().unwrap().tools[0].sha256 = "not-a-digest".into();
    assert!(FrozenObligations::with_requirements(scope).is_err());
    let mut scope = required();
    scope.values_mut().next().unwrap().configurations[0].sha256 = "B".repeat(64);
    assert!(FrozenObligations::with_requirements(scope).is_err());
}

#[test]
fn missing_target_and_unrelated_obligation_do_not_discharge_required_target() {
    let mut document = fixture::valid();
    document["results"][0]["coverage"]["expected"] = serde_json::json!(["unrelated.py"]);
    document["results"][0]["coverage"]["observed"] = serde_json::json!(["unrelated.py"]);
    assert!(
        gaps(&document)
            .iter()
            .any(|gap| gap == "missing target python/app/lint/ruff:src/main.py")
    );
}

#[test]
fn duplicate_tool_and_config_identities_are_rejected_even_if_digests_differ() {
    let mut scope = required();
    scope.values_mut().next().unwrap().tools.push(RequiredTool {
        id: "ruff".into(),
        version: "other".into(),
        sha256: "d".repeat(64),
    });
    assert!(FrozenObligations::with_requirements(scope).is_err());
    let mut scope = required();
    scope
        .values_mut()
        .next()
        .unwrap()
        .configurations
        .push(RequiredConfiguration {
            reference: "pyproject.toml".into(),
            sha256: "d".repeat(64),
        });
    assert!(FrozenObligations::with_requirements(scope).is_err());
}

struct AllocationProbe;
static LARGE_ALLOCATIONS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
unsafe impl std::alloc::GlobalAlloc for AllocationProbe {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        if layout.size() >= 64 * 1024 {
            LARGE_ALLOCATIONS.fetch_add(layout.size(), std::sync::atomic::Ordering::Relaxed);
        }
        unsafe { std::alloc::System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, size: usize) -> *mut u8 {
        if size >= 64 * 1024 {
            LARGE_ALLOCATIONS.fetch_add(size, std::sync::atomic::Ordering::Relaxed);
        }
        unsafe { std::alloc::System.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: AllocationProbe = AllocationProbe;

#[test]
fn reject_required_dimension_amplification_before_large_copy_or_gap_allocation() {
    let mut scope = required().into_values().next().unwrap();
    scope.rules = (0..64).map(|i| format!("rule-{i}")).collect();
    let required = BTreeMap::from([("x".repeat(256 * 1024), scope)]);
    let before = LARGE_ALLOCATIONS.load(std::sync::atomic::Ordering::Relaxed);
    assert!(FrozenObligations::with_requirements(required).is_err());
    assert_eq!(
        LARGE_ALLOCATIONS.load(std::sync::atomic::Ordering::Relaxed) - before,
        0
    );
}
