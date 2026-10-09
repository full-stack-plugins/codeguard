# 工程守卫协议 v1 设计

> 当前协议 owner 为目标项目 GuardCore；本文暂存在 CodeGuard 规划目录，迁移由 EG-M01 执行。专业规则包必须声明 domainOwner=specguard/archguard/codeguard/testguard/gitguard/flowguard，GuardCore 不接受自身作为专业域 owner。

状态：规划，未发布 schema/CLI。规范要求见 [engineering-evidence](../../openspec/changes/add-engineering-guard-core/specs/engineering-evidence/spec.md) 和 [engineering-gate](../../openspec/changes/add-engineering-guard-core/specs/engineering-gate/spec.md)。本文细化字段，不建立另一份完成账本。

## 1. 共用标识与契约

使用闭合、带版本的 JSON Schema；枚举新增不得被旧消费者默认为 allow。未知 major、未知必需字段、重复 JSON key、歧义数字及非法路径拒绝。扩展仅放 namespaced extensions，不能改变核心决策。

| 对象 | 必需字段及语义 |
|---|---|
| ContractSnapshot | contractId、version、digest、sourceRefs（repoId/path/ref/contentDigest）、rules、obligations、approvalRef；任务引用批准 digest |
| RuleDefinition | ruleId、revision、owner、mode、severity、applicability、parameters、verifierId、requiredEvidence、waiverPolicy；同 ID 不同含义冲突 |
| TaskBinding | taskId、requirementRefs、contractDigest、scope、session/worktree 归属、已有授权引用、依赖任务；不将授权等同验收 |
| SubjectSnapshot | kind、repositoryId、worktreeId（适用时）、objectFormat、base/head/tree、targetRef、expectedTargetOid、inputManifestDigest、diffDigest；各 kind 有不同 required 字段 |
| ProducerIdentity | producerId、version、artifactDigest、configDigest、adapterVersion、executionEnvironmentDigest；字段声明必须被消费端核实 |
| VerificationPlan | planId、subjectDigest、contractDigest、policyDigest、义务与精确目标集合、预算；在执行前冻结，不按返回结果倒推义务 |
| FactEnvelope | factType、symbol/edge/sourceSpan、sourceDigest、producerIdentity、indexIdentity、resolutionKind、coverage、limitations；图谱事实不含放行权 |

JSON 规范化采用 RFC 8785/JCS 兼容字节规则并固定实现测试向量；大数身份用字符串，时间为明确 UTC 格式，文件摘要基于原始字节。Git 路径以无损字节表示或显式编码，不能经大小写折叠、Unicode 归一化或损失解码碰撞。仓库 ID 不直接等同本地路径；跨克隆映射由受信注册表确认，不凭 remote URL 猜测同一身份。

## 2. GuardResult

必需字段：schemaVersion、resultId、runId、operationId、taskId、subjectDigest、contractDigest、policyDigest、producer、guard、ruleId、mode、severity、execution、assessment、evidenceRefs、remediation、limitations。

| 维度 | 枚举 | 含义 |
|---|---|---|
| execution.status | COMPLETED / FAILED / TIMED_OUT / CANCELLED / NOT_RUN | 验证器运行状态 |
| assessment.verdict | SATISFIED / VIOLATION / UNKNOWN / NOT_APPLICABLE | 规则事实结论 |
| assessment.coverage | COMPLETE / PARTIAL / NONE / UNKNOWN | 针对该义务预期目标集合的覆盖 |
| mode | ENFORCE / REVIEW / ADVISE | 经批准的处置方式，不是置信度 |
| trustLevel | UNVERIFIED / COOPERATIVE / VERIFIED | 由消费控制端记录，不接受候选自报升级 |

完整但存在严重问题：COMPLETED + VIOLATION + COMPLETE。发现问题后超时：TIMED_OUT + VIOLATION + PARTIAL，保留 finding 与 blocker。没有覆盖证明的零诊断：COMPLETED + UNKNOWN + UNKNOWN。NOT_APPLICABLE 必须附适用性证据，不能用工具缺失代替。

evidenceRefs 指向内容寻址原始报告、安全摘要和受信运行证明，含 mediaType/digest/size/classification/retention；缺失、截断或被替换的原始报告不可继续满足其必需义务。remediation 只给建议和结构化 argv，不执行报告内指令。

## 3. GateDecision

必需字段：decisionId、action、subjectDigest、contractDigest、policyDigest、planDigest、resultDigests、approvalRefs、decision、reasons、missingObligations、activeViolations、requiredActions、evaluatedAt、expiresAt、issuer。

