# 语法初检候选报告读者验收（2026-09-29）

此阶段增加 `syntax_precheck_candidate` 1.0.0 的 JSON schema 和 Rust 严格读者。它仅供候选资产的局部观察使用，不能成为原生 lint、交付门禁或用户源码违规的证明。生产端、正式 CLI、插件宿主和任务工作台尚未接线。

目标测试先因读者函数不存在而编译失败。读者对递归重复键、未知字段和未知版本直接拒绝；核对本轮源码字节 SHA-256、内置 grammar 固定清单身份、候选发行状态，并重新计算文件聚合。当前 Java/TypeScript grammar 仍为 `candidate_unvalidated`，即使没有恢复节点也只能得到 `incomplete`，伪造 `grammar_qualified=true` 或 `clean` 均被拒绝。空文件集为 `not_run`，枚举不完整仍为 `incomplete`，局部已观测恢复节点的数量保留。报告明示原生检查 `not_run` 与交付 `not_evaluated`。

示例形状见 `crates/codeguard-adapters/tests/syntax_precheck_report_contract.rs`；对应 schema 为 `schemas/syntax-precheck-candidate.schema.json`。读者只验证传入的当前源码映射，不能证明宿主如何安全读取文件；排除项只有计数，没有原因与嵌入区域；没有位置化恢复节点、setup、持久化与任务引用。因此此候选不是 OpenSpec 14.7 的完整协议，不能据此勾选任务。

验收命令与实际结果在本次变更提交前记录。目标命令：

```text
cargo test -p codeguard-adapters --test syntax_precheck_report_contract
cargo clippy -p codeguard-adapters -p codeguard-core --all-targets -- -D warnings
cargo test --workspace --features codeguard-runtime/wasm-precheck --locked --quiet
openspec validate introduce-rust-codeguard-cli --strict
```

实际结果：目标 6 项测试通过；adapters/core Clippy 无警告；启用 WASM 特性的完整 Rust workspace 测试退出码 0，外部原生工具条件测试按既有标记 ignored；OpenSpec 严格校验通过；`git diff --check` 通过。全仓 `cargo fmt --check` 仍会报告本次未改动的 `eslint_finding_identity.rs` 既有格式差异，因此仅对本次新增或修改的 Rust 文件执行了 `rustfmt --edition 2024`，未顺带改动无关文件。
