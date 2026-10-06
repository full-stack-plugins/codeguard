# 57 项旧语言注册表迁移核对

本表核对插件 `dec5f9d1361eefff493e120b4278939a243814a9` 的最新注册表与 Rust 固定快照。57个ID及其54 stable/3 planned归属一致；仅3个语言、4个字段不同。元数据一致不代表原生检测能力已迁移或通过。完整字段、摘要与分类由 Rust 审计工具保存。

| Language | Legacy status | Lint declaration | Formatter declaration | Changed fields |
|---|---|---|---|---|
| ansible | stable | yes | yes | — |
| arkts | planned | no | no | — |
| astro | stable | yes | yes | — |
| c | stable | yes | yes | — |
| cfml | stable | yes | no | — |
| clojure | stable | yes | yes | — |
| cobol | planned | no | no | — |
| cpp | stable | yes | yes | extensions, gate |
| crystal | stable | yes | yes | — |
| csharp | stable | yes | yes | — |
| css | stable | yes | yes | — |
| cuda | stable | yes | yes | — |
| dart | stable | yes | yes | — |
| dockerfile | stable | yes | yes | — |
| elixir | stable | yes | yes | — |
| elm | stable | yes | yes | — |
| erlang | stable | yes | yes | — |
| fsharp | stable | yes | yes | — |
| go | stable | yes | yes | — |
| graphql | stable | yes | yes | — |
| groovy | stable | yes | yes | — |
| haskell | stable | yes | yes | — |
| html | stable | yes | yes | — |
| java | stable | yes | yes | — |
| julia | stable | no | yes | — |
| kotlin | stable | yes | yes | — |
| liquid | stable | yes | no | — |
| lua | stable | yes | yes | — |
| luau | stable | yes | yes | — |
| markdown | stable | yes | yes | — |
| metal | planned | no | no | — |
| nim | stable | yes | yes | — |
| nix | stable | yes | yes | — |
| objc | stable | yes | yes | — |
| ocaml | stable | yes | yes | — |
| pascal | stable | no | yes | — |
| perl | stable | yes | yes | — |
| php | stable | yes | yes | — |
| powershell | stable | yes | yes | — |
| protobuf | stable | yes | yes | — |
| python | stable | yes | yes | — |
| r | stable | yes | yes | extensions |
| ruby | stable | yes | yes | — |
| rust | stable | yes | yes | — |
| scala | stable | yes | yes | — |
| shell | stable | yes | yes | — |
| solidity | stable | yes | yes | — |
| sql | stable | yes | yes | — |
| svelte | stable | yes | yes | — |
| swift | stable | yes | yes | — |
| terraform | stable | yes | yes | — |
| toml | stable | yes | yes | — |
| typescript | stable | yes | yes | extensions |
| vbnet | stable | yes | yes | — |
| vue | stable | yes | yes | — |
| yaml | stable | yes | yes | — |
| zig | stable | yes | yes | — |

```bash
cargo run --locked -p codeguard-cli --example audit_legacy_registry -- --check tests/acceptance/evidence/legacy-registry-audit-2026-10-06.json
```

分类依据：

- **保留正确行为**：57个规范身份和未变化字段保留；Rust的 `.mts/.cts`、`.R` 和 C++明确后缀覆盖也保留，不因插件较窄范围而回退。共享 `.h` 仍不猜测为 C++。对应 `detect_cli` 两个实际扩展名用例。
- **明确纠偏**：stable只是旧命令声明，planned不等于不适用；格式化器不冒充lint、目录/工具声明不证明检查完成、缺工具/坏报告不能假通过。这些新流程语义以现有 native-tool-adapters、verdict-integrity 和 syntax-precheck 规格为准。
- **legacy兼容**：全部旧 lint/format/gate 命令仅记录历史声明，不能直接形成新版执行计划或交付allow；C++ gate模板的差异明确归于此范围。数字/Hook映射见 [兼容边界](Codeguard-Legacy-Compatibility.zh_CN.md)。

[逐字段审计](../tests/acceptance/evidence/legacy-registry-audit-2026-10-06.json) 和 [验收](../tests/acceptance/legacy-registry-identity-audit.md)。每行共同 fixture 为 `legacy_registry_identity` 的双快照57身份对照；有差异行另外关联具体扩展名/兼容fixture。仅登记和核对完成，OpenSpec1.1的其它旧源码行为及真实对照仍需完成，不勾选父任务。
