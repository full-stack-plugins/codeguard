# 误报处置的领域门禁基线

`codeguard-core::evaluate_delivery` 现在同时保留原始阻断 finding、经精确匹配处置的 finding 与仍活跃的阻断 finding。完整项目、完整义务和可信绑定前提下，唯一阻断都获得独立批准且未过期时，结论为 `allow_with_exceptions`；同轮仍有未处置阻断为 `deny`，义务/覆盖/批准冲突为 `incomplete`。`conclude_check` 的请求结论使用独立的 `passed_with_exceptions`，不会把例外显示成普通 `passed`；取消或内部故障不能留下交付 `allow`。

领域输入中的 `trusted_bindings_verified` 和 `independent_approval_verified` 必须由受保护服务核验，项目工作区候选文件不得直接置真。当前 CLI 尚未接入该受保护服务，因此本基线不意味着真实项目可获得 `allow_with_exceptions`。

验收：`cargo test -p codeguard-cli --test delivery_gate_contract --test check_session_contract --offline`。样本覆盖唯一例外、同轮真实阻断、义务未完成、过期/目标失配/自批、重复 finding、同一决策 ID 跨 finding 复用、取消及内部故障。

新增碰撞反例：两个不同原生规则和义务意外产生同一 finding ID，旧实现按 ID 集合去重，使针对其中一条规则的批准同时消除另一条阻断并返回 `allow_with_exceptions`。现重复 ID 使两个受影响义务无效，白名单对该 ID 不生效，结论为 `incomplete`；即使未提供白名单，单个义务内部的重复 ID 也不允许签发 `allow`。原始 finding 仍由输入报告保留，领域 gate 显示活跃阻断 ID。
