//! 准备计划与 readiness 关联，不包含质量发现或交付授权。
use crate::{PreparationTask, ReadinessOutcome};
use serde::{Deserialize, Serialize};

/// 按本次可信证据生成的任务计划；未核验来源只留下诊断。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparationPlan {
    /// 准备汇总，ready 不等于原生检查通过。
    pub readiness: ReadinessOutcome,
    /// 去重且稳定排序的必要准备任务。
    pub tasks: Vec<PreparationTask>,
}
