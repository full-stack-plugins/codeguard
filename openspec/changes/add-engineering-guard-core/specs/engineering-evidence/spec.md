## Purpose

为 Partme Guard 提供 engineering-evidence 的可观察行为契约，统一已批准约束、当前候选和独立验证证据的关联，并明确未知、失败、兼容模式与越权情况下不得伪造工程通过的边界。

## ADDED Requirements

### Requirement: EG-EVI-001 Separate dimensions

系统 SHALL 分开报告执行状态、规则结论和精确范围覆盖；部分执行发现的违规及未完成原因同时保留，空结果不能推断通过。

#### Scenario: EG-EVI-001 contract behavior
- **WHEN** 工具先发现违规然后超时
- **THEN** 保留违规与超时/部分覆盖，不输出完整通过

### Requirement: EG-EVI-002 Exact subject

系统 MUST 将证据绑定仓库、任务、候选类型、实际输入清单、基线、契约、策略、验证器和环境身份；暂存区、HEAD、最终合入候选不能互相冒用。

#### Scenario: EG-EVI-002 contract behavior
- **WHEN** 把 index 的通过报告用于不同 merge_candidate
- **THEN** 拒绝复用并要求对实际候选验证

### Requirement: EG-EVI-003 Freshness and ordering

系统 SHALL 按受信运行序列和身份判断证据新鲜度；源码、配置、目标、必需测试、策略或外部数据变化时使受影响证据过期，迟到旧通过不能覆盖最新失败。

#### Scenario: EG-EVI-003 contract behavior
- **WHEN** 最新运行失败之后收到旧运行 PASS
- **THEN** 旧结果仅作审计，不恢复放行

### Requirement: EG-EVI-004 Versioned projections

系统 SHALL 使用版本化闭合协议和明确适配器保留生产者原报告；不把 CodeReview success、GitFlow allow、CodeGuard exit 0 或 CodeGraph 查询成功映射成工程通过。

#### Scenario: EG-EVI-004 contract behavior
- **WHEN** 旧消费者收到未知 major 或 unknown 枚举
- **THEN** 返回协议未完成，不能默认为 allow

### Requirement: EG-EVI-005 Durable private evidence

系统 SHALL 对证据进行内容完整性检查、私有原文与公开摘要分离，并支持并发、崩溃恢复、幂等记录及有界保留；本地可写状态不能自行取得可信身份。

#### Scenario: EG-EVI-005 contract behavior
- **WHEN** 落盘中断或原始文件被替换后重启
- **THEN** 隔离不完整记录，保留恢复线索；不得签发放行
