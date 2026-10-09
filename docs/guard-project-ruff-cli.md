# Explicit Ruff projection CLI

`guard-project-ruff` is an additional Unix command for the reviewed single-file
Ruff0.16.8/F401 profile. It only reads already captured evidence. All mature
CodeGuard native commands, formatters, rules and checks remain intact; the older
`guard-project` keeps its separate unqualified run_report1.0 profile.

```sh
codeguard guard-project-ruff \
  --invocation invocation.json --native-report feedback.json \
  --context context.json --mapping mapping.json --contract contract.json
```

All five flags are required once. Native, context, mapping and JSON-only contract
inputs are bounded at1MiB each; invocation at16KiB. The shared regular-file reader
rejects leaf symlinks/nonregular files; parent-directory trust remains the caller's
responsibility. No executable path in invocation metadata is executed, and there
is no native runner, installer, repository mutation or default artifact publication.

New context schema `codeguard.ruff-context/v1alpha1` includes runId, strict shared
RunBinding, target, producerSha256, startedAt and finishedAt. RunBinding requires
its nullable mergeGroupId and baselineDigest fields explicitly, including null.
sourceSnapshotDigest is the selected-file hash, not automatically a whole Git
candidate proof. Caller-protected context supplies frozen source/producer/target;
the SDK fixes the qualified tool/config/F401 scope. Invocation schema
`codeguard.ruff-invocation/v1alpha1` records packageVersion0.1.4, nativeSchema0.13.0,
executable, argv, runId, processExit3 and producerSha256. Only the exact captured
argv shape `lint python ROOT --format=json --file TARGET --ruff-tool ABSOLUTE_TOOL`
is accepted. Executable/root/tool paths must be absolute metadata, strings contain
no control characters, target/run/producer must match context, and feedback
recheck_cwd must match captured ROOT. This metadata is not authentication: a trusted
controller must capture it independently from the producer.

| Result | Process exit | Output |
| --- | --- | --- |
| Complete narrow ALLOW |0| Completed envelope and exact artifacts on stdout |
| BLOCK, including incomplete scope |2| Completed envelope and exact artifacts on stdout |
| Complete narrow REQUIRE_APPROVAL |3| Completed envelope and exact artifacts on stdout |
| Before binding failure |4| Empty stdout; structured unbound stderr diagnostic |
| After binding failure |4| Error/null envelope; structured bound stderr diagnostic |

The existing `codeguard.projection/v1alpha1` bundle is reused. nativeExit remains3;
raw CodeGuard feedback retains exit_code3/incomplete/not_evaluated. Engine decisions
apply only to the named F401 scope. Extra contract rules reject before binding.
Bad/unknown/duplicate input, invalid frozen binding or conflicting invocation never
emits a bound envelope. After binding, invalid/missing native input yields null
without attaching foreign bytes; projection budget failure retains only validated
matching raw evidence. Failed serialization falls back to bounded error output;
stdout write failure emits a bound diagnostic and exits4. Cancellation checks
produce cancelled/null4; no actual signal timing or native130 profile support is
claimed. REQUIRE_APPROVAL is not authenticated approval consumption.

Before/after: this command previously reached unknown-command handling. It now
provides real0/2/3/4 paths from captured qualified evidence. Old guard-project
stdout is byte-identical for its fixed input fixture. Actual native `check all
--format=json --output FILE` stays3, stdout equals FILE, and before/after reports
differ only in run IDs. Detailed captures and preservation audit are retained in
the implementation ledger; this is not a claim that every57-language combination
or every platform was executed. No old test/assertion was removed or weakened.
