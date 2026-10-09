# CodeGuard evidence projection design

## Context

本变更是[架构](../../../docs/architecture.md)和[技术设计](../../../docs/technical-design.md)的 C0–C5 增量适配切片，遵守[集成草案](../../../docs/integration-contract.md)及[路线图](../../guard-roadmap.md)。源码基线 `b499f13922647d4bf1e2344eaa5c37c450e302b5`；现有 `openspec/config.yaml` 和旧变更完整保留。

实际证据：`crates/codeguard-cli/src/{main,check_command,legacy_v1_protocol,report_export,check_plan,run_report}.rs`；`crates/codeguard-core/src/{verdict,aggregate,check_session,delivery_gate}.rs`；`crates/codeguard-runtime/src/source_snapshot.rs`。`check_all_partial_contract.rs`、`legacy_v1_protocol_contract.rs`、`check_session_contract.rs` 有现有测试定义，不代表本轮重跑。原生 `check --output` 同时写 stdout，导出失败保留 finding；早期错误可能仅 stderr。旧 check/CVE 和 Dockerfile 的混合优先级不同，兼容结果恒 not_evaluated。Rust Verdict 的0/1/3/4/130及入口用法2保持原样。

## Goals / Non-Goals

目标是默认关闭、证据输入只读、可独立撤销的 CG-ADAPTER。只对明确报告版本及可证明 native 范围提供能力。非目标：重写原生检查器、实现旧任务中的全语言义务/repair closure、改变 CLI/Hook/包版本、默认联网安装、签发批准或合并。外部 codeguard-plugin、codegraph-plugin、codereview-plugin 未检查；真实宿主能力待固定版本验收。

## Decisions and interfaces

以下路径/类型均为拟新增，当前不存在，不是可调用 API：

| 拟议模块 | 输入 → 输出 | 边界 |
|---|---|---|
| `guard_integration/profile.rs` | InvocationDescriptor + report type/version → CapabilityProfile | profile 固定 command、flags、schema、语言/类别/平台、mapping版本和限制 |
| `guard_integration/reader.rs` | bounded bytes + profile → NativeEvidence 或诊断 | 保留 raw digest、原退出码、finding/缺口；拒绝未知版本、重复键、超限、格式矛盾 |
| `guard_integration/scope.rs` | NativeEvidence + FrozenObligations + binding → ScopeAssessment | 必需集合来自保护策略，不从报告反推；不扩大有限 native coverage |
| `guard_integration/projection.rs` | ScopeAssessment + protected mapping → EngineInputs 或 Unsupported/Error | 只构造现行精确关系，不增加当前字段/算子 |
| `guard_integration/envelope.rs` | verified binding/producer/coverage + outcome → 独立信封 | 验证 GE-CONTRACT，保持 report decision 相等；前置故障无信封 |
| `guard_integration/consumer.rs` | immutable artifacts + controller trust ports → eligibility assessment | 不自发批准；摘要一致和来源可信分别核验 |
| `guard_integration/audit.rs` | bounded artifact refs + run/binding → immutable local audit | 私有输出、最小信息、原子发布；默认仅内存，显式输出才写新目录 |

内部接口约定 `read_native(bytes, invocation, profile) -> Result<NativeEvidence, ProjectionError>`，`assess_scope(evidence, obligations, binding) -> ScopeAssessment`，`project(scope, mapping) -> ProjectionOutcome`；冻结具体 Rust 字段/错误枚举在任务1.2/1.3，依赖 GE schema 后再导入类型，不先创建与引擎重名 wire 对象。`NativeEvidence` 保留原报告引用而不复写。ScopeAssessment 区分 CompleteWithinProvenScope、Partial、Unsupported；执行状态另外表示 completed/error/cancelled。

首期 SDK 还是外部进程由 ADR 选择，可逆默认内部纯模块+固定源码 revision 测试，不新增必需后台服务。选定入口仅用于投影已有工件，参数/输出协议先冻结后接线，本文不提供伪可运行新命令。

## Scope dependencies and mapping

| 新任务依赖的 native 事实 | 旧 owner / 未完成任务 | 对适配器的准入含义 |
|---|---|---|
| findings与完成度、必需义务 | 旧2.3/2.4 | 仅对已证明的窄范围开放；不能将当前 aggregate check 的 incomplete变完整 |
| schema和原始报告/导出 | 旧2.5/2.9 | 只登记实际版本；缺原工件或导出失败不当完整证据 |
| 快照及执行前后稳定性 | 旧3.5/3.7，5.4 | exact candidate或dirty快照缺失时不适用可信消费 |
| native实际规则/范围 | 旧5.5及6–8语言项 | 必须有真工具正反例；WASM候选不是完整lint资格 |
| 精确例外及task closure | 旧4.5，9.10 | 保留审批/修复范围，task resolved不推项目ALLOW |

