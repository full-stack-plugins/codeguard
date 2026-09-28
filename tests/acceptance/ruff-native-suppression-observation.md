# Ruff 源码注释抑制的原生对照与复检防逃逸（2026-09-25）

固定 Ruff 0.16.8 的配置项目在同一工具、配置、源码和总截止时间内，先运行普通 JSON 扫描，再运行 `ruff check --no-cache --ignore-noqa --output-format json <file>`。Rust runtime 校验两次报告、路径归属、工具/配置/源码前后字节身份和诊断包含关系；对照失败时局部状态为 `incomplete`，普通扫描的有效诊断仍保留。对照原文只写私有日志，公开反馈仅有摘要、被源码注释抑制的诊断数和规则 ID。

公开 Python lint 反馈为 0.10，本地报告为 0.8。只有普通扫描无活动 finding 且对照发现注释抑制时，文件状态为 `suppressed`，不显示 `passed`。抑制诊断不被造成本轮活动 finding 或普通修复任务；`lint python` 仍退出 3、`delivery_decision=not_evaluated`。逐文件配置忽略可能同时作用于普通与对照扫描，必须结合 `rule_settings.per_file_ignores_present` 解释；本对照不是受批准规则覆盖证明。

对已有 F401 任务，若修改为 `# noqa: F401`，`task verify` 的观察为 `suppression_requires_review`，任务仍 open；`next` 提示检查抑制与批准策略，不称原问题已修复。真正移除问题且无抑制时仍仅为 `candidate_absent_unverified_policy`。本地事件及复检预览协议升为 0.2，旧事件只读兼容。工作台同步器拒绝 0.8 报告缺失或伪造的抑制观察。

再用真实 Ruff 验证两种配置逃逸：把原 F401 改为只选 E501 时，复检为 `rule_coverage_requires_review`；为目标文件加入 `per-file-ignores = { 'app.py' = ['F401'] }` 时，复检为 `suppression_requires_review`。两者都保留原任务 open，由 `next` 指向规则覆盖或抑制核查。由于设置观察只表示存在逐文件忽略，不解析其精确目标，第二种分类是保守的“需要核查”，并非断言具体配置已获批或确实遮蔽该 finding。

原生样本覆盖 `# noqa`、正常 F401、逐文件配置忽略、复检前后任务状态及白名单候选边界；全工作区测试与 OpenSpec 验证见变更 `verification.md`。工具/规则/策略可信批准、完整源码抑制语义、其它语言和全格式交付门禁仍未实现。

同步器还要求 `suppressed` 仅出现在 0.8 报告中，且注释抑制计数非零、活动 finding 为空；普通 `passed` 必须没有活动 finding 及抑制差额。缺失观察、计数和规则列表矛盾、用 `suppressed` 隐藏活动诊断、用 `passed` 隐藏抑制都拒绝导入。合法的 `suppressed` 文件可同步观察，但不会凭它创建“已修复”任务或签发质量通过。
