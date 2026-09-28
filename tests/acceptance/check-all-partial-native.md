# `check all` 局部原生链验收

`codeguard check all [path] [--ruff-tool ABS_PATH] [--cargo-tool ABS_PATH] --format json` 列出六类候选，以 `required_obligations=null` 表示正式义务未确定。Python 源文件接入原生 Ruff 与本地任务同步，Rust 源文件接入局部原生 Cargo Clippy 观察、持久同步与下一步简报。未接入类别保持 `not_integrated`；原生链未完成为 `native_incomplete`，局部完成但权威/覆盖未验证为 `observed_unverified`。`check_feedback` 0.6 固定退出 3、`delivery_decision=incomplete`。

`check all` 将适用的 `python.lint` 与 `rust.lint` 提交同一 DAG。Python 工具缺失不会吞掉 Rust 的有效诊断；两个任务共享截止时间与取消标记，`--jobs` 限制同时调度数。资源名尚未规范化为跨进程构建目录锁。Rust 局部链需要显式绝对 `--cargo-tool`，经 Rust runtime 运行 `cargo clippy --offline --all-targets --message-format=json`，使用私有 target 目录。仅可读普通 `clippy.toml`/`.clippy.toml` 标为配置存在；Cargo.toml 本身不等于 Clippy 已配置。坏 JSON、缺或重复终结、编译错误、异源路径、进程故障及扫描中 manifest 变化均保持未完成，已解析的有效 Clippy 诊断保留。仅观察默认 features 的 all-targets；Cargo `[lints]`、嵌套 workspace、全 features、工具锁、批准规则和构建脚本隔离仍待实现。

已初始化工作区中，当前源码可复核的 Clippy finding 归并为稳定任务；缺 Cargo 或原生执行不完整归并为环境任务。重复扫描更新忽略入库的观察记录，不复制任务；`check all` JSON/human 均反馈下一步。`task verify <ID> --cargo-tool ABS_PATH` 复用原生 Clippy、任务租约和持久复检事件；再次检出为 `still_present`。普通扫描零诊断时，会针对原规则运行同工具、同输入的原生 `--force-warn` 对照：重新检出为 `suppression_requires_review`，仍无发现为 `candidate_absent_unverified_policy`，对照失败或输入变化为 `incomplete`。这些局部观察都不能证明全部特性组合、批准规则或交付通过。环境任务恢复仅为 `environment_restored_unverified_policy`；所有任务保持 open，正式交付未评估。项目可写的 `codeguard/decisions` 即使含 `approved=true`，也不隐藏 Clippy finding 或授予门禁；可信白名单仍需独立批准与本轮精确匹配。

局部执行预算支持 `--timeout <正整数ms|s|m|h>`，默认 30m、最大 24h；无效预算在任何原生进程前返回用法错误 2。`check all` 从参数解析后建立同一个截止时间，传给 Ruff 版本探测和逐文件原生扫描；探测超时保留 `request_deadline_exceeded`，已逾期的扫描不得给出本地完整证明。本轮尚未实现发现、持久同步、简报、清理的硬 I/O 截止时间，也未实现跨命令 jobs/重试统一预算，OpenSpec 2.8 保持未完成。

公开反馈的 `execution_budget` 记录最终 `timeout_ms`、`source=cli|registered_environment|project_default|builtin_default` 和 `enforcement=native_execution_only`；嵌套 Python 对话反馈为 0.10，仍保留独立的本地原始报告 0.8。当前优先级为显式 `--timeout` > 登记的 `CODEGUARD_TIMEOUT` > 可选 `codeguard/runtime.json` 1.0 > 内置 30m；选中的非法值在执行前拒绝。项目文件 1.0 仅承载运行超时，1.1 可附 `jobs`；未知质量排除字段、链接或超限文件不能成为放行依据。对应 JSON 协议见 `schemas/runtime-options.schema.json` 和 `schemas/runtime-options-1.1.schema.json`。`check all` 的 `--jobs`/`CODEGUARD_JOBS`/项目 1.1/内置值按优先级选择 1–64 的并行上限；反馈记录 `jobs_limit`、`jobs_source`、`native_task_count`、`started_native_task_count`。混合 Python/Rust 项目可有两个独立节点；实际并发和跨进程资源互斥尚需专项验收。

