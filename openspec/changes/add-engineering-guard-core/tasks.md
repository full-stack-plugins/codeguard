# Partme Guard 跨项目临时实施账本

状态：全部待实施。本轮仅完成规划；下面的 fixture/测试模块为拟新增，不声称已经存在或执行。只在实现、目标 RED→GREEN、受影响回归及所需真实环境证据齐备后勾选。

历史依赖前缀 K 为旧方案编号，现已按任务分别移交 GuardCore/ArchGuard/CodeGuard/GitGuard/FlowGuard，不能按前缀推断所有权；详见 ownership.md。其他前缀：F=codegraph-plugin，H=codeguard-plugin，R=codereview-plugin，G=gitflow-plugin，W=flowguard-plugin。跨仓依赖通过任务 ID 引用，完成状态只在所属仓账本维护。所有实现先复核当前 HEAD 与用户修改；目标路径与唯一 owner 见 [任务所有权](../../../docs/engineering-guard/ownership.md)；所有新项目代码应在目标仓库实现。

## 1. P0 协议与批准契约

- [ ] 1.1 **EG-K01** 冻结协议 v1 和跨仓字段清单；新增 protocol crate、闭合 schemas 及正负黄金 fixture，验证未知版本/重复字段拒绝与旧报告保留。 依赖：EG-M01。规格：EG-CON-001, EG-EVI-004。
- [ ] 1.2 **EG-K02** 实现 JSON Schema 校验和契约编译：组织锁定、来源引用、冲突、验证器可用性；先用缺 OPA/缺规则/冲突配置写 RED，再验证只读无安装/初始化。 依赖：EG-K01。规格：EG-CON-002, EG-CON-005。
- [ ] 1.3 **EG-K03** 实现 TaskBinding、批准快照与冻结义务集合；用候选自改规则、漏模块、运行中契约变化反例验证；批准快照不能由结果倒推。 依赖：EG-K02。规格：EG-CON-001, EG-CON-003, EG-CON-004。
- [ ] 1.4 **EG-K04** 实现 SubjectSnapshot/index/commit/merge_candidate 输入清单与无损路径身份；真实临时 Git 验证 SHA1/SHA256、rename、mode、symlink/submodule 支持或明确拒绝。 依赖：EG-K01。规格：EG-EVI-002。
- [ ] 1.5 **EG-K05** 实现 SQLite/文件证据存储和事务恢复；验证并发写、busy、落盘中断、链接替换、丢失 artifact、脱敏与保留期限，不改旧 .codeguard 状态。 依赖：EG-K01。规格：EG-EVI-005。
- [ ] 1.6 **EG-K06** 实现有界 Tokio runner、取消整棵受管进程树、运行序列和部分结果；验证超时/输出截断/迟到 PASS/崩溃恢复。 依赖：EG-K04, EG-K05。规格：EG-EVI-001, EG-EVI-003, EG-GATE-007。
- [ ] 1.7 **EG-K07** 适配当前 CodeGuard completion/findings/evaluate_delivery；真实报告与旧协议双读对照，local_unverified/not_evaluated 不升级。 依赖：EG-K03, EG-K06。规格：EG-EVI-004, EG-GATE-001。
- [ ] 1.8 **EG-K08** 接入固定版本 OPA argv 子进程和政策包；编写 Rego 正负测试，覆盖 undefined/超时/错误、不可豁免项和不同 mode；通用执行器和已批准规则包差分一致；流程组合策略由 FlowGuard 提供。 依赖：EG-K03, EG-K06。规格：EG-GATE-001, EG-GATE-005。
- [ ] 1.9 **EG-K09** 实现独立 receipt/授权验证端口与可信提供者；伪造 actor、错 audience、撤销、过期和授权范围扩大均拒绝；范围不变不重复询问。 依赖：EG-K03, EG-K08。规格：EG-GATE-002, EG-GATE-003。
- [ ] 1.10 **EG-K10** 实现最小可信 controller/授权消费框架（Git 动作实现归 GitGuard）：候选 runner 无写凭据、grant 精确绑定与幂等对账；先注入越权与重放，再验证合法操作成功。 依赖：EG-K04, EG-K09。规格：EG-GATE-004, EG-GATE-006。
- [ ] 1.11 **EG-K11** 实现 GuardCore Clap CLI 的 doctor/contract/plan/run/evidence/policy；定义副作用和退出映射，保留原 codeguard CLI；验证无内核/策略时不弱化回退。 依赖：EG-K02, EG-K08。规格：EG-CON-005, EG-GATE-007。
- [ ] 1.12 **EG-K12** 建立 Java Maven 多模块/JUnit/ArchUnit 真工具试点及 GitHub 独立 CI；完成正常交付与绕过反例，包含必需 review 缺口处理，不把模型无发现当通过。 依赖：EG-K07, EG-K10, EG-K11, EG-G06, EG-W05, EG-H04, EG-R04, EG-A01, EG-Q01, EG-J03, EG-L03。规格：EG-GATE-001, EG-GATE-006, EG-ARCH-001, EG-ARCH-002。

