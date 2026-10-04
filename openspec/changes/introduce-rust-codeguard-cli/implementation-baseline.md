# 当前实现切片与未完成边界

2026-09-28 从当前源码及定向回归补录。任务勾选只在 [tasks.md](tasks.md)；本表不另建任务状态。已完成的是下列有界契约，不能推导整项语言支持、正式交付或真实宿主验收通过。

本轮执行 28 个 CLI 集成测试目标：266 通过、0 失败、46 忽略。多数原生集成使用受控工具输出；忽略的真实环境测试未重新执行。历史原生实测另见 [verification](verification.md)，不计作本轮实测。

| 子任务 | 已实现范围 / 规格 | 测试与源码入口 | 仍未完成 |
|---|---|---|---|
| 2.1.1 | 只读 plan 预览与非法选择拒绝；[unified-cli-contract](specs/unified-cli-contract/spec.md) | [plan_preview_cli](../../../crates/codeguard-cli/tests/plan_preview_cli.rs) | 完整政策计划、别名和正式 DAG |
| 2.2.1 | RunReport 版本、枚举和结构消费契约；[unified-cli-contract](specs/unified-cli-contract/spec.md) | [run_report_contract](../../../crates/codeguard-cli/tests/run_report_contract.rs) | 报告可信来源与正式门禁 |
| 4.7.1 | 配置静态查询与非法输入诊断；[unified-cli-contract](specs/unified-cli-contract/spec.md) | [config_inspection_contract](../../../crates/codeguard-cli/tests/config_inspection_contract.rs) | 配置修改全流程 |
| 4.7.2 | 规则目录只读列表；[rulepack-governance](specs/rulepack-governance/spec.md) | [rules_list_cli](../../../crates/codeguard-cli/tests/rules_list_cli.rs) | 完整规则来源、升级和可信政策 |
| 4.9.1 | 白名单只读命令和有界纠错提案；[rulepack-governance](specs/rulepack-governance/spec.md) | [whitelist_command_contract](../../../crates/codeguard-cli/tests/whitelist_command_contract.rs), [whitelist_propose_contract](../../../crates/codeguard-cli/tests/whitelist_propose_contract.rs) | 审批权威、全量适配器应用和生命周期 |
| 5.7.1 | 项目静态发现公开入口；[project-initialization](specs/project-initialization/spec.md) | [detect_cli](../../../crates/codeguard-cli/tests/detect_cli.rs) | 完整架构识别和所有构建器解析 |
| 5.8.1 | 能力选择及不支持组合反馈；[native-tool-adapters](specs/native-tool-adapters/spec.md) | [capability_selection](../../../crates/codeguard-cli/tests/capability_selection.rs) | 全部语言真实能力 |
| 5.2.1 | doctor 观察导入工作台；[remediation-workflow](specs/remediation-workflow/spec.md) | [doctor_work_sync](../../../crates/codeguard-cli/tests/doctor_work_sync.rs) | 通用安装、升级和恢复闭环 |
| 6.2.1 | Java P3C 公开检查适配契约；[native-tool-adapters](specs/native-tool-adapters/spec.md) | [java_p3c_cli](../../../crates/codeguard-cli/tests/java_p3c_cli.rs) | 完整 Java 模型、所有项目组合和真实工具矩阵 |
| 6.3.1 | Java Javadoc 公开检查适配契约；[native-tool-adapters](specs/native-tool-adapters/spec.md) | [java_javadoc_cli](../../../crates/codeguard-cli/tests/java_javadoc_cli.rs) | 所有项目模型和原生抑制组合 |
| 6.3.2 | Checkstyle 检查与工作台衔接契约；[native-tool-adapters](specs/native-tool-adapters/spec.md) | [java_checkstyle_workbench](../../../crates/codeguard-cli/tests/java_checkstyle_workbench.rs) | 全部配置组合与完整 Java 闭环 |
| 7.1.1 | Rust 原生聚合、Rustdoc 与构建公开入口契约；[native-tool-adapters](specs/native-tool-adapters/spec.md) | [check_all_rust_native](../../../crates/codeguard-cli/tests/check_all_rust_native.rs), [rust_comments_cli](../../../crates/codeguard-cli/tests/rust_comments_cli.rs), [rust_build_cli](../../../crates/codeguard-cli/tests/rust_build_cli.rs) | 全部 Cargo 配置组合及完整交付认证 |
| 7.1.2 | cargo-audit 公开漏洞检查契约；[native-tool-adapters](specs/native-tool-adapters/spec.md) | [cargo_audit_cli](../../../crates/codeguard-cli/tests/cargo_audit_cli.rs) | 完整数据库新鲜度和发布证明 |
| 7.2.1 | Ruff 与 Python CVE 公开适配契约；[native-tool-adapters](specs/native-tool-adapters/spec.md) | [lint_python_cli](../../../crates/codeguard-cli/tests/lint_python_cli.rs), [python_cve_cli](../../../crates/codeguard-cli/tests/python_cve_cli.rs) | Python 全类别和环境矩阵 |
| 7.3.1 | ESLint 目录检查与 npm audit 局部报告契约；[native-tool-adapters](specs/native-tool-adapters/spec.md) | [eslint_directory_cli](../../../crates/codeguard-cli/tests/eslint_directory_cli.rs), [npm_audit_cli](../../../crates/codeguard-cli/tests/npm_audit_cli.rs) | Node 全类别、完整语义与数据库证明 |
| 8.2.1 | Go 检查结果导入持久任务；[remediation-workflow](specs/remediation-workflow/spec.md) | [go_work_sync](../../../crates/codeguard-cli/tests/go_work_sync.rs) | Go 全类别原生验收 |
| 9.1.1 | init 命令及受管工作区入口契约；[project-initialization](specs/project-initialization/spec.md) | [init_command_contract](../../../crates/codeguard-cli/tests/init_command_contract.rs) | 完整画像、事务恢复和全部架构模型 |
| 9.4.1 | 跨类别工作同步与坏报告隔离；[remediation-workflow](specs/remediation-workflow/spec.md) | [work_sync_cross_category](../../../crates/codeguard-cli/tests/work_sync_cross_category.rs) | 通用跨工具去重和所有故障恢复 |
| 2.3.1 | check all 局部结果和未完成契约；[unified-cli-contract](specs/unified-cli-contract/spec.md) | [check_all_partial_contract](../../../crates/codeguard-cli/tests/check_all_partial_contract.rs) | 所有入口的完整聚合门禁 |
| 9.8.1 | 任务租约公开契约；[remediation-workflow](specs/remediation-workflow/spec.md) | [task_lease_contract](../../../crates/codeguard-cli/tests/task_lease_contract.rs) | 所有跨进程故障与受控 fix 接线 |
| 9.10.1 | task verify 的已支持复检契约；[remediation-workflow](specs/remediation-workflow/spec.md) | [task_verify_contract](../../../crates/codeguard-cli/tests/task_verify_contract.rs) | 正式可信关闭重开和全部语言复检 |
| 9.7.1 | next 任务选择与输入验证契约；[remediation-workflow](specs/remediation-workflow/spec.md) | [next_command_contract](../../../crates/codeguard-cli/tests/next_command_contract.rs) | 所有宿主自动反馈和工作流 |
| 2.7.1 | CLI 版本身份公开输出；[unified-cli-contract](specs/unified-cli-contract/spec.md) | [version_cli](../../../crates/codeguard-cli/tests/version_cli.rs) | 不可变源码及跨平台制品绑定 |
| 13.4.1 | macOS arm64 npm 发布与新缓存 npx | [实际验收](../../../tests/acceptance/npm-public-candidate.md)、[启动器](../../../npm/codeguard.cjs)、[打包器](../../../scripts/pack-npm-local.mjs) | 其他平台、签名、不可变源码绑定、完整 S13 |

