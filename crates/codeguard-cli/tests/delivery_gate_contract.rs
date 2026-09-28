use codeguard_core::{
    AllowlistDisposition, AllowlistTarget, Completion, DeliveryDecision, DeliveryInput,
    FalsePositiveIdentity, Finding, FindingLocation, GateImpact, NativeCheckerBinding,
    ObligationEvidence, ObligationResult, ObligationSpec, evaluate_delivery,
};

fn obligation(id: &str, targets: &[&str]) -> ObligationSpec {
    ObligationSpec {
        id: id.into(),
        expected_targets: targets.iter().map(|target| (*target).into()).collect(),
    }
}

fn evidence(id: &str, targets: &[&str]) -> ObligationEvidence {
    ObligationEvidence {
        result: ObligationResult {
            id: id.into(),
            completion: Completion::Complete,
            reason: None,
            findings: Vec::new(),
        },
        observed_targets: targets.iter().map(|target| (*target).into()).collect(),
    }
}

fn full_input() -> DeliveryInput {
    DeliveryInput {
        approval_scope: Some(codeguard_core::ApprovalScope {
            workspace_id: "workspace-one".into(),
            baseline_commit: "a".repeat(40),
        }),
        full_project: true,
        discovery_complete: true,
        trusted_bindings_verified: true,
        gate_time_unix: Some(100),
        policy_revision: Some("policy-r1".into()),
        frozen_obligation_ids: vec!["python/lint".into()],
        obligations: vec![obligation("python/lint", &["src/a.py"])],
        evidence: vec![evidence("python/lint", &["src/a.py"])],
        allowlist_dispositions: Vec::new(),
        native_checker_bindings: vec![NativeCheckerBinding {
            checker_id: "python.ruff".into(),
            tool_id: "ruff".into(),
            category: "lint".into(),
            obligation_id: "python/lint".into(),
        }],
    }
}

fn blocking(id: &str) -> Finding {
    Finding {
        id: id.into(),
        native_rule_id: "F401".into(),
        tool_id: "ruff".into(),
        severity: "error".into(),
        gate_impact: GateImpact::Blocking,
        message: "unused import".into(),
        obligation_id: "python/lint".into(),
        native_identity: Some(disposition(id).observed_identity),
        locations: vec![FindingLocation::Source {
            path: "src/a.py".into(),
            line: Some(1),
            column: None,
        }],
    }
}

fn disposition(id: &str) -> AllowlistDisposition {
    let identity = FalsePositiveIdentity {
        finding_id: id.into(),
        checker_id: "python.ruff".into(),
        native_rule_id: "F401".into(),
        category: "lint".into(),
        target: AllowlistTarget::Source {
            path: "src/a.py".into(),
            file_sha256: "a".repeat(64),
        },
        finding_fingerprint: "b".repeat(64),
        tool_sha256: "c".repeat(64),
        adapter_sha256: "d".repeat(64),
        rulepack_sha256: "e".repeat(64),
    };
    AllowlistDisposition {
        approval_scope: Some(codeguard_core::ApprovalScope {
            workspace_id: "workspace-one".into(),
            baseline_commit: "a".repeat(40),
        }),
        observed_identity: identity.clone(),
        decision_identity: identity,
        decision_id: "CG-FP-1".into(),
        approval_ref: "review:one".into(),
        approved_policy_revision: "policy-r1".into(),
        independent_approval_verified: true,
        observed_at: 100,
        expires_at: 200,
    }
}

#[test]
fn sole_approved_exact_false_positive_keeps_raw_finding_and_allows_with_exception() {
    let mut input = full_input();
    input.evidence[0].result.findings.push(blocking("f-1"));
    input.allowlist_dispositions.push(disposition("f-1"));
    let actual = evaluate_delivery(&input);
    assert_eq!(actual.decision, DeliveryDecision::AllowWithExceptions);
    assert_eq!(actual.blocking_finding_ids, ["f-1"]);
    assert_eq!(actual.whitelisted_false_positive_ids, ["f-1"]);
    assert!(actual.active_blocking_finding_ids.is_empty());
}

