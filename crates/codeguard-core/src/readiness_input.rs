//! 前置汇总输入；信任来源与时间由外部宿主负责核验。
use crate::{PrerequisiteObservation, PrerequisiteRequirement};
use serde::{Deserialize, Serialize};
/// 纯领域判定输入，不是可直接导入项目 JSON 的批准协议。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessInput {
    /// 宿主已经核验要求来自批准策略/适配器及本轮范围。
    pub requirements_verified: bool,
    /// 宿主已经核验观察来自对应原生或受保护配置来源。
    pub observations_verified: bool,
    /// 当前全部适用义务及其前置是否完整确定。
    pub requirements_complete: bool,
    /// 最终判定时的可信 UTC Unix 秒；缺失不能复用旧证据。
    pub now_unix: Option<u64>,
    /// 已确认的要求；完整集合未确定时仍可保留已确认必需阻塞。
    pub requirements: Vec<PrerequisiteRequirement>,
    /// 本轮观察；不得从候选字段或文件生成成功反推有效。
    pub observations: Vec<PrerequisiteObservation>,
}