上述依赖是切片证据检查，不复制旧实施任务；能力矩阵默认全部 unqualified，fixture仅可证明 adapter行为，不能单独升级 native资格。初次交付允许只有 partial/unsupported profiles；不得为制造绿色演示强行开放 complete。

严格引擎 wire 仍为 `guard.partme.ai/v1alpha1`，只允许 GuardContract YAML / GuardFacts JSON / GuardReport JSON、精确 `forbid_relation`、enforce/review/advise；没有通配符/count/任意extensions。受保护 mapping 对每个必需缺口定义精确关系和规则，不支持时拒绝投影，不能丢弃 finding。完整范围的规则命中由引擎求值；partial 有诊断并得到 INDETERMINATE/BLOCK。原生报告零诊断、exit0、not_applicable、安装成功或修复任务关闭均不足以生成 ALLOW。人工批准不能把partial或工具错误变完整。

## State and transport

`unbound → bound/frozen → parsed → scope_assessed → projected → completed`；绑定前失败返回独立诊断及新增入口错误码4，无 GuardRunEnvelope，不填空OID。producer、所有必需binding和coverage冻结后才允许 error/cancelled 信封，decision=null。completed partial合法引擎求值为BLOCK/2；无法生成合法facts的工具故障/error为4，取消为cancelled/4；原生130和原报告仍保留。Unsupported不能表示成功；使用可用性诊断/4，若尚不能冻结必要字段则无信封。native-only证据可作为独立domain工件，不为它发明一个无法证明的completed决策。

新入口目标0 ALLOW、2 BLOCK、3 REQUIRE_APPROVAL、4输入/运行/验证失败；stdout为已冻结的版本化结果/诊断，stderr诊断，完整信封与前置诊断是不同schema。现有 `check --format json|sarif --output` 和所有旧入口保持不变。引擎报告未签名，verify仅重算；审批后原报告及对应信封decision不变。

## Trust, freshness and concurrency

完整绑定按共享字段携带repo/task/worktree/requirementIds/candidateOid/baseOid/mergeGroupId/sourceSnapshotDigest/baselineDigest；OID按实际对象格式核验，dirty字节额外绑定，authoritative merge消费需clean冻结候选。未配置审批服务不接受自建公钥/工作区boolean/Markdown accepted；GE-TRUST只提供认证引用验证port，批准由外部控制器签发。旧native例外处置意义仍由旧owner定义，本适配器不再实施一套白名单。

候选/base/merge-group、源字节、批准基线、规则/工具/配置、mapping/analyzer版本、coverage或审批过期/撤销使相关资格失效。独立技术报告不原地改写；撤销只改变消费资格。缓存首版关闭跨run复用；后续按所有不可变绑定去重制品，消费时重新核验审批。两需求/任务/worktree隔离，重试有新runId，同一次重复导入幂等；晚到结果只归档自己，不能覆盖新候选。最终队列候选由GG-CANDIDATE提供，可信CI运行原生检查再投影，不拼接不同PR head的绿色结果。

输入路径/符号链接/字节和深度限额、私有目录权限、redaction与工件保留由适配器具体测试；不声称现有native CLI全都只读或沙箱化。默认不启动工具、不修改 `.codeguard`、不访问网络/宿主、无push/merge。审计记录raw、contract/facts/report摘要、mapping版本、绑定、批准引用及拒绝原因；日志不保存secret、全量源/argv/env。不执行未信任的policy scripts。

## Phases, migration and rollback

1. C0：冻结native兼容矩阵和全部默认关闭profile；可与GE并行。
2. C1/C2：等GE-CONTRACT/ADAPTER后实现严格投影、完整范围准入、绑定/错误和影子输出；不改变native消费者。
3. C3/C4：等GE-TRUST和所需GG-CANDIDATE后做只读宿主/CI消费、并行需求、审批失效、精确队列candidate联验；不要求旧9.10全任务关闭。
4. C5：等GE-RELEASE后固定发布依赖、N/N-1实际支持矩阵、显式启用与回滚；停用适配器恢复native接口，未满足的required check保持不通过，不能靠回滚绕过治理。

依赖地址：[GE各门](https://github.com/full-stack-plugins/guardengine/tree/docs/guard-design-20261009/openspec/changes/add-versioned-guard-integration-contracts)、[GG只读绑定](https://github.com/full-stack-plugins/gitguard/tree/docs/guard-design-20261009/openspec/changes/add-candidate-bound-git-governance)。本域quality规则不进入GuardEngine；平台授权不进入adapter。

## Risks / Open Questions

首批可独立证明complete的native切片、SDK/进程调用、具体新入口名、mapping关系词表、认证provider、证据保留时长/资源默认值、支持宿主/平台版本尚待1.x/3.x任务决策。默认窄范围、无complete资格、无网络、无跨run缓存、原始报告不改写。发现所需native能力缺失时向旧change登记证据而不在本change重新实现。验收必须保存真实输入/输出/退出、负例和版本；此规划不声称测试通过。
