## Purpose

定义 Partme Guard 当前七项目方案的可观察行为，目标 owner 与迁移见共同设计。当前 change 暂存规划，不表示所有规则在 CodeGuard 实现。

## ADDED Requirements

### Requirement: EG-TEST-001 Frozen test plan

TestGuard SHALL 在运行前根据验收义务、变更范围和批准回归基线生成测试计划，记录测试 ID/环境/工具/预期目标，不从实际执行的子集倒推覆盖。

#### Scenario: EG-TEST-001 contract behavior
- **WHEN** 只运行一个通过用例后上传全量标签
- **THEN** 拒绝全量通过，列出未执行义务和实际分母

### Requirement: EG-TEST-002 Real execution evidence

TestGuard MUST 区分测试失败、环境故障、跳过、隔离、超时和无测试，保留原始报告与退出并绑定当前输入/候选/工具链。

#### Scenario: EG-TEST-002 contract behavior
- **WHEN** 测试命令 exit 0 但零用例或全部 skipped
- **THEN** 报告不足/未完成，不计行为验收成功

### Requirement: EG-TEST-003 Coverage and adequacy

TestGuard SHALL 分开记录语句/分支覆盖、需求覆盖、契约覆盖及适用的变异测试；阈值来自批准策略，不能将高覆盖率或 mutation score 当作完全正确证明。

#### Scenario: EG-TEST-003 contract behavior
- **WHEN** 覆盖率超过阈值但必需异常场景没有断言
- **THEN** 该义务未满足；指出缺口而非宣布实现正确

### Requirement: EG-TEST-004 Contract and regression

TestGuard SHALL 管理契约/集成/回归与状态机/并发测试执行证据；领域不变量定义归 ArchGuard，影响图只帮助排序，不能静默删除批准回归。

#### Scenario: EG-TEST-004 contract behavior
- **WHEN** ArchGuard 要求取消任务并发不变量，影响图未找到相关测试
- **THEN** 必需测试仍执行或缺失阻断，unknown 图不缩减计划

### Requirement: EG-TEST-005 Trusted and reproducible tests

TestGuard SHALL 使用受保护验证器/必需测试基线核验结果；候选可提出测试变更，降低断言/阈值须独立审查；flaky 重试与 quarantine 原因保留，不重试到绿掩盖失败。

#### Scenario: EG-TEST-005 contract behavior
- **WHEN** 候选替换测试脚本或多次重试仅提交最后 PASS
- **THEN** 来源/运行序列核验拒绝，保留失败和独立例外处置
