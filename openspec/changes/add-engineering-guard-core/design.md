## Context

本设计是 Partme Guard 七项目的通用机制与集成设计，动机见 [proposal](proposal.md)。六仓当前事实与局限见[仓库基线](../../../docs/engineering-guard/repository-baseline.md)。当前 codeguard 已有四 crate、Tokio/Serde、原生工具适配、双维结果和 `evaluate_delivery`；不能把它描述成空项目，也不能将纯函数的 `trusted_bindings_verified` 布尔输入当成已经存在的可信边界。

此 change 暂存尚未建立目标仓库的共享规划；需求和任务的目标 owner 见 ownership.md，架构规则必须在 ArchGuard 实现。原 `introduce-rust-codeguard-cli` 继续拥有语言/平台/宿主迁移和质量检查任务。本轮没有代码迁移、依赖安装、CI 接线或运行验收。

## Goals / Non-Goals

**Goals:** 在批准范围内支持任意编码 Agent；七项目共享一个协议和有效契约；对未知、旧证据和绕过尝试不放行；从单项目 CLI 到可信 CI，再到平台治理逐步交付。

**Non-Goals:** 不训练新模型、不自研通用编译器、不把六守卫建成六个 Agent、不直接推翻旧协议、不迁移 FlowGuard 规格体系、不把七项目混成一个工作区内核、不替用户批准核心架构取舍。不把 Java 试点当成原 CodeGuard 全语言验收。

## Decisions

### D01 七项目与单向依赖

当前架构见[七项目边界](../../../docs/engineering-guard/product-architecture.md)。GuardCore 拥有 Rust/Tokio/Serde/Clap、JSON Schema、OPA 执行端口、SQLite/文件证据、身份/运行/授权机制；六专业项目依赖其协议和端口。GuardCore 不依赖任何专业守卫源码，不持有专业域策略包。

拟议 GuardCore 内部为 `protocol`、`core`、`runtime`、`cli`、`transports`，按实际复杂度划分 crate；具体守卫内部为规则、事实适配、检查和接入模块。ArchGuard 的四层能力和 GitGuard 的分支/范围/冲突能力均是内部模块。CodeGuard 现有 workspace 不新增全套公共内核 crate，也不为统一产品入口重写旧 CLI。

`engineering-guard` 仅是已废弃的工作名；目标可执行名称为 guardcore/specguard/archguard/codeguard/testguard/gitguard/flowguard。已有插件命令保持兼容，新命令需独立版本和安装清单。当前 CodeGuard 的 MSRV 约束保留；新项目的具体版本在实施时核验。

规划暂存在此旧名 change，正式移交见 EG-M01；不擅自初始化新项目或规格体系。GitGuard/FlowGuard 先适配现有实现，再经差分验证转移规则所有权，不复制并长期维护两份规则。

### D02 一个契约注册表，保留原生事实源

注册表为不可变索引和批准快照，不复制 OpenSpec/Spec Kit/Superpowers 正文。项目契约候选建议放 `docs/engineering/contracts/`，ADR 保留现有位置；批准快照由可信控制端保存。目录创建属于未来显式接入，本轮不修改被治理项目。

规则来源为组织基线、项目架构契约、任务范围、动作策略。只允许在父级约束允许的参数内细化；组织锁定项不能被下层降低。冲突返回契约错误，不能用“后者覆盖前者”处理。规则编译冻结 obligation ID、输入目标集合、验证器身份、适用条件、mode、失效条件和必需证据。

GitFlow `.gitflow/workflow.json` 仍是 Git 规则源，FlowGuard 十阶段 docs 仍是流程源。Registry 引用并规范化它们；GuardCore 不重新实现 GF001、规格一致性、测试阈值或十阶段顺序；规则所有权见产品迁移设计。JSON Schema 校验结构，GuardCore 校验通用引用、冲突、预算与验证器能力；各守卫校验本域语义。

Agent 可提交规则/测试变更候选，但该候选不能作为放行自身的策略。测试允许新增和修正；必需测试清单、验证器入口和降低验收强度的改动单独审查，禁止一刀切冻结全部测试。

### D03 六专业守卫与外部提供者

六个专业域及完整执行图见[项目架构](../../../docs/engineering-guard/product-architecture.md)。SpecGuard 校验规格，ArchGuard 校验架构，CodeGuard 校验代码，TestGuard 管理测试验证，GitGuard 管理变更，FlowGuard 管理流程。CodeGraph 只提供事实，CodeReview 只提供 REVIEW/ADVISE。

