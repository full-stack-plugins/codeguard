# 请求、计划、报告、工具锁协议验收（2026-09-24）

四类 JSON Schema 的事实源位于 `schemas/check-request.schema.json`、`check-plan.schema.json`、`run-report.schema.json`、`tool-lock.schema.json`。每类均有完整的正例 JSON 和多项反例，分别由 `crates/codeguard-cli/tests/check_request_contract.rs`、`check_plan_contract.rs`、`run_report_contract.rs`、`tool_lock_contract.rs` 作为 Rust 测试数据执行。

协议版本为 `major.minor`；当前仅接受 major 1。兼容 minor 可增加 `extensions` 中 `x-` 前缀的非权威元数据，未知顶层、身份、策略、工具、任务和结果字段一律拒绝，避免把未知的规则弱化或新枚举按旧版 PASS 读取。所有必需枚举与小写 SHA-256 格式由 Rust 消费边界再次核验。

计划解析拒绝遗漏必需义务、重复 ID、悬挂引用和任务 DAG 成环。报告解析拒绝缺失/非法 `gate_impact`、退出码与完成状态冲突、覆盖集合不一致、完整结果没有原生证据、显式语言无结果却成功、局部命令声称交付 allow，以及全部不适用却声称 allow。纯领域门禁也拒绝零目标的 `complete` 义务。有效的全部不适用结果仍可表达为 `not_applicable`，不会误判成违规。

运行报告当前 schema 为 1.2：1.0 继续可读取，1.1 要求项目检查器配置与本次状态分开，1.2 的每条 finding 还必须带非空 `locations`，每条检查器状态带 `build_root`。源码/项目定位使用项目内相对路径，依赖定位使用组件身份；拒绝项目外路径、零行号和控制字符。同一检查器可在不同构建根有不同状态，同一构建根重复登记被拒绝。领域 `FindingLocation` 具有同名序列化类别，后续报告生产可直接映射。`conversation-feedback.schema.json` 单独定义给宿主的对话摘要，不作为门禁来源。

这些 schema 和解析器验证**结构与内部一致性**。它们不证明 JSON 来自可信执行者、批准策略、真实快照或原生工具；实际 gate 仍须另行核验身份、覆盖、来源和全部义务。本阶段未实现统一运行报告生成器、human/SARIF 渲染或跨进程持久报告，因此 OpenSpec 2.5/2.9、5.4 与交付任务仍未完成。
