# Guard integration boundary, local SDK profile

## Selected interface

The selected interface is the pure Rust `codeguard_cli::guard_integration` library,
using the GuardEngine SDK in the CLI crate only. Existing native command dispatch
is unchanged. `InvocationDescriptor::parse` and `read_native` consume existing
bounded byte slices; `FrozenObligations::new` receives protected required targets;
`ProtectedMapping::parse` receives protected exact mapping bytes. `project` takes
that evidence, frozen obligations, mapping, typed GuardContract and GuardSubject.
`FrozenRun` checks structural binding before returning an in-memory result bundle.
These calls do not open artifact paths, spawn native tools, run policy scripts,
install tools, repair source, contact a host, or publish files.

Inputs are supplied by the caller from separately captured native evidence and
protected policy. Matching invocation, hashes and binding is an integrity check,
not caller/provenance authentication. No profile currently qualifies complete
native scope. Completed projections remain partial/BLOCK; native aggregate exit3
cannot mean REQUIRE_APPROVAL. Native failure130 remains in the original bytes and
maps to cancelled/null on the bound failure path.

The explicit Unix CLI entry, added in the subsequent transport slice, is:

```
codeguard guard-project --invocation <file> --native-report <file> \
  --context <file> --mapping <file> --contract <file>
```

`context` carries versioned structural binding, independently frozen obligations
and times; mapping/contract must come from protected policy. The explicit CLI
consumes existing bounded regular files only and emits an inline versioned bundle.
See [the CLI contract](../guard-project-cli.md) for pre-binding stderr diagnostics,
bound null-decision failures, JSON-only contract input and the current reachable
BLOCK2/error4 outcomes. It does not select the adapter for native commands.
File publication task3.5 and host consumer task3.7 remain separate. CLI2.5 is
implemented pending independent review, not a production qualification claim.

## Source and release gate

The CLI has the existing local path dependency `guardengine = { path =
"../../../guardengine" }`. The reviewed SDK source snapshot for this compatibility
fixture is `c80ec325449842d51007db2fd507040c53dc8f51`, recorded in
`tests/fixtures/guard-integration/sdk-source.json`. It includes the integration YAML
preflight, typed evaluation budget and borrowed FactBudget source-fragment builder.
Engine wire is `guard.partme.ai/v1alpha1`; envelope is
`guard.integration/v1alpha1`; neither is the crate package version or a qualification
claim. A different source revision requires rerunning the compatibility fixture.

This is a review-source pin, not a Cargo-enforced Git/registry dependency pin:
Cargo.lock does not pin content of a sibling path. Release remains unavailable
until an approved immutable artifact/source revision, archive and toolchain
provenance, clean independent consumer verification, dependency lock and actual
native/adapter version matrix are fixed. Publishing or claiming a released SDK is
not authorized by this local fixture. No engine process protocol is used.

## Qualified extent and evidence

Only native run_report schema1.0, lint with JSON output, package0.1.4 is registered.
The strict native run_report parser remains the format authority. Raw native bytes,
SHA-256, findings and original exit are retained. Feedback/export-failure output,
query/hook/legacy input, and unknown native schemas remain unsupported by this
reader. This does not change their native behavior.

`guard_integration_scope.rs` proves zero findings/exit0, a WASM-only inventory, and
resolved-task messages do not grant complete scope. Unknown proof kinds and a
task-verification preview cannot substitute for the registered native report.
The qualification fixture maps missing true-native requirements to the historical
task IDs; adapter tests cannot certify completion of those tasks.

`guard_integration_boundary.rs` repeats the selected byte-only projection ten times
and checks identical reports and raw domain bytes. Its explicit Linux strace test
records file/process/network syscalls between probe markers and sees no projection
file or process/network activity. An unselected native `--version --format=json`
query records only its own exec, no adapter/project artifact access or network,
and unchanged source, .codeguard events, Git index and ref sentinels. Dynamic-loader
file reads at native process startup remain normal native startup, not adapter I/O.
This is actual Linux fixture evidence, not Windows/macOS coverage or a sandbox.
Run the ignored trace test explicitly where strace is installed; absence of that
prerequisite is unverified, not a passing side-effect test.