## 2. P1 系统与领域架构

- [ ] 2.1 **EG-K13** 实现模块/层/服务禁止边与循环规则，结果含准确源位置和范围；合法分层、禁止边、生成代码/反射未知分别验收。 依赖：EG-K03, EG-K07。规格：EG-ARCH-001。
- [ ] 2.2 **EG-K14** 实现契约到状态机、不变量、并发及消费者测试的义务映射；取消任务合法/非法/并发路径实测，类名和描述不替代行为。 依赖：EG-K13。规格：EG-ARCH-002。
- [ ] 2.3 **EG-K15** 增加公共 API 与设计契约差异；以应用服务协调+聚合转换合法例检验误报；补 Gradle 模块档位，未支持模型明确 unknown。 依赖：EG-K13, EG-K14。规格：EG-ARCH-001, EG-ARCH-003。
- [ ] 2.4 **EG-K16** 实现规则变更/精确例外工作流与生效影响；验证不可自批、期限/撤销、批准后重验及原 finding 保留。 依赖：EG-K09, EG-K15。规格：EG-CON-004, EG-GATE-005。

## 3. P2 语言、MCP 与分发

- [ ] 3.1 **EG-K17** 接 Rust cargo metadata 与 TS dependency-cruiser 的架构事实；Clippy/tsc/ESLint 执行归 EG-Q02、测试充分性归 EG-T06，分别验证实际工具和能力边界；不宣称 crate 图覆盖方法调用。 依赖：EG-K12, EG-K15。规格：EG-ARCH-001, EG-EVI-001。
- [ ] 3.2 **EG-K18** 使用 rmcp 实现只读/验证工具集与能力协商；协议流日志隔离、取消、坏输入和重入测试；不暴露 shell/批准签发。 依赖：EG-K01, EG-K11。规格：EG-EVI-004, EG-GATE-007。
- [ ] 3.3 **EG-K19** 形成 macOS arm64、Linux x86_64、Linux aarch64、Windows x86_64 的制品/取消/隔离矩阵；每格独立证据，MSRV及签名摘要核验。 依赖：EG-K12, EG-K18, EG-H07。规格：EG-GATE-006。
- [ ] 3.4 **EG-K20** 完成五插件与 CLI/MCP/CI 的 N/N-1 互操作和升级回滚；冻结 release manifest，各仓不可变版本与市场指向分别核验。 依赖：EG-K18, EG-K19, EG-F03, EG-H08, EG-R07, EG-G07, EG-W08。规格：EG-EVI-004, EG-GATE-006。

## 4. P3 对象方法与并行冲突

- [ ] 4.1 **EG-K21** 实现 CodeGraph 事实适配和统一符号 ID/位置/解析覆盖；过期、截断、动态边未知 fixture 与真实查询对照。 依赖：EG-K13, EG-F05。规格：EG-ARCH-003, EG-ARCH-004。
- [ ] 4.2 **EG-K22** 实现对象/方法精确契约检查和 REVIEW 风险投影；规则 ID 对应可见性/副作用/调用限制，复杂度仅风险，维护独立合法反例。 依赖：EG-K15, EG-K21, EG-R05。规格：EG-ARCH-003。
- [ ] 4.3 **EG-K23** 实现任务 changedSymbols/consumedApis/read-write 集影响分析，模拟不同文件 API 冲突和独立任务；最终候选编译失败仍阻断。 依赖：EG-K21, EG-G08。规格：EG-ARCH-004。
- [ ] 4.4 **EG-K24** 执行独立标注语料与误报/漏报/覆盖评测，按语言/规则公布分母与限制；未知/启发式结果不提升 ENFORCE。 依赖：EG-K22, EG-K23。规格：EG-ARCH-003, EG-ARCH-004。

## 5. P4 演进与完整交付

