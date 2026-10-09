# 技术、组件与外部工具规划

技术方向已由本次用户确认；具体库版本在实施时核对官方支持、许可证、MSRV、平台与 artifact digest。本轮没有安装、升级或修改 Cargo.toml。新功能目录与依赖方向见 [design](../../openspec/changes/add-engineering-guard-core/design.md)。

## 1. 首期基础栈

| 技术 | 组件责任 | 当前/计划 | 不承担 |
|---|---|---|---|
| Rust | 共享 core/protocol/runtime/adapters/CLI | 复用已有 workspace，新增工程 crate | 不重写五个 Python/Node 宿主外壳 |
| Tokio | 有界子进程、取消、调度 | 已在 codeguard-runtime 使用，增量扩展 | 不是安全沙箱 |
| Serde/serde_json | DTO、闭合 JSON、版本协议 | 已有基础，新增 schema 对齐 | 不提供业务授权 |
| Clap | 独立 engineering-guard CLI | 新增；旧 CLI 不强制迁移 | 不承载领域规则 |
| JSON Schema | 契约/报告/发布清单结构验证 | 新增统一 schema，保留历史原件 | 不证明逻辑或证据真实 |
| OPA/Rego | 对已核验事实做动作策略决策 | 固定版本本地进程优先，后期可替换部署 | 不解析源码、不执行合入 |
| SQLite + 文件目录 | 本地状态/索引/内容寻址证据 | 新增私有状态存储；非可信根 | 不替代 FlowGuard docs 或 GitFlow 规则 |
| Git CLI + 独立 CI + 分支保护 | 实际候选、受信验证和合入边界 | 复用 GitFlow，补控制端 | Hook 单独不能阻止绕过 |
| rmcp | 标准 MCP 服务与取消/能力协商 | P2，CLI/协议稳定后接入 | 不开放批准签发和通用执行工具 |

SQLite 绑定、JSON Schema/JCS 实现、OPA 版本与 crate 具体版本在 EG-K01/02/05/08 锁定并留兼容测试。版本选择属于常规实现；若需提升现有 Rust 1.85 基线或改变支持平台，先作为兼容性变更处理。不得在五个 stdlib 插件内添加未经必要性证明的运行依赖。

## 2. 四守卫工具

| 范围 | 首选工具 | 能力限制 | 阶段 |
|---|---|---|---|
| Java 系统架构 | ArchUnit + Maven/JUnit | 字节码依赖/分层/循环，不能推导最合理聚合 | P0/P1 |
| Java 领域行为 | JUnit + 项目实际数据库/并发测试 | 测试覆盖范围显式声明；mock 不证明真实一致性 | P0/P1 |
| Java 代码/文档/安全 | 现有 CodeGuard P3C/Checkstyle/Javadoc/CVE 等适配 | 以已配置工具、原始报告和实际资格为准 | 复用/P2 |
| Rust | cargo metadata、Clippy、cargo test；API 兼容按需 cargo-semver-checks | crate 图不是完整方法图；feature/target 单独验证 | P2 |
| TypeScript | tsc、ESLint、dependency-cruiser、项目测试框架 | 动态模块/生成代码需能力声明 | P2 |
| 跨语言结构模式 | ast-grep，必要时复用既有 tree-sitter | AST 模式不等于类型/跨文件语义 | P1/P3 |
| 符号与影响 | 现有 CodeGraph；按语言需要接语义索引器/SCIP | SCIP 是协议，CodeGraph 是事实提供者，二者不直接判质量 | P3 |
| Git | 原生 Git CLI、托管平台受保护引用/合入队列 | 原始 shell 可绕本地 Hook，需独立远端控制 | P0/P2 |
| 流程 | FlowGuard 状态语义 + 统一 OPA 组合 | 不以阶段文本自证批准 | P0 |

语言声明采用逐格 capability matrix：language/build-system/version/feature/target/checker/coverage/trust。任何 not_supported/unknown 单元都保留，不因某一种语言成功而计全量。

## 3. 平台化扩展

P4 才增加 PostgreSQL、对象存储、组织身份服务和 OpenTelemetry；签名可使用组织密钥服务或 Sigstore/Cosign，但信任策略必须核验签发身份、有效期和 provenance。签名不证明测试充分。无需首期引入 Kafka、Neo4j、向量库、Kubernetes 或多 Agent 框架。

独立 Agent Runtime 通过 CLI/MCP/事件协议接入。Agent Fabric/Job/Buddy 等平台不作为首期硬依赖，也不在六仓规划中假定其当前接口。未来治理 UI 可读同一协议，默认尺寸遵循工作区规范，当前不创建界面实现。

## 4. 官方资料（本次对话已核对）

- OPA 的决策与执行分离：[官方部署说明](https://www.openpolicyagent.org/docs/deploy)。
- Rust MCP SDK：[官方仓库](https://github.com/modelcontextprotocol/rust-sdk)；异步基础：[Tokio](https://tokio.rs/)。
- Java 字节码架构规则：[ArchUnit](https://www.archunit.org/userguide/html/000_Index.html)。
- TS 依赖规则：[dependency-cruiser](https://github.com/sverweij/dependency-cruiser/blob/main/doc/rules-reference.md)。
- AST 模式：[ast-grep](https://ast-grep.github.io/guide/introduction)；包元数据：[Cargo](https://doc.rust-lang.org/stable/cargo/commands/cargo-metadata.html)。
- 符号索引协议：[SCIP](https://github.com/scip-code/scip/blob/main/scip.proto)。
- Git 工作树：[Git](https://git-scm.com/docs/git-worktree)；受保护分支：[GitHub](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches)。
- 合入候选检查：[GitHub required checks](https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/troubleshooting-required-status-checks)。
- 候选代码与权限隔离：[GitHub 安全说明](https://docs.github.com/en/actions/reference/security/securely-using-pull_request_target)。
- 文件类产物签名：[Cosign](https://docs.sigstore.dev/cosign/signing/other_types/)。

官方能力说明不是本机安装、兼容性或真实集成验收证明。
