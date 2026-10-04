//! 将候选误报裁定绑定到预先固定的批准快照；摘要来源仍由受保护宿主负责。

use std::collections::HashSet;

use codeguard_core::{AllowlistTarget, FalsePositiveIdentity};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::false_positive_decision::{
    CandidateContext, CandidateResolution, FalsePositiveDecisionCandidate,
    classify_false_positive_candidates, parse_false_positive_decision_candidate,
};

const MAX_REVISION_HOPS: usize = 32;

/// 逐跳验签并验证替代链；每跳宿主上下文须独立固定。
///
/// 历史时间来自受保护审核记录，公钥撤销状态须刷新。本接口不证明基线 Git 祖先关系或交付批准。
pub fn verify_and_bind_replacement_chain(
    candidate_bytes: &[u8],
    observed: &FalsePositiveIdentity,
    snapshot_bytes: &[u8],
    envelope_bytes: &[u8],
    trust: &crate::ApprovalTrustKey,
    context: &crate::ApprovalVerificationContext<'_>,
    priors: &[crate::SignedPriorApprovalInput<'_>],
) -> Result<SnapshotResolution, &'static str> {
    if priors.is_empty() {
        return Ok(SnapshotResolution::RevisionChainTruncated);
    }
    if priors.len() > MAX_REVISION_HOPS {
        return Ok(SnapshotResolution::RevisionChainTooLong);
    }
    let current = crate::verify_signed_approval(envelope_bytes, snapshot_bytes, trust, context)?;
    check_signed_snapshot_revision(snapshot_bytes, current.policy_revision())?;
    check_signed_candidate_lifetime(candidate_bytes, &current, context)?;
    let mut child_sequence = current.revision_sequence();
    let mut child_issued = current.issued_at();
    let mut pins = Vec::with_capacity(priors.len());
    for prior in priors {
        if prior.context.workspace_id != context.workspace_id {
            return Err("approval_prior_workspace_mismatch");
        }
        let verified = crate::verify_signed_approval(
            prior.envelope_bytes,
            prior.snapshot_bytes,
            prior.trust,
            &prior.context,
        )?;
        check_signed_snapshot_revision(prior.snapshot_bytes, verified.policy_revision())?;
        check_signed_candidate_lifetime(prior.decision_bytes, &verified, &prior.context)?;
        if verified.revision_sequence() >= child_sequence
            || verified.issued_at() > child_issued
            || prior
                .context
                .now_unix
                .is_none_or(|time| time > child_issued)
        {
            return Err("approval_revision_order_invalid");
        }
        let candidate = parse_false_positive_decision_candidate(prior.decision_bytes)
            .map_err(|_| "approval_prior_candidate_invalid")?;
        let time = prior.context.now_unix.ok_or("approval_clock_unavailable")?;
        if time < candidate.created_at || time >= candidate.expires_at {
            return Err("approval_prior_candidate_not_current_at_review");
        }
        child_sequence = verified.revision_sequence();
        child_issued = verified.issued_at();
        pins.push(verified.snapshot_sha256().to_owned());
    }
    // 只使用逐跳验签得到的摘要，历史候选不接受调用者自行传入有利的 pin。
    let inputs: Vec<PriorApprovalInput<'_>> = priors
        .iter()
        .zip(&pins)
        .map(|(prior, pin)| PriorApprovalInput {
            decision_bytes: prior.decision_bytes,
            snapshot_bytes: prior.snapshot_bytes,
            expected_snapshot_sha256: Some(pin.as_str()),
        })
        .collect();
    Ok(bind_replacement_chain(
        candidate_bytes,
        observed,
        snapshot_bytes,
        Some(current.snapshot_sha256()),
        &inputs,
        context.now_unix,
    ))
}