#[test]
fn raw_native_identity_drift_cannot_reuse_a_matching_approval() {
    for drift in [
        "missing",
        "source_bytes",
        "fingerprint",
        "tool_bytes",
        "adapter_bytes",
        "rulepack_bytes",
    ] {
        let mut input = full_input();
        input.evidence[0].result.findings.push(blocking("f-1"));
        input.allowlist_dispositions.push(disposition("f-1"));
        let identity = &mut input.evidence[0].result.findings[0].native_identity;
        match drift {
            "missing" => *identity = None,
            "source_bytes" => {
                identity.as_mut().unwrap().target = AllowlistTarget::Source {
                    path: "src/a.py".into(),
                    file_sha256: "1".repeat(64),
                }
            }
            "fingerprint" => identity.as_mut().unwrap().finding_fingerprint = "1".repeat(64),
            "tool_bytes" => identity.as_mut().unwrap().tool_sha256 = "1".repeat(64),
            "adapter_bytes" => identity.as_mut().unwrap().adapter_sha256 = "1".repeat(64),
            "rulepack_bytes" => identity.as_mut().unwrap().rulepack_sha256 = "1".repeat(64),
            _ => unreachable!(),
        }
        let gate = evaluate_delivery(&input);
        assert_eq!(gate.decision, DeliveryDecision::Incomplete, "{drift}");
        assert_eq!(gate.blocking_finding_ids, ["f-1"], "{drift}");
        assert_eq!(gate.active_blocking_finding_ids, ["f-1"], "{drift}");
        assert!(gate.whitelisted_false_positive_ids.is_empty(), "{drift}");
    }
}

#[test]
fn legacy_finding_without_native_identity_retains_its_blocker() {
    let mut input = full_input();
    input.evidence[0].result.findings.push(blocking("f-1"));
    input.allowlist_dispositions.push(disposition("f-1"));
    let mut legacy = serde_json::to_value(&input).unwrap();
    legacy["evidence"][0]["result"]["findings"][0]
        .as_object_mut()
        .unwrap()
        .remove("native_identity");
    let restored: DeliveryInput = serde_json::from_value(legacy).unwrap();
    let gate = evaluate_delivery(&restored);
    assert_eq!(gate.decision, DeliveryDecision::Incomplete);
    assert_eq!(gate.active_blocking_finding_ids, ["f-1"]);
    assert!(gate.whitelisted_false_positive_ids.is_empty());
}

#[test]
fn approval_lifetime_is_checked_at_final_gate_time_instead_of_observation_time() {
    for now in [
        Some(100),
        Some(150),
        Some(199),
        Some(200),
        Some(250),
        Some(99),
        Some(0),
        None,
    ] {
        let mut input = full_input();
        input.evidence[0].result.findings.push(blocking("f-1"));
        input.allowlist_dispositions.push(disposition("f-1"));
        input.gate_time_unix = now;
        let gate = evaluate_delivery(&input);
        let current = matches!(now, Some(100..=199));
        assert_eq!(
            gate.decision,
            if current {
                DeliveryDecision::AllowWithExceptions
            } else {
                DeliveryDecision::Incomplete
            },
            "{now:?}"
        );
        if !current {
            assert_eq!(gate.active_blocking_finding_ids, ["f-1"]);
            assert!(gate.whitelisted_false_positive_ids.is_empty());
        }
    }
}

#[test]
fn legacy_input_without_a_final_clock_cannot_replay_an_old_whitelist_observation() {
    let mut input = full_input();
    input.evidence[0].result.findings.push(blocking("f-1"));
    input.allowlist_dispositions.push(disposition("f-1"));
    let mut value = serde_json::to_value(&input).unwrap();
    value.as_object_mut().unwrap().remove("gate_time_unix");
    let legacy: DeliveryInput = serde_json::from_value(value).unwrap();
    assert_eq!(
        evaluate_delivery(&legacy).decision,
        DeliveryDecision::Incomplete
    );
    input.allowlist_dispositions.clear();
    input.gate_time_unix = None;
    assert_eq!(evaluate_delivery(&input).decision, DeliveryDecision::Deny);
}