普通测试：`cargo test -p codeguard-cli --test check_all_partial_contract --test check_all_rust_native --offline`。真实 Ruff 测试：`CODEGUARD_RUFF_BIN=/Library/Frameworks/Python.framework/Versions/3.13/bin/ruff cargo test -p codeguard-cli --test check_all_partial_contract original_ruff_finding_survives_check_all_partial_result --offline -- --ignored`。真实 Cargo Clippy：`CODEGUARD_CARGO_BIN=/opt/homebrew/opt/rustup/bin/cargo cargo test -p codeguard-cli --test check_all_rust_native real_cargo_clippy_rechecks_a_persistent_task --offline -- --ignored --exact`。样本覆盖 Rust-only 局部观察、混合项目独立结果而非虚构必需义务、Python 缺工具、已初始化 scan→sync→next、人类反馈、稳定任务原工具复检、真实 `#[allow(clippy::needless_return)]` 抑制后原生对照重新检出、坏对照报告不产生修复候选、非法预算在启动前拒绝、原生探测超时不假完成，以及原生 F401 违规在全项目未完成结果中仍可见。

0.6 接线回归额外覆盖真实 SIGINT：`check all` 启动 Ruff 原生探测后接收 Ctrl-C，`execution_tasks.python.lint=cancelled`，保留发现到的 Python 类别、将 lint 候选标为 `native_incomplete`；延迟写入标记确认原生后台子进程未在返回后继续工作。初版错误地退出 3。0.16 协议修正为 `command_status=cancelled`、退出 130；同轮并行完成的 Rust Clippy 原生 finding 仍保留在 `native_results`。真实 Ruff 0.16.8 的 F401 样本仍保留 `native_observed_unverified` 执行状态与 F401 诊断，不获得交付 allow。

未完成：可信质量策略与工具锁、全部语言/六类别原生适配、冻结义务账本、完整来源身份、Rust 跨配置/特性/目标的规则覆盖完整核验、SARIF/MCP/Hook 接线、可信误报白名单、正式 allow/deny 交付门禁。


## 2026-09-27 Cargo Clippy原生补充验收

明确选择本机stable-aarch64-apple-darwin的真实Cargo（Rust工具链1.98.1），执行check_all_rust_native两项ignored用例，实际2项通过（6.70秒）。包含原生局部扫描及已初始化稳定任务复检；原生allow抑制经force-warn对照不能解释成已修复，局部缺失仅为未核验策略的候选，任务不自动关闭。此结果不是MSRV1.85实测、完整features/targets覆盖、可信工具/规则政策、Windows或交付门禁验收，7.1/9.7及全计划保持未完成。

修复依赖边界后的全工作区测试仍沿同一运行继续，最新中间结果105组、581通过/0失败/72忽略；忽略项不算通过，原生补充另列，未提前签发整轮成功。
# 原生执行后的局部范围复核

`check all/java` 在原生节点后再次运行静态项目发现，比对源码和清单路径集合、构建根、清单/锁/规则配置摘要与配置状态。若工具运行期间新增源码或更改规则配置，公开 `unresolved_conditions` 加入 `project_scope_changed_during_check`；已取得的原生诊断不被丢弃。若总截止时间已过，不执行无预算的末次发现，报告 `project_scope_recheck_deadline_exceeded`。自有 `codeguard/state` 记录的写入不算项目范围漂移。集成测试 `check_all_partial_contract` 用真实子进程修改源码集合/规则配置并验证反例，另验证自有记录不会误报。此增量没有冻结所有源码字节，也不建立受保护义务账本或签发交付许可。

同一路径内容复核补充：初次发现后，CLI 对所选源码建立有界 `SourceSnapshot`（最多 4096 文件、单文件 16 MiB、总计 256 MiB）；原生节点结束后复核原路径字节。伪 Ruff 子进程将 `app.py` 从 `import os` 改为等长的 `import io`，即使路径集合不变也报告 `project_source_changed_during_check`；human 输出提示核对并发编辑或检查器副作用。超出单文件限制报告 `project_source_snapshot_unavailable`，仍为 incomplete；仅写自有记录不产生源码变化提示。`ruff.toml` 同时是 TOML 源文件，修改它可同时触发范围和内容变化提示。快照不是原子全局切面，改后又改回、末次复核后变化与未发现目标仍是缺口。

