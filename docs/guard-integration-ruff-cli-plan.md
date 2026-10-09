# Additive Ruff SDK CLI plan

Base: preserved mature PR31 lineage throughd70d075, reviewed Ruff SDK689a316.
Task2.3 acceptance registered only on this integrated branch:5/26. Original
adapter branch remains4/26. Existing native handlers/checks/formatters/rulepacks,
all original command entries and assertions remain intact.

Add a separate Unix-only `guard-project-ruff` command with exactly the existing
five file flags: --invocation, --native-report, --context, --mapping, --contract.
It consumes the reviewed one-file Ruff SDK; it does not invoke/install Ruff or
modify guard-project's run_report1.0 profile. No native command changes exit3.

New strict `codeguard.ruff-context/v1alpha1` freezes runId, RunBinding, target,
producerSha256, startedAt, finishedAt. Binding.sourceSnapshotDigest supplies the
source hash. Private RuffF401Policy derives exact scope from those plus its fixed
qualified tool/config/F401. New `codeguard.ruff-invocation/v1alpha1` records exact
packageVersion0.1.4/nativeSchema0.13.0, executable, argv, runId, processExit3,
producerSha256. Only actual canonical `lint python ROOT --format=json --file
TARGET --ruff-tool ABSOLUTE_TOOL` argv is supported; paths are metadata and never
executed. Report recheck_cwd must equal captured ROOT. Frozen producer/run/target
and source binding must agree before publication; controller provenance trust
remains explicit and is not replaced by self-declared JSON fields.

Read all policy/invocation inputs with shared bounded regular-file reader before
creating FrozenRun. Bad arguments/context/invocation/mapping/contract: stdout empty,
structured unbound stderr diagnostic and4. After binding, missing/malformed native
or failed projection: bound error/null envelope, structured stderr and4; only
validated matching native evidence can be attached. Completed narrow report maps
to common exits ALLOW0/BLOCK2/REQUIRE_APPROVAL3, preserving nativeExit3 and raw
feedback bytes in existing projection bundle. Reuse existing bounded serializer,
diagnostic and exit helpers without changing their behavior. Adapter cancellation
is null/cancelled4; output-write failure is bound diagnostic4. No native130 support
is invented for this strict selected-file0.13 capture profile.

TDD subprocess tests use real previously captured Ruff feedback for actual new
CLI0/2/3 and read/parse failure4, plus prebinding empty stdout, exact domain/engine
recomputation, producer/run/root mismatch, duplicate/unknown/oversize/symlink
inputs, help and stderr/write failure. Retain old CLI/binding/scope/allocation and
native command contract regressions; no assertion deletion or weakening. Add
schema/docs and explicit before/after record. No compatibility conflict identified:
new command and versions are additive. Task2.5 still requires independent review;
no checkboxes inferred from implementing this plan.
