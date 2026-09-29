# C、Go、JavaScript、Rust grammar：局部候选验收

CodeGraph 固定来源提交 `1072f82ce24db3d133258d30165cef6b74d108b2` 的四份随仓 WASM 已逐字节复制，并与[来源覆盖库存](../../grammars/codegraph-coverage.json)中的长度和 SHA-256 一致。上游标签分别为 tree-sitter-c v0.24.2、tree-sitter-go v0.23.4、tree-sitter-javascript v0.25.0、tree-sitter-rust v0.24.2；[候选清单](../../grammars/manifest.json)固定完整 Git 提交、MIT 许可证原文摘要、资产摘要和实测 ABI（C/JavaScript/Rust 为 15，Go 为 14）。

先将候选数断言从 7 改为 11，清单与状态测试均因实际只有 7 份而失败。接入后，Rust `WasmStore` 离线加载四份资产；每种语言的基础合法样例无恢复错误，未闭合或损坏样例有解析错误。隔离 worker 对四份合法样例均返回 `incomplete`、`grammar_qualified=false`，零恢复节点没有变成通过。清单测试同时核对原始字节、许可证及不允许宣称已发布状态。

当前源码本地验收：`cargo test --locked --workspace --all-features` 退出 0，182 组、1134 通过、0 失败、106 条有条件忽略；`cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` 与 `cargo fmt --all -- --check` 均退出 0；`openspec validate introduce-rust-codeguard-cli --strict` 通过。`codeguard grammar status --format=json` 实际报告 32 种来源、11 份候选、0 项已发行。忽略项不计入通过，也不证明对应原生工具已执行。

此处只验收资产来源和窄范围可加载性。语言版本、方言（包括 JavaScript 嵌入式 JSX）、真实项目合法/非法语料、原生工具对照、误报率、资源预算、公开 `lint` 路由与发行包安装均未完成。原生 lint 的项目义务仍保留，不能用这些候选观察关闭任务或批准交付。
