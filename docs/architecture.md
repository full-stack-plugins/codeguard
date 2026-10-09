# CodeGuard — 代码守卫总体架构

> 版本：2.0（面向独立 Guard 产品的架构边界文档） · 日期：2026-10-09 · **现有实现**：Rust Workspace 0.1.4。该文档补充而不覆盖 [既有技术架构](Codeguard-Architecture.md) 与 [技术方案](Codeguard-Technical-Design.md)。

## 1. 定位：守住实现质量，而非重新设计系统

**CodeGuard** 负责验证“AI 编写的代码实现是否符合选定编程语言、项目配置、质量、安全及构建规则”。其价值不是在 Rust 中重新实现 ESLint/Clippy/P3C，而是对原生工具的发现、配置、调用、诊断、任务化修复和复验提供**可复现的统一工程契约**。

已有实现是一套 Rust CLI 工作台：按项目发现实际语言和工具配置，在明确范围内运行 Maven/P3C/Checkstyle/Javadoc、Ruff、Cargo/Clippy/Rustdoc、ESLint 等适配路径，对原生诊断保留规则、源码、工具版本、执行状态与补救动作。已能在部分语言组合中建立任务与修复循环，**不意味着完整质量门禁、所有语言覆盖、可信合并授权已完成**。

在独立七项目架构中，CodeGuard 不实现公共 GuardEngine，也不负责 ArchGuard 的设计合理性判断、TestGuard 的验收覆盖、GitGuard 的合入或 FlowGuard 的批准。CodeGuard 通过版本化协议提供**本域技术证据**。

## 2. 实现事实源与技术资产

以代码和现有规范为准：
- [Rust workspace](../Cargo.toml)：`codeguard-cli`、`codeguard-core`、`codeguard-runtime`、`codeguard-adapters` 四个成员，edition 2024、MSRV 1.85、0.1.4。
- [现有架构详解](Codeguard-Architecture.md) 与 [技术实现详解](Codeguard-Technical-Design.md)：真实命令、原生适配、限制、版本与验收索引。
- [CLI 入口](../crates/codeguard-cli/src/main.rs) 与 [check 编排](../crates/codeguard-cli/src/check_command.rs)：功能分发、原生调度与报告同步。
- [核心规则/聚合](../crates/codeguard-core/src/aggregate.rs) 与 [delivery gate](../crates/codeguard-core/src/delivery_gate.rs)：已有局部纯判定；**不能据此声明受信合并门禁已生效**。
- [适配器契约](Codeguard-Adapter-Contracts.md)、[测试证据](../tests/acceptance)、[既有 OpenSpec](../openspec/changes/introduce-rust-codeguard-cli/)：当前行为及未完成任务的权威记录。

现有 `check_feedback` 等 Schema 使用自有版本，不能直接等同于 GuardEngine `v1alpha1` 的 GuardReport。跨守卫接入必须通过明确投影适配，保留原始报告、不降低未知和部分覆盖。

## 3. 产品组件与数据流

~~~text
                     Project + Configuration
                             │
                      Project Discovery
       language / roots / checker declaration / versions
                             │
                   Capability and Policy Plan
                       scope + allowed tools
                             │
                  Native Checker Orchestrator
          Maven / P3C / Checkstyle / Javadoc / Cargo
                  Ruff / ESLint / Audits / ...
                             │
                   Native Structured Reports
               exit / diagnostic / version / source span
                             │
              Normalizer & Partial Evidence Aggregator
                             │
                   Findings / Blockers / Coverage
                             │
                    Repair Task Workbench
                 claim → attempt → verify → repeat
                             │
             CodeGuard Result + Guard Protocol Adapter
                             │
                        GuardEngine
                    Rule / Contract / Evidence
                             │
             CLI / Host Plugin / CI / MCP / FlowGuard
~~~

### 3.1 Discovery 与能力分层

区分：声明了工具、安装了工具、可执行、适用于当前语言/文件、确实执行、完成了要求的范围、结果足以验证一个契约。这些状态绝不可合并为 `supported: true`。发现过程默认静态只读，不隐式下载工具或执行用户构建脚本。

### 3.2 Native Checker Adapters

原生检查器拥有其规则含义，CodeGuard 拥有：
- 配置来源和实际生效范围的提取；
- argv、工作目录、目标输入和资源预算；
- 原始报告/退出码一致性校验；
- 源码位置、规则 ID、原生严重性与策略严重性的分离；
- 缺工具、错误配置、工具故障、部分范围和安全风险的完整反馈。

明确不通过 grep 文本中的“error”推导成代码违规。对 JS/TS 要识别最近 package 根及 ESLint flat config；对 Java 要保留 Maven/JDK/插件差异；对 Rust 要明确 Cargo target/features；对 Python 要区分 Ruff/pip-audit 的实际能力与规则覆盖。WASM Grammar 可作为**未获原生能力时的有限候选**，不能假装完成编译/类型/业务语义验证。

### 3.3 Findings 与修复任务工作台

