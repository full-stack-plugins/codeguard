//! 误报决策文档的候选解析；可解析不代表可信来源已批准。

use codeguard_core::{FalsePositiveIdentity, valid_false_positive_identity};
use serde::Deserialize;

/// 结构完整但来源仍未核验的误报裁定候选。
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FalsePositiveDecisionCandidate {
    /// 决策文档协议版本。
    pub schema_version: String,
    /// 必须为 false_positive；真实风险另行处置。
    pub kind: String,
    /// 决策稳定标识。
    pub id: String,
    /// 修订候选引用的旧决策；1.1 必填，1.0 不得出现。
    pub replaces_decision_id: Option<String>,
    /// 可精确匹配的一条原生发现。
    pub identity: FalsePositiveIdentity,
    /// 结构化误报原因代码。
    pub reason_code: String,
    /// 人工裁定依据；仅作为数据展示。
    pub rationale: String,
    /// 最小复现或原生证据的受控引用。
    pub reproducer_ref: String,
    /// 声称采用的策略修订；必须由独立策略边界再核验。
    pub approved_policy_revision: String,
    /// 声称采用的批准引用；此字段本身不构成授权。
    pub approval_ref: String,
    /// 评审人身份描述；不能代替签发验证。
    pub reviewer: String,
    /// 创建时刻，UTC Unix 秒。
    pub created_at: u64,
    /// 失效时刻，UTC Unix 秒；信任边界还须核验当前时间和最长有效期。
    pub expires_at: u64,
}

/// 外部策略边界提供的时间和修订上下文；字段存在本身不证明来源可信。
#[derive(Clone, Copy, Debug)]
pub struct CandidateContext<'a> {
    /// 当前受保护策略修订；调用方须核验其来源。
    pub policy_revision: Option<&'a str>,
    /// 本次判定的 UTC Unix 秒；调用方须保证时钟可靠。
    pub now_unix: Option<u64>,
    /// 当前受保护策略允许的最大有效秒数。
    pub max_lifetime_seconds: Option<u64>,
}

/// 结构和时间筛选结果；任何值都不能单独签发例外批准。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateResolution {
    /// 没有相同精确身份的候选。
    NoMatch,
    /// 唯一候选通过本地检查，仍须独立核验批准与策略来源。
    ReadyForAuthorityCheck { index: usize },
    /// 本次原生发现身份不完整。
    InvalidObservation,
    /// 某候选在解析后被改坏或包含非法身份。
    InvalidCandidate,
    /// 同一发现有多条候选，不能自行挑选较宽松者。
    ConflictingCandidates,
    /// 时钟、策略修订或期限上限缺少可信输入。
    UnverifiableContext,
    /// 候选绑定了不同的策略修订。
    PolicyRevisionMismatch,
    /// 裁定的创建时刻在本次时钟之后。
    NotYetValid,
    /// 当前时间已达到或超过失效时刻。
    Expired,
    /// 候选有效期超过当前策略上限。
    LifetimeExceeded,
}

/// 严格解析单条误报候选；返回值不能直接用于调整交付门禁。
///
/// 参数 `bytes` 是待解析 JSON；返回结构仅证明形状和精确目标合法，
/// 不证明文档来自受保护策略或批准仍有效。
pub fn parse_false_positive_decision_candidate(
    bytes: &[u8],
) -> Result<FalsePositiveDecisionCandidate, String> {
    let candidate: FalsePositiveDecisionCandidate =
        serde_json::from_slice(bytes).map_err(|error| format!("误报决策格式错误：{error}"))?;
    if !valid_candidate(&candidate) {
        return Err("误报决策协议、精确身份、裁定依据或期限无效".into());
    }
    Ok(candidate)
}

/// 对一组候选做精确匹配、期限和冲突筛选；成功仍需可信审批边界复核。
///
/// `observed` 必须来自本次原生报告，`candidates` 是严格解析的候选，`context`
/// 必须由上层可信策略/时钟边界提供。此函数既不读取本地批准标志，也不改变 gate。
#[must_use]
pub fn classify_false_positive_candidates(
    observed: &FalsePositiveIdentity,
    candidates: &[FalsePositiveDecisionCandidate],
    context: CandidateContext<'_>,
) -> CandidateResolution {
    if !valid_false_positive_identity(observed) {
        return CandidateResolution::InvalidObservation;
    }
    if candidates
        .iter()
        .any(|candidate| !valid_candidate(candidate))
    {
        return CandidateResolution::InvalidCandidate;
    }
    // 稳定 finding ID 重复时，即使目标字节不同也不能挑中恰好匹配观察的一条。
    if candidates
        .iter()
        .filter(|candidate| candidate.identity.finding_id == observed.finding_id)
        .count()
        > 1
    {
        return CandidateResolution::ConflictingCandidates;
    }
    let matches: Vec<_> = candidates
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.identity == *observed)
        .collect();
    let [(index, candidate)] = matches.as_slice() else {
        return if matches.is_empty() {
            CandidateResolution::NoMatch
        } else {
            CandidateResolution::ConflictingCandidates
        };
    };
    let (Some(policy_revision), Some(now), Some(max_lifetime)) = (
        context.policy_revision,
        context.now_unix,
        context.max_lifetime_seconds,
    ) else {
        return CandidateResolution::UnverifiableContext;
    };
    if policy_revision.is_empty() || now == 0 || max_lifetime == 0 {
        return CandidateResolution::UnverifiableContext;
    }
    if candidate.approved_policy_revision != policy_revision {
        return CandidateResolution::PolicyRevisionMismatch;
    }
    if now < candidate.created_at {
        return CandidateResolution::NotYetValid;
    }
    if now >= candidate.expires_at {
        return CandidateResolution::Expired;
    }
    if candidate.expires_at - candidate.created_at > max_lifetime {
        return CandidateResolution::LifetimeExceeded;
    }
    CandidateResolution::ReadyForAuthorityCheck { index: *index }
}

fn valid_candidate(candidate: &FalsePositiveDecisionCandidate) -> bool {
    ((candidate.schema_version == "1.0" && candidate.replaces_decision_id.is_none())
        || (candidate.schema_version == "1.1"
            && candidate
                .replaces_decision_id
                .as_ref()
                .is_some_and(|prior| exact_token(prior) && prior != &candidate.id)))
        && candidate.kind == "false_positive"
        && exact_token(&candidate.id)
        && valid_false_positive_identity(&candidate.identity)
        && reason_code(&candidate.reason_code)
        && !candidate.rationale.trim().is_empty()
        && exact_token(&candidate.reproducer_ref)
        && exact_token(&candidate.approved_policy_revision)
        && exact_token(&candidate.approval_ref)
        && exact_token(&candidate.reviewer)
        && candidate.created_at > 0
        && candidate.expires_at > candidate.created_at
}

fn exact_token(value: &str) -> bool {
    !value.trim().is_empty()
        && value
            .chars()
            .all(|ch| !ch.is_control() && ch != '*' && ch != '?')
}

fn reason_code(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}
