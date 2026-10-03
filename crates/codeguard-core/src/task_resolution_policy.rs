use crate::TaskIdentity;
use serde::Deserialize;
/// 专用于原生语法任务关闭的签名策略；不是白名单，也不批准全项目交付。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskResolutionPolicy {
    /// 快照协议版本。
    pub schema_version: String,
    /// 限定任务策略的专用类型。
    pub report_type: String,
    /// 独立批准的稳定工作区、检查器与目标身份。
    pub identity: TaskIdentity,
    /// 批准策略修订，必须匹配宿主上下文。
    pub policy_revision: String,
    /// 首次任务报告的精确字节摘要。
    pub original_report_sha256: String,
    /// 原始反例源码的精确字节摘要。
    pub original_source_sha256: String,
    /// 首次疑似观察的固定 grammar 身份，不代表原生覆盖。
    pub grammar_sha256: String,
    /// 批准的原生可执行制品摘要。
    pub tool_sha256: String,
    /// 宿主可执行制品摘要，包含本次适配实现。
    pub adapter_sha256: String,
    /// 允许复检的原生语法规则。
    pub native_rule_id: String,
    /// 限定原生工具版本。
    pub native_version: String,
}
