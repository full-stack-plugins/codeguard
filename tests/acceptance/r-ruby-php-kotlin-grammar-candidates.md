# R、Ruby、PHP、Kotlin grammar 候选局部验收

日期：2026-09-29。固定 CodeGraph 来源：`1072f82ce24db3d133258d30165cef6b74d108b2`。四份随仓 WASM 与[来源库存](../../grammars/codegraph-coverage.json)逐字节一致，许可证和上游版本固定在[候选资产清单](../../grammars/manifest.json)。

| 语种 | 固定上游来源 | WASM SHA-256 | 许可证 SHA-256 | Rust 实测 ABI |
| --- | --- | --- | --- | --- |
| R | `r-lib/tree-sitter-r@e9944e9801595ad484f49be492daf0c4c81547ef`，`v1.2.0` | `2a8f5acd1c53d91e0ec5c01a6830d8ac7f5a7f96f0ac4b3768c016c8e9d07711` | `de2e49529f03d573bc3fa229dc83acfe22c63a5ad1766b289563edfd45b72dea` | 14 |
| Ruby | `tree-sitter/tree-sitter-ruby@71bd32fb7607035768799732addba884a37a6210`，`v0.23.1` | `4cb5a4b12870876ca864c1e92fe1f5cd47036b2adc083e9306488af88867dbb4` | `ee006f02a3d856df282e409be2a86e24a65bb573a98b9c28343771141351bb6b` | 14 |
| PHP | `tree-sitter/tree-sitter-php@5b5627faaa290d89eb3d01b9bf47c3bb9e797dea`，`v0.24.2-full-php` | `6545a9a110bc878e26ed329950147e190c83da038bb17e999de646fe6c4d6c82` | `664764fabf40c0b318e06324a5a12379092a2df25b8cbf2415f71bc0bb352346` | 15 |
| Kotlin | `fwcd/tree-sitter-kotlin@e1a2d5ad1f61f5740677183cd4125bb071cd2f30`，`0.3.8` | `c80c88867a589a1a0959bcea89de84b7e9684b3693b2cdb2944812458e62ff48` | `948495f61768f7de26bcc61113d8cd95f50bbc15adb678c28c941c6c8fcd5903` | 14 |

四份 grammar 经 Rust `WasmStore` 离线加载，基础合法样例无恢复错误、损坏样例出现恢复错误；PHP 的合法样例还覆盖 HTML 与 PHP 混合内容，核对使用的是完整 PHP grammar。隔离 worker 将这些样例标为 `incomplete`、`grammar_qualified=false`。测试先将清单期望由 18 改为 22 而失败，随后加入固定资产和校验逻辑后通过。

Dart 仍未接入。CodeGraph 固定来源中的 Dart WASM 旧 `dylink` 元数据可作有界转换，但 Rust `WasmStore` 随后拒绝实例化，错误为 `invalid import 'tree_sitter_dart_external_scanner_create'`。外部 scanner 是语法的一部分，不能用空实现让测试假通过。`grammar status` 将其标为 `rust_worker_external_scanner_import`；需要固定可再分发的重建资产或等效、经语料验证的 scanner 适配，再重新做 ABI 与解析验收。

实际源码构建的 `codeguard grammar status --format=json` 返回退出码 3：32 种来源 grammar、30 份随仓 WASM、22 份候选、0 项已发行；Dart 为 `not_integrated`，`parser_capability=unverified`、`gate_effect=none`、`delivery_decision=not_evaluated`。

本次 `cargo test --locked --workspace --all-features` 退出 0：182 组、1142 通过、0 失败、106 忽略；忽略的原生工具用例不算验收。全目标 Clippy、fmt、OpenSpec strict、Draft 2020-12 清单 schema 及 `git diff --check` 均通过。

尚缺：逐语言版本/方言语料、原生工具对照、公开原生优先 lint 路由、任务及宿主反馈、资源预算和发行包实装。四份只是候选，不等于已发行语法检查能力。