架构行为约束由 ArchGuard 定义，测试执行计划和覆盖证据由 TestGuard 负责；第一阶段通过临时 native-test provider 执行已有真实测试，不冒充完整 TestGuard。SpecGuard 未交付前批准的需求基线仍必须绑定；缺少契约要求的守卫不能因分期而绕过。

FlowGuard 产品拥有目标流程语义，现有 flowguard_lib 在迁移验收前仍是唯一判定实现。GuardCore 运行 FlowGuard 的已批准组合策略，不重新定义阶段规则。controller、候选 runner、执行器维持独立身份边界。

### D04 分离报告、决策、授权

协议细节见[协议设计](../../../docs/engineering-guard/protocol.md)。`GuardResult` 说明观察，`GateDecision` 说明某动作的条件是否满足，`ExecutionGrant` 说明受信身份获准执行的精确动作。Agent 提交的 `authority=trusted` 或 `trusted_bindings_verified=true` 都是数据，可信控制端必须重新核验。

现有 codeguard `evaluate_delivery` 作为代码守卫子结论继续复用，不另造规则绕过它。FlowGuard 的工程门禁策略在其上合取规格、架构、测试、Git、流程、审查处置与授权；OPA 输入包含已验证子结果和上下文，输出闭合 decision/reasons/requiredActions。OPA undefined、错误、超时、缺结果或未知字段均为 incomplete。程序内不可豁免的来源/身份/版本校验在 OPA 之前执行。

### D05 事实与模式不能混淆

ENFORCE/REVIEW/ADVISE 是批准的处置模式；execution、verdict、coverage 分别表达运行、发现和覆盖。部分执行中确定的违规必须保留，缺工具不能掩盖违规，也不能变成无违规。仅在已证明适用目标为空时给 NOT_APPLICABLE；空扫描/漏选目标不是 PASS。

自然语言职责标签只提供评审上下文。禁止将 LLM 分数、方法行数或启发式图边升级为确定性领域违规。方法归属已经被批准契约精确规定时才可强制检查；如 `cancelTask` 应用协调与聚合状态转换可以同时存在。

### D06 Git 与候选身份

`SubjectSnapshot` 区分 worktree、index、commit、merge_candidate、release_artifact。包含仓库 ID、worktree ID、SHA 算法与完整 OID、base/head、tree、差异、实际输入清单摘要；不能用一个 commitSha 描述所有操作。

本地 commit 对应 index 树与父提交；push 对应实际目标 ref、期望远端 OID 和待推对象；merge 对应目标基线与确切候选树。最终检查后目标变化则失效重建候选。可信服务通过平台合并队列或支持预期版本的原子引用更新落实比较更新；不采用“检查后直接普通 push”冒充消除竞态。

CodeReview 当前 index 快照不能自动重标为 HEAD 或 merge_candidate；缺对应输入能力时返回 incomplete，由其自身受控候选适配补齐。语义冲突图谱仅预警；无文本冲突不能替代最终编译、契约和集成验证。

### D07 存储与一致性

本地默认 SQLite + 内容摘要文件目录，置于仓库外宿主私有状态根（具体根显式配置，CLI 不假设某宿主环境变量存在）。FlowGuard docs、GitFlow `.gitflow/state/`、CodeGuard `.codeguard/` 和 CodeReview 仓外状态各自保留；共享数据库不变成另一份阶段或 Git 配置事实源。

SQLite 表族：contract_snapshots、runs、obligations、evidence_refs、decisions、grants、events、leases；外键/唯一约束限定 requestId、operationId、subjectDigest。WAL、busy timeout、有界重试；事务不跨长时间工具执行。文件先写临时文件、校验摘要、原子 rename，事务再登记；启动时隔离孤儿或缺失产物，不删除活跃任务证据。取消终止整个受管进程树，保留部分观察，不盲重放 Git 写操作。

追加事件用于审计与恢复；本机同权限写者能篡改 SQLite，所以它不是可信账本。可信 CI/控制端单独保留证据及授权。平台阶段再接 PostgreSQL/对象存储，使用 outbox 或等价事务边界保证上传与事件一致性，不先引入消息队列或分布式工作流引擎。

### D08 可信执行与隐私

三等级：advisory、cooperative、enforced。只有独立身份、批准规则来源、受限候选执行、受保护目标与真实绕过反例全部验收，才可声明 enforced。文件权限、Worktree、摘要或签名其中任一项都不单独构成隔离。

构建脚本/测试/模型报告按不可信内容处理。Runner 无合入、签名和生产凭据；controller 不执行候选代码；执行器只接受已验证 grant。规则来源、验证器包和必需测试清单不能来自该候选自改的控制配置。控制配置及 `.github` 等点目录身份核验属于授权面，不改变旧 CodeGuard 点前缀源码扫描规则。