Python 注释原生诊断增量：Ruff 0.16.8 的 [D100 原生规则](https://docs.astral.sh/ruff/rules/undocumented-public-module/) 用于检查公共模块缺少 docstring。项目 `ruff.toml` 显式选择 D100 时，真实 `app.py` 无顶层 docstring 的 `check all` 返回 D100 原生位置和 Python `comments` 候选 `observed_unverified`，修复提示要求补充准确模块说明并用 Ruff 复检。补充 docstring 后再次运行原工具，D100 观察消失；候选不伪称注释类别已全量通过，整体交付仍 incomplete。复现命令：`CODEGUARD_RUFF_BIN=/opt/anaconda3/bin/ruff cargo test -p codeguard-cli --offline --test check_all_partial_contract native_ruff_d100_is_reported_as_python_comment_evidence -- --ignored --exact`。本增量只归类真实 D100 发现；其它 docstring 规则、无诊断时的完整启用/覆盖、可信规则包和白名单批准仍未实现。

D100 工作台与设置一致性验收：已初始化工作区的真实 `check all` 生成同一稳定 D100 任务，`next` 给出原工具与源码范围；`task verify` 在原问题仍存在时返回 `still_present`，补上顶层 docstring 后返回 `candidate_absent_unverified_policy`，记录事件但事实仍 open。另以受控伪 Ruff 返回 D100、同时 `--show-settings` 返回空启用列表，修改前扫描错误地报告本地完整；现保留 D100 原诊断并标 `rule_settings_report_mismatch`。完整原生启用集合仅用于内部交叉核对，不扩大对外规则包或交付结论。真实回放命令：`CODEGUARD_RUFF_BIN=/opt/anaconda3/bin/ruff cargo test -p codeguard-cli --offline --test task_verify_contract d100_check_all_creates_a_stable_task_and_rechecks_with_native_ruff -- --ignored --exact`。

pydocstyle 扩展验收：在项目 `ruff.toml` 精确启用 D101 时，固定 Ruff 0.16.8 对缺少 docstring 的公共类报告真实 D101，`check all` 将其归入 Python `comments` 候选并保持 `observed_unverified`。归类器只接受 D 后恰好三位数字，D1000、DOC201 和其它 lint 规则不借此进入注释类别。受控伪 Ruff 分别声称 D100/D101，而同轮生效设置均未启用，扫描保留诊断并标 `rule_settings_report_mismatch`。回放：`CODEGUARD_RUFF_BIN=/opt/anaconda3/bin/ruff cargo test -p codeguard-cli --offline --test check_all_partial_contract native_ruff_d101_class_docstring_is_python_comment_evidence -- --ignored --exact`；`cargo test -p codeguard-cli --offline --test python_lint_scan_contract docstring_report_without_enabled_native_rule_is_incomplete_not_an_actionable_comment -- --exact`。该扩展只接受实际原生诊断，不证明完整注释覆盖或批准。

D101 修复指引及抑制/自批反例：同一真实 Ruff 诊断的对话反馈给出“公共类缺少文档字符串”和限定源码路径的补文档动作，初始化工作区中的稳定任务文件也明确要求为类补充准确 docstring。向 `codeguard/decisions/` 自写 `approved=true` 后重跑 `check all`，D101 原生 finding、唯一稳定任务和 open 状态均保留，交付仍 incomplete；`rules whitelist propose` 仍为 candidate=null、authority=unverified、gate_effect=none，并要求批准的 rulepack 身份。仅加 `# noqa: D101` 后，原工具对照仍检出问题，`task verify` 记录 `suppression_requires_review` 且事实保持 open；实际补充类 docstring 后记录 `candidate_absent_unverified_policy`，仍不自动关闭。真实回放：`CODEGUARD_RUFF_BIN=/opt/anaconda3/bin/ruff cargo test -p codeguard-cli --offline --test task_verify_contract d101_task_explains_class_docstring_and_requires_native_recheck -- --ignored --exact`。此反例只证明当前未接可信批准时的拒绝路径，不证明独立批准后的正式例外门禁。

未映射与未批准分流：D101 尚无经核对的 Ruff 规则映射，`rules whitelist propose` 因此返回 `rulepack_mapping_unavailable`，同时列出 `reviewed_native_rule_mapping`、`approved_rulepack_identity` 和 `review_native_rule_and_tool_version`；human 输出包含同一动作。已映射 F401 的真实 Ruff 回归仍为原 `evidence_incomplete`，不把缺独立批准误写成规则不存在。两种路径均不生成可批准候选。
