# Frozen scope dimensions

The SDK's `FrozenObligations::with_requirements` accepts a protected caller's
per-obligation `RequiredScope`: exact targets, native rule IDs, tool IDs with
versions and SHA-256 digests, and configuration references with SHA-256 digests.
All dimensions must be nonempty. Duplicates, blank identities and noncanonical
SHA-256 values are rejected. The constructor checks the combined worst-case
escaped diagnostic/copy budget before cloning target identities or creating gaps.
Accepted requirements are private and cannot be changed after construction.

The existing `FrozenObligations::new` remains the explicitly weaker targets-only
SDK surface used by `codeguard.context/v1alpha1`. It does not acquire richer
requirements implicitly. Both surfaces remain unqualified; neither can create
complete scope. No native command, native schema or current CLI context changes.

The supported `run_report1.0` schema reports target coverage and global tool
identities. The adapter compares required tool ID, version and binary hash
exactly, but also records absent per-obligation tool execution coverage. It has
no observed executed-rule or configuration coverage: these requirements always
produce explicit missing-evidence gaps, even if a finding has the same rule ID
or the policy digest equals a configuration digest. Extra unrelated successes
cannot discharge required identities. Gaps are domain evidence and must be
covered by the protected projection mapping or projection returns an error.

This API makes richer scope requirements representable and preserves their
failure semantics. It does not finish task2.2 or qualify an actual producer.
A future strict native profile must supply independently verified per-obligation
observations and real positive/negative/fault/missing-coverage acceptance before
any complete scope or CLI ALLOW/REQUIRE_APPROVAL positive is reachable. A richer
CLI context requires its own version and compatibility tests.
