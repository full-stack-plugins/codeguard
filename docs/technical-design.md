# CodeGuard — 技术实施方案与 GuardEngine 集成设计

> 文档版本：V2.1（独立产品集成方案），创建：2026-10-09。现有实现为 CodeGuard Rust workspace `0.1.4`；不是本方案所有功能已落地的声明。

## 1. 实施基础与不可破坏的兼容约束

现有四个 Cargo 成员：`codeguard-cli`（命令分发与运行用户交互）、`codeguard-core`（结果、任务与规则数据模型）、`codeguard-runtime`（执行/资源/证据基础）、`codeguard-adapters`（语言与工具适配）。构建基线：Rust 2024、MSRV 1.85、Cargo resolver 2；既有事实以 [Cargo](../Cargo.toml) 和 [当前实现说明](Codeguard-Technical-Design.md) 为准。

不可贸然修改既有 CLI 命令、已有 `check_feedback`/`check_aborted` Schema、原生工具选择策略、.codeguard 本地工作台结构、现有 repair tasks 语义、WASM 限制声明与已有 npm 制品渠道。

既有 [OpenSpec 变更](../openspec/changes/introduce-rust-codeguard-cli/) 是当前代码实现/待办的事实源；新增 GuardEngine 适配任务应建立增量变更，而不是将旧代码“批量重写”或将本蓝图视为已完成任务。

## 2. 技术选型与分层

| 层 | 已有技术/目标 | 设计理由 |
|---|---|---|
| Runtime | Rust，std；序列化用 Serde/serde_json，runtime 下载使用 Tokio/Reqwest | 单一性能敏感的可独立部署 CLI，不依赖模型 |
| Native | Java Maven/P3C/Checkstyle/Javadoc，Rust fmt/check/Clippy，Ruff，ESLint 等 | 检查规则归原生工具，避免自己猜语义 |
| 配置 | 静态 XML/TOML/JSON/ESLint 配置发现与摘要 | 区分声明/安装/执行/成功/覆盖 |
| 任务与恢复 | 现有 lease、attempt、task verify | 保留历史与修复无进展检测 |
| 格式 | 现有机器协议 + JSON Schema | 避免破坏当前消费者 |
| Guard Protocol | **目标** GuardEngine `guard.partme.ai/v1alpha1` | 公共契约/规则/证据结果采用版本化适配 |
| Agent 集成 | CLI + 外部 codeguard-plugin 兼容目标（未在本轮验证） | 插件只提供宿主事件与反馈，不重复规则 |
| CI | GitHub Actions / 受保护规则包 | 本地检查无权自行授予合并 |
| 通用规则边界 | 当前共享协议仅 `forbid_relation` | OPA/Rego 未实现，也不是本次接入前提 |

**现有代码已经包含内部名为 Core/Runtime 的 crate**：这不违反外部 GuardEngine 的独立性，但它们应继续只承担 CodeGuard 业务实现，不能承接六个 Guard 的通用协议和专业规则所有权。

## 3. 各模块目标执行契约（增量设计，非现有字段清单）

### 3.1 Discovery

先定位 Git 仓库、语言版本、模块根、Native tools、配置与编译环境，记录 `CapabilityObservation`：
`language/dialect/buildRoot/sourceSet/toolId/version/configRef/installed/executable/scope/limitations`。
发现结果不能倒推“所有必要工具已经执行”；找不到配置与工具是一种透明的阻塞状态，不自动安装或生成严格配置。

### 3.2 Plan

`CheckPlan` 包含：task/candidate、checkedSources、checkerVersion+argv、configurationDigest、enforcementMode、expectedCoverage、deadline、untrustedCodeExecutionRisk、resourceBudget。计划应在执行前冻结；不接受 Agent 在执行之后删掉失败 obligation 来给自己放行。

### 3.3 Execute

原生适配器以固定 argv 执行，受控 cwd、stdin、PATH、CPU/memory/time 及输出限制；若运行用户项目脚本，需要隔离并明确授权。并发调度按资源冲突和 dependency DAG 限制；取消要释放子进程与缓存资源。CLI 不尝试从未被认证的互联网地址下载第三方工具。

### 3.4 Parse & Normalize

逐工具验证原始报告格式和错误码，不使用关键词 grep 当作违规判断。正常退出 + 零诊断只说明**该工具在声明范围内没有检测到违规**；工具失败、输出截断、跳过文件/规则产生 `partial` 与 blocker，不能成为“清洁扫描”。

保留原生 ruleId、nativeSeverity、source path/line、构建根、插件配置版本；策略 severity、例外审批另设字段，源工具结果不因批准例外而重写为 PASS。

### 3.5 Repair Loop

