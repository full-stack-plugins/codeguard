# CodeGraph grammar 来源覆盖库存：局部验收

固定来源为 `partme-ai/codegraph@40f112453583a2304c4b605a3a9d6545919662bd`。其 `src/extraction/grammars.ts` 映射 33 个语言 ID；`jsx` 与 `javascript` 共用 WASM，因此有 32 份独立 grammar。来源提交的 `src/extraction/wasm/` 有 30 份随仓字节；`objc`、`solidity` 通过 CodeGraph 的 `tree-sitter-wasms` 依赖解析，当前库存不假设其字节摘要。`grammars/codegraph-coverage.json` 是**来源覆盖库存**，不是已批准、可加载或已发行的资产清单；后者仍只有 Java/TypeScript 两份候选。

本地从该固定 Git 提交逐项读取 30 份 WASM，重新计算 SHA-256 和字节长度，与覆盖库存全部一致（`pinned_source_bytes_verified 30`）。未使用 CodeGraph 当前工作目录作为字节来源。库存不包含 WASM 本体，也不在运行时装入这些尚未审定的文件。

`cargo test --locked -p codeguard-cli --test grammar_status_cli`：2/2 通过。实际调用 `target/debug/codeguard grammar status --format=json` 返回 `grammar_coverage_inventory`、`codegraph_grammar_count=32`、`codegraph_vendored_count=30`、`candidate_count=2`、`released_count=0`、`execution=not_run`、`delivery_decision=not_evaluated`。Java/TypeScript 是 `candidate_unvalidated`；`objc`/`solidity` 为 `dependency_bytes_not_pinned`。COBOL 源文件 16,355,286 字节，超过现有 Rust 加载器 8 MiB 限额，明确显示 `current_loader_size_limit`。

未完成：其余 30 份资产的上游版本/许可证、Rust ABI 与真实加载、各语言版本/方言/语料、原生对照、运行时预算和发行打包。此库存不证明任何新增语言已可进行语法初检，不改变原生 lint 或交付门禁。

后续增量：[TSX 候选纠错](tsx-grammar-candidate.md)将 TSX 纳入候选资产，当前源码构建的 `grammar status` 候选数变为 3，仍无已验收发行能力。上面的 2 候选与 2/2 测试记录保留为本次覆盖库存引入时的验收快照。

再后续的 [Python 资产候选](python-grammar-asset-candidate.md)使当前候选数变为 4；Python 尚未接入 lint，已发行能力仍为 0。上述数值均为各次增量当时的记录。

2026-09-29 增量：来源库存更新到 CodeGraph `1072f82ce24db3d133258d30165cef6b74d108b2`，当前工作树 30/30 份随仓 WASM 的长度与 SHA-256 与新库存一致。Zig 源 WASM 已变更为修复空容器误报的构建；其原始字节与 Rust 适配后的候选资产、加载测试见 [Zig 局部验收](zig-grammar-candidate.md)。当前候选资产为 5，已验收发行仍为 0。`objc` 与 `solidity` 的精确字节可从 CodeGraph 锁定的 `tree-sitter-wasms@0.1.13` 本机 npm 缓存重现，但本增量尚未把它们写入 CodeGuard 资产清单或验收为可加载能力。历史 2/3/4 候选数仅表示各次验收快照。

2026-09-29 后续增量：Objective-C 与 Solidity 的依赖包原始字节、上游许可证及可复现的 Rust `dylink` 元数据适配现已入[候选清单](../../grammars/manifest.json)；局部加载和隔离 worker 证据见[依赖 grammar 候选验收](dependency-grammar-candidates.md)。当前候选数 7，已验收发行仍为 0。上述 2/3/4/5 候选数是历史快照，旧的 `dependency_bytes_not_pinned` 状态也只适用于当时的库存。
