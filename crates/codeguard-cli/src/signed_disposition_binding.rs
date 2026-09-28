//! 将签名/快照精确绑定转换为只读预览，不自行授予门禁批准。

use crate::approval_snapshot::{SnapshotResolution, verify_and_bind_candidate};
use crate::false_positive_decision::parse_false_positive_decision_candidate;
use crate::{
    ApprovalTrustKey, ApprovalVerificationContext, BoundFalsePositiveDisposition,
    verify_signed_approval,
};
#[cfg(unix)]
use crate::{
    GitApprovalBaselineContext, SignedPriorApprovalInput,
    verify_and_bind_replacement_chain_with_git,
};
use codeguard_core::{AllowlistDisposition, ApprovalScope, FalsePositiveIdentity};
use sha2::{Digest, Sha256};

/// 验证普通候选的签名及精确身份，生成最早期限受约束的非授权预览。
///
/// 所有参数来源仍须由可信宿主认证；返回值不关闭任务，不自动置独立批准标记。
pub fn bind_signed_false_positive_preview(
    candidate_bytes: &[u8],
    observed: &FalsePositiveIdentity,
    snapshot_bytes: &[u8],
    envelope_bytes: &[u8],
    trust: &ApprovalTrustKey,
    context: &ApprovalVerificationContext<'_>,
) -> Result<BoundFalsePositiveDisposition, &'static str> {
    bounded_candidate(candidate_bytes)?;
    let result = verify_and_bind_candidate(
        candidate_bytes,
        observed,
        snapshot_bytes,
        envelope_bytes,
        trust,
        context,
    )?;
    if result != SnapshotResolution::BoundToPinnedSnapshot {
        return Err("approval_candidate_not_bound");
    }
    build_preview(
        candidate_bytes,
        observed,
        snapshot_bytes,
        envelope_bytes,
        trust,
        context,
    )
}

/// 先核对完整签名与 Git 替代链，再生成当前条目的非授权预览。
///
/// 历史时刻、公钥、仓库及工具来源仍由宿主认证；历史到期不缩短当前合法条目期限。
#[cfg(unix)]
#[allow(clippy::too_many_arguments)]
pub fn bind_signed_false_positive_replacement_preview(
    candidate_bytes: &[u8],
    observed: &FalsePositiveIdentity,
    snapshot_bytes: &[u8],
    envelope_bytes: &[u8],
    trust: &ApprovalTrustKey,
    context: &ApprovalVerificationContext<'_>,
    priors: &[SignedPriorApprovalInput<'_>],
    git_host: &GitApprovalBaselineContext<'_>,
) -> Result<BoundFalsePositiveDisposition, &'static str> {
    bounded_candidate(candidate_bytes)?;
    let result = verify_and_bind_replacement_chain_with_git(
        candidate_bytes,
        observed,
        snapshot_bytes,
        envelope_bytes,
        trust,
        context,
        priors,
        git_host,
    )?;
    if result != SnapshotResolution::BoundToPinnedSnapshot {
        return Err("approval_candidate_not_bound");
    }
    build_preview(
        candidate_bytes,
        observed,
        snapshot_bytes,
        envelope_bytes,
        trust,
        context,
    )
}

fn bounded_candidate(bytes: &[u8]) -> Result<(), &'static str> {
    if bytes.len() > 64 * 1024 {
        Err("approval_candidate_limit_exceeded")
    } else {
        Ok(())
    }
}

fn build_preview(
    candidate_bytes: &[u8],
    observed: &FalsePositiveIdentity,
    snapshot_bytes: &[u8],
    envelope_bytes: &[u8],
    trust: &ApprovalTrustKey,
    context: &ApprovalVerificationContext<'_>,
) -> Result<BoundFalsePositiveDisposition, &'static str> {
    let candidate = parse_false_positive_decision_candidate(candidate_bytes)
        .map_err(|_| "approval_candidate_invalid")?;
    // 不从候选文字恢复签名期限；再取不可变原始输入的验签元数据。
    let verified = verify_signed_approval(envelope_bytes, snapshot_bytes, trust, context)?;
    let preview = AllowlistDisposition {
        approval_scope: Some(ApprovalScope {
            workspace_id: context.workspace_id.into(),
            baseline_commit: context.baseline_commit.into(),
        }),
        observed_identity: observed.clone(),
        decision_identity: candidate.identity,
        decision_id: candidate.id,
        approval_ref: candidate.approval_ref,
        approved_policy_revision: verified.policy_revision().into(),
        independent_approval_verified: false,
        observed_at: context.now_unix.ok_or("approval_clock_unavailable")?,
        expires_at: candidate
            .expires_at
            .min(verified.expires_at())
            .min(trust.valid_until),
    };
    Ok(BoundFalsePositiveDisposition::new(
        preview,
        format!("{:x}", Sha256::digest(candidate_bytes)),
        &verified,
        context,
    ))
}
