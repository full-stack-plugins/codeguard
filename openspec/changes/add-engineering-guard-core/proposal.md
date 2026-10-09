## Why

Partme Guard 采用七个独立项目：六个专业守卫与一个公共内核。原先把公共内核和架构规则放进 CodeGuard 的规划会混淆产品职责，需要在实施前调整；现有六个插件/引擎仓的事实与兼容义务继续保留。

## What Changes

- 公共机制归独立 `guardcore`；系统/领域/对象/方法规则归 `archguard`；CodeGuard 专注代码质量、安全、静态分析和复杂度。
- 补充 `specguard` 和 `testguard`，分别治理需求/规格与测试充分性/证据；`gitguard` 与 `flowguard` 通过明确迁移复用现有插件能力。
- 七项目目标均支持 CLI、MCP、GitHub Actions、Agent Hook、API，使用共同契约、运行、证据及授权机制；专业规则不进入 GuardCore。
- 第一阶段交付 GuardCore/ArchGuard/CodeGuard/GitGuard/FlowGuard，第二阶段补全 SpecGuard/TestGuard；保留全部 P0—P4 范围和原语言/平台验收。
- 本 change 作为尚未创建目标仓库时的临时规划事实源。EG-M01 负责逐项移交、保持 ID 与来源，禁止双账本；不是把七产品代码放进 CodeGuard。

## Capabilities

### New Capabilities

- `engineering-contracts`: 通用批准快照、规则包引用和冻结义务。
- `engineering-evidence`: 身份、版本、覆盖、存储及证据时效。
- `engineering-gate`: 通用策略执行、独立授权及受控执行机制。
- `architecture-governance`: 归 ArchGuard 的系统/领域/对象/方法规则。
- `specification-governance`: 归 SpecGuard 的需求基线、一致性、追溯与验收质量。
- `test-governance`: 归 TestGuard 的测试计划、执行覆盖、契约/回归与可信证据。
- `guard-product-boundaries`: 七项目边界、五类入口和单一所有权迁移。

### Modified Capabilities

无。已有 CodeGuard 迁移 change 继续拥有原生检查、修复和旧协议；各插件原有行为继续。需要改变既有运行行为时，在其所属事实源增量处理。

## Impact

新产品的实现目标在各独立仓库；本轮只编辑规划，不创建 Git 仓库、初始化规格体系、安装依赖或修改 runtime/manifest。现有插件分别保持宿主适配、事实生产与评审职责。

总入口：[Partme Guard](../../../docs/engineering-guard/README.md)；当前临时任务账本：[tasks](tasks.md)；[任务移交及目标 owner](../../../docs/engineering-guard/ownership.md)。
