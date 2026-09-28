# status 与 task show 的本地只读视图

`codeguard status [path] --format json` 从当前工作区事实生成开放任务计数、finding/blocker 分类、任务摘要和 `next` 建议。未初始化时返回初始化动作；已初始化但空任务时仍要求完整检查，`delivery_decision=not_evaluated`。待同步报告显示 `pending_reports=true` 与同步建议。画像及检查证据的 freshness 固定 `unverified`，不显示虚构的历史 allow。

`codeguard task show <CG-id> [path] --format json` 复用 `next` 的受限 fact/事件核对构造指定任务简报，展示结构化证据引用、范围、步骤、约束、历史和复检命令。任务 Markdown 仅检查存在，不作为指令来源。非法 ID 返回 2；不存在、事实损坏或冲突返回 3。两项查询不扫描、不同步、不领取租约，也不修改工作区。

`status_show_contract` 四项普通集成测试覆盖公开 JSON schema、未初始化、空任务、Ruff blocker、恶意 Markdown、待同步报告及损坏事实。当前视图未实现完整事件父关系证明、可信 policy/tool freshness、历史 gate 收据、依赖图、所有状态和正式关闭/重开，故 OpenSpec 9.7/9.26 保持未完成。
