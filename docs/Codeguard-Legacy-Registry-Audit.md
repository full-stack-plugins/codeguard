# Migration audit of 57 legacy language registrations

This table compares the plugin registry at `dec5f9d1361eefff493e120b4278939a243814a9` with the frozen Rust snapshot. All 57 IDs and their 54 stable/3 planned identities agree; three languages have four changed fields. Matching metadata does not establish native adapter migration or acceptance. The Rust audit stores field-level comparison, hashes and dispositions.

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

Dispositions:

- **Preserve correct behavior:** retain all 57 identities and unchanged metadata, plus the corrected Rust coverage for `.mts/.cts`, `.R` and explicit C++ suffixes. Shared `.h` is not guessed as C++. The two extension tests in `detect_cli` provide behavioral fixtures.
- **Correct interpretation:** stable denotes historical command declarations, planned remains an explicit gap, formatter-only is not lint, and missing tools or malformed reports cannot imply acceptance. The existing native-tool-adapters, verdict-integrity and syntax-precheck specifications govern these semantics.
- **Legacy compatibility:** old lint/format/gate argv remain declarations rather than new authorized execution plans or delivery allow. The C++ gate-template difference belongs here. See [compatibility boundaries](Codeguard-Legacy-Compatibility.md).

[Field-level audit](../tests/acceptance/evidence/legacy-registry-audit-2026-10-06.json) and [acceptance](../tests/acceptance/legacy-registry-identity-audit.md). Every row uses the two-snapshot 57-identity fixture; changed rows link specific extension/compatibility fixtures. The inventory audit alone is insufficient for OpenSpec1.1; the complete source audit below now covers its remaining source-delta scope. See [source acceptance](../tests/acceptance/legacy-source-migration-audit.md). Native implementation and host acceptance remain separate open tasks.

## Complete source delta audit

The fixed source range is `03ebb24..dec5f9d`: all 21 changed files under bin/scripts/hooks are listed below. Source and referenced fixture bytes are checked directly against Git, independently of the active checkout. This audit is not implementation or native migration acceptance.

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

[Full source/fixture hashes and rationale](../tests/acceptance/evidence/legacy-source-audit-2026-10-06.json). `preserve`, `correct`, `legacy` mean retain valid behavior, explicitly correct semantics, and retain named legacy compatibility. The engine inventory and exit-zero observations do not grant protected delivery authority; Python PATH discovery remains legacy only.
