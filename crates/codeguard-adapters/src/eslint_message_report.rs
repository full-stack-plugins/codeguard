use serde::Deserialize;

/// 原生 ESLint 消息的有限输入模型；修复脚本及建议不参与指令执行。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EslintMessageReport {
    pub(crate) rule_id: Option<String>,
    pub(crate) severity: u8,
    pub(crate) message: String,
    pub(crate) line: Option<u32>,
    pub(crate) column: Option<u32>,
    #[serde(default)]
    pub(crate) fatal: bool,
}
