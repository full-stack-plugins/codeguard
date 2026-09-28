//! 原生检查证据中的单条发现。

use crate::FalsePositiveIdentity;
use serde::{Deserialize, Serialize};

/// 经已批准策略判定的门禁影响；与原生严重度分开保存。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateImpact {
    /// 有确定依据阻断本次请求。
    Blocking,
    /// 有确定依据记录但不阻断本次请求。
    NonBlocking,
    /// 严重度、规则或策略依据不足，不能推断为通过或违规。
    Undetermined,
}

/// 发现的可核查定位；原生绝对路径由适配器转成项目内相对路径。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FindingLocation {
    /// 项目内源码位置，行列为一基数。
    Source {
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        line: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        column: Option<u32>,
    },
    /// 依赖组件及可选清单位置。
    Dependency {
        component: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        version: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
    },
    /// 无行号的项目级配置位置。
    Project { path: String },
}

/// 检查器确认的发现；未知严重度保持原值，不推断为低危。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    /// 同一规则、位置和内容下的稳定发现 ID。
    pub id: String,
    /// 原生工具规则标识。
    pub native_rule_id: String,
    /// 来源工具标识，不按相似文案跨工具抵消。
    pub tool_id: String,
    /// 原生严重度字符串，包括 UNKNOWN。
    pub severity: String,
    /// 经批准策略解析的影响；不得仅由原生严重度字符串猜测。
    pub gate_impact: GateImpact,
    /// 脱敏后的公开说明。
    pub message: String,
    /// 此发现所属义务。
    pub obligation_id: String,
    /// 宿主从本轮原生结果及目标内容复核后冻结的精确身份；缺失时不能应用白名单。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_identity: Option<FalsePositiveIdentity>,
    /// 本次可核查的位置；新报告协议要求至少一处，第一处为精确白名单的主定位。
    #[serde(default)]
    pub locations: Vec<FindingLocation>,
}
