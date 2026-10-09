# Explicit GuardEngine projection CLI

On Unix builds, opt in with all five inputs:

```
codeguard guard-project --invocation invocation.json --native-report native.json \
  --context context.json --mapping mapping.json --contract contract.json
```

This command consumes existing artifacts through the SDK. It does not run a native
checker, discover a repository, install/repair tools, modify source/Git/.codeguard,
contact a provider, or publish files. Caller input paths and parent directories must
be protected by the caller; leaf symlinks, special files and oversized files are
rejected through the existing runtime bounded reader. This is not a hostile-parent
filesystem sandbox. Windows support is unavailable for this command; the byte-only
library remains separate.

Invocation is the existing narrow codeguard.invocation/v1alpha1 profile. Native
input remains run_report1.0/lint/JSON/package0.1.4; feedback/export-failure formats
remain unsupported. Mapping uses the existing exact protected mapping schema.
Contract input is **JSON**, a subset of the native loader's YAML language. The new
entry never sends arbitrary YAML/aliases to the native loader. Native commands and
native YAML loading are unchanged.

Context uses the closed codeguard.context/v1alpha1 schema:

```json
{
  "version": "codeguard.context/v1alpha1",
  "runId": "native-001",
  "binding": {
    "repoId": "repo", "taskId": "task", "worktreeId": "worktree",
    "requirementIds": ["req"],
    "candidateOid": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "baseOid": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    "mergeGroupId": null,
    "sourceSnapshotDigest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "baselineDigest": null
  },
  "requiredTargets": {"python/app/lint/ruff": ["src/main.py"]},
  "startedAt": "2026-10-09T00:00:00Z",
  "finishedAt": "2026-10-09T00:01:00Z"
}
```

The values above are synthetic examples, not real Git/provenance proof. Binding is
structural: this command cannot authenticate a caller, resolve an unknown repository,
prove actual candidate/base objects, or qualify rule/tool/config coverage. Protected
policy and real source identity remain caller/controller responsibilities. Missing
or malformed mandatory binding and unfrozen/oversized targets reject. Profile and
context run IDs must agree. Every current profile remains unqualified.

Invocation is capped at16KiB; each context, mapping, contract and native input at1MiB.
Duplicate JSON keys and unknown version/fields reject. Shared scope/fact/evaluation
budgets remain in force. The complete serialized inline bundle, including escaped
artifact text and newline, is capped at16MiB before any stdout write. This bounds
transport admission, not exact process RSS or caller-owned input allocations.

## Output and exits

| Condition | stdout | stderr | exit |
| --- | --- | --- | ---: |
| Bad arguments, context, invocation or unreadable/invalid protected policy before binding | empty | GuardTransportDiagnostic, phase=unbound | 4 |
| Valid completed projection, including native0 or aggregate3 | GuardProjectionBundle, completed/partial/BLOCK | empty | 2 |
| Bound native4 | bundle with error/null and original native bytes | bound diagnostic | 4 |
| Bound native130 | bundle with cancelled/null and original native130 | bound diagnostic | 4 |
| Bound read/parse/stale/mapping/budget/serialization failure | bundle with error/null and no successful report | bound diagnostic | 4 |

Bundle version is codeguard.projection/v1alpha1. Its envelope is the unchanged
GuardRunEnvelope; inline `artifacts.contract/facts/report/domain` are nullable UTF-8
strings containing exact artifact bytes. The contract is the projection's serialized
JSON contract, not the original file's whitespace. Native bytes remain exact.
`nativeExit` preserves the captured invocation process exit; it is not the new CLI
exit. Matching valid native input is retained on projection/serialization errors;
malformed or foreign-run/snapshot bytes are not represented as bound domain evidence.

A bounded serialization failure produces a smaller bound error/null bundle before
stdout. An actual stdout write failure returns4 and a bound stderr diagnostic;
already written pipe bytes cannot be retracted and must not be treated as a complete
result. A received SIGINT flag is checked before transport emission and becomes
cancelled/null; preservation of captured native130 is exercised by the CLI tests.

The generic exit mapper defines ALLOW0 and REQUIRE_APPROVAL3 for future qualified
completed evidence, with unit tests only. Those decisions are **not reachable from
the currently registered unqualified profile**. Native aggregate3 is incomplete
native execution, never approval. A native-only envelope cannot satisfy an
engine-backed consumer.

Schemas: guard-integration-context.schema.json,
guard-integration-projection-bundle.schema.json and
guard-integration-transport-error.schema.json under schemas/. The engine envelope
schema reference uses its existing canonical identifier; offline validators should
resolve it to their pinned GuardEngine schema rather than fetching mutable content.
Runtime validation uses the linked Rust types and validators, with no schema network
lookup.

## Compatibility evidence

Before this slice, guard-project was unknown, returned native usage2 and plain
stderr; six new CLI tests failed for that missing entry. After wiring, explicit
completed calls return2 with recomputable partial bundles, and new errors return4
with the appropriate binding phase. The baseline and post-change native comparison
both execute actual `check all --format=json --output FILE`: exit3, identical JSON
on stdout and FILE, no envelope fields. Existing native dispatch arms are unchanged;
only the explicit guard-project arm and help descriptor were added. No `--output`
is accepted by the new entry and no implicit output directory is created.
