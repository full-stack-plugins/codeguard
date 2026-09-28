# Rust WASM 语法恢复节点候选验收（2026-09-29）

本记录只证明固定 Java/TypeScript grammar 的原始恢复节点提取与源码位置核对，不宣称已支持正式初检、原生复检、准确率或交付门禁。来源与字节身份见 [grammar-asset-candidates.md](grammar-asset-candidates.md)。

TDD 先为恢复节点 API 和映射 API 分别运行失败测试，两次均因目标接口不存在而编译失败。实现后，`WasmGrammar` 解析固定 WASM，`scan_wasm_recoveries` 只提取 Tree-sitter 的 `ERROR` 和 `MISSING` 节点，保留 grammar 节点种类、原始字节范围与零起始字节列。重复的同类同范围节点去重；超过诊断或遍历预算显式返回 `truncated=true`，消费者不得将结果视为完整。原始节点不含源码内容，不是 Codeguard `Finding`。

`SourceMap` 在 adapters 层验证 UTF-8、源码上限、字节边界与解析器行列是否吻合，再提供一起始 Unicode 标量列，同时保留原始字节锚点。映射失败必须进入“不完整”路径，不得以推算位置生成源码违规。Java 样本覆盖正常代码、ERROR、MISSING、中文字符及 CRLF；TypeScript 样本覆盖异常恢复节点。跨行 Java 样本中的缺失 `}` 被 grammar 锚定在上一条语句末尾，而不是文件末尾，因此后续提示不能把锚点描述成确定的插入点。

已运行的离线目标测试：

```text
CARGO_NET_OFFLINE=true cargo test --locked -p codeguard-runtime --features wasm-precheck --test wasm_recovery_scan --test wasm_grammar_load
CARGO_NET_OFFLINE=true cargo test --locked -p codeguard-adapters --test syntax_recovery_mapping
```

尚缺：按同一恢复原因归并级联节点、版本/方言边界、模板和嵌入语言区域、原生语法对照、独立合法/非法语料、资源隔离与正式 CLI 报告。上述缺口未解决时，OpenSpec 14.4 不勾选，资产仍是 `candidate_unvalidated`。