实现入口：[CLI](../../../crates/codeguard-cli/src/main.rs)、[适配器](../../../crates/codeguard-adapters/src)、[领域契约](../../../crates/codeguard-core/src)、[执行器](../../../crates/codeguard-runtime/src)。测试文件包含具体场景与断言，可追到对应入口；这里不将受控 fixture 当作全部原生工具实测。

## 复现命令

```bash
cargo test --offline -p codeguard-cli \
  --test plan_preview_cli --test run_report_contract --test config_inspection_contract --test rules_list_cli --test whitelist_command_contract --test whitelist_propose_contract --test detect_cli --test capability_selection --test doctor_work_sync --test java_p3c_cli --test java_javadoc_cli --test java_checkstyle_workbench --test check_all_rust_native --test rust_comments_cli --test rust_build_cli --test cargo_audit_cli --test lint_python_cli --test python_cve_cli --test eslint_directory_cli --test npm_audit_cli --test go_work_sync --test init_command_contract --test work_sync_cross_category --test check_all_partial_contract --test task_lease_contract --test task_verify_contract --test next_command_contract --test version_cli
```

npm 发布证据为此前已执行记录，本轮没有重新发布或运行远程安装。

## 当前聚合 ESLint 接线

`check all` 的模块本地 ESLint 原生阶段、相同源码免重复 WASM、串行同步稳定任务和聚合 next 已接通。对应 7.3、14.6、14.10，证据集中维护在 [check-all-eslint](../../../tests/acceptance/check-all-eslint.md)。受控工具测试证明编排与任务契约；本次未执行真实 ESLint 或宿主安装验收。父任务还包含其它包管理器、语言及完整闭环，不据此整体勾选。


