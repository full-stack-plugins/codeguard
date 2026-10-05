use crate::ruby_candidate_slot::RubyCandidateSlot;
use serde::Deserialize;
/// Ruby 运行时方言、版本输入和六类别候选档案；对应 OpenSpec 8.16。
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RubyCandidateProfile {
    /// 版本化档案协议。
    pub schema_version: String,
    /// 固定语言 ID。
    pub language: String,
    /// 需要分别验证的运行时，不证明当前原生探针支持这些运行时。
    pub runtime_dialects: Vec<String>,
    /// 运行时组合尚未验收。
    pub runtime_validation: String,
    /// 候选平台集合。
    pub platforms: Vec<String>,
    /// 平台验收状态。
    pub platform_validation: String,
    /// 版本约束可能来自这些输入；此档案不读取或执行它们。
    pub target_version_inputs: Vec<String>,
    /// 版本兼容性剩余缺口。
    pub version_gaps: Vec<String>,
    /// 六类别的完整候选集合。
    pub categories: Vec<RubyCandidateSlot>,
}