从一份报告转为稳定 Finding、原始证据、可执行修复建议和任务。维护任务申领 lease、尝试历史、修复后的真实原工具复验。一个原生诊断消失不必然说明任务已完成：可能因工具未运行、配置改变、源码范围变更、结果被截断。工作台必须保留失败尝试和导致结果未知的原因。

### 3.4 多维结果模型

独立返回：
- `execution`：COMPLETED / FAILED_TOOL / TIMED_OUT / CANCELLED；
- `assessment`：SATISFIED / VIOLATION / UNKNOWN；
- `coverage`：COMPLETE_WITHIN_SCOPE / PARTIAL / NOT_RUN；
- `scope`：目标文件/模块/语言/工具及忽略规则来源；
- `evidence`：工具、规则版本、报告摘要、源输入和诊断位置。

历史 Schema 可继续存在；向 GuardEngine 映射时必须保留分离信息，不能把退出码 0、空诊断或 CodeReview `success` 推成全局 ALLOW。

## 4. 六大守卫的职责边界

| 项目 | 拥有 | CodeGuard 如何协作 |
|---|---|---|
| SpecGuard | 需求、规格、批准验收基线 | CodeGuard 只关联任务/需求 ID，不定义验收需求 |
| ArchGuard | 模块、领域、对象、方法架构契约 | CodeGuard 提供静态质量/复杂度事实，设计风险不升格为硬规则 |
| TestGuard | 必需测试计划、行为验证和覆盖 | CodeGuard 可提供构建检查结果；不得替 TestGuard 判定测试充分 |
| GitGuard | 版本、分支、变更范围、最终合并候选 | 共享绑定同一候选的检查证据，不触发受保护合并 |
| FlowGuard | 阶段、人工批准、下一动作资格 | 消费 CodeGuard 本域报告，缺证据不能自填 PASS |
| GuardEngine | 通用协议、契约、规则执行、证据基元 | CodeGuard 依赖其版本化 SDK，不把六 Guard 规则塞回 CLI |

现有 [codeguard-plugin](https://github.com/full-stack-plugins/codeguard-plugin) 是宿主集成层：Hooks/命令/反馈/安装；要避免在插件中再造独立代码质量规则引擎。其它 [codegraph-plugin](https://github.com/full-stack-plugins/codegraph-plugin) 与 [codereview-plugin](https://github.com/full-stack-plugins/codereview-plugin) 分别是结构事实和 REVIEW/ADVISE 来源，不是 CodeGuard 完成质量门禁的替代证据。

## 5. 系统信任与安全

**本地 CLI 不能保证不可绕过。** AI 可以控制其工作区和本地 hook，因此“本地检查绿”只能反馈检查事实。要实现强制治理，CI 必须从受保护规则版本生成固定检查义务，使用受信工具版本与正确的 checkout、重新执行并绑定当前 Git commit/tree；合并必须由 Git 平台分支保护/required checks/可信操作者执行。

安全执行约束：
- CLI 只运行经明确批准的原生工具及参数；避免将源内容拼成 shell 命令；
- 原生执行可能执行项目脚本或加载插件，必要时隔离容器/VM，并剥离生产凭据；
- 解析输入路径、符号链接、报告内容和下载资产需要边界/大小/签名检查；
- 无效/过期报告、异常、截断不得默默改成 PASS；报告回放须匹配源与检查计划；
- 本地生成的白名单提案、压低规则严重性的更改不能自行获得批准。

## 6. 架构 ADR 与质量函数

- CG-ADR-001：沿用成熟原生检查工具，Rust 负责统一发现、执行、解释和修复任务。
- CG-ADR-002：通用规则/证据属于 GuardEngine；CodeGuard 独有代码质量领域规则与适配器。
- CG-ADR-003：区分执行成功、质量结论、覆盖完整度，缺一不可。
- CG-ADR-004：安全/检查器适配来源与原始结果不可被模型文本覆盖。
- CG-ADR-005：不得将模型审查分数、行数阈值或单一静态指标当成完整架构质量证明。
- CG-ADR-006：正式合并强制性必须在受保护的 Git 平台验收，CLI/Hook 不冒充强制能力。

## 7. 当前完成和目标演进

| 项目 | 状态 | 判定 |
|---|---|---|
| 0.1.4 Rust 多 crate CLI、配置发现、部分原生工具接入 | 已有可测试代码和分范围测试 | 仅声明工具明确支持的语言/范围 |
| 稳定的修复任务、claim/attempt/selected verify 流程 | 部分实现 | 不能夸大自动闭环/代码正确性 |
| 32 语法候选等 WASM 路由 | 候选能力 | 无法取代编译器/原生分析和语言正式资格 |
| 与 GuardEngine 协议互操作 | 目标 | 需要版本转换和 N/N-1 回归 |
| 与 CodeGuard 插件/真实宿主完全打通 | 目标 | 需要真实 Codex/ZCode/Kimi/Claude 宿主验收 |
| Trusted CI / GitHub required check | 目标 | 需实际证明改规则/漏测试不能合并 |

具体 crate 适配、协议映射与 Wave 实施见 [技术方案](technical-design.md)。**本文件为新的体系角色与边界说明，不取代已有 1.2.4 技术手册和 OpenSpec 实施账本。**
