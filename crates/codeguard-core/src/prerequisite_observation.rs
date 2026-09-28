//! 前置观察与本轮冻结身份的关联，不使用 mtime 或本地声明授权。
use crate::PreparationEvidenceState;
use serde::{Deserialize, Serialize};
/// 由宿主核验来源的前置观察；有效期与要求身份必须精确匹配。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrerequisiteObservation {
    /// 对应前置标识。
    pub id: String,
    /// 当时观察所绑定的原始输入摘要。
    pub binding_sha256: String,
    /// 准备观察状态，与质量检查发现分离。
    pub state: PreparationEvidenceState,
    /// 可信时钟的观察时刻。
    pub observed_at: u64,
    /// 来源和策略限制的最早失效时刻。
    pub expires_at: u64,
}