#[test]
fn whitelist_disposition_must_bind_the_independently_frozen_current_policy_revision() {
    for revision in [Some("policy-r1"), Some("policy-r2"), Some(""), None] {
        let mut input = full_input();
        input.evidence[0].result.findings.push(blocking("f-1"));
        input.allowlist_dispositions.push(disposition("f-1"));
        input.policy_revision = revision.map(str::to_owned);
        let gate = evaluate_delivery(&input);
        if revision == Some("policy-r1") {
            assert_eq!(gate.decision, DeliveryDecision::AllowWithExceptions);
        } else {
            assert_eq!(gate.decision, DeliveryDecision::Incomplete);
            assert_eq!(gate.active_blocking_finding_ids, ["f-1"]);
            assert!(gate.whitelisted_false_positive_ids.is_empty());
        }
    }
}

#[test]
fn whitelist_cannot_cross_the_frozen_workspace_or_approval_baseline() {
    use codeguard_core::ApprovalScope;
    let correct = ApprovalScope {
        workspace_id: "workspace-one".into(),
        baseline_commit: "a".repeat(40),
    };
    for scope in [
        Some(correct.clone()),
        Some(ApprovalScope {
            workspace_id: "workspace-two".into(),
            ..correct.clone()
        }),
        Some(ApprovalScope {
            baseline_commit: "b".repeat(40),
            ..correct.clone()
        }),
        Some(ApprovalScope {
            baseline_commit: "0".repeat(40),
            ..correct.clone()
        }),
        Some(ApprovalScope {
            baseline_commit: "HEAD".into(),
            ..correct.clone()
        }),
        None,
    ] {
        let mut input = full_input();
        input.evidence[0].result.findings.push(blocking("f-1"));
        input.allowlist_dispositions.push(disposition("f-1"));
        input.approval_scope = scope.clone();
        let gate = evaluate_delivery(&input);
        if scope == Some(correct.clone()) {
            assert_eq!(gate.decision, DeliveryDecision::AllowWithExceptions);
        } else {
            assert_eq!(gate.decision, DeliveryDecision::Incomplete);
            assert_eq!(gate.active_blocking_finding_ids, ["f-1"]);
            assert!(gate.whitelisted_false_positive_ids.is_empty());
        }
    }
    let mut input = full_input();
    input.evidence[0].result.findings.push(blocking("f-1"));
    input.allowlist_dispositions.push(disposition("f-1"));
    input.allowlist_dispositions[0].approval_scope = None;
    assert_eq!(
        evaluate_delivery(&input).decision,
        DeliveryDecision::Incomplete
    );
}

#[test]
fn equal_invalid_scopes_and_legacy_scope_less_inputs_cannot_waive_findings() {
    for (workspace, commit, valid) in [
        ("workspace-one", "a".repeat(64), true),
        ("", "a".repeat(40), false),
        ("workspace\nforged", "a".repeat(40), false),
        ("workspace-one", "0".repeat(40), false),
        ("workspace-one", "0".repeat(64), false),
        ("workspace-one", "A".repeat(40), false),
        ("workspace-one", "a".repeat(39), false),
        ("workspace-one", "HEAD".into(), false),
    ] {
        let scope = codeguard_core::ApprovalScope {
            workspace_id: workspace.into(),
            baseline_commit: commit,
        };
        let mut input = full_input();
        input.evidence[0].result.findings.push(blocking("f-1"));
        input.allowlist_dispositions.push(disposition("f-1"));
        input.approval_scope = Some(scope.clone());
        input.allowlist_dispositions[0].approval_scope = Some(scope);
        assert_eq!(
            evaluate_delivery(&input).decision,
            if valid {
                DeliveryDecision::AllowWithExceptions
            } else {
                DeliveryDecision::Incomplete
            }
        );
    }
    let mut input = full_input();
    input.evidence[0].result.findings.push(blocking("f-1"));
    input.allowlist_dispositions.push(disposition("f-1"));
    let mut legacy = serde_json::to_value(&input).unwrap();
    legacy.as_object_mut().unwrap().remove("approval_scope");
    legacy["allowlist_dispositions"][0]
        .as_object_mut()
        .unwrap()
        .remove("approval_scope");
    let mut decoded: DeliveryInput = serde_json::from_value(legacy).unwrap();
    assert_eq!(
        evaluate_delivery(&decoded).decision,
        DeliveryDecision::Incomplete
    );
    decoded.allowlist_dispositions.clear();
    assert_eq!(evaluate_delivery(&decoded).decision, DeliveryDecision::Deny);
}