## 2026-10-04 原生语法确认任务接线

源码已支持记录在已有工作台中的 Zig 确认任务原生复检，复用 task verify 的租约、attempt 和追加事件。诊断指导当前源码修复，工具/版本/输入失败保留未完成；next 返回原生报告引用、当前位置和复检 argv，repair_ready 可接入。原生 AST 零诊断仍待策略/覆盖，不能关闭问题。通用简报 0.3、复检外层 0.12、局部 syntax_task_recheck 0.1；旧 schema 原件留存。内置 grammar 校验只复用进程内不可变数据，外部字节仍重新核验。详细测试与范围见[验收](../../../tests/acceptance/syntax-native-task-verification.md)。父任务、其它语言 adapter 和宿主/正式关闭仍未完成。

## 2026-10-04 限定任务解决与复发

源码 SDK `verify_zig_task_resolution` 可在宿主独立固定的信任上下文下验签限定 Zig 语法策略，重放原始反例和当前输入、追加解决或待核验事件；普通 task verify 已可追加同一任务的原生复发。源事实不改写，本地查询不签发关闭或交付授权，所有收据仍为 not_evaluated。实现、反例、协议和未接入范围见 [任务生命周期验收](../../../tests/acceptance/task-resolution-lifecycle.md)。当前尚无公开 CLI/默认插件的可信策略提供者；不改变完整工作流、全语言精度和门禁的未完成状态。


## 2026-10-04 0.1.4 公开候选发行完成（局部验收）

来源 `1cd458f6e01a44a74388243e964e3f45290ac18e` 的 Linux CI、4 项包测试、注册表/本地摘要、新缓存 npx、真实公开包 Zig 修复反馈及 GitHub prerelease/tag/asset 身份已核对；见 [公开发行验收](../../../tests/acceptance/npm-0.1.4-candidate.md)。此前“尚未发布”的本批记录是发行前快照。插件 lock/默认 Hook、真实宿主、多平台、完整精度和门禁仍缺，13.4/14.18/11.17 不勾选。库存输出 schema 缺口保留，不能称全部公开报告 schema 已验收。


### 2026-10-04 P3C 混合诊断与执行失败修复

