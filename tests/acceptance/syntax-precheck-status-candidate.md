# 语法初检状态聚合候选验收（2026-09-29）

此记录只覆盖 core 的纯状态判定；它尚未由 `lint` 或 `check` 命令调用，不能代表真实源码检查或门禁通过。

目标测试先因 `SyntaxFileObservation`、`SyntaxFileState`、`SyntaxPrecheckStatus` 与 `assess_syntax_precheck` 不存在而编译失败。实现后，聚合只在已完成的非空选定范围内、每个文件都已由适用且验收过版本/方言的 grammar 完整解析、无 ERROR/MISSING 恢复节点、无取消时返回 `clean`。已有恢复节点返回 `suspected_issue`，仍不是原生违规；局部失败、不支持区域、未知版本、恢复节点截断、范围枚举失败或取消使总体 `incomplete`，同时保留已观察疑似数量。全部文件不支持时单独返回 `unsupported`，空范围返回 `not_run`。重复或越界相对路径、空失败原因被拒绝。

目标测试命令：

```text
CARGO_NET_OFFLINE=true cargo test --locked -p codeguard-core --test syntax_precheck_status
CARGO_NET_OFFLINE=true cargo clippy --locked -p codeguard-core --all-targets -- -D warnings
CARGO_NET_OFFLINE=true cargo test --workspace --features codeguard-runtime/wasm-precheck --locked --quiet
openspec validate introduce-rust-codeguard-cli --strict
```

本轮 7 项目标测试通过，完整 Rust workspace 回归以退出码 0 结束；外部原生工具条件未满足的 ignored 测试仍不构成真实工具链验收。OpenSpec 严格校验通过。

本层接收调用方的文件观察与 `scope_complete`、`cancelled`，没有自行枚举、解析或核验 grammar 身份。后续须把运行时的截断和版本资格映射到该契约，建立版本化报告及 JSON schema，并让每个公开消费者拒绝未知 major；本候选不得使 OpenSpec 14.7 勾选。
