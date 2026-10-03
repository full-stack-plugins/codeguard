# 已知 grammar 限制的只读报告

日期：2026-10-03。对应 OpenSpec 14.1、14.7、14.17、14.19 的局部增量。

先写 RED 测试：`grammar status --format=json` 的逐语种行没有 `known_limitations`，而可变清单接受超过 1024 字节的限制说明。随后以固定资产清单作为只读来源，逐行投影限制并将只读报告协议升为 `1.1.0`；清单解析拒绝空限制、超过 8 条、单条超过 1024 字节或控制字符。`grammar status` 不加载 WASM，不运行原生工具，保持退出码 3、`released_count=0`、`delivery_decision=not_evaluated`。

目标测试：`cargo test --locked -p codeguard-adapters --test grammar_asset_manifest candidate_limitations_are_bounded_before_dialog_projection -- --exact` 和 `cargo test --locked -p codeguard-cli --test grammar_status_cli codegraph_coverage_is_explicit_and_never_claims_parser_or_gate_completion -- --exact`。本轮在修改前分别失败于超长文本被接受、报告缺字段；修改后两项通过。VB.NET 行明确包含已知未缩进方法的误报提示，详见[原始复现](vbnet-unindented-method-false-positive.md)。该报告没有完成原生对照或修复 grammar，不改变候选和门禁状态。

扩展反例还覆盖 9 条限制与换行控制字符。完整 manifest 测试 11/11、`grammar_status_cli` 2/2、受影响 crate 的 all-targets Clippy `-D warnings`、`cargo fmt --all --check`、OpenSpec strict 和 JSON Schema 对当前清单的校验均通过。已发布的 npm 0.1.3 由旧源码构建，不包含本次报告字段；此处只证明当前源码候选，不冒称已发布行为。