// 快照中较宽松的期限不能扩大该跳已核验的签名窗口或宿主上限。
fn check_signed_candidate_lifetime(
    bytes: &[u8],
    verified: &crate::VerifiedApprovalSnapshot,
    context: &crate::ApprovalVerificationContext<'_>,
) -> Result<(), &'static str> {
    // 非法候选仍由严格绑定器返回原有 InvalidCandidate，不提升其身份。
    if let Ok(candidate) = parse_false_positive_decision_candidate(bytes) {
        if candidate.expires_at > verified.expires_at()
            || candidate
                .expires_at
                .checked_sub(candidate.created_at)
                .is_none_or(|lifetime| lifetime > context.max_lifetime_seconds)
        {
            return Err("approval_candidate_lifetime_outside_signature");
        }
    }
    Ok(())
}

fn check_signed_snapshot_revision(bytes: &[u8], revision: &str) -> Result<(), &'static str> {
    let snapshot: ApprovalSnapshot =
        serde_json::from_slice(bytes).map_err(|_| "approval_snapshot_invalid")?;
    if !valid_snapshot(&snapshot) {
        return Err("approval_snapshot_invalid");
    }
    if snapshot.policy_revision != revision {
        return Err("approval_snapshot_revision_mismatch");
    }
    Ok(())
}

/// 先核验宿主公钥下的签名，再严格绑定普通候选；不签发项目交付。
///
/// 参数中的公钥和上下文须来自受保护宿主，不能由项目文件或本次 PR 自选。
/// 返回值仍要求原生身份、策略及检查完整性核验；替代候选须另走完整签名修订链。
pub fn verify_and_bind_candidate(
    candidate_bytes: &[u8],
    observed: &FalsePositiveIdentity,
    snapshot_bytes: &[u8],
    envelope_bytes: &[u8],
    trust: &crate::ApprovalTrustKey,
    context: &crate::ApprovalVerificationContext<'_>,
) -> Result<SnapshotResolution, &'static str> {
    let verified = crate::verify_signed_approval(envelope_bytes, snapshot_bytes, trust, context)?;
    check_signed_snapshot_revision(snapshot_bytes, verified.policy_revision())?;
    check_signed_candidate_lifetime(candidate_bytes, &verified, context)?;
    Ok(bind_candidate_to_snapshot(
        candidate_bytes,
        observed,
        snapshot_bytes,
        Some(verified.snapshot_sha256()),
        context.now_unix,
    ))
}

/// 批准快照中的单条候选字节身份。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PinnedDecision {
    id: String,
    sha256: String,
}

/// 受保护策略边界在扫描前固定的候选集合。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovalSnapshot {
    schema_version: String,
    policy_revision: String,
    max_lifetime_seconds: u64,
    decisions: Vec<PinnedDecision>,
    revoked_decision_ids: Option<Vec<String>>,
}

/// 前一策略修订的决策及快照输入；摘要来源由调用方的受保护边界负责。
#[derive(Clone, Copy, Debug)]
pub struct PriorApprovalInput<'a> {
    /// 前一条已批准决策的原始字节。
    pub decision_bytes: &'a [u8],
    /// 前一策略快照的原始字节。
    pub snapshot_bytes: &'a [u8],
    /// 扫描前由可信边界固定的前序快照摘要。
    pub expected_snapshot_sha256: Option<&'a str>,
}

struct ValidatedPrior {
    decision: FalsePositiveDecisionCandidate,
    snapshot: ApprovalSnapshot,
}

