# Swift grammar 候选来源

固定 CodeGraph 提交中的 `tree-sitter-swift.wasm`，SHA-256 `cc77a63b8487956270e2f385e29a03ba0773ba532a3c8a8844a26b4c98793843`。CodeGraph [Swift 来源记录](https://github.com/colbymchenry/codegraph/blob/main/docs/design/swift-kernel-port-checklist.md) 指出实际生成源是 crates.io `tree-sitter-swift` **0.7.3** crate（CodeGraph Cargo.lock 校验和 `fe36052155b9dd69ca82b3b8f1b4ccfb2d867125ac1a4db1dd7331829242668c`），其 `src/parser.c` SHA-256 `d3edff6effe31b9a507f496577407987343b101b23eb7bee7a9b050e8ab5d27a`。清单中的 `31d17fe7e818a2048c808b5c6fdc2dc792f4f5b5` 是对应语法规约的上游标签提交；**该标签生成的 C 文件与 crate 发行物不同，不能当作 WASM 的字节来源**。本目录 `LICENSE` 取自固定 crate，SHA-256 `3533cec129bb4bba015c0d61d86dd7c3b7e82110e4d2ff7837a01eff5bad5ccc`。

Rust 离线加载实测 ABI 15，基础正反例及隔离 worker 通过。Swift 6.4 原生 `swiftc -frontend -parse` 与同一 worker 的 13 例独立差分中，12 例分类一致；`func f(_ x: ) {}` 被原生编译器拒绝为缺少参数类型，而固定 WASM 无恢复节点，是已确认漏检。见[局部差分验收](../../tests/acceptance/swift-native-differential.md)。因此此候选不得签发语法通过；Swift 版本/方言语料、修复后的编译器对照、公开原生优先 lint 路由和发行包仍未验收。