#[test]
fn approved_identity_cannot_dispose_a_native_finding_from_a_different_tool() {
    let mut input = full_input();
    let mut finding = blocking("f-1");
    finding.tool_id = "unrelated-native-checker".into();
    input.evidence[0].result.findings.push(finding);
    input.allowlist_dispositions.push(disposition("f-1"));
    let gate = evaluate_delivery(&input);
    assert_eq!(gate.decision, DeliveryDecision::Incomplete);
    assert_eq!(gate.active_blocking_finding_ids, ["f-1"]);
    assert!(gate.whitelisted_false_positive_ids.is_empty());
}

#[test]
fn matching_decision_and_observation_cannot_hide_a_different_native_primary_target() {
    use codeguard_core::FindingLocation;
    for locations in [
        vec![],
        vec![FindingLocation::Source {
            path: "src/other.py".into(),
            line: Some(1),
            column: None,
        }],
        vec![
            FindingLocation::Source {
                path: "src/other.py".into(),
                line: Some(1),
                column: None,
            },
            FindingLocation::Source {
                path: "src/a.py".into(),
                line: Some(1),
                column: None,
            },
        ],
    ] {
        let mut input = full_input();
        let mut finding = blocking("f-1");
        finding.locations = locations;
        input.evidence[0].result.findings.push(finding);
        input.allowlist_dispositions.push(disposition("f-1"));
        let gate = evaluate_delivery(&input);
        assert_eq!(gate.decision, DeliveryDecision::Incomplete);
        assert_eq!(gate.active_blocking_finding_ids, ["f-1"]);
        assert!(gate.whitelisted_false_positive_ids.is_empty());
    }
}

#[test]
fn missing_ambiguous_or_inconsistent_native_checker_binding_cannot_authorize_a_whitelist() {
    for change in [
        "missing",
        "duplicate",
        "tool",
        "category",
        "obligation",
        "checker",
    ] {
        let mut input = full_input();
        input.evidence[0].result.findings.push(blocking("f-1"));
        input.allowlist_dispositions.push(disposition("f-1"));
        match change {
            "missing" => input.native_checker_bindings.clear(),
            "duplicate" => input
                .native_checker_bindings
                .push(input.native_checker_bindings[0].clone()),
            "tool" => input.native_checker_bindings[0].tool_id = "another".into(),
            "category" => input.native_checker_bindings[0].category = "comments".into(),
            "obligation" => {
                input.native_checker_bindings[0].obligation_id = "python/comments".into()
            }
            "checker" => input.native_checker_bindings[0].checker_id = "unrelated.ruff".into(),
            _ => unreachable!(),
        }
        let gate = evaluate_delivery(&input);
        assert_eq!(gate.decision, DeliveryDecision::Incomplete, "{change}");
        assert_eq!(gate.active_blocking_finding_ids, ["f-1"], "{change}");
        assert!(gate.whitelisted_false_positive_ids.is_empty(), "{change}");
    }
}