/// 快照绑定结果；即使绑定成功，也不能证明外部摘要来源可信或直接签发交付。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotResolution {
    /// 调用者没有提供扫描前固定的快照摘要。
    Unpinned,
    /// 快照字节与预期摘要不一致。
    PinMismatch,
    /// 快照协议错误或包含重复、宽泛条目。
    InvalidSnapshot,
    /// 候选文档非法。
    InvalidCandidate,
    /// 本次原生发现与候选的精确身份不同。
    ObservationMismatch,
    /// 缺少本次可核验的时钟上下文。
    UnverifiableContext,
    /// 候选绑定的策略修订与快照不同。
    PolicyRevisionMismatch,
    /// 候选尚未生效。
    NotYetValid,
    /// 候选已经失效。
    Expired,
    /// 候选期限超过快照允许的上限。
    LifetimeExceeded,
    /// 候选 ID 或字节不在预先固定的快照中。
    AbsentFromSnapshot,
    /// 此决策 ID 已被同一受保护策略修订显式撤销。
    Revoked,
    /// 替代候选的旧决策未在同次快照中撤销。
    ReplacementPredecessorNotRevoked,
    /// 替代候选缺少可比较的旧决策字节。
    ReplacementPriorIdentityUnverified,
    /// 替代候选改变了稳定 finding 或精确目标的归属。
    ReplacementPriorIdentityMismatch,
    /// 替代绑定接口收到普通候选。
    ReplacementCandidateRequired,
    /// 前序快照没有由调用方提供固定摘要。
    PriorSnapshotUnpinned,
    /// 前序快照与固定摘要不符。
    PriorSnapshotPinMismatch,
    /// 前序快照格式或集合无效。
    PriorSnapshotInvalid,
    /// 旧决策字节无效或与前序策略修订不符。
    PriorDecisionInvalid,
    /// 旧决策字节不在前序快照内。
    PriorDecisionAbsentFromSnapshot,
    /// 旧决策在前序快照中已撤销。
    PriorDecisionRevoked,
    /// 旧决策有效期超出前序快照允许的上限。
    PriorDecisionLifetimeExceeded,
    /// 新旧快照使用相同策略修订，不能证明一次修订转换。
    PriorPolicyRevisionConflict,
    /// 修订链没有追溯到不再替代其它决策的根候选。
    RevisionChainTruncated,
    /// 修订链在根候选后又提供多余前序记录。
    RevisionChainUnexpectedExtra,
    /// 决策 ID 或策略修订在同一链中重复。
    RevisionChainCycle,
    /// 前序修订链超过受控上限。
    RevisionChainTooLong,
    /// 仅证明候选字节与固定快照一致；仍需宿主核验 pin 来源及原生 finding。
    BoundToPinnedSnapshot,
}

/// 比对候选字节、快照摘要、策略修订和期限。
///
/// `expected_snapshot_sha256` 只能由受保护的宿主/CI 提供；项目文件、环境变量或
/// 同次 PR 自行计算的值不构成批准。`observed` 须由本次原生检查结果构造。
#[must_use]
pub fn bind_candidate_to_snapshot(
    candidate_bytes: &[u8],
    observed: &FalsePositiveIdentity,
    snapshot_bytes: &[u8],
    expected_snapshot_sha256: Option<&str>,
    now_unix: Option<u64>,
) -> SnapshotResolution {
    bind_with_prior(
        candidate_bytes,
        None,
        observed,
        snapshot_bytes,
        expected_snapshot_sha256,
        now_unix,
    )
}

/// 比对替代候选与旧决策的精确归属，再绑定同轮撤销快照。
///
/// `prior_candidate_bytes` 的历史可信来源仍须由受保护边界核验；本函数
/// 不签发交付许可。
#[must_use]
pub fn bind_replacement_to_snapshot(
    candidate_bytes: &[u8],
    prior_candidate_bytes: &[u8],
    observed: &FalsePositiveIdentity,
    snapshot_bytes: &[u8],
    expected_snapshot_sha256: Option<&str>,
    now_unix: Option<u64>,
) -> SnapshotResolution {
    bind_with_prior(
        candidate_bytes,
        Some(prior_candidate_bytes),
        observed,
        snapshot_bytes,
        expected_snapshot_sha256,
        now_unix,
    )
}

