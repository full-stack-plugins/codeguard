# 宿主事件局部执行验收（2026-09-29）

初始切片的 `codeguard hook execute PATH --timeout DURATION --format=json [--ruff-tool ABS_PATH]` 从 stdin 读取有界 `hook_trigger_request` 1.0.0，复用 `hook plan` 的严格解析与 core 路由。超时必须显式给出，至多 120s。当时返回 `hook_execution_feedback` 0.1；当前版本见下方增量。固定 `delivery_decision=not_evaluated`、`host_blocking_verified=false`、`soft_result_reused=false`；普通候选观察退出 3，取消 130，内部故障 4，非法输入 2。插件尚未自动调用它，这些 CLI 退出码不能直接照搬为宿主动作。

初始两个真实动作：`session_start` 仅静态发现语言和检查器配置，最多展示 32 条配置，不执行源码检查；确认成功且 core 规划的目标全是 `.py` 的编辑调用既有有界 Ruff 文件快检，内嵌 `python_lint_feedback` 0.13。此局部报告不导入完整工作台。失败写入不检查；未知/缺范围要求重新确定；超过逐文件预算不扫描子集；混合语言不偷偷只扫 Python；当时修复、提交、推送、CI 等未接线动作明确 `not_run`。实际 Git 范围和宿主阻断能力没有被验证。

目标测试先因 `hook execute` 命令不存在而失败。实现后常规事件目标测试 7 项通过，原生 Ruff 0.16.8 单独实跑 1 项通过，确认只返回目标文件 F401。独立 Draft 2020-12 校验器用实际 CLI 输出覆盖启动、成功编辑、失败编辑和提交事件；四类输出通过新增封闭 schema，内嵌 Python 报告通过 0.13 schema，伪造 `delivery_decision=allow` 被拒。无效路径、超 120s 预算在执行前拒绝；Clippy `-D warnings` 和 OpenSpec 严格校验通过。

后续必须接入插件的真实宿主事件与受信二进制、逐语言/批量计划、软反馈等价身份缓存、任务原工具复检、真实 Git/CI 门禁及跨平台验收。OpenSpec 11.17 不勾选。

## 后续增量：本轮暂存 index 安全观察

`hook execute` 增加显式 `--git-tool ABS_PATH`，`pre_commit` 在 Git 观察阶段按传入预算和 15s 上限复用只读 Git index 观察。外层反馈升为 0.2，历史 0.1 schema 单独保留。新 `hook_git_index_summary` 仅公布暂存条目数、index 摘要、路径违规总数及有界路径列表（最多 32 条、单路径 512 字节）、截断标志与对象状态，固定 `source_check=not_run`、`delivery_decision=not_evaluated`。不指定 Git 工具、缺失工具或观察失败仍是不完整，不利用此前文件快检结果。

## Stop 只读指引增量（0.3）

`stop` 现在从既有 `.codeguard` 事实读取下一步摘要；反馈固定 `source_check=not_run`、`authority=local_unverified`、`delivery_decision=not_evaluated`。Hook 入口最多预检 64 个 finding、64 份报告与 8 MiB 报告总字节；超出返回 `not_run/guidance_scope_exceeded`，不静默截断。不读取 Markdown 作为指令，也不执行建议命令。未初始化时建议显式 init；已初始化但无任务时要求完整检查。`prompt_submitted` 因缺可信意图上下文仍未接入；插件宿主 Hook 也尚未调用本入口。

TDD 目标测试先在 `stop` 返回 `action_not_wired` 时失败；实现后 `cargo test -p codeguard-cli --test hook_execute_cli stop_` 四项通过，覆盖未初始化、无任务、超预算及稳定任务选择（篡改任务 Markdown 不成为指令）。完整事件测试 14 项通过、1 项需显式原生 Ruff 而保持 ignored。Draft 2020-12 对启动、Stop、提示三个实际 CLI 输出通过；伪造 `delivery_decision=allow` 被拒。`cargo test --workspace --all-features -- --test-threads=1` 退出 0，`cargo clippy --workspace --all-features --all-targets -- -D warnings` 退出 0，OpenSpec 严格校验通过。原生依赖条件下的 ignored 测试及插件真实宿主接线仍未验收。

目标测试先 RED：旧 CLI 不识别 `--git-tool`。实现后真实 Git 测试确认已暂存的 `.env` 被报告、未暂存文件不计入；相对 `GIT_INDEX_FILE` 的替代 index 单独观察，缺失 Git 工具返回 `not_run`；33 条违规只展示 32 条但保留总数与截断标志。目标 Hook 测试 10 项通过、1 项需显式 Ruff 而跳过；既有 Git index 契约 13 项通过。独立 Draft 2020-12 验证器检查四类实际事件输出，旧 0.1 schema 和伪造 `allow` 均拒绝新版结果。workspace WASM feature 回归第一次在 Python CVE 输出上限测试出现间歇失败；该项单独两次及该文件 11 项串行重跑通过，随后完整 workspace 串行回归通过。完整提交质量义务、插件宿主阻断与 CI 仍未接线，因此 11.17 继续保持未完成。
