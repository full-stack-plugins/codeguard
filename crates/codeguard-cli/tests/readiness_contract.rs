use codeguard_core::{
    PreparationEvidenceState, PrerequisiteObservation, PrerequisiteRequirement, ReadinessInput,
    ReadinessState, evaluate_readiness,
};

fn input() -> ReadinessInput {
    ReadinessInput {
        requirements_verified: true,
        observations_verified: true,
        requirements_complete: true,
        now_unix: Some(100),
        requirements: vec![PrerequisiteRequirement {
            id: "java/jdk".into(),
            required: true,
            applicable: Some(true),
            binding_sha256: "a".repeat(64),
        }],
        observations: vec![PrerequisiteObservation {
            id: "java/jdk".into(),
            binding_sha256: "a".repeat(64),
            state: PreparationEvidenceState::Satisfied,
            observed_at: 90,
            expires_at: 200,
        }],
    }
}

#[test]
fn current_required_satisfaction_is_ready_and_optional_failure_does_not_block() {
    let mut request = input();
    assert_eq!(evaluate_readiness(&request).state, ReadinessState::Ready);
    request.requirements.push(PrerequisiteRequirement {
        id: "optional/tool".into(),
        required: false,
        applicable: Some(true),
        binding_sha256: "b".repeat(64),
    });
    request.observations.push(PrerequisiteObservation {
        id: "optional/tool".into(),
        binding_sha256: "b".repeat(64),
        state: PreparationEvidenceState::Missing,
        observed_at: 90,
        expires_at: 200,
    });
    assert_eq!(evaluate_readiness(&request).state, ReadinessState::Ready);
}

#[test]
fn required_missing_incompatible_or_conflict_is_incomplete() {
    for state in [
        PreparationEvidenceState::Missing,
        PreparationEvidenceState::Incompatible,
        PreparationEvidenceState::Conflict,
    ] {
        let mut request = input();
        request.observations[0].state = state;
        let result = evaluate_readiness(&request);
        assert_eq!(result.state, ReadinessState::Incomplete);
        assert_eq!(result.blocked_ids, ["java/jdk"]);
    }
}

#[test]
fn unprobed_unresolved_expired_or_mismatched_evidence_is_unknown() {
    for state in [
        PreparationEvidenceState::Unprobed,
        PreparationEvidenceState::Unresolved,
    ] {
        let mut request = input();
        request.observations[0].state = state;
        assert_eq!(evaluate_readiness(&request).state, ReadinessState::Unknown);
    }
    for state in [
        PreparationEvidenceState::Satisfied,
        PreparationEvidenceState::Missing,
    ] {
        let mut request = input();
        request.observations[0].state = state;
        request.now_unix = Some(200);
        assert_eq!(evaluate_readiness(&request).state, ReadinessState::Unknown);
        request.now_unix = Some(100);
        request.observations[0].binding_sha256 = "b".repeat(64);
        assert_eq!(evaluate_readiness(&request).state, ReadinessState::Unknown);
    }
}

#[test]
fn missing_confirmed_requirement_has_priority_over_unresolved_siblings_and_partial_set() {
    let mut request = input();
    request.requirements_complete = false;
    request.observations[0].state = PreparationEvidenceState::Missing;
    request.requirements.push(PrerequisiteRequirement {
        id: "java/maven".into(),
        required: true,
        applicable: None,
        binding_sha256: "b".repeat(64),
    });
    let result = evaluate_readiness(&request);
    assert_eq!(result.state, ReadinessState::Incomplete);
    assert_eq!(result.blocked_ids, ["java/jdk"]);
    assert_eq!(result.unresolved_ids, ["java/maven"]);
    assert!(
        result
            .reasons
            .contains(&"requirements_set_incomplete".into())
    );
}

