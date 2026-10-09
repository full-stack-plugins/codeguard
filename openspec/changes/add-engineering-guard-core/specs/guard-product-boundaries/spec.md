## Purpose

定义 Partme Guard 当前七项目方案的可观察行为，目标 owner 与迁移见共同设计。当前 change 暂存规划，不表示所有规则在 CodeGuard 实现。

## ADDED Requirements

### Requirement: EG-PROD-001 Seven independent products

系统 SHALL 以 guardcore/specguard/archguard/codeguard/testguard/gitguard/flowguard 七个项目组织产品；对象、方法、分支子能力不另建产品仓；GuardCore 仅提供通用机制，不编译依赖专业域。

#### Scenario: EG-PROD-001 contract behavior
- **WHEN** 开发者向 GuardCore 加入阶段顺序或层次依赖业务规则
- **THEN** 依赖/所有权检查拒绝，规则移入所属专业守卫

### Requirement: EG-PROD-002 Five interfaces

七产品 SHALL 分别提供 CLI、MCP、GitHub Actions、Agent Hook 和版本化 API；共用规范化请求与结果，并逐产品逐入口声明支持/未验收状态。

#### Scenario: EG-PROD-002 contract behavior
- **WHEN** 单个 MCP wrapper 测试通过但 TestGuard API 未执行
- **THEN** TestGuard API 仍未验收，不宣布七项目五入口全部完成

### Requirement: EG-PROD-003 Single migration owner

系统 MUST 为每条现有规则和迁移任务保留唯一 owner；目标仓库接收需求、用例、任务与来源后才切换权威，现有 GitFlow/FlowGuard 逻辑不得被未对齐实现静默替换。

#### Scenario: EG-PROD-003 contract behavior
- **WHEN** Rust 与 Python 对未知历史或 accepted 来源判断不同
- **THEN** 迁移阻断，现有判定保持权威，不双写或择宽放行

### Requirement: EG-PROD-004 Generation and verification

系统 SHALL 将 architecture-plugin/spec-workflow-plugin 的候选设计生成与 ArchGuard/SpecGuard 验证分开；插件不能自动批准自身产物。

#### Scenario: EG-PROD-004 contract behavior
- **WHEN** 设计插件生成新 ADR 并声明已批准
- **THEN** 守卫要求可核验批准，生成成功不计入批准证据

### Requirement: EG-PROD-005 Honest staged delivery

第一阶段 SHALL 完成 GuardCore/ArchGuard/CodeGuard/GitGuard/FlowGuard，并实际运行必需测试和绑定批准需求；第二阶段补全 SpecGuard/TestGuard；缺少已要求的 provider 不能放行。

#### Scenario: EG-PROD-005 contract behavior
- **WHEN** 第一阶段任务契约要求 TestGuard 完整覆盖但 provider 尚缺
- **THEN** 保留阻塞，不能用 native-test 投影冒充完整 TestGuard 或删除要求

### Requirement: EG-PROD-006 Independent and composed use

六专业守卫 SHALL 独立执行本域检查，组合模式由 FlowGuard 消费技术证据，GuardCore 核验来源并执行已批准策略；API/Hook 不能自动取得签发或合入权限。

#### Scenario: EG-PROD-006 contract behavior
- **WHEN** 用户单独执行 archguard check 并得到通过
- **THEN** 仅获得该范围架构结论，不自动推进流程、签发授权或执行 Git 写操作
