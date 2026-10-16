use codeguard_core::{DeliveryGate, Finding};
use guard_protocol::{Digest, GuardResult};
/// 原件与新协议并列保存的无升权投影；解析成功不表示工程通过。无Java对应类型。
#[derive(Clone, Debug)]
pub struct ProjectionResult<'a> {
    pub(super) raw: &'a [u8],
    pub(super) raw_digest: Digest,
    pub(super) gate: DeliveryGate,
    pub(super) findings: Vec<Finding>,
    pub(super) results: Vec<GuardResult>,
    pub(super) issues: Vec<String>,
    pub(super) unbound: Vec<super::UnboundNativeDiagnostic>,
}
impl ProjectionResult<'_> {
    /// 返回未修改的原始字节，供私有原件保存或旧消费者继续使用。
    pub fn original_bytes(&self) -> &[u8] {
        self.raw
    }
    /// 返回原始字节摘要，不使用JSON重序列化摘要冒充原件。
    pub fn original_digest(&self) -> &Digest {
        &self.raw_digest
    }
    /// 显式固定适配器版本，独立于源报告版本。
    pub fn adapter_version(&self) -> &str {
        "codeguard/engineering-projection-0.1.0"
    }
    /// 无论源报告或上下文声明什么，均不由本适配器升级信任。
    pub fn trust_level(&self) -> &str {
        "UNVERIFIED"
    }
    /// 缺上下文或协议错误必须保持incomplete。
    pub fn status(&self) -> &str {
        if self.issues.is_empty() {
            "projected_unverified"
        } else {
            "incomplete"
        }
    }
    /// 返回局部旧门禁重算值，永不由候选trusted字段开启通过。
    pub fn gate(&self) -> &DeliveryGate {
        &self.gate
    }
    /// 保留已确认的原生发现，包括不完整请求中的发现。
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }
    /// 读取共享协议结果声明；仍需独立核实生产者、运行、输入和批准。
    pub fn results(&self) -> &[GuardResult] {
        &self.results
    }
    /// 未绑定诊断只用于审计/修复定位，不产生通过或已确认违规。
    pub fn unbound_diagnostics(&self) -> &[super::UnboundNativeDiagnostic] {
        &self.unbound
    }
    /// 明确的准备/解析缺口，不从空结果推断通过。
    pub fn issues(&self) -> &[String] {
        &self.issues
    }
}
