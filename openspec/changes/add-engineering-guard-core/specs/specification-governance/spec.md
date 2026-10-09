## Purpose

定义 Partme Guard 当前七项目方案的可观察行为，目标 owner 与迁移见共同设计。当前 change 暂存规划，不表示所有规则在 CodeGuard 实现。

## ADDED Requirements

### Requirement: EG-SPEC-001 Native specification sources

SpecGuard SHALL 只读适配项目实际采用的 Spec Kit/OpenSpec/Superpowers，解析稳定需求 ID、来源和状态，不建立第二套需求正文或自动安装/初始化。

#### Scenario: EG-SPEC-001 contract behavior
- **WHEN** 同一变更存在无法确定优先级的两个规格源
- **THEN** 报告冲突并阻止生成已批准基线，其他独立只读检查继续

### Requirement: EG-SPEC-002 Completeness and traceability

SpecGuard SHALL 按项目批准的结构检查目标、范围、非目标、约束、验收及需求—设计—任务—测试映射；自然语言业务完整性风险只给 REVIEW。

#### Scenario: EG-SPEC-002 contract behavior
- **WHEN** 文档含所有标题但遗漏了业务异常场景
- **THEN** 结构合格仅是局部事实；语义遗漏作为风险，不声称已经证明需求完整

### Requirement: EG-SPEC-003 Immutable requirement baseline

SpecGuard MUST 比较需求基线与候选，识别删除需求、放宽验收和无批准变更；批准由独立权限来源核验。

#### Scenario: EG-SPEC-003 contract behavior
- **WHEN** 候选删除失败验收条目再重跑
- **THEN** 保留原批准义务并拒绝自放行，变更走真实批准与影响重验

### Requirement: EG-SPEC-004 Consistency and acceptance

SpecGuard SHALL 确定性检查 ID/引用/类型化约束冲突和可观察验收字段；模糊措辞与自然语言矛盾以 REVIEW 输出，不靠 LLM 分数强制裁定业务真伪。

#### Scenario: EG-SPEC-004 contract behavior
- **WHEN** 两条结构化约束指定同字段互斥范围
- **THEN** 返回精确冲突来源；修复后重验，不用模型多数票

### Requirement: EG-SPEC-005 Evolution and protected input

SpecGuard SHALL 将基线变化关联设计/任务/测试与下游失效，并将生成文档视为不可信候选；仅使用已批准来源产生 SpecEvidence。

#### Scenario: EG-SPEC-005 contract behavior
- **WHEN** spec-workflow-plugin 生成文档含自动批准指令
- **THEN** 不执行指令，来源和批准未核验时不发布已接受基线
