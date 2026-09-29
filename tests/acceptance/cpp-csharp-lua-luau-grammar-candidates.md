# C++、C#、Lua、Luau grammar：局部候选验收

四份 WASM 来自固定 CodeGraph 提交 `1072f82ce24db3d133258d30165cef6b74d108b2`，原始字节与[覆盖库存](../../grammars/codegraph-coverage.json)的长度和 SHA-256 一致。[候选清单](../../grammars/manifest.json)固定上游标签提交、MIT 许可证字节、WASM 摘要及 Rust 实测 ABI：C++ 14、C# 15、Lua 15、Luau 14。C# 的公开语言 ID 是 `csharp`，WASM 导出符号是 `c_sharp`；清单显式固定 `loader_symbol`，篡改会被拒绝。

先把清单候选数断言从 11 改为 15，测试因尚未接入而失败。C++ 初次加载测试预期 ABI 15 时失败并报告实际为 14；更正声明后，四份资产均由 Rust `WasmStore` 离线加载，基础合法样例无恢复错误，未闭合或损坏样例有解析错误。隔离 worker 对合法样例均只返回 `incomplete`、`grammar_qualified=false`。

资产与 worker 变更的本地整轮验证：`cargo test --locked --workspace --all-features` 退出 0，182 组、1136 通过、0 失败、106 条有条件忽略；`cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`、`cargo fmt --all -- --check` 及 `openspec validate introduce-rust-codeguard-cli --strict` 均通过。随后 schema 修正的定向测试为 6/6 通过；更新后整轮由 PR `gate` 复核。有条件忽略的原生工具用例不计入通过，也不作为原生对照证据。

补充发现与修复：早期 `schemas/grammar-asset-manifest.schema.json` 仍只接受四份候选，不能验证当前清单。现已更新为 32 种目标语言的封闭字段结构，并在 Rust 回归中检查当前清单所有字段和语言均被 schema 声明。使用本机既有 `/opt/anaconda3/bin/python3` 的 `jsonschema` 4.25.1 进行 Draft 2020-12 实际验证：15 份当前清单通过，伪造 `release_status=supported` 与篡改 C# `loader_symbol` 两个反例均被拒绝；未安装新依赖。此校验只证明清单协议一致，不证明 grammar 语法精度。

这些是窄范围资产与加载验证。C++ 宏/模板、C# 预处理、Lua/Luau 版本差异及嵌入语法均需独立语料和原生对照；误报/漏报、资源预算、公开 `lint` 路由和发行包安装未验收。原生 lint 仍是正式检查依据，零恢复节点不构成 clean 或任务关闭证据。
