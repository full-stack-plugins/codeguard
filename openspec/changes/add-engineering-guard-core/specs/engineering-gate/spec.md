## Purpose

为 Partme Guard 提供 engineering-gate 的可观察行为契约，统一已批准约束、当前候选和独立验证证据的关联，并明确未知、失败、兼容模式与越权情况下不得伪造工程通过的边界。

## ADDED Requirements

### Requirement: EG-GATE-001 Unified action decision

系统 SHALL 依据有效契约与独立生产者证据组合工程动作判定，保留已知违规、缺失证据和待审查事项；策略求值故障/超时/无定义返回 incomplete。

#### Scenario: EG-GATE-001 contract behavior
- **WHEN** 流程满足但代码必需检查未执行
- **THEN** 门禁不放行，明确缺失检查，不用 accepted 替代

### Requirement: EG-GATE-002 Independent authority

系统 MUST 独立核验生产者、批准和授权来源；普通 Agent、CLI 参数、项目文件及模型文本不能自声明 trusted 或签发执行授权。

#### Scenario: EG-GATE-002 contract behavior
- **WHEN** Agent 提供自造 user receipt 或 trusted_bindings_verified=true
- **THEN** 无法验证来源即拒绝授权，保留审计原因

### Requirement: EG-GATE-003 Scoped reusable approvals

系统 SHALL 在原授权有效的身份、意图、范围和期限内复用授权；需求范围批准与候选验收分离，扩大范围、撤销或候选变化按批准种类失效。

#### Scenario: EG-GATE-003 contract behavior
- **WHEN** 同一授权任务重复检查且内容仍在授权范围内
- **THEN** 不重复请求范围批准；新代码仍需对应的新鲜技术证据

### Requirement: EG-GATE-004 Enforced execution

系统 MUST 将候选执行与控制/签发/合入身份隔离；只针对已验证动作和当前对象发放最小授权，消费时核验 audience、目标版本、期限、撤销和幂等状态。

#### Scenario: EG-GATE-004 contract behavior
- **WHEN** 验证后目标 ref 改变或相同 grant 被重放
- **THEN** 拒绝过期对象；重复请求不能重复写入，未知执行结果先对账

### Requirement: EG-GATE-005 Nonweakening exceptions

系统 SHALL 保留原始违规，对允许例外绑定精确目标、批准人、原因、期限和撤销；不可豁免规则不能放行，修复不能通过删测试或改阈值自批完成。

#### Scenario: EG-GATE-005 contract behavior
- **WHEN** 候选通过修改策略/必需测试清单降低门槛
- **THEN** 继续采用受保护基线，拒绝自我放行

### Requirement: EG-GATE-006 Honest operating modes

系统 SHALL 显式区分 advisory/cooperative/enforced；只有独立 CI、目标保护和真实绕过验收成立时才能声明 enforced。

#### Scenario: EG-GATE-006 contract behavior
- **WHEN** 仅本地 Hook 或直接脚本重放通过
- **THEN** 仅报告相应局部验证，不宣称服务端强制或真实宿主通过

### Requirement: EG-GATE-007 Recovery and compatibility

系统 SHALL 提供取消、超时、并发单飞与崩溃恢复，保留旧 CLI/Hook 显式映射；修复阻塞所需读取、澄清和受允许测试路径保持可用。

#### Scenario: EG-GATE-007 contract behavior
- **WHEN** 验证过程中取消或宿主能力缺失
- **THEN** 取消受管子进程并保留部分证据；不得回退较弱实现签发通过
