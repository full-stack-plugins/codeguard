use crate::NpmAuditComponent;
/// npm机器报告的局部一致观察，不证明漏洞库新鲜度、锁归属或覆盖。
#[derive(Debug)]
pub struct NpmAuditObservation {
    /// 报告一致性不能证明漏洞库覆盖，解析阶段固定为not_evaluated。
    pub advisory_coverage: &'static str,
    /// 原生声明的依赖总数；类别计数可能重叠，不重新计算依赖图。
    pub native_dependency_total: u64,
    /// 保留原生易受影响组件和间接关系，不据退出1构造错误。
    pub components: Vec<NpmAuditComponent>,
}
