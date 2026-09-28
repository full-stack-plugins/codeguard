/// 已归属显式文件的 ESLint 原生规则诊断；消息是不可信数据，不是智能体指令。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EslintDiagnostic {
    /// 原生绝对文件路径；调用方还须核验物理输入身份。
    pub path: String,
    /// 原生规则身份，不从消息文本猜测。
    pub rule_id: String,
    /// 原生严重度：1 为 warning，2 为 error。
    pub severity: u8,
    /// 一基行号。
    pub line: u32,
    /// 一基列号。
    pub column: u32,
    /// 原生消息；公开呈现前需脱敏。
    pub message: String,
}