保留现有 stable task ID、issue origin、attempt、lease 与 native verify；检查前后源内容摘要、工具配置与规则版本变化会导致旧 recheck 失效。Agent 不能仅删除诊断或改白名单就宣称已关闭任务；Task Closure 需要执行与当前源一致的原生复验以及完成范围检查。

### 3.6 GuardEngine Adapter（新增）

目标先提供无外部写入的投影模块：读取版本化 `check_feedback`，保存本域 finding 及绑定，仅把当前协议可表达的关系送入 GuardFacts，再由引擎生成 GuardReport。`NativeFinding`/`GuardReportProjection` 不是现有引擎类型。以下元数据属于本域旁车或集成信封，不能追加到封闭的 v1alpha1 facts/report：

- 精确记录 `producer=codeguard`、`producerVersion`、`candidateDigest`、`configDigest`、`ruleSetDigest`、`executionStatus`、`assessment`、`coverage`、`sourceSpans`、`rawReportDigest`。
- 旧 schema 字段不足时明确 `unverified/unknown`，不能补造值或把未知枚举映射为 ALLOW。
- 若只需要 Guard Protocol v1alpha1 的 `forbid_relation` 关系，则只能对该类关系进行通用精确检查；`TestPlan` 或 `ArchDesignQuality` 不得伪装成当前引擎提供的能力。
- 验证器/协议 major 不兼容、摘要无法计算或目标 changed 的报告不具备自动放行资格。
- 升级走独立 `CodeGuard <-> GuardEngine` compatibility matrix，旧 CLI 的输出语义不得受新规则包影响。

## 4. 数据模型与调用状态

目标逻辑模型（不替换现有同名 Rust 类型，不作为当前序列化 Schema）：

~~~text
SourceSnapshot
  ├── repo/ref/index/candidate OIDs
  ├── included files and digests
  ├── target/features/build root
  └── toolchain/config identity
CheckPlan
  ├── expected checks and categories
  ├── permitted executors / budget
  └── frozen coverage obligations
ExecutionRecord
  ├── result (completed/tool_failed/timeout/cancelled)
  ├── raw report IDs and produced files
  └── attempted/cancelled/remaining scope
Finding
  ├── original native rule ID and severity
  ├── source location, reason, remediation
  └── independent exception approval reference
CodeGuardResult
  ├── assessment
  ├── coverage
  ├── contract/subject/analyzer digest
  └── supported Guard Protocol projection
~~~

既有 schema 与本域数据模型应保持独立版本号。不可把多个具有不同级别保障的事实合并成单一 `passed=true`。

## 5. 统一 CLI、MCP、CI 的兼容路线

**当前 CLI（选定真实命令示例，因平台/配置而可能不可用）：**

~~~sh
codeguard detect .
codeguard capabilities
codeguard doctor .
codeguard check all .
codeguard task verify <TASK_ID> .
~~~

实际命令参数以 [命令参考](Codeguard-Command-Reference.md) 为准。不能因为示例中写有参数就假定所有版本均支持。

**不可运行的未来接口提案（尚未实现，参数待独立 OpenSpec 批准）：**

~~~sh
codeguard evidence export --candidate <oid> --protocol guard.partme.ai/v1alpha1
codeguard gate preview --task TASK-104 --contract approved.yaml
codeguard doctor --integration guardengine
~~~

CLI 对本域发现和检查给出报告，MCP 暴露 read-only check/diagnostics/evidence，真实源码修复与 Git 操作由现有 host plugin/授权环境处理。API 未来仅为私有/本地可选服务，不自动公开监听端口。

CI 的验证序列：读取受保护的 CheckPlan/规则 → 在最新合并候选独立运行适配工具 → 保存真实原生报告 → 归一化到 GuardEngine → 核对任务/版本/覆盖 → FlowGuard + GitGuard 处理是否允许合入。不得把 PR 中的 CI 配置用作唯一可信策略。

## 6. 安全与可靠性要求

- **命令注入**：argv 列表执行、限制外部 shell；源码/日志/规则文件均按不可信输入处理。
- **工具链供应链**：明确工具来源、版本、digest/安装权限；未知资产不自动执行。
- **源码执行隔离**：编译器/测试脚本可能执行不可信代码，CI Runner 需最小权限，不暴露高权限凭据。
- **证据抗篡改**：候选 SHA、原工具输出、输入文件/配置和受信规则绑定；修改检查脚本/降低阈值必须触发独立审核。
- **可恢复性**：执行取消、环境故障、OOM/超时、已有 Finding 的部分结果必须保留；新失败不能被旧 PASS 覆盖。
- **质量评估**：分类统计具体规则的误报/漏报，不宣称所有注册语言已经受支持；空扫描或未解析状态不等于 PASS。
- **迁移无破坏**：保持 npm/CLI/宿主兼容，升级功能默认显式 opt-in，验证 N/N-1 回滚路径。

