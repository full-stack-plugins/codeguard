# CodeGraph grammar 来源覆盖库存：局部验收

固定来源为 `partme-ai/codegraph@40f112453583a2304c4b605a3a9d6545919662bd`。其 `src/extraction/grammars.ts` 映射 33 个语言 ID；`jsx` 与 `javascript` 共用 WASM，因此有 32 份独立 grammar。来源提交的 `src/extraction/wasm/` 有 30 份随仓字节；`objc`、`solidity` 通过 CodeGraph 的 `tree-sitter-wasms` 依赖解析，当前库存不假设其字节摘要。`grammars/codegraph-coverage.json` 是**来源覆盖库存**，不是已批准、可加载或已发行的资产清单；后者仍只有 Java/TypeScript 两份候选。

本地从该固定 Git 提交逐项读取 30 份 WASM，重新计算 SHA-256 和字节长度，与覆盖库存全部一致（`pinned_source_bytes_verified 30`）。未使用 CodeGraph 当前工作目录作为字节来源。库存不包含 WASM 本体，也不在运行时装入这些尚未审定的文件。

`cargo test --locked -p codeguard-cli --test grammar_status_cli`：2/2 通过。实际调用 `target/debug/codeguard grammar status --format=json` 返回 `grammar_coverage_inventory`、`codegraph_grammar_count=32`、`codegraph_vendored_count=30`、`candidate_count=2`、`released_count=0`、`execution=not_run`、`delivery_decision=not_evaluated`。Java/TypeScript 是 `candidate_unvalidated`；`objc`/`solidity` 为 `dependency_bytes_not_pinned`。COBOL 源文件 16,355,286 字节，超过现有 Rust 加载器 8 MiB 限额，明确显示 `current_loader_size_limit`。

未完成：其余 30 份资产的上游版本/许可证、Rust ABI 与真实加载、各语言版本/方言/语料、原生对照、运行时预算和发行打包。此库存不证明任何新增语言已可进行语法初检，不改变原生 lint 或交付门禁。
