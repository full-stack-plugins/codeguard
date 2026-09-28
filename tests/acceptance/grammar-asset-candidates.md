# 固定 WASM grammar 候选资产：来源与边界（2026-09-29）

`grammars/manifest.json` 固定 [CodeGraph](https://github.com/partme-ai/codegraph/tree/40f112453583a2304c4b605a3a9d6545919662bd) 提交 `40f112453583a2304c4b605a3a9d6545919662bd` 中的 Java、TypeScript WASM 字节。CodeGraph 工作树当前有其它未提交改动；复制使用 `git show <commit>:<path>`，不从活动目录读取。CodeGraph `src/extraction/grammars.ts` 在该来源记录 `tree-sitter-cli 0.25.10 build --wasm`、上游 grammar 提交与版本。来自同一提交的 CodeGraph MIT 许可及两个上游提交的 MIT 许可随制品保留。

| 语言 | 上游版本与提交 | WASM SHA-256 | 字节 | 实测 CodeGraph ABI |
|---|---|---|---:|---:|
| Java | `tree-sitter-java v0.23.5` / `94703d5a6bed02b98e438d7cad1136c01a60ba2c` | `181a6fbc34d7864a551d91c13882fc33007e923b3c81a4bdbe7fa47492090077` | 414653 | 14 |
| TypeScript | `tree-sitter-typescript v0.23.2` / `f975a621f4e7f532fe322e13c4f79495e0a7b2e7` | `3a44d634c9840dccec36f33b99592bd086a2f940e9cd80c64347c45b7dce662f` | 1414055 | 14 |

ABI 14 是用 CodeGraph 当前已安装的 `web-tree-sitter` 0.25.3 `Language.load(...).abiVersion` 读取这两份**固定字节**得到的观察；尚未证明 CodeGuard Rust 加载器可用。CodeGuard 语言版本、语料、错误节点与原生对照均未验收，所以清单明确 `candidate_unvalidated`、`codeguard_runtime_validation=pending`、`language_versions=[]`。本次不修改能力矩阵，也不启用 CLI 初检。

TDD：新增 `grammar_asset_manifest` 测试先因缺少 Rust API 编译失败；实现后 3/3 通过。测试核对内置来源、资产和许可证原始字节；缺来源、重复键、未知语言、改动的 WASM/许可证和错误 ABI 声明均拒绝。`schemas/grammar-asset-manifest.schema.json` 约束结构；Rust 解析进一步固定两个来源和精确摘要。字节摘要只能证明引用的一致性，不证明 Rust 运行时兼容性或语法准确率。

OpenSpec 14.1 仍不勾选：完整语言/方言版本边界、Rust 实际 ABI 加载反例及资产可发行验收要随 14.2/14.4/14.17 补齐。下一步在 Rust runtime 中加载这些固定字节，实际检查 ABI/损坏样本；失败保持 `unsupported/incomplete`，不得由清单存在推断可用。