## 7. 分阶段实施计划

| Wave | 工作与依赖 | 真实验收条件 |
|---|---|---|
| C0 | 当前 CLI 工作区/原生检查与修复任务的行为基线冻结 | 现有命令、用户文档和受支持平台回归 |
| C1 | GuardEngine SDK 依赖和读投影：check_feedback + incomplete | 完整/部分/旧版本/未知字段黄金 fixture |
| C2 | 固定 CheckPlan + 输入范围证据 + 差异检测 | 候选删掉必需检查不能放行 |
| C3 | codeguard-plugin 宿主 Hook / MCP/本域 CI | 实际宿主输入、取消、重入、凭据隔离 |
| C4 | ArchGuard/TestGuard/FlowGuard 的证据互操作 | 有效规则可以被独立重算，缺项阻断 |
| C5 | 受保护分支、真实项目及多平台回归、发布 | 本地绕过 Hook 无法合并、兼容矩阵与回滚通过 |

C0 已有多个部分能力，不能简单打勾整阶段；后续每个小交付按现有 OpenSpec 追加必要 proposal/design/specs/tasks，实际 RED→GREEN→回归才计完成。

## 8. 端到端验收用例

| 场景 | 期待结果 |
|---|---|
| Java/Rust/TS 项目配置合法且检查完全覆盖 | 提供本域范围内 PASS 及工具身份 |
| Ruff/Clippy/P3C 不存在或配置错误 | 显示准备缺口，不误判源码 |
| 原生检查先报违规后超时 | 保留已知违规和未完成范围 |
| 语法 WASM 仅有候选推断 | 标记 unqualified，不给 full lint PASS |
| AI 删除失败规则或使其变 warning | 可信契约仍要求原规则，产生未授权变更 |
| 修复后执行原生复验但源码又变化 | 报告过期，不完成 task |
| 代码复杂度高，但没有已批准禁用条件 | REVIEW/ADVISE，不以经验阈值谬称架构失败 |
| PR 修改自身 checks.yml 绕过强制检查 | 受保护的 CI 规则仍阻止合并 |
| 本地 CLI/插件反馈 PASS 但缺 TestGuard/ArchGuard | FlowGuard 阻断工程合并 |

## 9. 文档权威与版本管理

本文件描述 **CodeGuard 在独立 6 Guards + GuardEngine 架构中的目标集成设计**。现有可调用能力以 [原详细架构](Codeguard-Architecture.md)、[原详细技术方案](Codeguard-Technical-Design.md)、[适配器契约](Codeguard-Adapter-Contracts.md) 和 [OpenSpec](../openspec/changes/introduce-rust-codeguard-cli/) 为事实源，不以本文件替代或宣称历史事项已完成。


## 10. 三代退出码与输出契约：禁止静默归一化

本次核对 main `b499f13922647d4bf1e2344eaa5c37c450e302b5`。以 [main](../crates/codeguard-cli/src/main.rs)、[check](../crates/codeguard-cli/src/check_command.rs)、[Verdict](../crates/codeguard-core/src/verdict.rs)、[legacy mapper](../crates/codeguard-cli/src/legacy_v1_protocol.rs) 为依据。

| 接口层 | 数字/语义 | 消费方式 |
|---|---|---|
| 历史插件具名映射 | check/CVE FAIL→2 优先于 UNVERIFIED→1；Dockerfile UNVERIFIED→1 优先于 FAIL→2；CVE 错参→3 | 只按入口族解释；兼容投影恒 `new_delivery_decision=not_evaluated`；不是旧插件已部署验证 |
| 当前 Rust 请求领域 Verdict | passed / passed_with_exceptions / not_applicable→0，violations→1，incomplete→3，internal_error→4，cancelled→130；参数错误由入口返回2 | 请求结论不是交付结论，不能套 GuardEngine 表 |
| 当前公开 `check` | 正常局部反馈固定3；取消130；已接线内部故障4；错参2；不可读项目3 | 不会因零 finding 变0/ALLOW；部分早期错误只有 stderr，没有 JSON |
| 未来显式 GuardEngine check 适配入口 | 0 ALLOW / 2 BLOCK / 3 REQUIRE_APPROVAL / 4 输入、运行或验证错误 | 仅新增适配面使用；保留原 CLI/旧 JSON，并记录原始退出码和运行状态 |