私有原始日志与可公开摘要分离；日志脱敏、大小/时间预算、目录权限与保留期限可配置。CodeReview 外发目的地、模型及内容范围沿用明确授权，核心不自动发送源码。报告正文中的命令或提示词不执行。散列提供完整性，可信身份/隔离/收据提供来源，二者分别验收。

### D09 CLI/MCP/宿主接线

七项目分别提供自己的 CLI，GuardCore 提供通用 contract/plan/run/evidence/policy，六守卫提供本域 check；完整 CLI/MCP/Actions/Hook/API 规划见 [interfaces](../../../docs/engineering-guard/interfaces.md)。均为拟议能力。

查询默认不写项目、不安装、不索引、不 fetch。`check` 可能执行项目工具并写私有报告，必须在命令元数据披露副作用与授权；只读 plan 不运行构建器。CLI 0 表示该命令成功，质量必须读结构化 gate；1 确定拒绝、2 用法错误、3 未完成/待审查、4 内部错误、130 取消。旧 CodeGuard/GitFlow/宿主退出语义继续由显式适配器映射，不按数字猜测。

所有入口传递 operationId/subjectDigest/action，单飞与幂等去重基于此组合；不同内容不能误合并。FlowGuard 负责流程缺口交互，CodeReview 负责模型调用授权，GitFlow 负责 Git 操作确认；协调器发布一个组合问题，不把已有范围授权重复询问。MCP 默认只提供查询/验证，不开放通用 shell 或批准签发工具。

### D10 P0—P4 技术与语言范围

P0：Rust/Tokio/Serde/Clap、JSON Schema、固定版本 OPA 子进程、SQLite、Git、独立 CI、分支保护；Java Maven 多模块 fixture、JUnit 和 ArchUnit 构成首条真实闭环。选择 Java 因其可直接验证分层和领域契约，不以缺少 CodeGraph 阻塞确定性工具能完成的检查。

第二阶段以 P1 补全 SpecGuard/TestGuard，同时深化系统/领域四类核心规则，保留合法应用协调和聚合行为；补 Gradle 档位与批准的适用条件。P2：CodeGuard 原生能力衔接、Rust Cargo/Clippy、TS tsc/ESLint/dependency-cruiser、rmcp 与实际宿主；每语言独立能力清单。P3：符号/API/方法风险、并行任务影响集合与冲突预警。P4：基线趋势、ADR 演进、跨仓消费、治理平台可观测性。

不锁定“所有工具最新版”。实施时复核官方版本、许可证、MSRV、平台和摘要；OPA CLI 在本轮 PATH 中不存在，只记录准备项，不安装。工具依据见[技术选型](../../../docs/engineering-guard/technology.md)。

## Risks / Trade-offs

- [旧 Hook 可绕过] → 保留协作等级，服务端独立门禁才可声明强制。
- [多仓与协议漂移] → 共享黄金 fixture、固定 manifest 摘要、N/N-1 兼容与未知拒绝，不共享 Python import 路径。
- [领域规则误判] → 精确约束使用 ENFORCE，启发式使用 REVIEW；维护合法反例与独立语料。
- [验证很慢] → 先冻结影响集合，缓存键包含源码/契约/验证器/环境；必需回归不能被启发式裁剪。
- [信任布尔值被伪造] → 可信输入由控制端取得，普通 CLI 不签发执行授权。
- [旧规划/验收描述冲突] → 各原 tasks 与源绑定报告保留；总览不能覆盖原验收。执行前刷新受影响基线。

## Migration Plan

1. 协议/schema/黄金 fixture 先冻结，所有消费者先读新协议，不改旧行为。
2. Shadow 模式运行并对比旧结论，不能签发 grant；新增结果不得削弱已有阻断。
3. 显式对一个试点启用 cooperative，保留原生检查与原流程授权。
4. 独立 CI/controller 配置受保护规则与目标，通过所有负向验收后启用 enforced。
5. 分仓不可变发布，release manifest 固定每个 repo SHA/tag/artifact digest/schema/policy 版本；CLI → 消费者 → 宿主 → 受控启用。
6. 回滚只恢复已验证版本及兼容 schema；不删除证据、不恢复已撤销 grant、不自动关闭远端保护。若旧版本不能满足新必需策略，停止交付而不是降级放行。

所有实施与完成状态只在本 change [tasks](tasks.md) 和各消费者原生任务账本记录；总路线图只汇总依赖和验收。
