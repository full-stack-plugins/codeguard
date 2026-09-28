# 宿主事件只读 CLI 入口局部验收（2026-09-29）

`codeguard hook plan [--format=json]` 从 stdin 读取最多 64 KiB 的 `hook_trigger_request` 1.0.0。Rust CLI 递归拒绝重复 JSON 字段、未知字段/枚举/版本和非法相对路径，交给 core 纯路由；现输出 `hook_trigger_plan` 1.1.0，固定 `execution=not_run`、`delivery_decision=not_evaluated`、`may_claim_delivery=false`，退出码 3。请求/当前响应 schema 分别为 `schemas/hook-trigger-request.schema.json` 与 `schemas/hook-trigger-plan-v1.1.schema.json`；1.0 旧响应 schema 原件保留。

目标测试先因 CLI 命令缺失而失败。实现后，确认成功的编辑去重路径并选择 `fast_file_check`；失败写入为 `no_check`，未知写入为 `resolve_changed_scope`；提交/推送忽略宿主猜测路径，要求重新取得 Git 快照。超长输入、未知 major、重复字段和附加字段被拒绝，且无候选计划输出。

2026-09-29 增量：超预算反例先使 core 测试失败，响应原因字段反例先因类型/字段缺失而编译失败。现在确认编辑仅在不超过 8 个不同路径、单路径 512 字节、总路径 2 KiB 时列出逐文件快检；重复路径只计一次。超预算时清空目标、返回 `resolve_changed_scope` 和 `fast_scope_budget_exceeded`，与缺路径、写入未知分别用 `changed_paths_missing`、`write_outcome_unknown` 区分；之后应选择有界批量检查而不是把同一超预算列表重复输入。后续非法路径仍拒绝，失败写入不启动源码检查。core 8 项及 CLI 6 项目标测试通过，实际批量编辑响应通过当前 1.1 schema；本逻辑仍未执行任何检查器或完成宿主接线。

独立使用本机已有 `jsonschema 4.25.1` 的 Draft 2020-12 校验器，实际调用当前二进制生成单文件、批量超限、缺路径、写入未知、写入失败及提交六类响应，均匹配 1.1 schema，均不匹配旧 1.0 schema；把 `execution` 改成 `completed` 或 `delivery_decision` 改成 `allow` 均被拒。Python 仅作独立协议验收，不属于 Rust 产品运行路径。目标 Rust 文件 rustfmt、CLI/core Clippy `-D warnings`、OpenSpec 严格校验、分层检查和 diff 空白检查通过。批量检查器执行、实际时间/并发预算和三宿主事件验收仍缺。

调用示例：

```bash
printf '%s' '{"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":["src/A.java"],"task_id":null,"write_outcome":"confirmed","host_claims_blocking":false}}' | codeguard hook plan --format=json
```

本入口只规划档位；没有运行 `detect`/`lint`/`task verify`/`gate`，不读取真实源码或 Git 内容，不验证宿主阻断能力。宿主仍需将事件可信地映射到实际命令与统一报告；不能因本命令输出 `commit_gate` 而声称提交已受保护。OpenSpec 11.17 保持未完成。

目标与回归验证：

```text
cargo test -p codeguard-cli --test hook_plan_cli
cargo clippy -p codeguard-cli -p codeguard-core --all-targets -- -D warnings
cargo test --workspace --features codeguard-runtime/wasm-precheck --locked --quiet
openspec validate introduce-rust-codeguard-cli --strict
```

实际结果：CLI 目标 5 项和 core 路由 7 项测试通过；CLI/core Clippy `-D warnings` 通过；启用 WASM 特性的完整 workspace 回归退出码 0，部分需要外部原生工具的测试按既有条件 ignored；OpenSpec 严格校验与 `git diff --check` 通过。这些测试没有提供三宿主真实 Hook 运行证据，也没有测试缓存身份与节流。
