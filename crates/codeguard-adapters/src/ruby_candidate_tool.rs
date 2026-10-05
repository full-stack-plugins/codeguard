use serde::Deserialize;
/// Ruby 原生候选工具契约；对应 OpenSpec 8.16，仅供研究，不可执行或签发能力。
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RubyCandidateTool {
    /// 原生工具和检查族的固定标识。
    pub tool_id: String,
    /// 需由项目锁解析的版本，尚未通过制品及兼容性验收。
    pub candidate_version: String,
    /// 候选版本状态。
    pub version_status: String,
    /// 研究命令示例；包含占位符时必须由未来批准计划显式绑定。
    pub candidate_argv: Vec<String>,
    /// Ruby、Bundler、Rails、YARD 或 gem 项目的具体适用条件。
    pub scope: String,
    /// 核对过的原生一手文档。
    pub sources: Vec<String>,
}