#[test]
fn dependency_disposition_requires_exact_component_and_an_explicit_native_version() {
    for (component, version, graph_byte, advisory, expected) in [
        (
            "pkg:maven/org.example/demo",
            Some("1.0"),
            "1",
            "CVE-2026-1000",
            DeliveryDecision::AllowWithExceptions,
        ),
        (
            "pkg:maven/org.example/demo",
            Some("2.0"),
            "1",
            "CVE-2026-1000",
            DeliveryDecision::Incomplete,
        ),
        (
            "pkg:maven/org.example/demo",
            None,
            "1",
            "CVE-2026-1000",
            DeliveryDecision::Incomplete,
        ),
        (
            "pkg:maven/org.example/other",
            Some("1.0"),
            "1",
            "CVE-2026-1000",
            DeliveryDecision::Incomplete,
        ),
        (
            "pkg:maven/org.example/demo",
            Some("1.0"),
            "2",
            "CVE-2026-1000",
            DeliveryDecision::Incomplete,
        ),
        (
            "pkg:maven/org.example/demo",
            Some("1.0"),
            "1",
            "CVE-2026-2000",
            DeliveryDecision::Incomplete,
        ),
    ] {
        let mut input = full_input();
        input.frozen_obligation_ids = vec!["java/cve".into()];
        input.obligations = vec![obligation("java/cve", &["pkg:maven/org.example/demo@1.0"])];
        input.evidence = vec![evidence("java/cve", &["pkg:maven/org.example/demo@1.0"])];
        let mut finding = blocking("f-1");
        finding.tool_id = "owasp-dependency-check".into();
        finding.native_rule_id = "CVE-2026-1000".into();
        finding.obligation_id = "java/cve".into();
        finding.locations = vec![FindingLocation::Dependency {
            component: component.into(),
            version: version.map(str::to_owned),
            path: None,
        }];
        input.evidence[0].result.findings.push(finding);
        let mut approval = disposition("f-1");
        approval.observed_identity.checker_id = "java.owasp".into();
        approval.observed_identity.native_rule_id = "CVE-2026-1000".into();
        approval.observed_identity.category = "cve".into();
        approval.observed_identity.target = AllowlistTarget::Dependency {
            component: "pkg:maven/org.example/demo".into(),
            version: "1.0".into(),
            graph_sha256: "1".repeat(64),
            advisory_id: "CVE-2026-1000".into(),
        };
        approval.decision_identity = approval.observed_identity.clone();
        let mut native_identity = approval.observed_identity.clone();
        native_identity.target = AllowlistTarget::Dependency {
            component: "pkg:maven/org.example/demo".into(),
            version: "1.0".into(),
            graph_sha256: graph_byte.repeat(64),
            advisory_id: advisory.into(),
        };
        input.evidence[0].result.findings[0].native_identity = Some(native_identity);
        input.allowlist_dispositions.push(approval);
        input.native_checker_bindings = vec![NativeCheckerBinding {
            checker_id: "java.owasp".into(),
            tool_id: "owasp-dependency-check".into(),
            category: "cve".into(),
            obligation_id: "java/cve".into(),
        }];
        let gate = evaluate_delivery(&input);
        assert_eq!(gate.decision, expected);
        if expected == DeliveryDecision::Incomplete {
            assert_eq!(gate.active_blocking_finding_ids, ["f-1"]);
            assert!(gate.whitelisted_false_positive_ids.is_empty());
        }
    }
}

#[test]
fn legacy_delivery_input_can_be_read_but_cannot_claim_a_whitelist_without_mapping() {
    let mut input = full_input();
    input.evidence[0].result.findings.push(blocking("f-1"));
    input.allowlist_dispositions.push(disposition("f-1"));
    let mut bytes = serde_json::to_value(&input).unwrap();
    bytes
        .as_object_mut()
        .unwrap()
        .remove("native_checker_bindings");
    let restored: DeliveryInput = serde_json::from_value(bytes).unwrap();
    assert!(restored.native_checker_bindings.is_empty());
    assert_eq!(
        evaluate_delivery(&restored).decision,
        DeliveryDecision::Incomplete
    );
    input.allowlist_dispositions.clear();
    input.native_checker_bindings.clear();
    assert_eq!(evaluate_delivery(&input).decision, DeliveryDecision::Deny);
}

