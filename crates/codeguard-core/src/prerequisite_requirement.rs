//! 受保护策略与选中适配器共同确定的准备条件。
use serde::{Deserialize, Serialize};
/// 可信宿主冻结的单项前置要求；项目配置候选不能直接取得该权威。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrerequisiteRequirement {
    /// 本次范围内稳定且唯一的前置标识。
    pub id: String,
    /// 是否由批准义务确认为必需；可选条件不改变汇总。
    pub required: bool,
    /// 适用性；None 表示条件尚未解析。
    pub applicable: Option<bool>,
    /// 绑定策略、目标、工具及配置输入的原始证据身份摘要。
    pub binding_sha256: String,
}
