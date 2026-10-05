use codeguard_core::TaskIdentity;
use serde::Deserialize;
/// 专用于原生语法任务关闭的签名策略；不是白名单，也不批准全项目交付。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskResolutionPolicyInput {
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
    /// 首次 WASM 观察的 grammar 身份；原生首次任务为 None，不虚构语法资产。
    pub grammar_sha256: Option<String>,
    /// 批准的原生可执行制品摘要。
    pub tool_sha256: String,
    /// Go专用策略绑定的同SDK格式器制品摘要；旧版本不得携带此字段。
    pub gofmt_sha256: Option<String>,
    /// Go辅助制品规范路径与字节的联合摘要；项目不能自行替换批准身份。
    pub companion_binding_sha256: Option<String>,
    /// 宿主可执行制品摘要，包含本次适配实现。
    pub adapter_sha256: String,
    /// 允许复检的原生语法规则。
    pub native_rule_id: String,
    /// 限定原生工具版本。
    pub native_version: String,
}
