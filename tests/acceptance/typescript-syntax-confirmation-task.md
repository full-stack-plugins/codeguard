# TypeScript/TSX WASM 初检的原生确认任务：局部验收（2026-09-29）

对应 OpenSpec 14.9、14.10。可选 `wasm-precheck` 构建中，`codeguard lint typescript <单文件> --workspace <已初始化工作区> --format json` 仅在未观察到项目本地 ESLint 包时运行候选语法初检。CLI 不把恢复节点写成已确认源码 finding，而是复用 ESLint 前置报告、`work sync` 和 `next`，按工作区与源码范围生成一张稳定的原生确认阻塞任务。未提供工作区时沿用 0.3.0（TypeScript）或 0.4.0（TSX）报告，任务 ID 为 null；提供工作区时报告升为[0.5.0](../../schemas/eslint-local-feedback-v0.5.schema.json)，同步成功后才返回真实 `setup.task_id`。

先写端到端反例：已初始化工作区的语法初检返回 `workbench=not_connected`，断言失败。接线后，连续两次对含语法恢复节点的源码初检，只产生一个 `node.eslint.preparation`、`kind=blocker` 的任务，第二次新增阻塞数为零；把源码改成无恢复节点后仍复用同一开放任务，`delivery_decision=not_evaluated`。超过 1 MiB 的源码使初检未运行时，也保留单独的未完成诊断和真实环境任务，不产生源码 finding。`next_action` 明确要求同一输入的适用原生语法复检，不能把 WASM 零恢复、安装成功或任务勾选当作关闭证据。工作台不可用时不填虚构任务 ID。

当前持久报告只保存源码范围、源码 SHA-256、前置诊断和报告摘要；CLI 当轮反馈显示的**逐个疑似位置尚未持久关联**到该任务。任务也没有能力匹配的原生关闭收据，Java 初检和多模块调度未接入这条工作台路径。因此 14.9、14.10、14.11 均不勾选，不能据此宣称完整修复闭环或语法能力已发布。

定向验证：`CARGO_NET_OFFLINE=true cargo test --locked -p codeguard-cli --features wasm-precheck --test typescript_syntax_fallback_candidate --test eslint_lint_cli --test grammar_status_cli`，共 21 项通过、4 项要求真实 ESLint 10 的用例忽略；新增端到端反例覆盖 TypeScript 重扫、源码改动后不自动关闭、TSX 独立任务、初检未运行与 `next` 和 `setup.task_id` 一致。`cargo clippy --locked -p codeguard-cli --features wasm-precheck --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`python3 scripts/check_layering.py`、`openspec validate introduce-rust-codeguard-cli --strict` 与 `git diff --check` 均通过。真实 ESLint 10 运行、完整工作区 CI 及宿主对话验收不由这些局部测试证明。
