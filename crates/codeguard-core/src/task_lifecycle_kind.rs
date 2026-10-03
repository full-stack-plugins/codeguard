use crate::ResolutionCause;
use serde::{Deserialize, Serialize};
/// 有效生命周期动作；尝试结束和工具安装不属于关闭动作。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub enum TaskLifecycleKind {
    /// 原检查观察到问题。
    Observed,
    /// 解决证据引用；可信性由独立应用边界复核。
    Resolved {
        /// 解决归因。
        cause: ResolutionCause,
        /// 绑定解决证据原字节的身份。
        evidence_sha256: String,
    },
    /// 当前输入需要重新核验或误报调查，不能继续沿用旧关闭。
    VerificationRequired {
        /// 当前原生观察的原字节摘要。
        evidence_sha256: String,
        /// 有界诊断原因码。
        reason_code: String,
    },
    /// 原问题重新被观察到。
    Reopened,
}
