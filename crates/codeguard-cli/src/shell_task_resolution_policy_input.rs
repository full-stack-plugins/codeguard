use codeguard_core::TaskIdentity;
use serde::Deserialize;
use serde_json::Value;
/// ShellCheck限定宿主策略；原配置和原规则均不可由复检参数弱化。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ShellTaskResolutionPolicyInput {
    pub schema_version: String,
    pub report_type: String,
    pub identity: TaskIdentity,
    pub policy_revision: String,
    pub original_report_sha256: String,
    pub original_source_sha256: String,
    pub grammar_sha256: Option<String>,
    pub tool_sha256: String,
    pub adapter_sha256: String,
    pub native_rule_id: String,
    pub native_version: String,
    pub dialect: String,
    pub project_configuration: Value,
}
