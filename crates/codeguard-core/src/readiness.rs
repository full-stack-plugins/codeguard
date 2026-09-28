//! 前置 readiness 纯判定；不访问进程、文件系统或授予质量通过。
use crate::{PreparationEvidenceState, ReadinessInput, ReadinessOutcome, ReadinessState};
use std::collections::{BTreeMap, BTreeSet};

/// 按本轮可信要求、绑定与时钟判定准备状态，返回具体阻塞/未知标识。
/// 可选条件不改变汇总；缺来源、旧证据与零必需集合不能返回 ready。
#[must_use]
pub fn evaluate_readiness(input: &ReadinessInput) -> ReadinessOutcome {
    let mut reasons = BTreeSet::new();
    let mut blocked = BTreeSet::new();
    let mut unresolved = BTreeSet::new();
    if !input.requirements_verified {
        reasons.insert("requirements_source_unverified".into());
    }
    if !input.observations_verified {
        reasons.insert("observations_source_unverified".into());
    }
    if !reasons.is_empty() {
        return outcome(ReadinessState::Unknown, blocked, unresolved, reasons);
    }
    if !input.requirements_complete {
        reasons.insert("requirements_set_incomplete".into());
    }
    let mut counts = BTreeMap::new();
    for requirement in &input.requirements {
        *counts.entry(requirement.id.as_str()).or_insert(0_usize) += 1;
    }
    let mut applicable_count = 0;
    for requirement in input
        .requirements
        .iter()
        .filter(|requirement| requirement.required)
    {
        let id = &requirement.id;
        if !valid_id(id) || counts[id.as_str()] != 1 || !valid_hash(&requirement.binding_sha256) {
            unresolved.insert(id.clone());
            reasons.insert("requirement_identity_invalid".into());
            continue;
        }
        match requirement.applicable {
            Some(false) => continue,
            None => {
                unresolved.insert(id.clone());
                reasons.insert("applicability_unresolved".into());
                continue;
            }
            Some(true) => applicable_count += 1,
        }
        let observations: Vec<_> = input
            .observations
            .iter()
            .filter(|observation| observation.id == *id)
            .collect();
        if observations.len() != 1 {
            unresolved.insert(id.clone());
            reasons.insert("observation_missing_or_ambiguous".into());
            continue;
        }
        let observation = observations[0];
        if observation.binding_sha256 != requirement.binding_sha256 {
            unresolved.insert(id.clone());
            reasons.insert("observation_binding_mismatch".into());
            continue;
        }
        if !input.now_unix.is_some_and(|now| {
            now > 0
                && observation.observed_at > 0
                && observation.observed_at <= now
                && now < observation.expires_at
        }) {
            unresolved.insert(id.clone());
            reasons.insert("observation_not_current".into());
            continue;
        }
        match observation.state {
            PreparationEvidenceState::Satisfied => {}
            PreparationEvidenceState::Missing
            | PreparationEvidenceState::Incompatible
            | PreparationEvidenceState::Conflict => {
                blocked.insert(id.clone());
            }
            PreparationEvidenceState::Unprobed | PreparationEvidenceState::Unresolved => {
                unresolved.insert(id.clone());
                reasons.insert("preparation_not_confirmed".into());
            }
        }
    }
    if applicable_count == 0 {
        reasons.insert("no_confirmed_applicable_requirement".into());
    }
    let state = if !blocked.is_empty() {
        ReadinessState::Incomplete
    } else if !unresolved.is_empty() || !reasons.is_empty() {
        ReadinessState::Unknown
    } else {
        ReadinessState::Ready
    };
    outcome(state, blocked, unresolved, reasons)
}
fn outcome(
    state: ReadinessState,
    blocked: BTreeSet<String>,
    unresolved: BTreeSet<String>,
    reasons: BTreeSet<String>,
) -> ReadinessOutcome {
    ReadinessOutcome {
        state,
        blocked_ids: blocked.into_iter().collect(),
        unresolved_ids: unresolved.into_iter().collect(),
        reasons: reasons.into_iter().collect(),
    }
}
pub(crate) fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'/' | b'.' | b':')
        })
}
pub(crate) fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        && value.bytes().any(|byte| byte != b'0')
}
