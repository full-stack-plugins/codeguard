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
