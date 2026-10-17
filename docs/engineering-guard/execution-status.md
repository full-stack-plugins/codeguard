# 实施状态与规划缺口

核对日期：2026-10-17。依据当前本地原生任务账本、源码核对及对应已发布提交的 GitHub 检查结果。本文是事实与缺口说明，不是第二份任务完成账本，也不替代正式规格或运行验收。

## 总体判断

最新远端复核：guardengine `0981705d3dfbfa472cbc60d02c16d3ebaf5094ea` 的 [CI 38963571148](https://github.com/full-stack-plugins/guardengine/actions/runs/38963571148) 已终态成功，15/15 jobs通过。CodeGuard `d978e3b50aa3e18ec0d4e2780617562adf2a2d68` 的 [CI 38964163817](https://github.com/full-stack-plugins/codeguard/actions/runs/38964163817) 终态失败：MSRV通过，真实CLI输出与历史Schema不一致，后续真实Maven步骤未执行；同提交安装检查成功。下文旧CI表及“仍待终态”为当时快照，不能代替此处最新结果。当前精确0.73修正已获root独立30项回归、107份Schema闭包与分层检查通过，并经独立只读审查；仍须提交后的Ubuntu真实CLI/Maven及完整CI验收，EG-K07保持未完成。

整体架构已有职责划分、协议设计、路线、所有权和验收矩阵。原先缺失的插件任务已按既有规划重建，当前113项稳定任务、66条需求、94个局部场景及43个端到端用例均可追踪，内部依赖无环；全部实现和验收仍未完成。最近八个增量批次的任务勾选和七仓 CI 成功，只覆盖各自批次与检查范围，不能证明完整 P0–P4 路线已交付。

## 实际仓库与规格事实源

| 逻辑角色 | 实际仓库 | 总体任务事实源 |
|---|---|---|
| GuardCore | `guardengine` | 当前 CodeGuard 临时跨仓账本；公共机制在此仓实现 |
| ArchGuard | `archguard` | 总体义务保留临时账本；已实施批次沿用本仓 OpenSpec |
| CodeGuard | `codeguard` | `add-engineering-guard-core` 与既有 `introduce-rust-codeguard-cli` 各自维护范围 |
| GitGuard | `gitguard` | 临时跨仓账本，尚未逐条移交 |
| FlowGuard | `flowguard` | 临时跨仓账本，尚未逐条移交 |
| SpecGuard | `specguard` | 临时跨仓账本，尚未逐条移交 |
| TestGuard | `testguard` | 临时跨仓账本，尚未逐条移交 |

GuardCore 是角色名，`guardengine` 是已有实现仓库名；此映射不意味着新协议、授权或存储能力已经验收，不新建或重命名仓库。相关插件保留自身既有规格事实源；不得把插件兼容任务与独立产品实现任务合并为一项已完成能力。

## 原生任务快照

| 仓库 / change | 已勾选 / 总数 | 解释 |
|---|---:|---|
| guardengine / add-agents-context-governance | 5 / 5 | 已有批次记录 |
| guardengine / add-guard-agent-installation | 6 / 6 | 已有批次记录 |
| guardengine / bootstrap-guard-protocol | 7 / 10 | 发布、受保护规则基线和分析器能力词汇仍待完成 |
| guardengine / execution-trace-evidence | 6 / 6 | 既有证据链范围 |
| guardengine / integrate-architecture-methods | 6 / 6 | 既有技能整合范围 |
| guardengine / professional-guard-checks | 7 / 7 | 既有专业检查范围 |
| archguard / add-cargo-workspace-guard | 7 / 10 | 版本化依赖、多语言分析和受保护策略仍有缺口 |
| archguard / add-maven-architecture-loop | 6 / 6 | Java/Maven 既有批次 |
| archguard / java-source-architecture | 6 / 6 | Java 源码既有批次 |
| archguard / project-model-baseline | 6 / 6 | 项目基线与需求增量既有批次 |
| codeguard / introduce-rust-codeguard-cli | 322 / 354 | 32 项未勾选；勾选不等于本次重新验收全部语言与宿主 |
| codeguard / add-engineering-guard-core | 9 / 62 | EG-M01、K01–K06、S01及A01局部任务已验收；K07及其他任务保持未完成 |
| 四插件集成changes / FlowGuard追加计划 | 0 / 51 | 按既有总体要求重建的原生定义；不是历史任务已完成证明 |

以上为人工复核时点快照；任务状态仍以各原生账本为准。未勾选不代表没有任何源码，已勾选也不能替代与实际候选、工具及范围绑定的证据。

## 规划引用缺口与重建

重建前，62项总体任务的内部依赖无环，但存在24个尚无任务定义的外部引用，历史失败快照保留在guardengine的 `docs/delivery/audit.before-reconstruction.json`：

- CodeGraph：EG-F03、EG-F05、EG-F08。
- GitFlow：EG-G01、EG-G04、EG-G06、EG-G07、EG-G08、EG-G10。
- CodeGuard 宿主：EG-H04、EG-H07、EG-H08、EG-H10。
- CodeReview：EG-R04、EG-R05、EG-R07、EG-R10。
- FlowGuard 插件：EG-W01、EG-W03、EG-W05、EG-W08、EG-W10、EG-W12、EG-W13。

重建前，四个 `integrate-engineering-guard-facts/host/review/git` change 在对应插件检出和本地可用历史中未找到；FlowGuard原计划也没有EG-W定义。这不能证明其他机器或未获取引用中从未存在。现已依据既有traceability、acceptance、protocol和各插件维护约束重建这些原生定义，明确不是恢复历史原文；所有新增任务仍待实施。历史验证页保留原报告边界，不用新结构校验追认历史结论。

四个新change已通过OpenSpec严格校验；FlowGuard沿用原生Superpowers追加章节。provider兼容验证先于最终迁移验收，原任务最终义务不减少。库存逐项核对正式需求/场景/端到端用例、依赖和权威owner，113任务引用可解析且无环。EG-M01按原任务的“建立移交清单”范围取得独立验收，证据在guardengine/docs/delivery/EG-M01-evidence.md；实际接收和权威切换仍由后续迁移任务完成，不扩大M01范围，也不据此关闭运行验收。

## 实现与验收缺口

| 范围 | 当前可复用基础 | 完整任务尚缺 |
|---|---|---|
| 公共协议与契约 | 旧类型化协议、规则校验；新 protocol crate 候选已通过本地测试 | 公开协议发布、真实批准服务及产品接入；本地协议/字段冻结、契约编译与冻结义务已验收 |
| 输入与证据 | 文件摘要、显式签名回执、内容复核 | 运行序列、旧结果覆盖裁决及runner集成；Git候选与本地事务存储局部任务已验收 |
| 测试执行 | 真实 Maven/Cargo 执行及报告复核 | 共享有界 runner、进程树取消、隔离身份、输出预算及崩溃恢复 |
| 架构治理 | Java 包依赖/分层/领域对象、Cargo 图、项目基线与增量 | 真 ArchUnit、领域不变量行为测试、方法契约、Gradle/TS、独立评测、ADR 演进 |
| 交付授权 | 本地选定公钥的签名核验，技术通过仍需批准 | 独立授权来源、撤销/范围/audience、grant 消费、精确目标条件和断连对账 |
| 流程与插件 | 各域检查及 TestGuard→SpecGuard→FlowGuard 证据接线 | 六域统一动作策略、固定 provider 版本、宿主兼容和受保护目标防绕过验收 |
| 发布与运行 | 七仓 CI 成功及现有安装机制 | N/N-1 互操作、四平台发布、跨仓恢复、租户隔离及生产可观测性 |

协议已完成本地K01冻结，包含D06的独立release_artifact模型；真实制品capture/provider与K20完整产品/provider/policy/toolchain发布组合仍未实现。K05存储基础也不能替代D07列出的全部运行/决策/授权账本与控制端恢复。后续必须逐项实现这些显式范围，不能用现有库测试关闭对应总体任务。

本地签名只证明所选密钥签过内容，不能证明受保护 CI 身份、合法批准或执行权限。当前文件快照不等于 Git 的 index/commit/merge_candidate。源码检查不等于方法调用解析完整。后续必须保持这些边界。

## 已发布 CI 证据

| 仓库 | 已检查提交 | 成功运行 |
|---|---|---|
| guardengine | `55b0a643a732b0ba46a1479f7b1e631fd57b8bb7` | [CI](https://github.com/full-stack-plugins/guardengine/actions/runs/38900385008) |
| archguard | `151b99b95a17710e9e5a4319a35dae95f5d61ab3` | [CI](https://github.com/full-stack-plugins/archguard/actions/runs/38900393114)、[安装](https://github.com/full-stack-plugins/archguard/actions/runs/38900392942) |
| codeguard | `8d7c0e997bf21b3306573b3be89ff8c312c9de38` | [CI](https://github.com/full-stack-plugins/codeguard/actions/runs/38919855480)、[安装](https://github.com/full-stack-plugins/codeguard/actions/runs/38919855655) |
| gitguard | `6d017a43c270e768360c6abc2e1f69568c210f86` | [CI](https://github.com/full-stack-plugins/gitguard/actions/runs/38888942256) |
| flowguard | `d690c367b697ba686c8e41cfe081ffbca56a9f03` | [CI](https://github.com/full-stack-plugins/flowguard/actions/runs/38888948113) |
| specguard | `cfdc676a5e2fd1f7449ba09d42f5e06c920b392f` | [CI](https://github.com/full-stack-plugins/specguard/actions/runs/38888949317) |
| testguard | `2100f3e4d5ea079aa91579bdfa5140b99591c90b` | [CI](https://github.com/full-stack-plugins/testguard/actions/runs/38888948106) |

上述成功绑定这些提交；后续未提交修改、新协议 crate 和文档补齐不在这些运行覆盖范围内。

## 新增局部验证

guard-protocol已本地冻结11类根交换对象、闭合Schema、严格重复键入口、无损路径、旧报告保留与完整RFC8785规范字节；新增独立release_artifact分型并保留四种Git wire。Rust1.85协议27项通过，96个样本（25有效/71无效）、112个LF锁输入及24个RFC数字向量覆盖核心边界；独立schema--check与全锁覆盖/摘要复算通过。EG-K01按本地冻结范围勾选，保持candidate-unpublished，详见guardengine/docs/delivery/EG-K01-evidence.md。

库存v2共113项稳定任务，9项verified、104项pending：EG-M01清单建设、EG-K01–K06六项局部任务、EG-S01来源适配及EG-A01字节码闭环。任务完成仅按原生任务字面与独立证据，不关闭CON/EVI整条需求。逐项证据位于guardengine/docs/delivery/的对应EG-K01至K06-evidence.md。

K02真实Draft2020-12/契约组合23项测试通过，原82个样本区分23有效/40原始或Schema拒绝/19语义拒绝；release补充后当前96个样本及contract23项受影响回归也通过。K03真实Git/SQLite12项通过，作者、独立审查及root最终Rust1.85回归一致；候选普通源码复用批准scope，保护摘要固定，repo/task唯一头与批准迁移CAS已验证。K05独立Rust1.85共27 reported测试通过，包含3个辅助进程入口；并发、busy、中断、链接替换、损坏持久隔离和retention均有真实反例。

K04真实Git全套9项通过；Windows别名与真实硬链接两个新例已独立focused复验通过，见guardengine/docs/delivery/EG-K04-evidence.md。K06新增runner18 reported、ledger10 reported、subject新增物化7及原9项均通过；runtime初轮12项为11通过/1真实Git边界失败，修复后受影响4项v2全部通过，root再独立4/4通过（164.50秒），当前源码unit4项及严格Clippy通过，详见guardengine/docs/delivery/EG-K06-evidence.md。辅助进程入口和分阶段结果不冒称独立场景或一次全套成功。其余集成任务仍未勾选，协议保持未发布，不代表P0集成可交付。库存22项回归实际21通过、1项Windows符号链接权限跳过；严格模式与audit均PASS退出0。公共内核新增CI已实际运行：f55501b为13/15成功，Windows账本准备busy及runtime路径拼法断言失败；测试修复已推送0981705，新运行38963571148仍待终态。CodeGuard新投影与同次Maven采集的真实Ubuntu验证仍待当前候选CI，不以Windows harness替代。

## 继续实施顺序

清单建设及K01–K06局部任务已验收；下一步完成K07真实CodeGuard双读和Java/Maven证据投影，以及ArchGuard真实ArchUnit适配，再推进策略、授权与执行。各域接入后，以真实 Java/Maven/JUnit/ArchUnit、受保护 CI 和精确目标操作完成闭环，保留 Cargo 回归。平台与跨仓发布验收按原路线继续，不能以本阶段结束替代全路线完成。

所有任务仅在实现、必要反例、受影响回归和要求的真实环境证据齐备后勾选。整体目标继续进行，当前不执行总体 sync/archive。

最新SpecGuard验收：main d4463477b2b665ab7909ff7a67b54acd78810971（CLI/插件0.3.0）实际GitHub CI38972402820的Ubuntu/Windows两job全部成功；新三类规格来源只读适配及稳定ID取得独立49项回归与真实语料缺口证据，EG-S01按原任务验收。总体路线现8/113 verified、105 pending，原生总体账本8/62；详细证据见guardengine/docs/delivery/EG-S01-evidence.md。其余专业实现和生产能力未据此扩大完成范围。

ArchGuard 0.6.0实现86f0c2a、工作流修复2a237fc已真实核验：CI38987740407三job全成功（Ubuntu/Windows Java19测试+14依赖锁+两个Rust E2E），安装CI38987740448双平台成功；root在最终工具pin上独立两个E2E通过325.13秒。EG-A01据原任务最小批准规则范围验收。当前总体9/113 verified、104 pending，总体原生账本9/62；EG-K07/K08及其他完整任务继续未勾选，详见guardengine/docs/delivery/EG-A01-evidence.md。