- [ ] 5.1 **EG-K25** 实现基线/ADR/公共 API/依赖趋势及变更评估；模型版本不同先验证可比性，历史债不自动豁免。 依赖：EG-K16, EG-K24。规格：EG-ARCH-005。
- [ ] 5.2 **EG-K26** 实现平台 PostgreSQL/对象存储端口、租户隔离、保留/撤销/恢复和 tracing/OpenTelemetry；单机仍可独立使用。 依赖：EG-K05, EG-K09, EG-K20。规格：EG-EVI-005, EG-GATE-002。
- [ ] 5.3 **EG-K27** 完成跨仓契约依赖、制品传播与发布阻断；不承诺跨仓 Git 原子事务，定义分阶段发布和恢复策略。 依赖：EG-K20, EG-K25, EG-K26, EG-W10, EG-G10。规格：EG-GATE-004, EG-ARCH-005。
- [ ] 5.4 **EG-K28** 完成全部必需场景、真实工具/宿主/平台/服务端证据、性能与升级回滚记录；各原生任务账本验证后再 sync/archive，不因预算或文档齐全关闭。 依赖：EG-K27, EG-F08, EG-H10, EG-R10, EG-W12, EG-P01, EG-P02, EG-P03, EG-P04, EG-P05, EG-P06, EG-P07。规格：EG-GATE-006。

## 6. 七项目补充与迁移任务

以下沿用当前临时账本，目标 owner 是实现归属；新仓接收后按 EG-M01 转移，不能在两个位置分别勾选。第一阶段为 P0，第二阶段为 P1/P2；P3/P4 保留深化范围。

- [ ] 6.1 **EG-M01** [P0; owner=guardcore] 建立七项目规格/任务/场景移交清单，核实目标仓库和初始化授权；保留稳定 ID、旧位置指针与唯一账本，未接收前不删原记录；用重复 owner 和丢失用例反例校验。 依赖：无。规格：EG-PROD-001, EG-PROD-003。

- [ ] 6.2 **EG-A01** [P0; owner=archguard] 实现独立 ArchGuard 产品入口、ArchUnit 适配和最小批准分层规则，合法/禁止边/不完整输入均真实验证；不将规则写入 GuardCore。 依赖：EG-K01, EG-K03, EG-K06。规格：EG-ARCH-001, EG-PROD-001, EG-PROD-006。

- [ ] 6.3 **EG-A02** [P1; owner=archguard] 固定架构规则包、领域测试义务与 TestGuard 的交接，以及 architecture-plugin 候选设计接口；未经批准 ADR 与模型分数不得成为 ENFORCE。 依赖：EG-A01, EG-K14。规格：EG-ARCH-002, EG-ARCH-003, EG-PROD-004。

- [ ] 6.4 **EG-Q01** [P0; owner=codeguard] 把现有 CodeGuard 检查接入公共协议并验证独立 CLI 保持原行为；增加 crate 依赖与规则所有权检查，公共内核或架构逻辑不得重新堆入本仓。 依赖：EG-K07。规格：EG-PROD-001, EG-PROD-006, EG-EVI-004。

- [ ] 6.5 **EG-Q02** [P2; owner=codeguard] 完成代码规范/安全/复杂度原生工具与平台档位，Clippy/tsc/ESLint 结果归代码域；测试覆盖归 TestGuard，复杂度不能取代 ArchGuard 的职责判断。 依赖：EG-Q01, EG-K17。规格：EG-PROD-001, EG-EVI-001。

- [ ] 6.6 **EG-J01** [P0; owner=gitguard] 建立 GitGuard Rust 产品入口并固定现有 GitFlow provider 版本，保持 GF 规则暂由旧实现单一拥有；公开 Python 兼容依赖和独立运行资格。 依赖：EG-M01, EG-K01, EG-G01。规格：EG-PROD-003, EG-PROD-006。

- [ ] 6.7 **EG-J02** [P0; owner=gitguard] 实现 Git 动作执行端口、候选/base/ref 条件与 grant 消费；测试目标漂移、撤销和断连对账，不把 Git 写操作实现放入 GuardCore。 依赖：EG-J01, EG-K09, EG-G04。规格：EG-PROD-001, EG-GATE-004。

- [ ] 6.8 **EG-J03** [P0; owner=gitguard] 完成 GitGuard/GitFlow 兼容入口、独立 CI 与受保护目标真实验证；维持旧 .gitflow 布局和显式 apply，不依靠 Hook 防绕过。 依赖：EG-J02, EG-G06。规格：EG-PROD-005, EG-GATE-006。

