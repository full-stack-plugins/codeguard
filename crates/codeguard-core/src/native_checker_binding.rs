//! 批准策略中的检查器到原生工具、类别和义务的显式关联。

use serde::{Deserialize, Serialize};

/// 由可信宿主冻结的检查器关联；字段存在不证明其来源已批准。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCheckerBinding {
    /// 白名单精确身份采用的稳定检查器 ID。
    pub checker_id: String,
    /// 原生 finding 实际记录的工具 ID。
    pub tool_id: String,
    /// 当前冻结计划中的具体义务 ID。
    pub obligation_id: String,
    /// 当前批准的检测类别。
    pub category: String,
}
