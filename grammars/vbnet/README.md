# VB.NET grammar 候选来源

固定 CodeGraph 提交中的 `tree-sitter-vbnet.wasm`，SHA-256 `e38a09e1826c7ec06340c0531a98bbb5ee79bdd710bba3d3349b79a19721e844`。CodeGraph 的[来源和补丁说明](https://github.com/colbymchenry/codegraph/blob/main/docs/grammars/tree-sitter-vbnet.md)把上游固定为 `govindbanura/tree-sitter-vbnet@538b7087bf80e86004531b392fe1186379c0a2b5`，并应用本目录 `source.patch`（SHA-256 `33963ebe3ba60929fb58d90efd54f206f98a54ea13ae325d784732c828b7b93a`）。`LICENSE` 与上游提交相同，SHA-256 `fa7ab39bcf92c216d429e8aa8be8a5a40db85bfbe4ff14a7bb0ecbbcca937454`。

Rust 离线加载实测 ABI 15，缩进后的类方法和破损声明样例及隔离 worker 通过。**已观察到误报：合法的未缩进类方法体被解析为 `MISSING ":"`**。该候选不得用于公开源码违规判定；需先修复 grammar 并与原生 VB.NET 编译器对照。版本语料、公开路由与发行包也未验收。

2026-10-03 复现：已发布的 0.1.3 二进制对 `Public Class C`、未缩进的 `Public Function F() As Integer`、`Return 1`、`End Function`、`End Class` 五行执行 `grammar probe vbnet FILE --format=json`，在第 2 行末字节 45 报出一处 `MISSING ":"`；仅增加方法体缩进的同义样例返回零恢复节点。两例均退出 3、`grammar_qualified=false`，不产生已确认源码违规。详见[误报局部复现](../../tests/acceptance/vbnet-unindented-method-false-positive.md)；本机没有 .NET 编译器，因此仍需独立原生语法对照。
