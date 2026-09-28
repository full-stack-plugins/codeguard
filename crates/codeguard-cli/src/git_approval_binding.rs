//! 将逐跳签名批准绑定与原生代码基线祖先观察串在同一入口。

use codeguard_core::FalsePositiveIdentity;
use codeguard_runtime::observe_git_ancestry;

use crate::approval_snapshot::{SnapshotResolution, verify_and_bind_replacement_chain};
use crate::{
    ApprovalTrustKey, ApprovalVerificationContext, GitApprovalBaselineContext,
    SignedPriorApprovalInput,
};

/// 先核验完整签名/替代链，再逐跳核对前序提交属于后续代码历史。
///
/// 参数为候选、原生身份、当前快照/签名/信任上下文及完整前序链；
/// git_host 仍须由受保护宿主提供。返回成功不签发门禁或证明宿主来源。
#[allow(clippy::too_many_arguments)]
pub fn verify_and_bind_replacement_chain_with_git(
    candidate_bytes: &[u8],
    observed: &FalsePositiveIdentity,
    snapshot_bytes: &[u8],
    envelope_bytes: &[u8],
    trust: &ApprovalTrustKey,
    context: &ApprovalVerificationContext<'_>,
    priors: &[SignedPriorApprovalInput<'_>],
    git_host: &GitApprovalBaselineContext<'_>,
) -> Result<SnapshotResolution, &'static str> {
    let binding = verify_and_bind_replacement_chain(
        candidate_bytes,
        observed,
        snapshot_bytes,
        envelope_bytes,
        trust,
        context,
        priors,
    )?;
    if binding != SnapshotResolution::BoundToPinnedSnapshot {
        return Ok(binding);
    }
    let mut child = context.baseline_commit;
    for prior in priors {
        if !observe_git_ancestry(
            git_host.root,
            git_host.git_tool,
            prior.context.baseline_commit,
            child,
            git_host.deadline,
            git_host.cancelled,
        )? {
            return Err("approval_baseline_not_ancestor");
        }
        child = prior.context.baseline_commit;
    }
    Ok(binding)
}