/// 同时核对旧决策所属的前序快照与新快照的撤销/替代关系。
///
/// 返回绑定成功仍只表示提供的字节和摘要相互一致；两个摘要必须由受保护
/// CI/宿主在对应修订边界预先固定，项目内自算摘要不能取得批准权威。
#[must_use]
pub fn bind_replacement_with_prior_snapshot(
    candidate_bytes: &[u8],
    observed: &FalsePositiveIdentity,
    snapshot_bytes: &[u8],
    expected_snapshot_sha256: Option<&str>,
    prior: PriorApprovalInput<'_>,
    now_unix: Option<u64>,
) -> SnapshotResolution {
    let result = bind_replacement_to_snapshot(
        candidate_bytes,
        prior.decision_bytes,
        observed,
        snapshot_bytes,
        expected_snapshot_sha256,
        now_unix,
    );
    if result != SnapshotResolution::BoundToPinnedSnapshot {
        return result;
    }
    let current_snapshot: ApprovalSnapshot =
        serde_json::from_slice(snapshot_bytes).expect("已通过绑定校验的当前快照");
    match validate_prior(prior, &current_snapshot.policy_revision) {
        Ok(_) => SnapshotResolution::BoundToPinnedSnapshot,
        Err(reason) => reason,
    }
}

/// 逐跳核对替代链直至普通根决策；不从本地摘要推断批准权威。
///
/// `priors` 从当前候选的直接前驱开始，顺序追溯到根候选。每一跳的
/// 快照摘要与修订顺序仍须由受保护发布边界独立证明。
#[must_use]
pub fn bind_replacement_chain(
    candidate_bytes: &[u8],
    observed: &FalsePositiveIdentity,
    snapshot_bytes: &[u8],
    expected_snapshot_sha256: Option<&str>,
    priors: &[PriorApprovalInput<'_>],
    now_unix: Option<u64>,
) -> SnapshotResolution {
    if priors.is_empty() {
        return SnapshotResolution::RevisionChainTruncated;
    }
    if priors.len() > MAX_REVISION_HOPS {
        return SnapshotResolution::RevisionChainTooLong;
    }
    let result = bind_replacement_to_snapshot(
        candidate_bytes,
        priors[0].decision_bytes,
        observed,
        snapshot_bytes,
        expected_snapshot_sha256,
        now_unix,
    );
    if result != SnapshotResolution::BoundToPinnedSnapshot {
        return result;
    }
    let mut child =
        parse_false_positive_decision_candidate(candidate_bytes).expect("已通过绑定校验的替代候选");
    let mut child_snapshot: ApprovalSnapshot =
        serde_json::from_slice(snapshot_bytes).expect("已通过绑定校验的当前快照");
    let mut decision_ids = HashSet::from([child.id.clone()]);
    let mut policy_revisions = HashSet::from([child_snapshot.policy_revision.clone()]);
    for prior in priors {
        let Some(expected_id) = child.replaces_decision_id.as_ref() else {
            return SnapshotResolution::RevisionChainUnexpectedExtra;
        };
        let validated = match validate_prior(*prior, &child_snapshot.policy_revision) {
            Ok(validated) => validated,
            Err(reason) => return reason,
        };
        if &validated.decision.id != expected_id
            || !same_finding_scope(&validated.decision.identity, &child.identity)
        {
            return SnapshotResolution::ReplacementPriorIdentityMismatch;
        }
        if !child_snapshot
            .revoked_decision_ids
            .as_ref()
            .is_some_and(|ids| ids.contains(expected_id))
        {
            return SnapshotResolution::ReplacementPredecessorNotRevoked;
        }
        if !decision_ids.insert(validated.decision.id.clone())
            || !policy_revisions.insert(validated.snapshot.policy_revision.clone())
        {
            return SnapshotResolution::RevisionChainCycle;
        }
        if validated
            .decision
            .replaces_decision_id
            .as_ref()
            .is_some_and(|id| decision_ids.contains(id))
        {
            return SnapshotResolution::RevisionChainCycle;
        }
        child = validated.decision;
        child_snapshot = validated.snapshot;
    }
    if child.replaces_decision_id.is_some() {
        SnapshotResolution::RevisionChainTruncated
    } else {
        SnapshotResolution::BoundToPinnedSnapshot
    }
}

fn validate_prior(
    prior: PriorApprovalInput<'_>,
    current_revision: &str,
) -> Result<ValidatedPrior, SnapshotResolution> {
    let Some(prior_pin) = prior.expected_snapshot_sha256 else {
        return Err(SnapshotResolution::PriorSnapshotUnpinned);
    };
    if !valid_sha256(prior_pin)
        || format!("{:x}", Sha256::digest(prior.snapshot_bytes)) != prior_pin
    {
        return Err(SnapshotResolution::PriorSnapshotPinMismatch);
    }
    let Ok(prior_snapshot): Result<ApprovalSnapshot, _> =
        serde_json::from_slice(prior.snapshot_bytes)
    else {
        return Err(SnapshotResolution::PriorSnapshotInvalid);
    };
    if !valid_snapshot(&prior_snapshot) {
        return Err(SnapshotResolution::PriorSnapshotInvalid);
    }
    let Ok(prior_decision) = parse_false_positive_decision_candidate(prior.decision_bytes) else {
        return Err(SnapshotResolution::PriorDecisionInvalid);
    };
    if prior_decision.approved_policy_revision != prior_snapshot.policy_revision {
        return Err(SnapshotResolution::PriorDecisionInvalid);
    }
    if prior_decision.expires_at - prior_decision.created_at > prior_snapshot.max_lifetime_seconds {
        return Err(SnapshotResolution::PriorDecisionLifetimeExceeded);
    }
    let prior_digest = format!("{:x}", Sha256::digest(prior.decision_bytes));
    if !prior_snapshot
        .decisions
        .iter()
        .any(|decision| decision.id == prior_decision.id && decision.sha256 == prior_digest)
    {
        return Err(SnapshotResolution::PriorDecisionAbsentFromSnapshot);
    }
    if prior_snapshot
        .revoked_decision_ids
        .as_ref()
        .is_some_and(|ids| ids.contains(&prior_decision.id))
    {
        return Err(SnapshotResolution::PriorDecisionRevoked);
    }
    if prior_snapshot.policy_revision == current_revision {
        return Err(SnapshotResolution::PriorPolicyRevisionConflict);
    }
    Ok(ValidatedPrior {
        decision: prior_decision,
        snapshot: prior_snapshot,
    })
}

fn bind_with_prior(
    candidate_bytes: &[u8],
    prior_candidate_bytes: Option<&[u8]>,
    observed: &FalsePositiveIdentity,
    snapshot_bytes: &[u8],
    expected_snapshot_sha256: Option<&str>,
    now_unix: Option<u64>,
) -> SnapshotResolution {
    let Some(expected) = expected_snapshot_sha256 else {
        return SnapshotResolution::Unpinned;
    };
    if !valid_sha256(expected) || format!("{:x}", Sha256::digest(snapshot_bytes)) != expected {
        return SnapshotResolution::PinMismatch;
    }
    let Ok(snapshot): Result<ApprovalSnapshot, _> = serde_json::from_slice(snapshot_bytes) else {
        return SnapshotResolution::InvalidSnapshot;
    };
    if !valid_snapshot(&snapshot) {
        return SnapshotResolution::InvalidSnapshot;
    }
    let Ok(candidate) = parse_false_positive_decision_candidate(candidate_bytes) else {
        return SnapshotResolution::InvalidCandidate;
    };
    if prior_candidate_bytes.is_some() && candidate.replaces_decision_id.is_none() {
        return SnapshotResolution::ReplacementCandidateRequired;
    }
    if snapshot
        .revoked_decision_ids
        .as_ref()
        .is_some_and(|ids| ids.contains(&candidate.id))
    {
        return SnapshotResolution::Revoked;
    }
    if candidate
        .replaces_decision_id
        .as_ref()
        .is_some_and(|prior| {
            !snapshot
                .revoked_decision_ids
                .as_ref()
                .is_some_and(|revoked| revoked.contains(prior))
        })
    {
        return SnapshotResolution::ReplacementPredecessorNotRevoked;
    }
    if let Some(prior_id) = &candidate.replaces_decision_id {
        let Some(prior_bytes) = prior_candidate_bytes else {
            return SnapshotResolution::ReplacementPriorIdentityUnverified;
        };
        let Ok(prior) = parse_false_positive_decision_candidate(prior_bytes) else {
            return SnapshotResolution::ReplacementPriorIdentityUnverified;
        };
        if prior.id != *prior_id || !same_finding_scope(&prior.identity, &candidate.identity) {
            return SnapshotResolution::ReplacementPriorIdentityMismatch;
        }
    }
    let resolution = classify_false_positive_candidates(
        observed,
        std::slice::from_ref(&candidate),
        CandidateContext {
            policy_revision: Some(&snapshot.policy_revision),
            now_unix,
            max_lifetime_seconds: Some(snapshot.max_lifetime_seconds),
        },
    );
    match resolution {
        CandidateResolution::ReadyForAuthorityCheck { .. } => {}
        CandidateResolution::UnverifiableContext => return SnapshotResolution::UnverifiableContext,
        CandidateResolution::PolicyRevisionMismatch => {
            return SnapshotResolution::PolicyRevisionMismatch;
        }
        CandidateResolution::NotYetValid => return SnapshotResolution::NotYetValid,
        CandidateResolution::Expired => return SnapshotResolution::Expired,
        CandidateResolution::LifetimeExceeded => return SnapshotResolution::LifetimeExceeded,
        CandidateResolution::NoMatch => return SnapshotResolution::ObservationMismatch,
        _ => return SnapshotResolution::InvalidCandidate,
    }
    let candidate_sha256 = format!("{:x}", Sha256::digest(candidate_bytes));
    if snapshot
        .decisions
        .iter()
        .any(|decision| decision.id == candidate.id && decision.sha256 == candidate_sha256)
    {
        SnapshotResolution::BoundToPinnedSnapshot
    } else {
        SnapshotResolution::AbsentFromSnapshot
    }
}

pub(crate) fn same_finding_scope(
    prior: &FalsePositiveIdentity,
    replacement: &FalsePositiveIdentity,
) -> bool {
    prior.finding_id == replacement.finding_id
        && prior.checker_id == replacement.checker_id
        && prior.native_rule_id == replacement.native_rule_id
        && prior.category == replacement.category
        && match (&prior.target, &replacement.target) {
            (
                AllowlistTarget::Source { path: left, .. },
                AllowlistTarget::Source { path: right, .. },
            ) => left == right,
            (
                AllowlistTarget::Dependency {
                    component: left_component,
                    version: left_version,
                    advisory_id: left_advisory,
                    ..
                },
                AllowlistTarget::Dependency {
                    component: right_component,
                    version: right_version,
                    advisory_id: right_advisory,
                    ..
                },
            ) => {
                left_component == right_component
                    && left_version == right_version
                    && left_advisory == right_advisory
            }
            _ => false,
        }
}

fn valid_snapshot(snapshot: &ApprovalSnapshot) -> bool {
    if !matches!(snapshot.schema_version.as_str(), "1.0" | "1.1")
        || !safe_token(&snapshot.policy_revision)
        || snapshot.max_lifetime_seconds == 0
        || (snapshot.schema_version == "1.0" && snapshot.revoked_decision_ids.is_some())
        || (snapshot.schema_version == "1.1" && snapshot.revoked_decision_ids.is_none())
    {
        return false;
    }
    if let Some(revoked) = &snapshot.revoked_decision_ids {
        let mut ids = HashSet::new();
        if !revoked
            .iter()
            .all(|id| safe_token(id) && ids.insert(id.as_str()))
        {
            return false;
        }
    }
    let mut ids = HashSet::new();
    let mut digests = HashSet::new();
    snapshot.decisions.iter().all(|decision| {
        safe_token(&decision.id)
            && valid_sha256(&decision.sha256)
            && ids.insert(decision.id.as_str())
            && digests.insert(decision.sha256.as_str())
    })
}

fn safe_token(value: &str) -> bool {
    !value.trim().is_empty()
        && value
            .chars()
            .all(|ch| !ch.is_control() && ch != '*' && ch != '?')
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
