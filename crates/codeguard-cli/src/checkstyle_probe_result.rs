use codeguard_adapters::CheckstyleParsed;

/// Checkstyle 本地执行一致性；不证明项目配置、规则批准、JDK 闭包或交付。
pub struct CheckstyleProbeResult {
    /// 输入与报告/退出在本轮前后保持一致，不能单独签发质量通过。
    pub local_coherent: bool,
    /// 稳定未完成原因。
    pub reason: Option<&'static str>,
    /// 私有局部报告证据；失败时不能直接提升为活动 finding。
    pub parsed: Option<CheckstyleParsed>,
}
