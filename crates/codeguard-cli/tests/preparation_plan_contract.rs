use codeguard_core::{
    PreparationAction, PreparationEvidenceState, PrerequisiteObservation, PrerequisiteRequirement,
    ReadinessInput, ReadinessState, plan_preparation,
};

fn input(state: PreparationEvidenceState) -> ReadinessInput {
    ReadinessInput {
        requirements_verified: true,
        observations_verified: true,
        requirements_complete: true,
        now_unix: Some(100),
        requirements: vec![PrerequisiteRequirement {
            id: "java:runtime".into(),
            required: true,
            applicable: Some(true),
            binding_sha256: "a".repeat(64),
        }],
        observations: vec![PrerequisiteObservation {
            id: "java:runtime".into(),
            binding_sha256: "a".repeat(64),
            state,
            observed_at: 90,
            expires_at: 110,
        }],
    }
}

#[test]
fn confirmed_blockers_produce_specific_preparation_actions() {
    for (state, action) in [
        (
            PreparationEvidenceState::Missing,
            PreparationAction::RestoreMissingPrerequisite,
        ),
        (
            PreparationEvidenceState::Incompatible,
            PreparationAction::ResolveCompatibility,
        ),
        (
            PreparationEvidenceState::Conflict,
            PreparationAction::ResolveConflict,
        ),
    ] {
        let plan = plan_preparation(&input(state));
        assert_eq!(plan.readiness.state, ReadinessState::Incomplete);
        assert_eq!(plan.tasks.len(), 1);
        let task = &plan.tasks[0];
        assert_eq!(task.prerequisite_id, "java:runtime");
        assert_eq!(task.action, action);
        assert_eq!(task.confirmed_state, Some(state));
        assert_eq!(task.binding_sha256, "a".repeat(64));
        assert!(!task.automatic_execution_authorized);
        assert!(!task.instruction.is_empty());
        assert!(!task.close_condition.is_empty());
    }
}

#[test]
fn unknown_evidence_requires_reverification_without_reusing_old_diagnosis() {
    for alteration in 0..5 {
        let mut data = input(PreparationEvidenceState::Missing);
        match alteration {
            0 => data.observations.clear(),
            1 => data.observations[0].expires_at = 100,
            2 => data.observations[0].binding_sha256 = "b".repeat(64),
            3 => data.observations.push(data.observations[0].clone()),
            _ => data.requirements[0].applicable = None,
        }
        let plan = plan_preparation(&data);
        assert_eq!(plan.readiness.state, ReadinessState::Unknown);
        assert_eq!(plan.tasks.len(), 1);
        assert_eq!(
            plan.tasks[0].action,
            PreparationAction::ReverifyPrerequisite
        );
        assert_eq!(plan.tasks[0].confirmed_state, None);
    }
}

#[test]
fn optional_inapplicable_and_satisfied_conditions_do_not_create_repair_tasks() {
    for variation in 0..3 {
        let mut data = input(PreparationEvidenceState::Missing);
        match variation {
            0 => data.requirements[0].required = false,
            1 => data.requirements[0].applicable = Some(false),
            _ => data.observations[0].state = PreparationEvidenceState::Satisfied,
        }
        assert!(plan_preparation(&data).tasks.is_empty());
    }
}

#[test]
fn unverified_sources_and_invalid_or_duplicate_ids_cannot_create_actionable_tasks() {
    for variation in 0..5 {
        let mut data = input(PreparationEvidenceState::Missing);
        match variation {
            0 => data.requirements_verified = false,
            1 => data.observations_verified = false,
            2 => data.requirements[0].id = "bad\nstatus=ready".into(),
            3 => data.requirements.push(data.requirements[0].clone()),
            _ => data.requirements[0].binding_sha256 = "0".repeat(64),
        }
        let plan = plan_preparation(&data);
        assert_eq!(plan.readiness.state, ReadinessState::Unknown);
        assert!(plan.tasks.is_empty());
        assert!(!plan.readiness.reasons.is_empty());
    }
}

#[test]
fn stable_task_keys_survive_diagnosis_and_binding_changes() {
    let first = plan_preparation(&input(PreparationEvidenceState::Missing));
    let mut changed = input(PreparationEvidenceState::Conflict);
    changed.requirements[0].binding_sha256 = "b".repeat(64);
    changed.observations[0].binding_sha256 = "b".repeat(64);
    let second = plan_preparation(&changed);
    assert_eq!(first.tasks[0].task_key, second.tasks[0].task_key);
    assert_ne!(
        first.tasks[0].binding_sha256,
        second.tasks[0].binding_sha256
    );
    assert_ne!(first.tasks[0].action, second.tasks[0].action);
    assert_eq!(second, plan_preparation(&changed));
}

#[test]
fn task_order_is_stable_and_plan_cannot_produce_quality_allow() {
    let mut data = input(PreparationEvidenceState::Missing);
    let mut requirement = data.requirements[0].clone();
    requirement.id = "a:config".into();
    let mut observation = data.observations[0].clone();
    observation.id = requirement.id.clone();
    data.requirements.push(requirement);
    data.observations.push(observation);
    let first = plan_preparation(&data);
    data.requirements.reverse();
    data.observations.reverse();
    assert_eq!(first, plan_preparation(&data));
    assert_eq!(first.tasks[0].prerequisite_id, "a:config");
    let json = serde_json::to_value(first).unwrap();
    assert!(json.get("delivery_decision").is_none());
    assert!(json.get("findings").is_none());
}

#[test]
fn incomplete_inventory_preserves_known_blocker_and_unprobed_sibling() {
    let mut data = input(PreparationEvidenceState::Missing);
    data.requirements_complete = false;
    let mut sibling = data.requirements[0].clone();
    sibling.id = "java:configuration".into();
    data.requirements.push(sibling);
    let plan = plan_preparation(&data);
    assert_eq!(plan.readiness.state, ReadinessState::Incomplete);
    assert!(
        plan.readiness
            .reasons
            .contains(&"requirements_set_incomplete".into())
    );
    assert_eq!(plan.tasks.len(), 2);
    assert_eq!(
        plan.tasks[0].action,
        PreparationAction::ReverifyPrerequisite
    );
    assert_eq!(
        plan.tasks[1].action,
        PreparationAction::RestoreMissingPrerequisite
    );
    let mut json = serde_json::to_value(&plan).unwrap();
    json["delivery_decision"] = "allow".into();
    assert!(serde_json::from_value::<codeguard_core::PreparationPlan>(json).is_err());
}
