# TSX grammar 候选纠错：局部验收

此前可选 `lint typescript` 单文件初检接受 `.tsx`，却总把源码交给普通 TypeScript grammar。有效 JSX 的回归测试先失败：报告 `syntax_precheck.language=typescript`，有把合法代码误报为疑似语法异常的风险。

从固定 `partme-ai/codegraph@40f112453583a2304c4b605a3a9d6545919662bd` 的 `src/extraction/wasm/tree-sitter-tsx.wasm` 复制原始字节；SHA-256 为 `8f647a1b2cafe9ab00fb2056d79021d2a144ba17a72f45511072311c1b05d08e`，长度 1,445,641 字节。TSX 与 TypeScript 来自同一 `tree-sitter/tree-sitter-typescript@f975a621f4e7f532fe322e13c4f79495e0a7b2e7`，共用仓内固定的 MIT 许可证。Rust `WasmStore` 离线加载实测 ABI 14；合法 JSX 无恢复错误，明显缺失属性值的 JSX 有恢复错误。仅测试语法恢复，不把 JSX 标签语义校验等同于 Tree-sitter 语法能力。

可选 `wasm-precheck` 构建下，`.tsx` 使用 TSX grammar、按其摘要绑定父子进程报告，输出独立 `eslint_local_feedback` 0.4.0；`.ts/.mts/.cts` 保留 0.3.0 TypeScript grammar 路径。两者都保持原生 `not_run`、候选 `incomplete`、交付 `not_evaluated`，不会产生原生 finding。报告读者拒绝 `.java` 路径冒充 TSX 方言，此测试同样先失败后通过。

验证：`cargo test --locked -p codeguard-adapters --test grammar_asset_manifest --test syntax_precheck_report_contract`、`cargo test --locked -p codeguard-runtime --features wasm-precheck --test wasm_grammar_load`、`cargo test --locked -p codeguard-cli --features wasm-precheck --test typescript_syntax_fallback_candidate`、`cargo test --locked -p codeguard-cli --test grammar_status_cli` 均通过。仍待逐语言/版本语料、原生确认、完整 lint/check 调度、公开制品和宿主真实反馈；OpenSpec S14 保持未完成。
