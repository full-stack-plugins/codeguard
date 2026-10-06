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

[逐字段审计](../tests/acceptance/evidence/legacy-registry-audit-2026-10-06.json) 和 [验收](../tests/acceptance/legacy-registry-identity-audit.md)。每行共同 fixture 为 `legacy_registry_identity` 的双快照57身份对照；有差异行另外关联具体扩展名/兼容fixture。语言登记核对不能单独完成OpenSpec1.1；下方完整源码审计现已覆盖其剩余差异范围，见[源码验收](../tests/acceptance/legacy-source-migration-audit.md)。原生实现和宿主验收仍由其它任务承担。

## 完整旧源码差异核对

固定范围为 `03ebb24..dec5f9d`：bin/scripts/hooks全部21个变化文件逐项归类如下。Rust审计器直接核对Git源码和引用夹具字节，不借用工作树或旧运行日志。这是迁移差异核对，不代表原生迁移或宿主验收完成。

| Source | Disposition | Specification | Fixture |
|---|---|---|---|
| `bin/codeguard` | preserve | hook-protocol | `tests/test_rust_lifecycle.cjs` |
| `hooks/__protocol__.md` | preserve | hook-protocol | `tests/test_rust_lifecycle.cjs` |
| `hooks/env_check.py` | legacy | hook-protocol | `tests/test_plugin_manifests.py` |
| `hooks/gate_lib.py` | legacy | hook-protocol | `tests/test_plugin_manifests.py` |
| `hooks/hooks.json` | preserve | hook-protocol | `tests/test_rust_lifecycle.cjs` |
| `hooks/post_tool_lint.py` | legacy | hook-protocol | `tests/test_plugin_manifests.py` |
| `hooks/pre_tool_git_guard.py` | legacy | hook-protocol | `tests/test_plugin_manifests.py` |
| `hooks/rust_runtime_dispatch.cjs` | preserve | hook-protocol | `tests/test_rust_lifecycle.cjs` |
| `hooks/stop_summary.py` | legacy | hook-protocol | `tests/test_plugin_manifests.py` |
| `hooks/user_prompt_validator.py` | legacy | hook-protocol | `tests/test_prompt_application.py` |
| `scripts/bump-plugin.mjs` | preserve | binary-distribution | `tests/test_plugin_manifests.py` |
| `scripts/check_architecture.py` | legacy | execution-kernel | `tests/test_architecture.py` |
| `scripts/codeguard/engine.py` | correct | verdict-integrity | `tests/test_engine_boundary.py` |
| `scripts/codeguard/gate.py` | preserve | execution-kernel | `tests/test_skip_gate_safety_scope.py` |
| `scripts/codeguard/git_guard_application.py` | legacy | hook-protocol | `tests/test_skip_gate_safety_scope.py` |
| `scripts/codeguard/prompt_application.py` | legacy | hook-protocol | `tests/test_skip_gate_safety_scope.py` |
| `scripts/codeguard/reporting.py` | legacy | hook-protocol | `tests/test_skip_gate_safety_scope.py` |
| `scripts/codeguard/toolchain.py` | preserve | execution-kernel | `tests/test_skip_gate_safety_scope.py` |
| `scripts/engine_report.py` | correct | verdict-integrity | `tests/test_engine_boundary.py` |
| `scripts/paths.py` | preserve | execution-kernel | `tests/test_skip_gate_safety_scope.py` |
| `scripts/validate_portable_plugin.py` | preserve | binary-distribution | `plugin.json` |

```bash
cargo run --locked -p codeguard-cli --example audit_legacy_source -- --check-source ../codeguard-plugin
```

[完整源码/夹具摘要和分类依据](../tests/acceptance/evidence/legacy-source-audit-2026-10-06.json)。preserve、correct、legacy分别表示保留正确行为、明确纠偏和具名旧兼容。能力自报或退出0都不能授予受保护交付权威；Python路径发现仍只属于旧兼容。
