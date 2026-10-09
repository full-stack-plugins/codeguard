# 工程守卫验收与反绕过矩阵

状态：验收设计，以下测试均为待实现/待执行。本轮只校验规划，不宣称产品通过。每个需求的场景与责任任务见 [追溯矩阵](traceability.md)。

## 1. 必须记录的证据

每次验收固定 repository/commit/tree/input manifest、contract/policy、producer/adapter/toolchain、平台/宿主/配置、运行时间、原始命令与退出、结构化结果、预期目标和实际覆盖、artifact digest。测试失败须区分行为失败与环境/授权阻塞；既有报告仅在等价性成立时复用。

层级分开：schema/纯函数 → 本地集成 → 实际原生工具 → CI → 实际宿主 → 服务端权限 → 目标平台/生产。任何前层不替代后层。独立 CI 记录实际 workflow/job/check-run 与候选身份，HTTP 200、退出 0 或文件存在不证明质量通过。

## 2. 跨仓端到端用例

| ID | 场景 | 预期与证据 | 首要责任任务 |
|---|---|---|---|
| EG-AT-001 | Java 合规变更完整交付 | 真实 Maven/JUnit/ArchUnit、必要审查处置、流程/Git 满足，授权精确合入；记录实际目标 OID | EG-K12 |
| EG-AT-002 | 候选修改自己的规则或 OPA 包 | 继续批准/base 策略，不能自放行 | EG-K03, EG-K08, EG-G05 |
| EG-AT-003 | 删除必需测试、降低断言或清单 | 受保护验证定义不被候选替换；降强度需独立审查，普通新增测试仍可提交 | EG-K16 |
| EG-AT-004 | 伪造 user/accepted/trusted 字段 | 来源验证失败，不能取得 grant | EG-K09, EG-W05 |
| EG-AT-005 | CodeReview success 且 limited | 报告为 advisory，未满足完整验收条件 | EG-R01, EG-W02 |
| EG-AT-006 | Git allow 但质量 not_evaluated | 只完成 Git 义务，工程交付不放行 | EG-G01, EG-K07 |
| EG-AT-007 | 工具崩溃、超时、解析失败 | incomplete，已发现违规保留，不借 exit 0/空输出通过 | EG-K06, EG-K08 |
| EG-AT-008 | 缺必需工具或规则验证器 | 准备阻塞，无自动安装、无删义务绕过 | EG-K02 |
| EG-AT-009 | 漏模块、空扫描、目标集合重复 | coverage mismatch；空集合需适用性证明 | EG-K03 |
| EG-AT-010 | 旧源码/契约/配置/工具链证据 | stale，受影响义务重新验证 | EG-K04, EG-K06 |
| EG-AT-011 | index 报告用于 HEAD/合入候选 | 明确 identity mismatch，不重新贴标签 | EG-R03, EG-K04 |
| EG-AT-012 | 迟到 PASS 覆盖新失败/取消 | 按受信序列拒绝恢复通过 | EG-K06, EG-H03 |
| EG-AT-013 | 检查后目标 ref 移动 | 原子条件拒绝旧 grant，重建并验证新候选 | EG-G03, EG-G04 |
| EG-AT-014 | 重放 grant、错 audience、撤销/过期 | 拒绝；同 operationId 不重复副作用 | EG-K09, EG-K10 |
| EG-AT-015 | push/merge 断连结果未知 | 对账真实 refs 与操作日志；不盲重试 | EG-G04 |
| EG-AT-016 | 绕过本地 Hook 直接推受保护目标 | 真实服务端仍拒绝；必须记录身份、保护规则和 check-run | EG-G06, EG-G07 |
| EG-AT-017 | PR 修改 workflow/验证器脚本 | 控制端用受信发布版本，候选 runner 无写凭据 | EG-G05, EG-K10 |
| EG-AT-018 | 双 Agent、双会话、不同 worktree | 报告与批准不串用；同事件单飞，不同内容不误去重 | EG-H03, EG-W04 |
| EG-AT-019 | 用户已有授权，重复检查 | 不重问相同范围；新内容需要新证据但不凭空扩大权限 | EG-K09, EG-R02 |
| EG-AT-020 | MUTED 与项目必需审查冲突 | 不外发/不重问；流程端解释缺口和允许处置 | EG-R02, EG-W04 |
| EG-AT-021 | 模型端点/内容范围扩大 | 旧外发许可不覆盖新范围 | EG-R02 |
| EG-AT-022 | 报告含恶意指令、路径或日志秘密 | 不执行报告内容；公开摘要脱敏，私有原件可控 | EG-R06, EG-H05 |
| EG-AT-023 | 缺/旧 CodeGraph 索引与截断输出 | unknown/stale，不自动 init/sync；确定性其他验证继续 | EG-F02, EG-F03 |
| EG-AT-024 | 禁止依赖和非法领域转换 | 当前真实工具/不变量测试拒绝；修复后重验通过 | EG-K13, EG-K14 |
| EG-AT-025 | 合法应用协调与聚合各有取消方法 | 不因同名/类长度自动失败；精确契约与职责评审区分 | EG-K15, EG-K22 |
| EG-AT-026 | 不同文件 API 提供方/消费方冲突 | 语义预警；最终候选集成验证拒绝不兼容 | EG-K23, EG-G08 |
| EG-AT-027 | 无冲突独立任务 | 不误阻，记录影响范围和图谱覆盖限制 | EG-K23, EG-R08 |
| EG-AT-028 | 例外生效、撤销和过期 | 原 finding 保留，精确批准范围之外仍拒绝 | EG-K16 |
| EG-AT-029 | SQLite/文件崩溃、并发、符号链接替换 | 隔离坏记录，正常任务可恢复；无虚假完整报告 | EG-K05 |
| EG-AT-030 | CLI/MCP/Hook 相同请求 | 统一内核结果等价，协议和退出映射正确 | EG-K18, EG-H06 |
| EG-AT-031 | N/N-1、未知 major 与回滚 | 已声明组合兼容，未知不放行；回滚不复活授权或撤保护 | EG-K20 |
| EG-AT-032 | 四目标平台、各宿主安装 | 各自运行取消/路径/权限/真实事件，缺格 unverified | EG-K19, EG-H07, EG-W08 |
| EG-AT-033 | ADR/契约/依赖改变与旧阶段批准 | 下游证据/验收失效，历史仍可追溯 | EG-K25, EG-W10 |
| EG-AT-034 | 跨租户/任务读取和证据复用 | 身份隔离拒绝，日志不泄露其他范围 | EG-K26, EG-W11 |
| EG-AT-035 | 跨仓发布中途失败 | 保留逐仓状态与依赖，按恢复方案继续；不声称原子发布 | EG-K27, EG-G09 |
| EG-AT-036 | 只读 doctor/plan，缺工具/缺索引 | 文件/refs/安装状态不变；无隐式网络和初始化 | EG-K02, EG-F02 |

