use codeguard_adapters::EslintParsed;

/// ESLint 输入与报告的局部观察；不证明配置/插件闭包、批准策略或项目覆盖。
pub struct EslintProbeResult {
    /// 本轮已冻结入口/输入、版本、报告与退出的局部一致性，不是质量通过。
    pub local_coherent: bool,
    /// 稳定未完成原因。
    pub reason: Option<&'static str>,
    /// 本轮原生报告解析；不完整时仅供调查，不自动提升为任务或门禁。
    pub parsed: Option<EslintParsed>,
}
