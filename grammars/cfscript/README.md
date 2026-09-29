# CFScript grammar 候选来源

固定同一 CodeGraph 提交的 `tree-sitter-cfscript.wasm`，SHA-256 `e381db5d2b8a7744fc7d8e51b0a14ff03f86c8ff497cdfe32d1da467f19d4096`；上游 `cfmleditor/tree-sitter-cfml@5279bde9b31d54efa68efb77a4b18ee9508518e4`，许可证字节与 [CFML 来源](../cfml/README.md)一致。Rust 离线加载实测 ABI 15，基础组件函数正反例及隔离 worker 通过。

`.cfs` 裸脚本与 `.cfc`/`.cfm` 中嵌入的 `<cfscript>` 有不同入口和源码定位需求。语境切换、原生对照、版本语料、公开 lint 路由与发行包均未验收。