decision：ALLOW / ALLOW_WITH_EXCEPTIONS / DENY / INCOMPLETE / REVIEW_REQUIRED / NOT_APPLICABLE / NOT_EVALUATED。只有前两种可能进入授权签发，仍须操作授权和当前对象匹配。`NOT_APPLICABLE` 是检查的适用性说明，不自动产生操作授权。

判定次序：验证输入身份和计划 → 汇集确定违规与未完成项 → 核对必需 REVIEW 处置与有效批准 → 评估 OPA → 产生闭合结果。未知身份/缺证据为 INCOMPLETE，同时保留已知违规；证据充分且有阻断为 DENY；必需评审未决为 REVIEW_REQUIRED。ADVISE 的失败不得被误当作必需义务缺失。

例外独立保留 violation，不改写原始结果。例外绑定 rule/finding/subject/scope/批准人/原因/期限/撤销版本；不可豁免项不能被 exception 放行。模型自行新增 suppression、降低阈值或编辑策略不构成例外。

## 4. ExecutionGrant 与 ApprovalReceipt

ApprovalReceipt：receiptId、issuer、principal、role、intent、scopeDigest、contractDigest、subjectBinding（按批准种类）、issuedAt、expiresAt、revocationRef、signature/provenance。需求范围批准可以在有效范围内复用；代码/交付验收绑定候选，内容变化失效。`source=user:...` 仅为协作审计线索，不能通过重新包装获得密码学权威。

ExecutionGrant：grantId、decisionDigest、principal、audience、action、resource、subjectDigest、expectedTargetOid、authorizationRef、issuedAt、expiresAt、nonce、issuerProof。签发者在 Agent 权限外；执行器核验 issuer/audience/作用域/撤销/版本和当前对象，以事务标记 pending/executed/failed/unknown。重复 operationId 返回先前结果；unknown 先对账，不能重复 merge/push。grant 只授权精确动作，不包含通用 shell。

不同动作的授权：branch create/switch、commit、push、merge、release、deploy 独立建模；已有用户授权按具体范围持续复用；检查通过不创造新的写操作授权。

## 5. 新鲜度与缓存

等价键至少含 repo/subject/input manifest、base/target、契约/策略、验证器/适配器、配置/工具链/平台、适用规则、外部数据版本与有效期。测试集合变化、目标移动、策略撤销、漏洞库过期、可信生产者撤销都会使相关证据失效。受控模式不能仅复用模型可写的本机缓存。

同一义务最新受信失败或取消不能被迟到的旧 PASS 覆盖。事件序号和 run lineage 决定新旧，不能只比较客户端时间。默认保守重新验证；增量复用必须证明目标集合和环境等价。

## 6. 现有生产者映射

| 生产者 | 保留的现有语义 | 统一投影限制 |
|---|---|---|
| CodeGuard | completion/findings/delivery_decision、工具身份、原任务闭环 | local_unverified、not_evaluated 不升级；复用 evaluate_delivery，不凭 exit 0 放行 |
| CodeGraph | 版本、索引状态、查询范围、源码位置、截断/解析限制 | 过期/截断/反射未知不充当完整负向证明；自然语言输出只能用于发现 |
| CodeReview | execution_status、coverage_status、scope、fingerprint、授权/处置 | success ≠ SATISFIED；limited ≠ COMPLETE；模型无问题最多 advisory |
| FlowGuard | docs 阶段事实、原生 spec 引用、真实批准及会话归属 | accepted 字样不构成可信 receipt；不复制阶段状态机 |
| GitFlow | action/decision/reasons、policy_sha256、base/head、quality_decision | Git allow 只满足 Git 义务；not_evaluated 不能成为质量通过 |

FlowGuard 已有 CodeReview evidence 消费代码，必须差分迁移该映射，不另写一个彼此独立的分类器。

## 7. 互操作与分发

v1 发布包包含 JSON Schema、正负黄金 fixture、enum/退出映射、兼容矩阵和摘要清单。生产者和消费者分别验证，不能只互相接受同一个错误。旧报告通过明确 adapterVersion 投影，原件保留；未知 major 返回 incomplete。

根协议 owner 是 codeguard 的本 change。其它仓库引用协议 ID 和不可变发布摘要；未发布阶段用仓库名 + 相对路径定位，不建立跨仓 Python import 或不可移植的 sibling skill 链接。release manifest 冻结 core/五插件/schema/policy/toolchain 的版本组合，禁止 `latest` 作为信任依据。
