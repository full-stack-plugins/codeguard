use crate::EslintDiagnostic;

/// ESLint 局部报告观察，不授予有效配置、批准或交付权威。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EslintParsed {
    /// 报告、文件集合、版本和退出契约本地一致；不证明质量通过。
    pub local_coherent: bool,
    /// 可归属的原生规则发现；fatal 或抑制存在时仍保留有效发现。
    pub findings: Vec<EslintDiagnostic>,
    /// 原生抑制消息数量，不是已批准误报白名单数量。
    pub suppressed_count: u64,
    /// 有限未完成原因。
    pub reason: Option<&'static str>,
}
