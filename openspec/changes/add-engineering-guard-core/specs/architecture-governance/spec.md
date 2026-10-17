## Purpose

为 Partme Guard 提供 architecture-governance 的可观察行为契约，统一已批准约束、当前候选和独立验证证据的关联，并明确未知、失败、兼容模式与越权情况下不得伪造工程通过的边界。

## ADDED Requirements

### Requirement: EG-ARCH-001 System boundaries

系统 SHALL 按批准模块、层次、服务边界和 API 契约检查真实差异；返回违规关系、源码位置、规则 ID 和受检覆盖。

#### Scenario: EG-ARCH-001 contract behavior
- **WHEN** Java domain 新增到 infrastructure 的禁止依赖
- **THEN** 确定性检查拒绝并定位边；移除依赖后当前候选重新验证

### Requirement: EG-ARCH-002 Domain invariants

系统 SHALL 将聚合不变量和状态机约束关联真实行为/并发/契约测试；自然语言职责标签不作为自动正确性证明。

#### Scenario: EG-ARCH-002 contract behavior
- **WHEN** 取消任务绕过批准状态机但代码能够编译
- **THEN** 相应不变量测试失败并阻断，不以编译通过覆盖

### Requirement: EG-ARCH-003 Design review separation

系统 MUST 将对象职责、抽象、继承和方法归属的启发式判断标记为 REVIEW/ADVISE；只有已批准的精确设计契约可用于 ENFORCE。

#### Scenario: EG-ARCH-003 contract behavior
- **WHEN** 模型认为类太长或 cancelTask 名称重复
- **THEN** 输出可定位风险及理由，不凭模型置信度认定确定性违规

### Requirement: EG-ARCH-004 Semantic conflicts

系统 SHALL 基于任务修改集合、消费接口和版本化符号事实识别并行风险；图谱覆盖不足明确 unknown，最终合并仍执行候选行为验收。

#### Scenario: EG-ARCH-004 contract behavior
- **WHEN** 两个任务修改不同文件但接口提供方与消费方不兼容
- **THEN** 产生冲突风险并重验最终候选，不把无文本冲突当作安全

### Requirement: EG-ARCH-005 Evolution and baselines

系统 SHALL 比较批准基线与候选的依赖、公共 API 和领域契约变化，关联 ADR/需求/测试及历史；基线仅描述差异，不自动豁免已有违规。

#### Scenario: EG-ARCH-005 contract behavior
- **WHEN** 历史技术债在候选中仍存在或公共 API 扩张
- **THEN** 保留规则处置与变更影响，不能用基线存在作为通过依据
