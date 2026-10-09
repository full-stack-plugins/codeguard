# CodeGuard GuardEngine Compatibility Adapter Implementation Plan

> For agentic workers: 后续实施使用 superpowers:subagent-driven-development 或 superpowers:executing-plans 逐任务执行；本次仅规划，不开始实施。所有框保持未完成。

**Goal:** 提供默认关闭、命令感知、只读既有native证据的可回滚GuardEngine投影适配器，不能扩大原生检查资格。

**Architecture:** CLI应用层新增 `guard_integration/` 纯读取/范围/投影模块，复用GuardEngine通用协议和证据机制；native解析/规则/执行/repair仍归旧change。控制器认证审批和当前候选资格，独立信封不扩展当前wire。

**Tech Stack:** 已有Rust2024/MSRV1.85、Serde/serde_json及SHA实现；拟采用固定GuardEngine接口，SDK/进程选择和版本需1.2决策。无新已安装依赖或发布承诺。

**Spec:** [native-evidence-projection](specs/native-evidence-projection/spec.md)、[bound-projection-consumption](specs/bound-projection-consumption/spec.md)、[本变更设计](design.md)。依据[架构](../../../docs/architecture.md)、[技术设计](../../../docs/technical-design.md)、[共享草案](../../../docs/integration-contract.md)、[路线图](../../guard-roadmap.md)。

## Global Constraints

- 严格 `guard.partme.ai/v1alpha1` 与草案 `guard.integration/v1alpha1` 分开；不添加未知字段，无隐式N/N-1。
- native数值/JSON/SARIF/Hook/导出不变；新显式入口才采用0/2/3/4。完整绑定前不得发信封；之后error/cancelled决策为空。
- 旧change/config/tasks不改动；旧2.3/2.4/2.5/2.9、3.5/3.7、4.5、5.4/5.5及6–9拥有native完备性，切片未证明则unsupported/partial，不等待或重做整个旧change。
- 无默认源码/工作台写入、工具启动、联网安装、消息、批准签发或合并。任务中的未来文件均未创建；fixture回归通过不代表生产native资格。
- 每项实施保存失败反例、最小实现、回归输出/版本及人工review；不得凭本文将任务勾选完成。

## Review Focus

- 同为数字2在历史/native/适配入口含义不同：1.1、2.5冻结三代fixture。
- 零finding但缺required scope或只有WASM：1.4、2.2拒绝complete；不要伪造首批绿色profile。
- 导出失败但stdout保留finding、磁盘残留旧绿报告：2.1、3.5不丢失局部证据且不误消费。
- dirty工作区、算法不同OID、基线在运行间变更：3.1、3.3绑定实际输入而非只用HEAD。
- 批准撤销和迟到成功并发：3.2–3.4保留原决策，消费时重验并CAS当前绑定。

## 1. C0 兼容边界与资格冻结（本地调查可与 GE 并行）