#[test]
fn conflicting_dispositions_do_not_leave_the_first_finding_marked_as_whitelisted() {
    for shared_finding in [false, true] {
        let mut input = full_input();
        input.evidence[0].result.findings.push(blocking("f-1"));
        let second = if shared_finding { "f-1" } else { "f-2" };
        if !shared_finding {
            input.evidence[0].result.findings.push(blocking(second));
        }
        input.allowlist_dispositions = vec![disposition("f-1"), disposition(second)];
        if shared_finding {
            input.allowlist_dispositions[1].decision_id = "CG-FP-2".into();
        }
        let gate = evaluate_delivery(&input);
        assert_eq!(gate.decision, DeliveryDecision::Incomplete);
        assert!(gate.whitelisted_false_positive_ids.is_empty());
        assert_eq!(
            gate.active_blocking_finding_ids,
            if shared_finding {
                vec!["f-1"]
            } else {
                vec!["f-1", "f-2"]
            }
        );
        input.allowlist_dispositions.reverse();
        assert_eq!(evaluate_delivery(&input), gate);
    }
}

#[test]
fn malformed_obligation_attribution_or_coverage_cannot_leave_a_whitelist_disposition() {
    for change in ["attribution", "unknown_obligation", "coverage"] {
        let mut input = full_input();
        input.evidence[0].result.findings.push(blocking("f-1"));
        input.allowlist_dispositions.push(disposition("f-1"));
        match change {
            "attribution" => {
                input.evidence[0].result.findings[0].obligation_id = "python/comments".into();
                input.native_checker_bindings[0].obligation_id = "python/comments".into();
            }
            "unknown_obligation" => {
                input.evidence[0].result.id = "unknown/lint".into();
                input.evidence[0].result.findings[0].obligation_id = "unknown/lint".into();
                input.native_checker_bindings[0].obligation_id = "unknown/lint".into();
            }
            "coverage" => input.evidence[0].observed_targets = vec!["src/other.py".into()],
            _ => unreachable!(),
        }
        let gate = evaluate_delivery(&input);
        assert_eq!(gate.decision, DeliveryDecision::Incomplete, "{change}");
        assert!(gate.whitelisted_false_positive_ids.is_empty(), "{change}");
        assert_eq!(gate.active_blocking_finding_ids, ["f-1"], "{change}");
    }
}

#[test]
fn other_blocker_still_denies_and_incomplete_still_precedes_exception() {
    let mut input = full_input();
    input.evidence[0]
        .result
        .findings
        .extend([blocking("f-1"), blocking("f-2")]);
    input.allowlist_dispositions.push(disposition("f-1"));
    let actual = evaluate_delivery(&input);
    assert_eq!(actual.decision, DeliveryDecision::Deny);
    assert_eq!(actual.active_blocking_finding_ids, ["f-2"]);
    input.evidence[0].result.completion = Completion::Incomplete;
    assert_eq!(
        evaluate_delivery(&input).decision,
        DeliveryDecision::Incomplete
    );
}

#[test]
fn expired_mismatched_self_approved_or_conflicting_disposition_cannot_allow() {
    let mut input = full_input();
    input.evidence[0].result.findings.push(blocking("f-1"));
    let mut expired = disposition("f-1");
    expired.expires_at = expired.observed_at;
    input.allowlist_dispositions = vec![expired];
    assert_ne!(
        evaluate_delivery(&input).decision,
        DeliveryDecision::AllowWithExceptions
    );
    let mut wrong_file = disposition("f-1");
    wrong_file.decision_identity.target = AllowlistTarget::Source {
        path: "src/other.py".into(),
        file_sha256: "a".repeat(64),
    };
    input.allowlist_dispositions = vec![wrong_file];
    assert_ne!(
        evaluate_delivery(&input).decision,
        DeliveryDecision::AllowWithExceptions
    );
    let mut self_approved = disposition("f-1");
    self_approved.independent_approval_verified = false;
    input.allowlist_dispositions = vec![self_approved];
    assert_ne!(
        evaluate_delivery(&input).decision,
        DeliveryDecision::AllowWithExceptions
    );
    input.allowlist_dispositions = vec![disposition("f-1"), disposition("f-1")];
    assert_eq!(
        evaluate_delivery(&input).decision,
        DeliveryDecision::Incomplete
    );
    input.allowlist_dispositions = vec![disposition("f-1")];
    input.trusted_bindings_verified = false;
    assert_eq!(
        evaluate_delivery(&input).decision,
        DeliveryDecision::Incomplete
    );
}

