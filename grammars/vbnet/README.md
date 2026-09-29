# VB.NET grammar 候选来源

固定 CodeGraph 提交中的 `tree-sitter-vbnet.wasm`，SHA-256 `e38a09e1826c7ec06340c0531a98bbb5ee79bdd710bba3d3349b79a19721e844`。CodeGraph 的[来源和补丁说明](https://github.com/colbymchenry/codegraph/blob/main/docs/grammars/tree-sitter-vbnet.md)把上游固定为 `govindbanura/tree-sitter-vbnet@538b7087bf80e86004531b392fe1186379c0a2b5`，并应用本目录 `source.patch`（SHA-256 `33963ebe3ba60929fb58d90efd54f206f98a54ea13ae325d784732c828b7b93a`）。`LICENSE` 与上游提交相同，SHA-256 `fa7ab39bcf92c216d429e8aa8be8a5a40db85bfbe4ff14a7bb0ecbbcca937454`。

Rust 离线加载实测 ABI 15，缩进后的类方法和破损声明样例及隔离 worker 通过。**已观察到误报：合法的未缩进类方法体被解析为 `MISSING ":"`**。该候选不得用于公开源码违规判定；需先修复 grammar 并与原生 VB.NET 编译器对照。版本语料、公开路由与发行包也未验收。
