# 交付门禁领域基线（2026-09-24）

`codeguard-core::evaluate_delivery` 目前只负责纯领域判定。输入先声明完整项目义务，再逐项核对结果、目标集合、门禁影响和身份验证状态。局部类别请求返回 `not_evaluated`；空项目完整发现返回 `not_applicable`；缺结果、覆盖差异、重复义务、未知结果、未确认身份或发现不完整返回 `incomplete`。未完成与已确认违规同时存在时保留阻断发现 ID，并以未完成为优先结论。

`crates/codeguard-cli/tests/delivery_gate_contract.rs` 用合成结构测试上述规则，仅证明函数判定；其中 `trusted_bindings_verified=true` 是测试输入，**不是**真实内容、批准策略、工具或原生报告的核验证据。当前 CLI 没有调用此函数，也没有签发项目 allow。OpenSpec 2.4 保持未完成，直到真实发现、身份核验、覆盖与适配器结果接通并有端到端反例验收。

后续 `codeguard-core::conclude_check` 将冻结义务账本与逐项结果合并：未执行的声明义务生成显式 incomplete，覆盖或账本缺口也提升请求结论为 exit 3；已确认 finding 保留，取消与内部故障保持更高优先级。`crates/codeguard-cli/tests/check_session_contract.rs` 的 5 项结构测试和 core 的“违规 + 缺工具”测试通过。该协调器仍只消费合成或上层注入的证明，尚无真实 CLI、快照及适配器链，不改变 2.3/2.4 的未完成边界。