#[test]
fn one_decision_id_cannot_waive_two_different_findings() {
    let mut input = full_input();
    input.evidence[0]
        .result
        .findings
        .extend([blocking("f-1"), blocking("f-2")]);
    input.allowlist_dispositions = vec![disposition("f-1"), disposition("f-2")];
    let actual = evaluate_delivery(&input);
    assert_eq!(actual.decision, DeliveryDecision::Incomplete);
    assert_eq!(actual.blocking_finding_ids, ["f-1", "f-2"]);
}

#[test]
fn same_finding_id_from_another_rule_cannot_be_waived_by_one_exact_decision() {
    let mut input = full_input();
    input.frozen_obligation_ids.push("python/comments".into());
    input
        .obligations
        .push(obligation("python/comments", &["src/a.py"]));
    input.evidence[0].result.findings.push(blocking("f-shared"));
    let mut other = evidence("python/comments", &["src/a.py"]);
    let mut other_finding = blocking("f-shared");
    other_finding.obligation_id = "python/comments".into();
    other_finding.native_rule_id = "D100".into();
    other.result.findings.push(other_finding);
    input.evidence.push(other);
    input.allowlist_dispositions.push(disposition("f-shared"));

    let actual = evaluate_delivery(&input);
    assert_eq!(actual.decision, DeliveryDecision::Incomplete);
    assert_eq!(
        actual.invalid_ledger_ids,
        ["python/comments", "python/lint"]
    );
    assert_eq!(actual.invalid_allowlist_finding_ids, ["f-shared"]);
    assert_eq!(actual.active_blocking_finding_ids, ["f-shared"]);
}

#[test]
fn duplicate_finding_ids_without_a_whitelist_still_make_the_ledger_incomplete() {
    let mut input = full_input();
    input.evidence[0]
        .result
        .findings
        .extend([blocking("f-shared"), blocking("f-shared")]);
    let actual = evaluate_delivery(&input);
    assert_eq!(actual.decision, DeliveryDecision::Incomplete);
    assert_eq!(actual.invalid_ledger_ids, ["python/lint"]);
    assert_eq!(actual.active_blocking_finding_ids, ["f-shared"]);
}

#[test]
fn exact_complete_required_ledger_can_allow() {
    let actual = evaluate_delivery(&full_input());
    assert_eq!(actual.decision, DeliveryDecision::Allow);
    assert!(actual.incomplete_obligation_ids.is_empty());
}

#[test]
fn category_request_never_signs_project_allow() {
    let mut input = full_input();
    input.full_project = false;
    assert_eq!(
        evaluate_delivery(&input).decision,
        DeliveryDecision::NotEvaluated
    );
}

#[test]
fn absent_adapter_evidence_is_incomplete() {
    let mut input = full_input();
    input.evidence.clear();
    let actual = evaluate_delivery(&input);
    assert_eq!(actual.decision, DeliveryDecision::Incomplete);
    assert_eq!(actual.incomplete_obligation_ids, ["python/lint"]);
}

#[test]
fn dropped_obligation_from_frozen_contract_cannot_allow() {
    let mut input = full_input();
    input.frozen_obligation_ids.push("java/cve".into());
    let actual = evaluate_delivery(&input);
    assert_eq!(actual.decision, DeliveryDecision::Incomplete);
    assert_eq!(actual.invalid_ledger_ids, ["java/cve"]);
}

#[test]
fn partial_target_coverage_cannot_allow() {
    let mut input = full_input();
    input.obligations[0]
        .expected_targets
        .push("src/b.py".into());
    let actual = evaluate_delivery(&input);
    assert_eq!(actual.decision, DeliveryDecision::Incomplete);
    assert_eq!(actual.coverage_mismatch_ids, ["python/lint"]);
}

#[test]
fn missing_identity_or_discovery_cannot_allow() {
    let mut input = full_input();
    input.trusted_bindings_verified = false;
    assert_eq!(
        evaluate_delivery(&input).decision,
        DeliveryDecision::Incomplete
    );
    input.trusted_bindings_verified = true;
    input.discovery_complete = false;
    assert_eq!(
        evaluate_delivery(&input).decision,
        DeliveryDecision::Incomplete
    );
}

