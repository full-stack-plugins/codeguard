use crate::eslint_message_report::EslintMessageReport;
use serde::Deserialize;

/// 原生 ESLint 文件报告；派生反序列化拒绝影响判定的重复字段。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EslintFileReport {
    pub(crate) file_path: String,
    pub(crate) messages: Vec<EslintMessageReport>,
    pub(crate) suppressed_messages: Vec<EslintMessageReport>,
    pub(crate) error_count: u64,
    pub(crate) warning_count: u64,
    pub(crate) fatal_error_count: u64,
    pub(crate) fixable_error_count: u64,
    pub(crate) fixable_warning_count: u64,
}
