# 本地只读 next 简报切片（2026-09-25）

`codeguard next [path] --format human|json` 现在从已初始化工作区读取本地 Ruff finding/blocker 事实，输出 `repair_brief_preview` 0.1。查询不执行项目代码、不领取任务、不修改工作区，也不签发交付通过。未初始化项目给出 init 动作；未消费报告优先要求 `work sync`；无任务且没有新鲜全量门禁证据给出 `verification_required`，不把空队列当作通过。环境 blocker 优先于源码 finding；缺 Ruff 配置给 `needs_decision`，缺工具给准备动作；源码内容与首次记录不同时不给旧代码修复步骤，改为原工具重检。

简报只从受限结构化字段和内置静态步骤生成，记录必须匹配工作区 ID、稳定 ID、摘要、开放状态、任务路径和本地未验证身份；任务 Markdown 自由文本不进入结果。输出包含范围、规则/原因、证据引用、约束、步骤、复检 argv、关闭条件和 `attempt_history_unavailable`，固定 `authority=local_unverified`、`delivery_decision=not_evaluated`。查询 exit 0 仅表示视图成功，损坏事实/消费标记 exit 3。`next_command_contract` 6 项通过，覆盖待同步、缺配置、缺工具优先级、手改任务文本隔离、丢失事实、源码变更和空任务。

这不是 OpenSpec 9.7/9.26 的完整实现：没有跨类别 RunReport、可信政策与覆盖新鲜度、任务依赖、租约、attempt 历史/预算、版本化 recipe、完整 status/show 或原生复检关闭。简报的修复动作仍须由执行者按后续受控协议领取和复检。