- [ ] 6.9 **EG-J04** [P2; owner=gitguard] 逐条迁移 GF 规则到 GitGuard，以合法/违规/unknown、并发/恢复黄金语料差分；接收完成后旧插件委托产品，旧规则停止双写。 依赖：EG-J03, EG-G07。规格：EG-PROD-003。

- [ ] 6.10 **EG-J05** [P4; owner=gitguard] 验证独立安装、旧命令迁移、跨仓基线/合入队列/版本回滚；发布前冻结 GitGuard 与 gitflow-plugin 兼容矩阵。 依赖：EG-J04, EG-G10。规格：EG-PROD-002, EG-PROD-003。

- [ ] 6.11 **EG-L01** [P0; owner=flowguard] 建立 FlowGuard Rust 产品入口和固定 flowguard_lib provider；十阶段仍由旧实现判定，流程 docs 不迁位置，不宣称已经纯 Rust。 依赖：EG-M01, EG-K01, EG-W01。规格：EG-PROD-003, EG-PROD-006。

- [ ] 6.12 **EG-L02** [P0; owner=flowguard] 实现消费独立领域结果的动作/阶段规则包，GuardCore 仅执行通用策略；技术 evidence 不充分、伪造 accepted 或缺批准均拒绝推进。 依赖：EG-L01, EG-K08, EG-W03。规格：EG-PROD-001, EG-PROD-006, EG-GATE-001。

- [ ] 6.13 **EG-L03** [P0; owner=flowguard] 验证五产品阶段闭环、native-test 临时 provider 和需求批准基线，缺 SpecGuard/TestGuard 的契约必需义务仍阻断，阶段完成不等于七产品齐备。 依赖：EG-L02, EG-W05, EG-K09。规格：EG-PROD-005。

- [ ] 6.14 **EG-L04** [P2; owner=flowguard] 差分迁移十阶段、批准/撤销/失效/恢复与 allowed_actions 到 FlowGuard；覆盖乱序/跨会话，接收后插件委托产品，不保留第二语义核。 依赖：EG-L03, EG-W08, EG-W13。规格：EG-PROD-003。

- [ ] 6.15 **EG-L05** [P4; owner=flowguard] 验证完整六守卫证据推进、平台身份、跨仓恢复及回滚；任何领域未知或旧报告不能被阶段文本覆盖。 依赖：EG-L04, EG-S06, EG-T06, EG-W12。规格：EG-PROD-005, EG-PROD-006。

- [ ] 6.16 **EG-S01** [P1; owner=specguard] 实现 Spec Kit/OpenSpec/Superpowers 只读来源适配和稳定需求 ID；未知/双事实源冲突不自动选择或初始化。 依赖：EG-M01, EG-K01。规格：EG-SPEC-001。

- [ ] 6.17 **EG-S02** [P1; owner=specguard] 实现结构完整性、引用和需求—设计—任务追溯检查；标题齐全的语义遗漏反例必须保持 REVIEW。 依赖：EG-S01, EG-K03。规格：EG-SPEC-002, EG-SPEC-004。

- [ ] 6.18 **EG-S03** [P1; owner=specguard] 实现需求/验收基线 diff、降低标准识别与独立批准绑定；删除失败条目不能自放行。 依赖：EG-S02, EG-K09。规格：EG-SPEC-003。

- [ ] 6.19 **EG-S04** [P1; owner=specguard] 实现验收可观察字段、类型化冲突与语义风险证据投影；自然语言判断不直接 ENFORCE。 依赖：EG-S02。规格：EG-SPEC-004。

- [ ] 6.20 **EG-S05** [P1; owner=specguard] 接 spec-workflow-plugin 候选文档和变更传播；拒绝自动批准/提示注入，保持生成与验证分离。 依赖：EG-S03, EG-S04。规格：EG-SPEC-005, EG-PROD-004。

- [ ] 6.21 **EG-S06** [P1; owner=specguard] 完成真实原生规格语料、基线演进、独立 CLI 与 FlowGuard 的第二阶段联验；缺映射、失效与未知分别留证。 依赖：EG-S05, EG-T05, EG-K12。规格：EG-SPEC-001, EG-SPEC-002, EG-SPEC-003, EG-SPEC-004, EG-SPEC-005, EG-PROD-005。

- [ ] 6.22 **EG-T01** [P1/P2; owner=testguard] 实现 TestGuard 的冻结测试计划及验收/回归映射，接管 native-test 投影协议但保持原历史证据和版本。 依赖：EG-M01, EG-K01, EG-K03。规格：EG-TEST-001。

