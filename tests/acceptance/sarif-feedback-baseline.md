# SARIF 反馈消费切片验收

`codeguard_cli::sarif_feedback::feedback_sarif` 将**结构有效、来源尚未核验**的 RunReport 投影为 SARIF 2.1.0。它是纯转换器，尚未接入正式 `--format sarif` CLI、MCP 或 Hook，也不证明报告中的批准来自受保护来源。

已覆盖的契约：

- 未完成且零 finding 时，`results=[]` 仍伴随 `executionSuccessful=false`、错误级工具通知、`codeguardDeliveryDecision=incomplete` 和未完成义务计数；不能被零结果误读为检查通过。
- `whitelisted_false_positive` finding 保留在 `results`，显示决策 ID、批准引用和到期时间；由于批准来源尚未核验，标记 `report_claim_unverified`，不产生 SARIF `suppressions`。
- 同轮白名单 finding 与另一活跃阻断 finding 都保留；总体判定为 `deny`，不会因白名单抹掉另一问题。
- 公开投影不复制原生消息、自由文本、路径或证据引用；规则和 finding ID 以哈希表示，原始定位仍在私有证据中。

定向验收：`cargo test -p codeguard-cli --test sarif_feedback_contract --offline`。运行报告的内容身份、独立批准、原生复检和正式全格式门禁仍需后续任务实现，不能据此宣称白名单已真实放行。