#[test]
fn blocking_finding_remains_visible_when_another_obligation_is_missing() {
    let mut input = full_input();
    input
        .obligations
        .push(obligation("python/cve", &["lockfile"]));
    input.frozen_obligation_ids.push("python/cve".into());
    input.evidence[0].result.findings.push(Finding {
        id: "f-1".into(),
        native_rule_id: "F401".into(),
        tool_id: "ruff".into(),
        severity: "error".into(),
        gate_impact: GateImpact::Blocking,
        message: "unused import".into(),
        obligation_id: "python/lint".into(),
        native_identity: None,
        locations: Vec::new(),
    });
    let actual = evaluate_delivery(&input);
    assert_eq!(actual.decision, DeliveryDecision::Incomplete);
    assert_eq!(actual.blocking_finding_ids, ["f-1"]);
    assert_eq!(actual.incomplete_obligation_ids, ["python/cve"]);
}

#[test]
fn empty_discovered_project_is_not_applicable_not_allow() {
    let mut input = full_input();
    input.obligations.clear();
    input.evidence.clear();
    input.frozen_obligation_ids.clear();
    assert_eq!(
        evaluate_delivery(&input).decision,
        DeliveryDecision::NotApplicable
    );
}

#[test]
fn a_complete_obligation_with_no_target_cannot_certify_delivery() {
    let mut input = full_input();
    input.obligations[0].expected_targets.clear();
    input.evidence[0].observed_targets.clear();
    let actual = evaluate_delivery(&input);
    assert_eq!(actual.decision, DeliveryDecision::Incomplete);
    assert_eq!(actual.coverage_mismatch_ids, ["python/lint"]);
}

#[test]
fn duplicate_or_unknown_report_cannot_replace_required_proof() {
    let mut input = full_input();
    input.evidence.push(evidence("python/lint", &["src/a.py"]));
    assert_eq!(
        evaluate_delivery(&input).decision,
        DeliveryDecision::Incomplete
    );
    input.evidence = vec![evidence("python/security", &["src/a.py"])];
    assert_eq!(
        evaluate_delivery(&input).decision,
        DeliveryDecision::Incomplete
    );
}

#[test]
fn not_applicable_cannot_hide_a_discovered_target() {
    let mut input = full_input();
    input.evidence[0].result.completion = Completion::NotApplicable;
    input.evidence[0].result.reason = Some("no_rules".into());
    input.evidence[0].observed_targets.clear();
    assert_eq!(
        evaluate_delivery(&input).decision,
        DeliveryDecision::Incomplete
    );
}

#[test]
fn duplicate_declared_targets_cannot_be_counted_as_coverage() {
    let mut input = full_input();
    input.obligations[0]
        .expected_targets
        .push("src/a.py".into());
    let actual = evaluate_delivery(&input);
    assert_eq!(actual.decision, DeliveryDecision::Incomplete);
    assert_eq!(actual.coverage_mismatch_ids, ["python/lint"]);
}

#[test]
fn duplicate_obligations_cannot_sign_allow() {
    let mut input = full_input();
    input
        .obligations
        .push(obligation("python/lint", &["src/a.py"]));
    let actual = evaluate_delivery(&input);
    assert_eq!(actual.decision, DeliveryDecision::Incomplete);
    assert_eq!(actual.invalid_ledger_ids, ["python/lint"]);
}

#[test]
fn duplicate_frozen_or_extra_declared_obligations_cannot_sign_allow() {
    let mut input = full_input();
    input.frozen_obligation_ids.push("python/lint".into());
    assert_eq!(
        evaluate_delivery(&input).decision,
        DeliveryDecision::Incomplete
    );
    assert_eq!(
        evaluate_delivery(&input).invalid_ledger_ids,
        ["python/lint"]
    );

    input.frozen_obligation_ids.pop();
    input.obligations.push(obligation("java/cve", &["pom.xml"]));
    let actual = evaluate_delivery(&input);
    assert_eq!(actual.decision, DeliveryDecision::Incomplete);
    assert_eq!(actual.invalid_ledger_ids, ["java/cve"]);
}
