//! pip-audit JSON 的局部解析结果；原生来源和漏洞库覆盖不由解析器证明。

use crate::{PipAuditDependency, PipAuditFinding};

/// 单轮原生 JSON 报告的保守结构化观察。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipAuditObservation {
    /// 数据库身份/时效与全量覆盖在此阶段均未评估。
    pub advisory_coverage: &'static str,
    /// 已解析组件，不包含因跳过而缺失的组件。
    pub dependencies: Vec<PipAuditDependency>,
    /// 逐组件原生漏洞事实。
    pub findings: Vec<PipAuditFinding>,
}
