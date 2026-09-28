# 冻结义务清单与交付门禁对照

`DeliveryInput` 现分别接收上游冻结请求契约的 `frozen_obligation_ids` 和实际计划的 `obligations`。领域门禁比较两套 ID：冻结契约中丢失的义务、计划中额外加入的义务、重复或空 ID 都形成 `invalid_ledger_ids`，不能产生 `allow`。已声明但没有有效原生证据的义务继续由原有完整性判定处理。完整发现确认为空项目时，两套清单均为空才可得到 `not_applicable`，仍不签发 allow。

测试先构造冻结清单包含 `java/cve`、计划仅含已完成 Python lint 的反例，旧实现错误返回 `Allow`；接入对照后 `delivery_gate_contract` 和 `check_session_contract` 均要求门禁 `incomplete`、请求退出 3。另验证重复冻结 ID、计划额外义务、混合真实 finding 与未执行义务、真正空项目。领域函数保留已发现问题，不因账本错误隐藏。

这只是**内部一致性约束**。调用方仍须从受保护策略与完整发现独立冻结清单，并在执行前核验其来源；项目可写的同名 JSON 或自行设置 `trusted_bindings_verified=true` 不能成为可信证明。实际 `plan/check` 尚未将该边界接线，OpenSpec 2.4 保持未完成。
