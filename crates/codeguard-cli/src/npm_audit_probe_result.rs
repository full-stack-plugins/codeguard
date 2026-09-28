use codeguard_adapters::{NpmAuditObservation, NpmLockedNode};
/// npm输入及本轮报告的局部观察；不授权数据库、白名单或交付。
pub struct NpmAuditProbeResult {
    /// 本轮显式输入、版本、报告及锁节点关联一致；不是质量通过。
    pub local_coherent: bool,
    /// 稳定未完成原因。
    pub reason: Option<&'static str>,
    /// 本轮原生报告观察；输入变更时不保留可消费结论。
    pub parsed: Option<NpmAuditObservation>,
    /// 本轮锁文件的普通安装节点；不证明依赖边或漏洞范围求值。
    pub locked_nodes: Vec<NpmLockedNode>,
}
