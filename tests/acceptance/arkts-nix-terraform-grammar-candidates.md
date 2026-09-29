# ArkTS、Nix、Terraform grammar 候选局部验收

日期：2026-09-29。固定 CodeGraph 来源：`1072f82ce24db3d133258d30165cef6b74d108b2`。三份随仓 WASM 与[来源库存](../../grammars/codegraph-coverage.json)的字节长度和 SHA-256 一致；不能因来源存在就宣称 CodeGuard 已发行语法能力。

| 语种 | 原始来源与固定身份 | WASM SHA-256 | 上游许可证 SHA-256 | Rust 实测 ABI |
| --- | --- | --- | --- | --- |
| ArkTS | `harmony-contrib/tree-sitter-arkts@56f7fc288715befe92c734d603f9e6bc3c65d2a8`，npm `tree-sitter-arkts@0.2.0`，包完整性见清单 | `db0812971109457d22b3fe9dcdb1cd8e614fba2a338e220670bd254485753623` | `048e4dcb7a71fb725495ebb3e7052d0e76dcb6705b828af8b9b461df10bcd8ca` | 14 |
| Nix | `nix-community/tree-sitter-nix@3d0173d903e630b6e14d17f1cf79488791379ded`，CodeGraph CLI 0.25.10 重建 | `4acffa1c013df751193a21ce429777ca214c7d3a7e3665085b6df00dad8869f0` | `c1b72e50266464ec5118d26200e17bf1f472d142e62ab637e1ffc23986657a78` | 15 |
| Terraform | `tree-sitter-grammars/tree-sitter-hcl@fad991865fee927dd1de5e172fb3f08ac674d914`，npm `@tree-sitter-grammars/tree-sitter-hcl@1.2.0`，包完整性见清单 | `0d9ef3ae926acc0411bfecba9b34df4ea659061917b96455570a45a70824e890` | `c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4` | 14 |

ArkTS 与 Terraform 的 npm tarball SHA-512 实际计算值与 npm 元数据中的 `dist.integrity` 一致；tarball 中的 WASM 与 CodeGraph 固定字节一致，tarball 中的许可证与固定上游提交一致。Nix 许可证取自固定上游提交。清单固定包完整性、WASM 与许可证摘要；改变 npm 完整性字符串的反例被拒绝。

测试先把候选数断言改为 18 而失败；ArkTS 加载时原预期 ABI 15 也失败并返回实际 14。纠正后，三份 WASM 均由 Rust 离线加载，窄范围合法样例无恢复错误，损坏样例有解析错误；独立 worker 对合法样例仍返回 `incomplete` 且 `grammar_qualified=false`。Draft 2020-12 的清单 schema 校验通过。

最终源码状态的 `cargo test --locked --workspace --all-features` 退出 0：182 组、1140 通过、0 失败、106 忽略；忽略的原生工具用例不算已验收。全目标 Clippy、fmt、OpenSpec strict 和 `git diff --check` 退出 0。实际 `codeguard grammar status --format=json` 返回退出码 3，报告 32 来源 / 30 随仓 / 18 候选 / 0 已发行，并固定 `parser_capability=unverified`、`gate_effect=none`、`delivery_decision=not_evaluated`。

尚缺：语言版本和方言语料、与原生工具的误报/漏报对照、公开 `lint/check` 的原生优先路由、不同宿主的对话反馈、资源预算及发行包实装。当前只是 18/32 候选，已发行验收仍为 0。