## 3. 语言、宿主和平台矩阵

- P0 Java Maven 多模块为首个完整 fixture，真实 JDK/Maven/JUnit/ArchUnit 版本与输入固定；Gradle 在 P1 单独验收。
- P2 Rust 分 feature/target，TS 分项目构建/模块模式；任一局部成功不关闭旧 CodeGuard 全语言或其他平台任务。
- 宿主：CodeGuard 插件声明的 Codex/ZCode/Kimi/Claude，各宿主至少包含安装加载、真实事件、允许/拒绝、取消、重入、用户批准来源和移除/回滚。其他镜像宿主仅在自己的矩阵通过后声明支持。
- 平台：macOS arm64、Linux x86_64、Linux aarch64、Windows x86_64；Windows 进程树/共享文件/路径语义不能借 Unix 结果替代。
- CI：GitHub 与 GitLab 各自记录保护与合入候选机制；没有相应账户权限时记录阻塞，不用本地 bare remote 证明托管保护。

## 4. 计划测试位置与执行门禁

以下为实施任务拟新增的测试组，不是已存在命令：core 的 `contract_snapshot`、`protocol_conformance`、`evidence_freshness`、`gate_policy`、`grant_replay`、`runner_recovery`、`architecture_boundaries`、`semantic_conflicts`；各插件新增 `engineering_bridge` 集成测试与共同黄金 fixture。测试路径随所属模块落实，不要求为每个协议字段机械建测试。

原有检查继续执行：Rust fmt/clippy/workspace 及受影响 feature/target；CodeGraph unittest/parity；CodeGuard 插件 unittest/run_all/架构/注册表/vendor；CodeReview pytest/validate_local/vendor；FlowGuard unittest/模板生成 parity/vendor；GitFlow unittest/validate_package/vendor_skills。真正实施某阶段后由当前仓指令确定完整命令，不引用旧报告当新通过。

## 5. 性能与预算

初始工程目标而非实测结果：纯本地 doctor/plan 在固定小型 fixture 上 p95 ≤ 2 秒，排除首次工具安装；OPA 单次默认预算 2 秒；外部验证器各自声明时间/输出/内存/并发预算，超预算为 incomplete，不截断后宣布通过。P0 冻结测量机器、冷/热条件、fixture 大小和至少 30 次样本；真实工具耗时独立报告，不把其耗时藏进“系统开销”。预算目标变更需记录原因，不降低 correctness 门禁。

P3 风险评测分别给 TP/FP/FN、样本量、独立标注来源与覆盖，不用一个综合分数代替规则资格。ENFORCE 每条必须有违规命中与合法反例，并披露语法/语言/版本限制。

## 6. 完成门槛

必需场景全部完成且绑定当前源码/输入/产物；未覆盖能力保持明确未验收。不得因通过 schema、建立文件、文档齐全、tasks 勾选或狭窄 fixture 通过而标记整个阶段完成。规格同步与归档仅在本仓实际实现/验证和跨仓必需依赖均满足后执行。
