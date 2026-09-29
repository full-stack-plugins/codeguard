# 固定 WASM grammar 候选资产：来源与边界（2026-09-29）

`grammars/manifest.json` 固定 [CodeGraph](https://github.com/partme-ai/codegraph/tree/40f112453583a2304c4b605a3a9d6545919662bd) 提交 `40f112453583a2304c4b605a3a9d6545919662bd` 中的 Java、TypeScript WASM 字节。CodeGraph 工作树当前有其它未提交改动；复制使用 `git show <commit>:<path>`，不从活动目录读取。CodeGraph `src/extraction/grammars.ts` 在该来源记录 `tree-sitter-cli 0.25.10 build --wasm`、上游 grammar 提交与版本。来自同一提交的 CodeGraph MIT 许可及两个上游提交的 MIT 许可随制品保留。

| 语言 | 上游版本与提交 | WASM SHA-256 | 字节 | 实测 CodeGraph ABI |
|---|---|---|---:|---:|
| Java | `tree-sitter-java v0.23.5` / `94703d5a6bed02b98e438d7cad1136c01a60ba2c` | `181a6fbc34d7864a551d91c13882fc33007e923b3c81a4bdbe7fa47492090077` | 414653 | 14 |
| TypeScript | `tree-sitter-typescript v0.23.2` / `f975a621f4e7f532fe322e13c4f79495e0a7b2e7` | `3a44d634c9840dccec36f33b99592bd086a2f940e9cd80c64347c45b7dce662f` | 1414055 | 14 |

ABI 14 最初用 CodeGraph 已安装的 `web-tree-sitter` 0.25.3 `Language.load(...).abiVersion` 读取这两份**固定字节**得到。CodeGuard 后用 Rust `tree-sitter` 0.25.10 的 `WasmStore` 加载相同字节并检查真实 ABI；两份资产均完成离线加载和有效源码解析，Java 的损坏源码产生错误节点。CodeGuard 语言版本、完整语料、错误节点映射与原生对照仍未验收，所以清单保持 `candidate_unvalidated`、`language_versions=[]`，仅将加载状态记为 `rust_loader_smoke_passed`。本次不修改能力矩阵，也不启用 CLI 初检。

TDD：新增 `grammar_asset_manifest` 测试先因缺少 Rust API 编译失败；实现后 3/3 通过。测试核对内置来源、资产和许可证原始字节；缺来源、重复键、未知语言、改动的 WASM/许可证和错误 ABI 声明均拒绝。`schemas/grammar-asset-manifest.schema.json` 约束结构；Rust 解析进一步固定两个来源和精确摘要。字节摘要只能证明引用的一致性，不证明语法准确率。

Rust 加载器的 `wasm_grammar_load` 测试先因缺少 `WasmGrammar` 编译失败；实现后覆盖 Java/TypeScript 固定资产、损坏模块、错误摘要和错误 ABI 声明。离线运行 `cargo test --locked -p codeguard-runtime -p codeguard-adapters --features codeguard-runtime/wasm-precheck`、`cargo test --workspace --locked --quiet` 均通过，前者显式覆盖新功能，后者验证默认功能回归；还通过带新特性的 runtime Clippy 检查。加载器是可选编译特性，当前仅为受控兼容性试验，不进入 CLI 调度，也未具备工作进程隔离。`cargo metadata` 显示当前解析到的依赖未声明高于 1.85 的 Rust 版本；本机未安装 Rust 1.85，实际 MSRV 编译尚未验收。

OpenSpec 14.1 和 14.2 仍不勾选：语言/方言版本边界、MSRV 实编、进程隔离、错误节点/位置和可发行验收仍需后续任务补齐。加载成功不意味着语法规则准确或原生 lint 义务已完成。

后续增量：[TSX 候选纠错](tsx-grammar-candidate.md)把同一来源仓库的 TSX 独立 WASM 加入固定候选清单；当前候选资产数为 3。本页的“两份”及初次验收命令属于原 Java/TypeScript 引入时的历史快照。
