## ADDED Requirements

### Requirement: Native interfaces remain unchanged
适配器 SHALL 默认关闭并只读既有证据。现有命令、报告 schema、退出码、stdout/stderr、导出和任务副作用 MUST 保持其具名协议；只有新增显式适配入口使用0/2/3/4。

#### Scenario: Adapter is disabled
- **WHEN** 用户未显式选择适配入口
- **THEN** native命令行为与冻结fixture相同，不加载投影或新增写入

#### Scenario: Legacy exit semantics differ
- **WHEN** 旧CVE用法错误为3或Rust用法错误为2
- **THEN** 按命令/参数/报告分类并保留原数字，不解释为REQUIRE_APPROVAL或BLOCK

### Requirement: Native evidence readers reject ambiguity
证据读取器 SHALL 按明确 profile 校验command、flags、report type/version及绑定；MUST 保留原始摘要和已知finding。未知/畸形/重复键/超限或矛盾报告 MUST NOT 按空成功消费。

#### Scenario: Unknown schema or duplicate key arrives
- **WHEN** 输入schema未登记或含重复decision键
- **THEN** 拒绝解析并保留受限诊断，不输出ALLOW

#### Scenario: Export failed with retained findings
- **WHEN** native check stdout含finding和export failed且所需文件未保存
- **THEN** 保留finding与导出失败状态，不把旧文件当本轮完成证据

### Requirement: Complete projection requires proven native scope
适配器 SHALL 将完整资格绑定已验证的native命令范围和独立冻结的必需义务。登记语言、零诊断、exit0、WASM候选或task resolved MUST NOT 推导完整。旧change继续拥有native实现；缺少证明 MUST 保持partial或unsupported。

#### Scenario: Aggregate feedback remains incomplete
- **WHEN** 当前check返回正常exit3和未完成义务
- **THEN** 投影不得升级complete；可表达的有效partial得到BLOCK

#### Scenario: Narrow profile gains qualification
- **WHEN** 一个明确版本/类别/平台/目标范围有真工具正反例和稳定输入证明
- **THEN** 只对该范围登记complete资格；其余语言和项目交付仍未获资格

### Requirement: Projection uses exact supported engine relations
投影 SHALL 仅输出严格guard.partme.ai/v1alpha1支持的对象和精确forbid_relation关系；每个必需缺口 MUST 有已验证的受保护映射。领域finding/元数据 MUST 保存为独立附件，不新增wire字段或默默丢弃未映射项。

#### Scenario: A finding has no supported mapping
- **WHEN** 一个必需finding没有可验证的精确关系映射
- **THEN** 返回unsupported而非遗漏后ALLOW

#### Scenario: Partial facts are evaluable
- **WHEN** 输入可形成合法partial facts及diagnostics
- **THEN** 引擎产生INDETERMINATE/BLOCK，即使所有规则为advise；不伪造完整范围

### Requirement: Execution outcome is separate from technical decision
适配器 SHALL 区分completed/error/cancelled。具备完整绑定的error/cancelled信封 MUST decision=null；合法partial评估为BLOCK。工具故障/取消 MUST 保留部分观察而不冒充业务违规或批准；native退出语义不变。

#### Scenario: Native tool crashes after producing findings
- **WHEN** 已冻结绑定的工具运行失败且保留局部finding
- **THEN** 保存domain诊断，运行error、decision null，新入口4，不抹去finding

#### Scenario: Native cancellation is projected
- **WHEN** 已绑定native结果为取消130
- **THEN** 原记录保留130，新信封cancelled且decision null，适配入口4

### Requirement: Binding precedes envelope publication
GuardRunEnvelope SHALL 只在producer、全部必需绑定和coverage冻结后生成，使用独立guard.integration/v1alpha1。绑定前失败 MUST 使用独立传输诊断；engine-backed completed信封decision MUST 等于引用的GuardReport。

#### Scenario: Candidate is unresolved
- **WHEN** candidate/base或repo身份不能解析
- **THEN** 无GuardRunEnvelope、无伪造OID或空必填字段，返回独立错误诊断

#### Scenario: Engine requires review
- **WHEN** 完整投影引擎报告为REQUIRE_APPROVAL
- **THEN** 信封同为REQUIRE_APPROVAL；后续批准不改写报告或原信封
