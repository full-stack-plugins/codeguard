# CFQuery grammar 候选来源

固定同一 CodeGraph 提交的 `tree-sitter-cfquery.wasm`，SHA-256 `9d4eaaec46eec7e6400d4b8d6c85fac7dfe33867ed5b4e5c218dfa7851c32558`；上游 `cfmleditor/tree-sitter-cfml@5279bde9b31d54efa68efb77a4b18ee9508518e4`，许可证字节与 [CFML 来源](../cfml/README.md)一致。Rust 离线加载实测 ABI 15。

该 grammar 面向 `<cfquery>` 内的 SQL 与 `#...#` 插值，不是完整 SQL 语义检查器。实测 `SELECT #x# FROM users` 无恢复、未闭合 `#` 有恢复，但 **`SELECT FROM` 也无恢复**。因此当前结果只能是未验收的结构观察，不能用零恢复声称 SQL lint 通过。没有公开原生优先路由或发行资格。

2026-10-06：WASM 字节不变，新增独立 AST 关键词规则 `codeguard.cfquery.distinct_projection`；`SELECT DISTINCT FROM users` 会生成结构候选，`SELECT FROM users` 不匹配。项目/编辑入口已接线，完整文件及片段摘要分别保留；参见 [局部验收](../../tests/acceptance/cfquery-structure.md)。这是结构规则补充，不冒充 grammar 恢复修复或原生 SQL adapter。
