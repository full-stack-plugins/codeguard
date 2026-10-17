use codeguard_core::GateImpact;
use guard_protocol::Identifier;
/// 宿主从批准规则冻结的原生ruleId及影响映射，不根据扫描结果生成。无Java对应类型。
#[derive(Clone, Debug)]
pub struct ProjectionNativeRule {
    pub native_rule_id: Identifier,
    pub gate_impact: GateImpact,
}
