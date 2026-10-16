# Partme Guard

**AI-Native Software Engineering Guardrails**

一套面向 AI 原生软件工程的确定性约束与质量治理工具体系。

产品决策：**7 个独立项目，6 个专业守卫 + 1 个公共内核**。2026-10-17 核对：七个实现仓库已经存在，公共内核的实际仓库名为 `guardengine`，承担本规划中 GuardCore 的角色。现有批次已有实现和 CI 证据，但总体任务尚未完成。原先缺失的插件任务已按既有规划重建：113项稳定任务、66条需求、94个局部场景及43个端到端用例的引用可解析，尚不等于实现验收。详见[实施状态与规划缺口](execution-status.md)。

## 七项目及唯一职责

| 产品 / 目标 Git 仓库 | 核心问题 | 规划 |
|---|---|---|
| GuardCore / `guardengine`（规划逻辑名 `guardcore`） | 如何统一执行、记录证据和核验授权？不持有领域业务规则 | [内核](projects/guardcore.md) |
| SpecGuard / `specguard` | 做什么？需求完整性、基线、一致性、验收标准 | [规格守卫](projects/specguard.md) |
| ArchGuard / `archguard` | 如何设计？系统、领域、对象、方法与依赖 | [架构守卫](projects/archguard.md) |
| CodeGuard / `codeguard` | 代码是否符合工程质量要求？ | [代码守卫](projects/codeguard.md) |
| TestGuard / `testguard` | 哪些行为已被实际验证、证据是否充分？ | [测试守卫](projects/testguard.md) |
| GitGuard / `gitguard` | 变更、合入和版本是否安全可控？ | [Git 守卫](projects/gitguard.md) |
| FlowGuard / `flowguard` | 当前证据和批准是否允许流程推进？ | [流程守卫](projects/flowguard.md) |

TestGuard 不承诺证明任意程序完全正确；报告精确记录测试范围、遗漏和证据等级。对象/方法/分支等能力是项目内部模块，不继续拆分产品仓库。

## 阅读入口

| 内容 | 文档 |
|---|---|
| 七项目架构与六个既有仓库的迁移 | [项目边界与迁移](product-architecture.md) |
| 统一技术、组件与工具 | [技术选型](technology.md) |
| 契约、报告、证据、决策和授权 | [协议](protocol.md) |
| 五种接入面的边界与资格 | [CLI / MCP / Actions / Hook / API](interfaces.md) |
| 阶段、任务归属及依赖 | [交付路线](roadmap.md)、[任务所有权](ownership.md) |
| 可验证行为、负向场景和可追溯关系 | [验收矩阵](acceptance.md)、[追溯矩阵](traceability.md) |
| 现有本地实现及验证边界 | [仓库基线](repository-baseline.md)、[本轮验证](planning-validation.md) |
| 暂存的跨项目正式增量规格 | [proposal](../../openspec/changes/add-engineering-guard-core/proposal.md)、[design](../../openspec/changes/add-engineering-guard-core/design.md)、[tasks](../../openspec/changes/add-engineering-guard-core/tasks.md) |

## 规划保存与迁移纪律

七个实现仓库已经建立，但整体任务尚未完成逐条移交，因此继续使用现有 OpenSpec change 作为**临时的跨项目规格与任务账本**。其目录名只为保持链接兼容，不意味着 GuardCore 或 ArchGuard 归 CodeGuard 实现。项目卡片是设计视图，不能另设任务完成状态。各现有插件的集成需求应由本仓原生规格拥有，FlowGuard 插件继续使用既有 Superpowers；尚不存在的引用保持未解析，不视为已移交。

通过 EG-M01 逐条核对并移交需求、场景、任务和未完成证据，保持原 ID 与来源映射，旧位置改为迁移指针。接收方接受前不删除原内容；切换后不得双写。各仓实际采用的规格体系须单独发现，不能从全局 CLI 安装状态推断，也不能因有实现目录就认定移交完成。

旧六仓方案中“公共内核与架构守卫放在 CodeGuard workspace”“只有四类守卫”的设计已由七项目决策替代。CodeGuard 保留现有代码检查与兼容义务；公共内核在实际仓库 `guardengine`，架构规则在 `archguard`。文档中的 GuardCore、`guardcore` 和 EG 任务 ID 保留为逻辑角色及历史追踪名称，不触发仓库重命名。
