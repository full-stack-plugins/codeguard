## Purpose

为 Partme Guard 提供 engineering-contracts 的可观察行为契约，统一已批准约束、当前候选和独立验证证据的关联，并明确未知、失败、兼容模式与越权情况下不得伪造工程通过的边界。

## ADDED Requirements

### Requirement: EG-CON-001 Approved snapshot

系统 SHALL 引用项目已采用的规格、ADR、Git 与流程规则，生成不可变有效契约快照并记录来源摘要、批准引用和适用范围；不能复制出第二套需求或阶段事实源。

#### Scenario: EG-CON-001 contract behavior
- **WHEN** 任务绑定批准版本后候选提交修改了规则
- **THEN** 本任务仍使用批准快照，候选规则不能放行自身

### Requirement: EG-CON-002 Rule composition

系统 MUST 检查组织锁定项、项目约束和任务授权之间的冲突；所有规则显式声明 ENFORCE/REVIEW/ADVISE、严重级别、适用条件、验证器与例外策略。

#### Scenario: EG-CON-002 contract behavior
- **WHEN** 项目试图覆盖锁定规则或引用不存在的验证器
- **THEN** 拒绝生成可执行计划并提供具体规则错误，不降级为空规则

### Requirement: EG-CON-003 Frozen obligations

系统 SHALL 在执行前固定动作、义务 ID、预期目标集合、必需验证和输入摘要；工具返回结果不得反向决定检查义务。

#### Scenario: EG-CON-003 contract behavior
- **WHEN** 扫描器漏掉一个必需模块但剩余模块零诊断
- **THEN** 结果为 incomplete，缺失模块明确列出

### Requirement: EG-CON-004 Contract evolution

系统 SHALL 对契约变更生成新修订，保留旧版本、批准与影响分析；已绑定任务只有经明确迁移才采用新版本，适用证据随变化失效。

#### Scenario: EG-CON-004 contract behavior
- **WHEN** 任务运行中契约新版本发布
- **THEN** 原任务不静默切换；显式迁移后受影响旧证据失效

### Requirement: EG-CON-005 Safe discovery

系统 SHALL 分别报告 CLI、Skills、项目初始化、语言/构建/平台及验证器能力；只读发现与计划不能安装工具、建索引、fetch 或初始化项目。

#### Scenario: EG-CON-005 contract behavior
- **WHEN** OPA 或语言工具不存在但用户只请求 plan
- **THEN** 列出准备阻塞且不改变文件、网络或工具安装状态
