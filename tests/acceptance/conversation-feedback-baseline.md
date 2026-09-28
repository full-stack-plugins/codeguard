# 智能体反馈摘要基线（2026-09-24）

`codeguard-cli::conversation_feedback` 从结构已校验的 `RunReport` 生成 JSON/human 摘要，分别列出项目检查器的 `configured/missing/invalid/unknown`、本次 `not_run/passed/findings/tool_error`、已确认规则 ID、未完成义务和复检 argv。反馈明确标记 `structure_validated_provenance_unverified`；调用方仍须将报告绑定真实 CLI 运行，不能仅凭可解析 JSON 宣称检查已经执行。

原生 finding `message`、检查器 `summary` 和报告内自由文本 `next_actions` 不进入该对话摘要；人类格式仅引用规则 ID、工具 ID，并把原生消息留作私有证据。回归样本证明恶意提示、凭据样例和任意 shell 建议不会穿透两个格式。这个限制牺牲了当前摘要中的详细自然语言诊断；后续应由有来源的规则包提供公开规则解释和位置，不能把原始工具文本直接交给智能体。

反馈协议见 `schemas/conversation-feedback.schema.json`；其顶层字段与实际生成物有契约测试。RunReport 1.2 的源码、项目或依赖定位可随规则 ID 显示，检查器状态按构建根区分；旧版无位置报告仍显示空位置列表。RunReport 1.3 的误报处置可在 0.2 版反馈中独立展示，且标记批准来源未核验；详见 `tests/acceptance/run-report-allowlist-protocol.md`。测试：`cargo test --offline -q -p codeguard-cli --test run_report_contract` 共 14 项通过。此处是反馈转换器，不是宿主接线；完整 `check`、报告生产、Codex/ZCode/Kimi 对话显示和失败同步回退仍待实施，不能勾选 OpenSpec 11.16。完整 workspace 与 Clippy 结果以最终验收记录为准。
