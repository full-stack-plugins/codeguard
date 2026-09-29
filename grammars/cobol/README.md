# COBOL grammar 候选来源

固定 CodeGraph 提交中的 `tree-sitter-cobol.wasm`，16355286 字节，SHA-256 `65b98799f92e831d8b5ed8cd811464c71fac31c4beabf66990a545a4f65e02f1`。CodeGraph 的[来源和补丁说明](https://github.com/colbymchenry/codegraph/blob/main/docs/grammars/tree-sitter-cobol.md)固定上游 `yutaro-sakamoto/tree-sitter-cobol@e99dbdc3d800d5fa2796476efd60af91f6b43d93` 与本目录 `source.patch`（SHA-256 `04fe11a51243ddf7d31e0b6378068c725a51463534218c43c7021761c69701e5`）。`LICENSE` 与上游提交相同，SHA-256 `d724405ce238a22c0d35769c5a36b386ad5958192efe8bbb304fb2896254575f`。

原 WASM 导出名为 `tree_sitter_COBOL`，固定清单以 `loader_symbol=COBOL` 加载，不改写二进制导出。Rust 加载实测 ABI 14。20 MiB 资产上限允许这一份固定字节进入隔离 worker；一次本机冷加载观察约 20 秒、峰值常驻内存约 458 MB，后续 worker 测试约 12 秒。这不构成正式资源预算。固定/自由格式转换、方言、copybook、原生编译器对照、公开 lint 路由及发行仍缺；大体积和解析成本使其不适合未经预算的编辑快检。
