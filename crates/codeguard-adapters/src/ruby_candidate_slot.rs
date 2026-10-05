use crate::ruby_candidate_tool::RubyCandidateTool;
use serde::Deserialize;
/// Ruby 六类别适用性和缺口；对应 OpenSpec 8.16，缺工具不等于类别不适用。
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RubyCandidateSlot {
    /// 检查类别。
    pub category: String,
    /// 全语言适用或需项目构建目标判定。
    pub applicability: String,
    /// 完整能力验收仍缺失。
    pub status: String,
    /// 有明确范围限制的原生工具候选。
    pub tools: Vec<RubyCandidateTool>,
    /// CVE 必须单独核验数据库身份和时效。
    pub requires_database_freshness: bool,
    /// 尚未验收的配置、报告、身份及覆盖条件。
    pub coverage_gaps: Vec<String>,
}
