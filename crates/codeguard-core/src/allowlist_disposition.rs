//! 本轮原生发现与受保护误报批准的组合证据；是否来自可信边界由调用服务证明。

use serde::{Deserialize, Serialize};

use crate::{ApprovalScope, FalsePositiveIdentity};

/// 单条原生阻断发现的精确误报处置输入；候选文件不能直接构造有效批准。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllowlistDisposition {
    /// 签名或受保护来源核验的批准范围；旧输入缺失时不得应用例外。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_scope: Option<ApprovalScope>,
    /// 从本轮完整原生报告和内容复核得到的身份。
    pub observed_identity: FalsePositiveIdentity,
    /// 独立批准的决策身份。
    pub decision_identity: FalsePositiveIdentity,
    /// 不可覆盖的批准决策 ID。
    pub decision_id: String,
    /// 受保护审批引用。
    pub approval_ref: String,
    /// 本次可信质量策略修订。
    pub approved_policy_revision: String,
    /// 独立审批来源是否经受保护边界验证；本地声明不得置真。
    pub independent_approval_verified: bool,
    /// 受信时钟对本轮输入取证的时间。
    pub observed_at: u64,
    /// 该批准的有限到期时间；宿主须采用决策、签发及密钥等约束中的最早有效边界。
    pub expires_at: u64,
}