#[test]
fn absent_sources_clock_requirement_set_or_applicability_cannot_be_ready() {
    for variant in 0..8 {
        let mut request = input();
        match variant {
            0 => request.requirements_verified = false,
            1 => request.observations_verified = false,
            2 => request.requirements_complete = false,
            3 => request.now_unix = None,
            4 => request.requirements.clear(),
            5 => request.requirements[0].required = false,
            6 => request.requirements[0].applicable = None,
            _ => request.requirements[0].applicable = Some(false),
        }
        assert_eq!(
            evaluate_readiness(&request).state,
            ReadinessState::Unknown,
            "{variant}"
        );
    }
    let mut request = input();
    request.observations[0].state = PreparationEvidenceState::Missing;
    request.observations_verified = false;
    let result = evaluate_readiness(&request);
    assert_eq!(result.state, ReadinessState::Unknown);
    assert!(result.blocked_ids.is_empty());
}

#[test]
fn invalid_ambiguous_or_future_evidence_cannot_select_a_favorable_observation() {
    for variant in 0..8 {
        let mut request = input();
        match variant {
            0 => request.requirements.push(request.requirements[0].clone()),
            1 => request.observations.push(request.observations[0].clone()),
            2 => {
                let mut conflicting = request.observations[0].clone();
                conflicting.state = PreparationEvidenceState::Missing;
                request.observations.push(conflicting);
            }
            3 => request.observations.clear(),
            4 => {
                request.requirements[0].binding_sha256 = "0".repeat(64);
                request.observations[0].binding_sha256 = "0".repeat(64);
            }
            5 => request.observations[0].observed_at = 101,
            6 => request.observations[0].observed_at = 0,
            _ => request.observations[0].expires_at = 90,
        }
        let result = evaluate_readiness(&request);
        assert_eq!(result.state, ReadinessState::Unknown, "{variant}");
        assert_eq!(result.unresolved_ids, ["java/jdk"]);
    }
}

#[test]
fn optional_failures_unknowns_or_stale_observations_do_not_block_required_readiness() {
    for state in [
        PreparationEvidenceState::Missing,
        PreparationEvidenceState::Conflict,
        PreparationEvidenceState::Unprobed,
        PreparationEvidenceState::Unresolved,
    ] {
        let mut request = input();
        request.requirements.push(PrerequisiteRequirement {
            id: "optional/tool".into(),
            required: false,
            applicable: None,
            binding_sha256: "b".repeat(64),
        });
        request.observations.push(PrerequisiteObservation {
            id: "optional/tool".into(),
            binding_sha256: "b".repeat(64),
            state,
            observed_at: 1,
            expires_at: 2,
        });
        let result = evaluate_readiness(&request);
        assert_eq!(result.state, ReadinessState::Ready);
        assert!(result.blocked_ids.is_empty());
        assert!(result.unresolved_ids.is_empty());
    }
}

#[test]
fn ready_preparation_does_not_supply_missing_native_check_evidence() {
    use codeguard_core::{DeliveryDecision, DeliveryInput, ObligationSpec, evaluate_delivery};
    let readiness = evaluate_readiness(&input());
    assert_eq!(readiness.state, ReadinessState::Ready);
    let delivery = DeliveryInput {
        approval_scope: None,
        full_project: true,
        discovery_complete: true,
        trusted_bindings_verified: true,
        gate_time_unix: Some(100),
        policy_revision: None,
        frozen_obligation_ids: vec!["java/lint".into()],
        obligations: vec![ObligationSpec {
            id: "java/lint".into(),
            expected_targets: vec!["src/App.java".into()],
        }],
        evidence: vec![],
        allowlist_dispositions: vec![],
        native_checker_bindings: vec![],
    };
    assert_eq!(
        evaluate_delivery(&delivery).decision,
        DeliveryDecision::Incomplete
    );
    assert_eq!(
        serde_json::to_string(&readiness.state).unwrap(),
        "\"ready\""
    );
    assert!(!serde_json::to_string(&readiness).unwrap().contains("allow"));
}