输出：`InvocationDescriptor`、默认unqualified的`CapabilityProfile`及版本化fixture。满足GE-CONTRACT后才冻结外部schema；对应GE各门见[engine change](https://github.com/full-stack-plugins/guardengine/tree/docs/guard-design-20261009/openspec/changes/add-versioned-guard-integration-contracts)。

- [ ] 1.1 在拟新增 `tests/fixtures/guard-integration/native-matrix.json` 与 `crates/codeguard-cli/tests/guard_integration_compat.rs` 冻结command/flags/schema/退出/stdout/stderr/副作用样本；对旧check/CVE/Dockerfile混合优先级、Rust0/1/2/3/4/130及query成功分别断言，验收不按数字跨入口猜测。追踪「Native interfaces remain unchanged」。
- [x] 1.2 在拟新增 `docs/adr/guard-integration-boundary.md` 决定纯模块+SDK或受限进程的调用边界、明确新opt-in入口名和参数；通过接口fixture断言未选入口无额外I/O/进程，所选接口只消费已有工件，固定GE源码revision和未来发布门。追踪「Native interfaces remain unchanged」「Rollout preserves native ownership and rollback」。
- [x] 1.3 在拟新增 `schemas/guard-integration-invocation.schema.json` 和 `guard_integration/profile.rs` 固定InvocationDescriptor/CapabilityProfile字段、独立版本及允许flags；`guard_integration_profile.rs` 验证未知版本/字段/命令组合全部拒绝，区分包版本、native schema、mapping和wire版本。追踪「Native evidence readers reject ambiguity」。
- [x] 1.4 在拟新增 `tests/fixtures/guard-integration/qualification.json` 建立所选native切片到旧任务/真实验收的映射；`guard_integration_scope.rs` 测试WASM、zero findings、task resolved均不能升级complete；无可证明切片时所有profile保持unqualified仍可进入partial开发。追踪「Complete projection requires proven native scope」。
- [x] 1.5 在拟新增 `schemas/guard-integration-transport-error.schema.json` 与 `guard_integration/envelope.rs` 固定前置诊断和绑定后结果接口，等待GE-CONTRACT冻结；`guard_integration_binding.rs` 至少覆盖坏参数、未知repo、缺candidate/base、未冻结scope四类无信封反例。追踪「Binding precedes envelope publication」。
- [x] 1.6 在拟新增 `tests/fixtures/guard-integration/relations.json` 和 `guard_integration/projection.rs` 定义受保护mapping词表、每个必需finding/缺口的精确规则覆盖及unsupported输出；`guard_integration_projection.rs` 对每条映射提供正反例，拒绝count/通配符/未映射项，无假ALLOW。追踪「Projection uses exact supported engine relations」。

## 2. C1/C2 严格读取与显式投影（依赖 GE-CONTRACT、GE-ADAPTER 和所选 native 切片）

接口：`read_native(bytes, invocation, profile) -> Result<NativeEvidence, ProjectionError>` → `assess_scope(evidence, obligations, binding) -> ScopeAssessment` → `project(scope, mapping) -> ProjectionOutcome`；类型按1.x冻结，本组不得改变旧native报告。

- [x] 2.1 在 `guard_integration/reader.rs` 实现有界只读读取、重复键/未知版本/格式及exit矛盾校验；`guard_integration_reader.rs` 覆盖完整、截断、重复键、超限、export failed和残留旧文件六类fixture，原finding与raw digest保留且坏输入无ALLOW。追踪「Native evidence readers reject ambiguity」。
- [x] 2.2 在 `guard_integration/scope.rs` 对照独立冻结义务及native实际目标/规则/工具/配置范围；`guard_integration_scope.rs` 逐项删除必需目标并加入额外无关成功项，全部保持partial/unsupported，当前aggregate exit3永不升级。追踪「Complete projection requires proven native scope」。
- [x] 2.3 在 `guard_integration/projection.rs` 将已证明范围映射到严格engine输入，保留domain finding附件；`guard_integration_projection.rs` 核验enforce/review/advise、partial优先、未知字段/不支持关系拒绝，重算结果与固定engine黄金向量一致。追踪「Projection uses exact supported engine relations」。
- [x] 2.4 在 `guard_integration/envelope.rs` 实现binding/producer/coverage前置校验和已绑定error/cancelled路径；`guard_integration_binding.rs` 覆盖前置失败无信封、工具故障null、取消null、completed decision与引擎一致四组反例，native130原值保存。追踪「Execution outcome is separate from technical decision」「Binding precedes envelope publication」。
- [x] 2.5 在拟新增 `guard_integration/command.rs` 接线1.2已批准的显式入口及help描述，最小修改既有CLI分发而不更改native处理；`guard_integration_cli.rs` 断言新增0/2/3/4、stdout结构化/诊断stderr、原生check仍正常3且--output同时输出stdout。追踪「Native interfaces remain unchanged」。
- [x] 2.6 在 `guard_integration/projection.rs` 增加只读shadow结果比较和profile能力声明；`guard_integration_shadow.rs` 确认native-only/unsupported工件无伪completed决策，要求engine验证的消费者拒绝较弱profile；相同输入重复10次得到相同语义facts/report。追踪「Complete projection requires proven native scope」「Rollout preserves native ownership and rollback」。

## 3. C2/C3 绑定、信任和安全消费（受信使用依赖 GE-TRUST）

输出：独立资格评估及不可变审计引用；原始report和信封不改写。先固定控制器port，不选择生产批准服务或持有签发密钥。

- [x] 3.1 在 `guard_integration/consumer.rs` 校验repo/task/worktree/requirement集合、完整OID、candidate/base/group及sourceSnapshotDigest；`guard_integration_candidate.rs` 覆盖SHA-1/SHA-256、dirty字节与HEAD不符、错repo四类反例，可信队列消费只接受clean冻结候选。追踪「Changed bindings invalidate eligibility」。
- [ ] 3.2 在 `guard_integration/consumer.rs` 接入GE-TRUST认证引用port，校验baseline不可变digest/ref、批准issuer/scope/expiry/revocation；`guard_integration_trust.rs` 覆盖伪accepted、错issuer、跨scope、过期、撤销、provider不可用，均不授权且原REQUIRE_APPROVAL字节不变。追踪「Approved references require authenticated consumption」。
- [ ] 3.3 在 `guard_integration/consumer.rs` 实现资格键和失效理由；`guard_integration_freshness.rs` 逐项变更candidate/base/group/source/baseline/contract/tool-config/mapping/analyzer/coverage及批准状态，全部重验，首版不跨run缓存执行结果。追踪「Changed bindings invalidate eligibility」。
- [ ] 3.4 在 `guard_integration/audit.rs` 实现runId尝试分离、同次导入幂等及绑定CAS；`guard_integration_concurrency.rs` 运行两个需求/两个worktree和迟到head1成功反例，断言记录不串用且不覆盖head2。追踪「Parallel projections preserve immutable ownership」。
- [ ] 3.5 在 `guard_integration/audit.rs` 实现显式私有目录的有界工件读取/原子发布/摘要复核；`guard_integration_storage.rs` 注入symlink、越界路径、截断、磁盘/权限失败和中断，拒绝覆盖原报告/用户文件且保留内存finding。追踪「Projection storage is bounded and read-only by default」。
- [ ] 3.6 在 `guard_integration/audit.rs` 固定脱敏字段白名单、访问权限和可配置保留策略；`guard_integration_redaction.rs` 注入token/env/源码/原生消息指令，断言公开诊断无秘密、不执行指令，缺必需工件使消费资格失效。追踪「Projection storage is bounded and read-only by default」。
- [ ] 3.7 在 `guard_integration/consumer.rs` 提供宿主可调用只读消费接口，不新增任意shell/MCP授权面；`guard_integration_side_effects.rs` 比较源码、.codeguard事件、refs/index与网络/进程记录，重复投影均无隐式native运行/repair/install/通知。追踪「Projection storage is bounded and read-only by default」「Native interfaces remain unchanged」。

## 4. C3/C4 精确候选与跨守卫联验（依赖适用 GE-TRUST、GG-CANDIDATE）

只读候选依赖[GitGuard change](https://github.com/full-stack-plugins/gitguard/tree/docs/guard-design-20261009/openspec/changes/add-candidate-bound-git-governance)，不等待其特权写执行器。真实native范围不足则联验标为不支持，不能用mock冒充生产资格。

- [x] 4.1 在拟新增 `tests/fixtures/guard-integration/queue/` 和 `guard_integration_queue.rs` 接入固定版本GG-CANDIDATE只读fixture；PR head与synthetic M不同、base推进、组成员重排均拒旧证据，M原生结果经投影后才可满足该范围。追踪「Protected gates consume exact queue candidates」。
- [ ] 4.2 在拟新增 `tests/acceptance/guard-integration-native-parity.md` 记录选定真工具的完整/违规/partial/故障样本及前后输出，运行1.1差分测试；native字段/退出/副作用无未解释差异，已知工具缺口仍由旧change登记。追踪「Native interfaces remain unchanged」「Complete projection requires proven native scope」。
- [ ] 4.3 在 `guard_integration_concurrency.rs` 联合队列fixture测试双需求并行、批准扫描后撤销、旧run迟到和新base重验；保存原始输入/输出/摘要，任一失效不能ALLOW，审批不能覆盖工具故障。追踪「Parallel projections preserve immutable ownership」「Changed bindings invalidate eligibility」。
- [ ] 4.4 在拟新增 `tests/acceptance/guard-integration-host.md` 固定一个获授权可检查宿主版本并记录能力/取消/重入/凭据边界；`guard_integration_host.rs` 测试read-only调用，不可取得宿主则记录unverified且不宣称兼容通过。追踪「Rollout preserves native ownership and rollback」。

## 5. C5 显式发布、兼容矩阵与回滚（生产分发等待 GE-RELEASE）

- [ ] 5.1 在拟新增 `docs/guard-integration-compatibility.md` 锁定已验证GE/native/mapping/envelope/平台/宿主组合及固定依赖制品；`guard_integration_version_matrix.rs` 对支持及未知N/N-1逐项测试，未知组合拒绝，不以crate semver推报告版本。追踪「Rollout preserves native ownership and rollback」。
- [ ] 5.2 在 `guard_integration/command.rs` 与拟新增 `tests/acceptance/guard-integration-rollback.md` 验证默认关闭→shadow→显式opt-in→停用适配器路径；原生命令历史fixture全部保持，required集成证据缺失仍阻断，停用不删除工件或改政策。追踪「Native interfaces remain unchanged」「Rollout preserves native ownership and rollback」。
- [ ] 5.3 在拟新增 `tests/acceptance/guard-integration-release.md` 汇总真实命令、版本、各spec scenario到测试/产物的矩阵及未支持范围；执行适用新测试和旧native受影响回归，全部指定负例通过、零false ALLOW且无未解释差异才宣布CG-ADAPTER切片验收，未完成旧任务保持未完成。追踪全部12条Requirement。
