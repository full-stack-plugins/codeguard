use codeguard_core::{
    AllowlistTarget, FalsePositiveIdentity, IdentityMismatch, match_false_positive_identity,
};

fn source(path: &str) -> FalsePositiveIdentity {
    FalsePositiveIdentity {
        finding_id: "CG-F401-1".into(),
        checker_id: "ruff".into(),
        native_rule_id: "F401".into(),
        category: "lint".into(),
        target: AllowlistTarget::Source {
            path: path.into(),
            file_sha256: "a".repeat(64),
        },
        finding_fingerprint: "b".repeat(64),
        tool_sha256: "c".repeat(64),
        adapter_sha256: "d".repeat(64),
        rulepack_sha256: "e".repeat(64),
    }
}

#[test]
fn exact_source_identity_matches_but_does_not_grant_authorization() {
    assert_eq!(
        match_false_positive_identity(&source("src/app.py"), &source("src/app.py")),
        Ok(())
    );
}

#[test]
fn similar_source_finding_cannot_reuse_another_files_decision() {
    assert_eq!(
        match_false_positive_identity(&source("src/other.py"), &source("src/app.py")),
        Err(IdentityMismatch::DifferentIdentity)
    );
    let mut changed = source("src/app.py");
    changed.target = AllowlistTarget::Source {
        path: "src/app.py".into(),
        file_sha256: "f".repeat(64),
    };
    assert_eq!(
        match_false_positive_identity(&changed, &source("src/app.py")),
        Err(IdentityMismatch::DifferentIdentity)
    );
}

#[test]
fn rule_tool_adapter_and_rulepack_drift_invalidate_match() {
    let approved = source("src/app.py");
    for changed in [
        FalsePositiveIdentity {
            finding_id: "CG-F401-2".into(),
            ..approved.clone()
        },
        FalsePositiveIdentity {
            checker_id: "other-checker".into(),
            ..approved.clone()
        },
        FalsePositiveIdentity {
            native_rule_id: "F402".into(),
            ..approved.clone()
        },
        FalsePositiveIdentity {
            category: "security".into(),
            ..approved.clone()
        },
        FalsePositiveIdentity {
            tool_sha256: "f".repeat(64),
            ..approved.clone()
        },
        FalsePositiveIdentity {
            adapter_sha256: "f".repeat(64),
            ..approved.clone()
        },
        FalsePositiveIdentity {
            rulepack_sha256: "f".repeat(64),
            ..approved.clone()
        },
        FalsePositiveIdentity {
            finding_fingerprint: "f".repeat(64),
            ..approved.clone()
        },
    ] {
        assert_eq!(
            match_false_positive_identity(&changed, &approved),
            Err(IdentityMismatch::DifferentIdentity)
        );
    }
}

#[test]
fn wildcard_or_malformed_identity_is_rejected_before_matching() {
    let observed = source("src/app.py");
    let mut wildcard = observed.clone();
    wildcard.target = AllowlistTarget::Source {
        path: "src/*.py".into(),
        file_sha256: "a".repeat(64),
    };
    assert_eq!(
        match_false_positive_identity(&observed, &wildcard),
        Err(IdentityMismatch::InvalidDecision)
    );
    wildcard.target = AllowlistTarget::Source {
        path: "../outside.py".into(),
        file_sha256: "a".repeat(64),
    };
    assert_eq!(
        match_false_positive_identity(&observed, &wildcard),
        Err(IdentityMismatch::InvalidDecision)
    );
    let mut missing = observed.clone();
    missing.tool_sha256.clear();
    assert_eq!(
        match_false_positive_identity(&missing, &observed),
        Err(IdentityMismatch::InvalidObservation)
    );
    let mut unsupported = observed.clone();
    unsupported.category = "unknown_category".into();
    assert_eq!(
        match_false_positive_identity(&observed, &unsupported),
        Err(IdentityMismatch::InvalidDecision)
    );
}

#[test]
fn dependency_match_binds_graph_version_and_advisory() {
    let mut approved = source("src/app.py");
    approved.category = "cve".into();
    approved.target = AllowlistTarget::Dependency {
        component: "pkg:maven/org.example/demo".into(),
        version: "1.0".into(),
        graph_sha256: "1".repeat(64),
        advisory_id: "CVE-2026-1000".into(),
    };
    assert_eq!(match_false_positive_identity(&approved, &approved), Ok(()));
    for target in [
        AllowlistTarget::Dependency {
            component: "pkg:maven/org.example/demo".into(),
            version: "1.1".into(),
            graph_sha256: "1".repeat(64),
            advisory_id: "CVE-2026-1000".into(),
        },
        AllowlistTarget::Dependency {
            component: "pkg:maven/org.example/demo".into(),
            version: "1.0".into(),
            graph_sha256: "2".repeat(64),
            advisory_id: "CVE-2026-1000".into(),
        },
        AllowlistTarget::Dependency {
            component: "pkg:maven/org.example/demo".into(),
            version: "1.0".into(),
            graph_sha256: "1".repeat(64),
            advisory_id: "CVE-2026-1001".into(),
        },
    ] {
        let changed = FalsePositiveIdentity {
            target,
            ..approved.clone()
        };
        assert_eq!(
            match_false_positive_identity(&changed, &approved),
            Err(IdentityMismatch::DifferentIdentity)
        );
    }
}
