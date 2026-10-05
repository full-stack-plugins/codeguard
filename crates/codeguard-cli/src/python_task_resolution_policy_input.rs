use codeguard_core::TaskIdentity;
use serde::Deserialize;
/// Python原生语法关闭的专用签名策略；明确绑定配置与lint目标，不能用于白名单。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PythonTaskResolutionPolicyInput {
    /// 策略协议版本。
    pub schema_version: String,
    /// 策略类型，固定为任务解决策略。
    pub report_type: String,
    /// 绑定工作区、任务、检查器与源码范围。
    pub identity: TaskIdentity,
    /// 宿主已批准策略修订。
    pub policy_revision: String,
    /// 首次已消费报告原字节摘要。
    pub original_report_sha256: String,
    /// 宿主保留原样本摘要。
    pub original_source_sha256: String,
    /// 首次固定语法资产摘要。
    pub grammar_sha256: String,
    /// 原生Ruff制品摘要。
    pub tool_sha256: String,
    /// 运行本服务的适配器摘要。
    pub adapter_sha256: String,
    /// 限定原生语法规则。
    pub native_rule_id: String,
    /// 已核对原生工具版本。
    pub native_version: String,
    /// 项目明确lint目标版本。
    pub target_version: String,
    /// 工作区内原生配置相对路径。
    pub configuration_ref: String,
    /// 原生配置原字节摘要。
    pub configuration_sha256: String,
}