- [ ] 6.23 **EG-T02** [P1/P2; owner=testguard] 接真实 JUnit/测试报告与测试进程，区分失败/环境故障/skipped/零用例，绑定输入和原始运行序列。 依赖：EG-T01, EG-K06。规格：EG-TEST-002。

- [ ] 6.24 **EG-T03** [P1/P2; owner=testguard] 实现语句/分支/需求/契约覆盖的独立分母与批准阈值；高覆盖缺断言反例必须阻断对应义务。 依赖：EG-T02。规格：EG-TEST-003。

- [ ] 6.25 **EG-T04** [P1/P2; owner=testguard] 实现契约/回归/状态机/并发测试计划与 ArchGuard 义务交接；影响图不缩减必需回归，先失败后最小修复验证。 依赖：EG-T02, EG-A02。规格：EG-TEST-004。

- [ ] 6.26 **EG-T05** [P1/P2; owner=testguard] 实现受保护测试来源、降低断言审查、flaky/retry/quarantine 历史及当前候选核验；复用核心身份机制，不创建自签可信标签。 依赖：EG-T03, EG-T04, EG-K09。规格：EG-TEST-005。

- [ ] 6.27 **EG-T06** [P1/P2; owner=testguard] 完成 Java 真工具与第二阶段闭环，并扩展 Rust/TS 及适用变异测试档位；按环境单独验收，移交临时测试 provider owner，保留未知矩阵。 依赖：EG-T05, EG-K12, EG-K17。规格：EG-TEST-001, EG-TEST-002, EG-TEST-003, EG-TEST-004, EG-TEST-005, EG-PROD-005。

- [ ] 6.28 **EG-P01** [P2/P4; owner=guardcore] 冻结 guardcore CLI/MCP/GitHub Actions/Agent Hook/API 五入口及权限，补 HTTP 身份/幂等/取消与库 API；逐入口真环境、跨入口等价、版本回滚验收，缺格保持未验收。 依赖：EG-K18, EG-K20。规格：EG-PROD-002, EG-PROD-006。

- [ ] 6.29 **EG-P02** [P2/P4; owner=specguard] 冻结 specguard CLI/MCP/GitHub Actions/Agent Hook/API 五入口及权限，补 HTTP 身份/幂等/取消与库 API；逐入口真环境、跨入口等价、版本回滚验收，缺格保持未验收。 依赖：EG-S06。规格：EG-PROD-002, EG-PROD-006。

- [ ] 6.30 **EG-P03** [P2/P4; owner=archguard] 冻结 archguard CLI/MCP/GitHub Actions/Agent Hook/API 五入口及权限，补 HTTP 身份/幂等/取消与库 API；逐入口真环境、跨入口等价、版本回滚验收，缺格保持未验收。 依赖：EG-K24, EG-A02。规格：EG-PROD-002, EG-PROD-006。

- [ ] 6.31 **EG-P04** [P2/P4; owner=codeguard] 冻结 codeguard CLI/MCP/GitHub Actions/Agent Hook/API 五入口及权限，补 HTTP 身份/幂等/取消与库 API；逐入口真环境、跨入口等价、版本回滚验收，缺格保持未验收。 依赖：EG-Q02, EG-H08。规格：EG-PROD-002, EG-PROD-006。

- [ ] 6.32 **EG-P05** [P2/P4; owner=testguard] 冻结 testguard CLI/MCP/GitHub Actions/Agent Hook/API 五入口及权限，补 HTTP 身份/幂等/取消与库 API；逐入口真环境、跨入口等价、版本回滚验收，缺格保持未验收。 依赖：EG-T06。规格：EG-PROD-002, EG-PROD-006。

- [ ] 6.33 **EG-P06** [P2/P4; owner=gitguard] 冻结 gitguard CLI/MCP/GitHub Actions/Agent Hook/API 五入口及权限，补 HTTP 身份/幂等/取消与库 API；逐入口真环境、跨入口等价、版本回滚验收，缺格保持未验收。 依赖：EG-J05。规格：EG-PROD-002, EG-PROD-006。

- [ ] 6.34 **EG-P07** [P2/P4; owner=flowguard] 冻结 flowguard CLI/MCP/GitHub Actions/Agent Hook/API 五入口及权限，补 HTTP 身份/幂等/取消与库 API；逐入口真环境、跨入口等价、版本回滚验收，缺格保持未验收。 依赖：EG-L05。规格：EG-PROD-002, EG-PROD-006。
