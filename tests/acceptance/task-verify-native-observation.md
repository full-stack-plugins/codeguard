# Ruff 任务原工具复检观察切片（2026-09-25）

`codeguard task verify <CG-id> [path] [--ruff-tool ABS_PATH] --format human|json` 现在从受限 finding/blocker 事实确定任务范围，复用公开 `lint python` 的同一 Rust→原生 Ruff 扫描链。它保存本轮 0.4 脱敏报告、调用 `work sync` 保留新发现，再以 append-only `verification_observed` 事件记录对原任务的观察：`still_present`、`still_blocked`、`candidate_absent_unverified_policy`、`environment_restored_unverified_policy` 或 `incomplete`。即使原问题消失，fact 仍 `open`，结果固定 `local_unverified`、`not_evaluated`、退出 3；本切片不签发正式 resolved。

`next` 会从最新的本轮报告、观察事件及工作区身份复核复检候选。原问题已不再出现时，简报改为“核验受保护工具、规则与覆盖后再决定关闭”，不重复推荐修改无关源码；新发现的真实问题仍优先推进。后续原生扫描再次检出阻塞时，观察事件覆盖此前候选并恢复待处理提示。仅修改事件文字而报告仍显示阻塞时，`next` 返回 `verification_event_invalid`，不能把本地声称升级为恢复结论。

`task_verify_contract` 5 项普通测试和固定 Ruff 0.16.8 的 2 项显式真实测试通过。样本覆盖缺配置反复复检、无效任务在启动工具前拒绝、事件篡改、本机忽略的复检报告缺失时不沿用未核验恢复结论、环境恢复并同时发现 F401、源码修复后原 finding 缺失、随后再次出现时重开行动提示。当前仍没有批准的工具/规则身份、策略差异防护、任务租约、attempt 预算、正式 resolved/reopen 事件及跨类别复检；OpenSpec 9.10 保持未完成。

后续原生注释抑制对照与 `suppression_requires_review` 复检观察见 [Ruff 抑制防逃逸](ruff-native-suppression-observation.md)；本文件保留初版验收记录。

后续局部预算验收：`task verify` 现复用检查类 `--timeout <正整数ms|s|m|h>`，默认 30m、最大 24h。无效预算在租约与原生执行前返回 2；CLI 建立一次截止时间并传入原生 Ruff 探测和逐文件扫描。超时保留本轮 `native_scan` 未完成原因、复检 `observation=incomplete`、`event_persisted=false`，不追加复检事件，释放自有租约，稳定任务保持待处理。`task_verify_contract` 普通 9 项通过、8 项需真实 Ruff 而默认忽略。租约/持久 I/O/清理仍无硬截止时间，不能视作 2.8 或 9.10 完成。

公开复检预览 0.3 增加 `execution_budget.timeout_ms/source/enforcement`；正常与前置条件不可用的 JSON 都报告最终预算来源。已登记 `CODEGUARD_TIMEOUT` 在无显式 CLI 值时提供预算，非法环境值在租约前拒绝。`enforcement=native_execution_only`，不把租约或事件 I/O 假称为受硬截止时间控制。

可选 `codeguard/runtime.json` 1.0 在无 CLI/登记环境预算时为复检提供 `project_default`，但不授权关闭任务。定向测试核对复检继续记录不完整观察、任务状态仍为 open。

## Rust Clippy 任务复检扩展

`task verify <CG-id> [path] --cargo-tool ABS_PATH` 通过任务的 `rust.cargo_clippy` 检查器身份选择 Rust 发现范围，复用原生 Clippy、租约和本地复检事件。再次检出同一稳定 finding 为 `still_present`。零诊断时，CLI 对原规则追加原生 `--force-warn` 对照；同一 finding 重现为 `suppression_requires_review`，仍未重现仅为 `candidate_absent_unverified_policy`。对照报告损坏、输入字节或工具/manifest 身份变化为 `incomplete`。环境 blocker 的本地检查恢复只得到 `environment_restored_unverified_policy`。`next` 会核对对应 Rust 报告、事件摘要和本轮观察，复检后源码变化则要求重跑；任务事实始终 open。普通测试见 `check_all_rust_native`，真实 Cargo Clippy 的 `real_cargo_clippy_rechecks_a_persistent_task` 验证 `#[allow(clippy::needless_return)]` 后强制告警仍检出。批准规则覆盖、完整特性/目标组合和正式关闭仍待实现。