当前 `check all . --format json --output /existing-directory/check.json` 是真实语法；`--report` **不是**当前 `check` 参数。`--output` 仅支持 json/sarif，human 组合执行前返回2。结构化报告仍写 stdout，同时尝试导出；成功文件和 stdout 同报 `export.status=saved`。导出失败时 stdout 保留 finding 和 `export.status=failed`，stderr 给原因；正常局部 check 仍为3，故障/取消仍保留其优先级。不能宣称所有错误路径都会输出 GuardReport 或统一错误 JSON。

[export_report](../crates/codeguard-cli/src/report_export.rs) 在同目录建立私有暂存文件，限制已有文件读取为16 MiB，拒绝非普通文件及非 CodeGuard 报告；已有目标比较原内容后替换，新目标通过硬链接避免覆盖。此措施不是跨机器事务或信任认证。既有 `check_feedback`、`check_aborted`、独立 RunReport 与 SARIF 均需按具体 schema/version 消费，不得只按 crate 0.1.4 推测版本。

## 11. 增量适配、失效和验收决策

[GuardRunEnvelope 草案](integration-contract.md) 与历史协议分开版本化；拟议错误信封不得伪装 GuardReport。完整有效分析才能形成技术决策；工具故障/取消的运行状态与 nullable decision 分离。partial facts 可进入引擎取得 BLOCK/INDETERMINATE，但控制器不能把缺证据改成人工批准通过。

迁移先做旁车读取，默认不改变原命令输出和副作用。现有 check 可以写 `.codeguard` 工作台并运行原生工具，因此“只读适配器”只指读取既有证据的新增投影，不是对所有 CLI 的承诺。未来 MCP、宿主和 CI 复用该投影，不另造规则解释器；无默认外部写入、通知、安装或合并。

受信控制器绑定不可变 repo/task/worktree/requirement/candidate/base/mergeGroup、批准基线 digest、规则及分析器版本/覆盖。执行前冻结，执行后复核，门禁时核验最新批准有效期和撤销。变化使受影响结果失效；按绑定隔离缓存和幂等键，保留晚到结果供审计但拒绝覆盖当前候选。合并队列重新分析实际候选树。路径和输入预算复用本域边界；CI 只读凭据、受保护规则、隔离原生插件/构建脚本、不执行不可信政策脚本，且不把进程超时误称全程 I/O/内存隔离已实现。

| 增量验收 | 最小可测通过条件（未来） |
|---|---|
| C0 契约冻结 | 保存历史具名混合优先级、当前 JSON/SARIF 和2/3/4/130故障样本；逐入口结构/语义回归无非预期变化（时间、运行 ID 等动态值单独校验） |
| C1 旁车投影 | 完整、部分、空范围、未知 major/字段、畸形输入五类以上 fixture；不支持 finding 关系不伪造事实；零 false ALLOW |
| C2 绑定/撤销 | candidate/base/merge-group、规则、分析器、覆盖、基线修订、批准过期/撤销逐项变更均失效；旧结果不能覆盖新结果 |
| C3 并发/宿主 | 两个独立 requirement/worktree 并发不串证据；同绑定重复幂等；取消和超时保留部分结果且无批准 |
| C4 引擎互操作 | 精确 forbid_relation 的 enforce/review/advise 正反例与 partial 全部符合冻结协议；重算与来源认证分开测试 |
| C5 CI/发布 | 实际 merge-queue 候选复算；工作区改规则不能绕过受保护控制器；N/N-1消费者与回滚均验证后再发布 |

开放决策：SDK 还是进程调用、最初哪些本域关系可无损表达、证据存储保留/脱敏时长、生产批准身份服务、首批宿主和平台资格。可逆默认是旁车文件、显式 opt-in、原始报告保留、未知/缺口阻断；不先承诺新命令可用或发布版本。

## 12. 本轮文档核验与已有测试索引

本轮只修改文档，进行相对链接、命令源码、命名和 diff 核对；未运行原生检查器、全量 Rust 测试或 OpenSpec validator。测试文件存在不代表本轮测试通过。

- [legacy_v1_protocol_contract](../crates/codeguard-cli/tests/legacy_v1_protocol_contract.rs)：具名旧入口优先级、固定 not_evaluated。
- [check_all_partial_contract](../crates/codeguard-cli/tests/check_all_partial_contract.rs)：局部 check、schema、预算、结构化导出边界。
- [check_sarif_cli](../crates/codeguard-cli/tests/check_sarif_cli.rs)：SARIF 与局部报告输出。
- [check_session_contract](../crates/codeguard-cli/tests/check_session_contract.rs)：冻结账本与请求/交付结论。
- [task_resolution_contract](../crates/codeguard-core/tests/task_resolution_contract.rs)：任务解决证据与作用域。
- [OpenSpec tasks](../openspec/changes/introduce-rust-codeguard-cli/tasks.md)：2.3/2.4/2.5/2.9等仍有未完成集成；历史验收记录不可替代本轮重跑。