RW21 与 verdict-integrity 的双维结果要求现覆盖原生有效诊断后异常退出：稳定 finding 与执行阻塞同时同步，完整计数不增加；正向复检、空失败、坏范围/身份、历史 0.2 兼容及 human 反馈有目标回归。证据：[部分执行验收](../../../tests/acceptance/java-p3c-partial-execution.md)。完整语言/规则/策略/宿主验收仍未完成，不更新父任务完成状态。


## 2026-10-04 Erlang 限定原生关闭与复发（已验证切片）

对应 RW21、9.7、9.10、9.11、12.7、14.10、14.11。SDK 复用既有 Zig 服务，新增 OTP 28 固定规则/版本的签名策略与证据；WASM 首次和原生首次均可按原样本诊断、当前改变且完整零诊断关闭，普通同工具 task verify 检出复发可追加同一父链重开。原生首次 grammar=null，禁止伪造资产或替换原始工具；重算跨语言历史在原生执行前拒绝。宏、空 forms、截断、失败、原样本合法和输入变化仍保留待核验/误报调查。旧 Zig API 构造及 schema 保持兼容。

三项 RED（入口缺失、null 策略拒绝、跨语言历史未提前绑定）后实现并验证。相关五组 47 passed/0 failed/5 ignored，显式真实 OTP 28 和 Zig 0.16.0 各1 passed；真实工具与签名夹具权威分开。200 schema 元定义、67 实际报告及12矛盾反例通过；最终全工作区/Clippy结果另按实际日志补录。[验收与实际报告](../../../tests/acceptance/erlang-task-resolution-lifecycle.md)。

默认宿主的受保护策略提供者、全检查器关闭、完整项目 lint/门禁、真实宿主、多平台及独立精度验收仍缺，父任务不勾选；公开 npm 0.1.4 不含本批，不更改 grammar 资格或 Erlang 漏检 RED 草稿。


本批最终默认 workspace/all-targets 214 组、1193 passed/0 failed/109 ignored，完整目标 WASM Clippy -D warnings 退出0；真实 OTP/Zig 各1 passed，相关特性47 passed/5 ignored。完整命令、日志摘要、原生首次 grammar=null 的实际收据和边界见本批生命周期验收。该证据未完成默认宿主可信策略接线、全语言精度或发布，父任务继续开放。


## 2026-10-04 原生配置观察与修复指引

config validate/explain 0.3 已接通有界原生静态观察，保留来源摘要、状态、原因和下一步；生效规则、suppression 及受保护批准仍未解析。重复旧 JSON 键拒绝、有界读取、截断/范围阻塞不完整已验证。首轮 6 项 RED 后发现正例缺错误处理声明，修正夹具并保持未知反例；终端来源动作另先 RED 后补。目标 5 组 58 passed，非全生态验收。

默认 workspace 回归已终态退出 0：214 组、1199 passed/0 failed/109 ignored；109 条件项未执行，不算原生或平台验收。全工作区 all-targets、wasm-precheck Clippy -D warnings 通过；fmt、分层、OpenSpec strict、665 条本轮修改文档本地链接通过。201 份 schema 元定义、实际 0.3 原始输出、六组计数一致性及 12 类伪造/矛盾反例通过；旧 0.2 schema 与 HEAD 字节一致，历史 0.2 结构仍可验证。实际默认二进制的 human 来源/动作和 explain/范围阻塞 validate JSON 也已核验，前后项目文件字节一致。

没有更换 grammar、安装工具或改插件锁；原有 Erlang 37 例 RED 草稿保持未提交。当前扩展仍非完整规则/suppression/批准来源解析，父任务及总体目标继续未完成。远端 CI 按新提交另行核验，不使用旧提交证明当前批次。

详细事实及日志身份见[配置验收](../../../tests/acceptance/config-native-observation.md)。4.1/4.2/4.7/5.10 保持开放，同一 OpenSpec change 延续；当前新增场景覆盖配置声明、不执行动态配置、输出预算及重复旧键。

## 2026-10-04 Cargo 输入稳定性与代理调用

