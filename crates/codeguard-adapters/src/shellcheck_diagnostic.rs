use serde::Serialize;
/// 已绑定冻结 stdin 的 ShellCheck 原生诊断；原消息和修复替换不进入对话。
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct ShellCheckDiagnostic {
    /// 原生 SC 规则编号，不从消息文本推断。
    pub rule_id: String,
    /// 原生严重度。
    pub severity: String,
    /// 一基行号。
    pub line: u32,
    /// 一基原生字符列，json1 中制表符计为一个字符。
    pub column: u32,
    /// 原生结束行号。
    pub end_line: u32,
    /// 原生结束列号，允许零长度范围。
    pub end_column: u32,
}
