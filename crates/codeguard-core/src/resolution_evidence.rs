use crate::{ResolutionCause, TaskIdentity};
/// 由应用边界独立复核后提供的关闭证据，不直接读取项目自报的布尔字段。
#[derive(Clone, Debug)]
pub struct ResolutionEvidence {
    /// 与原任务一致的工作区、检查器和范围。
    pub identity: TaskIdentity,
    /// 原任务观察内容。
    pub original_source_sha256: String,
    /// 本轮实际复检内容。
    pub current_source_sha256: String,
    /// 实际原生报告字节身份。
    pub native_report_sha256: String,
    /// 实际执行且在复检后重新核对的工具身份。
    pub tool_sha256: String,
    /// 运行适配器制品身份。
    pub adapter_sha256: String,
    /// 原规则集身份。
    pub rulepack_sha256: String,
    /// 独立批准的策略字节身份。
    pub policy_sha256: String,
    /// 批准策略修订。
    pub policy_revision: String,
    /// 原检查完整执行；安装或版本探测不满足此条件。
    pub native_completed: bool,
    /// 原规则或同等语法能力实际执行。
    pub original_rule_checked: bool,
    /// 原目标确实被检查，排除或忽略不满足此条件。
    pub target_covered: bool,
    /// 最终写事件前输入与工具仍匹配。
    pub inputs_current: bool,
    /// 由受保护宿主核验策略来源，不能从可写项目报告反向取得。
    pub policy_verified: bool,
    /// 原问题仍然存在。
    pub issue_still_present: bool,
    /// 原生抑制或执行规则发生改变。
    pub suppression_changed: bool,
    /// 目标缺失或移出原范围。
    pub target_removed: bool,
    /// 请求的归因。
    pub cause: ResolutionCause,
}