Clippy 现在在原生执行前冻结已观察源码及根配置/锁的字节或不存在状态，诊断绑定原字节，输入或工具改变撤回本轮发现。普通及抑制对照采用锁定离线命令，缺锁同步为具体环境准备任务；稳定部分诊断仍保留，取消保持优先级。Rustdoc/build 核验解析后字节但保留 Cargo 代理入口名执行，改指同字节目标也不认定完成；私有 Clippy 目录以独立序号避免同时间戳碰撞。

先复现输入、锁、指引、代理与取消反例，再通过目标及真实 Cargo 测试。三份实际公开反馈、任务详情与真实 Clippy 过期源码观察已归档；正式结果及历史失败见 [Cargo 输入验收](../../../tests/acceptance/rust-clippy-input-stability.md)。本批不改变 grammar、插件锁或公开包，不提供完整 Cargo 模型/沙箱/可信关闭，父任务保持开放。

本批最终默认workspace/all-targets回归216组、1213 passed/0 failed/110 ignored，完整目标WASM Clippy -D warnings通过；显式已安装Cargo三项原生验证通过。201 schema元定义、3实际公开反馈/12伪造反例、格式/分层/OpenSpec strict/双语架构命名均通过；日志、历史失败和完整范围见本批验收。没有运行已知Erlang RED的全套WASM测试，不更新grammar资格或公开包。远端CI按新提交另核验，完整目标与父任务保持未完成。


## 2026-10-04 Rust 原生目标免重复解析与重复诊断归并

7.1 / 9.3 / 14.6 / 14.10：聚合 Clippy 以本轮非缓存 artifact 的原根清单和启动前源码摘要绑定目标入口，WASM 只对当前字节相同的入口免重复；覆盖不从本地报告恢复，未证明模块/条件排除源码继续检查。同规则同文件同定位的库/测试诊断只同步一项 finding，error 优先；不同位置与执行阻塞保留，重复扫描更新原任务。四项目标反例先 RED 后实现，真实公开 Cargo 检查/重复任务/缺工具/部分失败已存档。完整覆盖、历史重复任务治理和宿主/发布仍缺，父任务保持开放；最终测试/日志见 [Rust 原生优先验收](../../../tests/acceptance/check-all-native-preferred-rust.md)。

本批最终默认workspace/all-targets 217组、1213 passed/0 failed/110 ignored；受影响WASM library+8集成目标9组、105 passed/0 failed/7 ignored，显式真实Cargo1 passed。默认/特性全目标Clippy -D warnings、格式/分层/OpenSpec strict/双语架构命名、201 schema与4实际反馈/任务详情及10伪造变体通过；旧schema不改。完整命令范围、实际终端/任务/重复扫描、保留的失败与日志摘要见本批验收。完整grammar、真实宿主和发布仍未完成。


## 2026-10-04 Cargo 自动发现与禁止隐式安装

7.1 / 9.3 / 14.5 / 14.6 / 14.10：当前Clippy/rustdoc/check及原任务复检从调用方绝对PATH选择首个Cargo，显式坏工具和首选执行失败不换工具；保留最终代理入口名、RUSTUP_TOOLCHAIN并强制RUSTUP_AUTO_INSTALL=0。原实现七项受控测试2通过/5失败，修复后七项通过；本机实际已安装及缺失自定义工具链的两项另显式通过。真实PATH三检查、重复扫描同一finding/新增0、still_present复检、空PATH及未安装工具链报告已归档。完整工具链/配置/项目范围、其它语言自动选择、可信关闭与实际宿主仍缺，父任务保持开放，见 [Cargo自动发现验收](../../../tests/acceptance/cargo-native-discovery.md)。

本批默认workspace/all-targets 218组、1220 passed/0 failed/112 ignored；受影响WASM CLI library+九集成目标10组、143 passed/0 failed/24 ignored，显式真实Cargo目标9 passed/0 failed/0 ignored（与受控/普通测试有重叠，不相加）。默认及特性全目标Clippy -D warnings、201 schema元定义与六实际报告/18伪造反例、格式/分层/OpenSpec strict/双语架构命名通过；历史schema不改。没有完整WASM suite或358例重跑，不更改grammar、插件锁及公开包；原有Erlang RED草稿保持未提交。完整目标继续未完成。


