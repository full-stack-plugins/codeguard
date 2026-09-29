# Scala grammar 候选来源

固定 CodeGraph 提交中的 `tree-sitter-scala.wasm`，SHA-256 `37d7fe5a91ca98941dc05493b0c05a0df0f36df5035890fa00b02497c68aaac3`。CodeGraph [Scala 来源记录](https://github.com/colbymchenry/codegraph/blob/main/docs/design/scala-kernel-port-checklist.md) 指出该字节是上游 `tree-sitter/tree-sitter-scala` 发行版 `v0.26.2` 的 WASM；标签提交 `b931fcc338390925eb893d70ad070033f5856ccf`。`LICENSE` 来自该提交，SHA-256 `1f95ed26e1f4074074c9c7083e61c0a9e4c3b9f435745044995f3beb4ed28575`。

Rust 离线加载实测 ABI 15，基础正反例及隔离 worker 通过。Scala 2/3 版本语料、原生编译器对照、公开 lint 路由及发行均未验收。
