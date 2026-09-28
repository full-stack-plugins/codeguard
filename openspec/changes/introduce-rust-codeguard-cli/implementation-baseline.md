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