## 2026-10-04 受检根本地 Ruff 与环境修复指引

7.1 / 9.3 / 14.5 / 14.6 / 14.10：已有PATH发现保留，已配置受检根新增显式入口 > 普通`.venv/bin/ruff` > 绝对PATH的选择。显式坏工具或所选版本/执行失败不换工具；目录链接/损坏/不可执行本地入口返回ruff_local_tool_invalid，不生成源码违规。重复阻塞更新同一任务，简报及任务文档指明本地路径、环境修复范围、原工具复检、历史及关闭条件。没有配置不启动工具，指定编辑Hook只检查目标文件。首批入口3通过/6失败、任务指引1失败先RED，再通过14个受控及1个真实Ruff 0.16.8用例；实际零诊断仍是待核验，不自动关闭。逐模块环境管理器、可信关闭、完整覆盖/精度与实际宿主仍缺，父任务保持开放，见 [本地Ruff验收](../../../tests/acceptance/ruff-local-discovery.md)。

最终默认workspace/all-targets 219组、1234 passed/0 failed/113 ignored；受影响WASM library+九集成目标10组、149 passed/0 failed/30 ignored，显式专用原生目标15 passed/0 failed/0 ignored（与受控/普通测试有重叠，不相加）。默认/特性全目标Clippy -D warnings、格式/分层/OpenSpec strict/架构命名及201 schema元定义通过，12份实际完整报告/36伪造反例通过；旧schema不改。实际原生发现、重复任务、Hook、修复待核验、再次出现、环境阻塞及复检原始输出已归档。没有完整WASM suite、独立精度或358例重跑；grammar、公开包、插件锁不变，预先Erlang RED草稿保持未提交。完整目标继续未完成。


## 2026-10-04 无定位语法观察与聚合工作台接线

同一change的9.3/9.9/14.7/14.10/14.11/14.19增量：固定Swift/Kotlin树的无定位错误原本只有incomplete提示，通用同步跳过零恢复，聚合WASM也未接通工作台。现在check all/java与确认写入Hook共用稳定任务；无定位且syntax_recovery_incomplete生成检查能力恢复任务，保留空位置、源码/grammar身份、环境范围及不修改源码约束。完整零恢复只推荐，不关闭历史任务；保存失败保留证据无假ID；缺adapter复检记录incomplete。聚合反馈0.38.0与新本地确认报告0.3.0，旧schema不改；原指纹与旧可定位/Erlang原生首次协议保留。

初始四项红测与独立聚合红测均已复现；整个Hook工作台目标12 passed/0 failed/0 ignored。具体证据与最终回归见[验收](../../../tests/acceptance/unlocated-syntax-recovery-tasks.md)。没有改变grammar字节、已知精度差异、资格、插件锁或公开npm；正式原生adapter、关闭/覆盖/批准与真实宿主仍缺，父任务不勾选。

本批终态：默认 workspace/all-targets 1234 passed/0 failed/113 ignored；受影响 WASM 16 组 210 passed/0 failed/30 ignored；默认及 WASM Clippy -D warnings、fmt、分层和 OpenSpec strict 通过。203 schema 元定义、16 份真实报告与 48 个伪造变体通过；新增开发协议回归 4 passed。实际报告揭露聚合 next 仅支持旧简报的 schema 缺口，新的 0.38 已引用完整既有简报版本而非放宽任意对象，旧 schema 字节不改。固定二进制实际捕获任务和复检；Swift 两例原生 parse 对照不提升资格。完整日志摘要及运行限制见[本批验收](../../../tests/acceptance/unlocated-syntax-recovery-tasks.md)。父任务不勾选，未重跑完整 WASM suite/358 例语料，公开发行和真实宿主未改变。
