# CodeGuard adapter implementation progress

Base: 003a904ddd908d575845dbe3aa42796866a7a5d0. Local slice; all OpenSpec task boxes remain unchecked pending review.

1. Format repair: preserve all 13 historical long requirement sentences under scenario detail; strict validate both changes. No historical task edits.
2. Baseline: run cargo test --workspace with isolated toolchain; preserve failures independently.
3. Tasks 1.3/1.4/2.1/2.2: test-first strict opt-in invocation profile, bounded native run_report reading with captured process/output binding, honest unqualified scope. Reuse mature native parser. Unknown commands and schema combinations reject.
4. Tasks 1.1/1.2: source-backed compatibility evidence and ADR for pure internal module with future explicit guard-project entry; no native dispatch changes.
5. Tasks 1.6/2.3: wait for shared engine API before engine mapping. No duplicate wire definitions, envelope, or fabricated complete qualification.

Tests will record RED/GREEN evidence in external slice report. Native-only parser/profile modules do not run tools or access disk. No production profile is qualified in this slice.

## Slice evidence

Format repair committed as 25da9ff; strict all-change validation passes 2/2, no diagnostics. Normative sentence preservation verified against base.

Baseline workspace run failed at `corpus_integrity::current_corpus_keeps_pending_samples_separate` because isolated checkout lacks sibling codeguard-plugin/scripts/codeguard/verdict.py. Existing deprecated fetch_update warning also observed. Continue unaffected work as authorized.

Reader RED initially failed on missing module, then behavioral RED failed on permissive profile, stale run acceptance, and empty obligations. GREEN passed four tests. Additional direct-Serde bypass regression failed before reader validation was added, then passed. Focused native compatibility run: 41 passed, three ignored (check_all_partial_contract includes existing ignored tool-dependent cases). Aggregate incomplete diagnostic check also passes (six adapter tests now).

Implemented substeps: task 1.1 source compatibility matrix (full captures pending); 1.2 pure read-only library boundary ADR (engine pin and CLI pending); 1.3 narrow strict invocation/schema (complete CapabilityProfile version matrix pending); 1.4 explicit unqualified fixture; 2.1 bounded run_report 1.0 lint reader with raw bytes/digest, process status and identity comparisons (export feedback unsupported); 2.2 frozen target gaps, mandatory unqualified partial outcome (rule/tool/config proof pending). No complete task claimed and no task boxes checked.

Final native regression: `cargo test --workspace --no-fail-fast` completed, 1433 passed / 2 failed / 126 ignored. Failing tests: corpus_integrity::current_corpus_keeps_pending_samples_separate (missing sibling fixture); python_syntax_probe::tests::version_probe_cannot_redirect_alias_before_syntax_invocation (expected tool-changed diagnostic, observed version-unverified). Earlier full run also saw transient lint_python_cli::native_probe_fixture_reaches_late_write_without_cancellation Text file busy, then passed exact rerun and final full run. These native paths are unchanged. Adapter suite: seven passed; capability declaration is explicitly unqualified with engine/mapping versions absent. Full suite is not green; no native profile qualification claimed.
Exact rerun of the Python alias-redirection probe passed (1/1); intermittent native failure remains documented, not fixed. Shared GE interface arrived at slice end and was inspected; direct SDK projection integration is the next reviewable slice, not part of this native reader commit.

## Slice 2 executable plan

- Restore actual official codeguard-plugin revision 9e4adb1 as sibling environment dependency and rerun corpus integrity; no fake fixture or native behavior changes.
- 1.6/2.3: test-first exact finding/gap-to-contract-rule mapping; reject missing/unknown/wildcard mappings; evaluate through shared GE and retain native domain bytes. All facts remain Partial.
- 1.5/2.4: test-first frozen binding wrapper using shared GE envelope validation; error/cancelled carry null decisions, completed partial decision equals verified engine report. No CLI/storage/trust claims.
- Use sibling guardengine path dependency for local development; record source revision and changed status, no release pin claim. Full workspace regression plus focused new tests; tasks remain unchecked pending review.

Slice2 evidence: official corpus restored at9e4adb16 without fixture fabrication; four corpus tests pass. Protected mapping now covers every finding/gap with exact existing engine rule relations; all facts Partial, all enforcement modes BLOCK/INDETERMINATE. Frozen binding rejects swapped targets even under same obligation ID; completed memory envelopes pass shared engine artifact verification, native4/130 preserve original bytes and null decisions. No CLI/storage/trust qualification implemented. Tests first exposed missing APIs, unknown mapping acceptance and target-set swap; focused regressions pass. Full default workspace after environment repair:1444passed/0failed/126ignored. Full log and transient-native investigation outside repo. Local siblingGE dependency is development-only, awaiting reviewed source pin/release.

A separate newly authorized PR31 repair will use another worktree at958a106636ad8430e40e45075e49c6eea1496629. This adapter branch remains preserved and is not merged intoPR31/main. Details: external codeguard-baseline-integration-strategy.md.

## Projection expansion review correction

Independent review found first engine evaluation could expand a valid487KBnative report into23MB. Behavioral regression failed before the fix, then passed after switching to shared GuardEngine evaluate_bounded (GE11e1d86). Adapter/boundary24and native-contract36checks pass,3ignored. This bounds engine evaluation before report allocation; source/profile qualification remains absent. A separate possible relation-string multiplication during projection construction is retained for review, not claimed covered by this engine-budget fix.

## Borrowed fact construction correction plan

Review P2 (e15c48a) leaves tasks 1.6/2.3 partial. Reproduce the review's 96 findings
and three 64 KiB contract relation fields with a real counting allocator, requiring
rejection before excess relation copies. Replace prebuilt owned sources and relation
clones with borrowed mapping lookup and shared GE FactBudget construction; retain
bounded evaluation and unchanged native/domain evidence behavior. Update the local
path dependency lock, run focused integration/native contract regressions and scoped
clippy, then record exact evidence and commit locally. No native qualification,
release pin, full-task acceptance, PR31 edits, or checkbox changes are implied.

Construction regression RED measured 18,874,368 bytes in 288 actual 64 KiB
allocations during project() (22,707-byte native report, 197,028-byte typed contract,
96 findings). With GE c80ec32 FactBudget source-parts API, GREEN measured 8,257,536
bytes (126 accepted copies), then rejected before excess copies. Mapping fields and
source fragments remain borrowed until the shared budget accepts them; no owned
source staging vector remains. Native/scope references retain their exact strings.
Focused integration, crate-boundary, run-report, legacy and check-all contracts:
61 passed, 3 existing pinned-Ruff-dependent tests ignored. Full native workspace
was not rerun for this focused correction; earlier native failures remain recorded.
Scoped clippy (`--no-deps --lib --test guard_integration_projection -- -D warnings
-A deprecated`) passed. Unsuppressed strict clippy was blocked by the preexisting
runtime `process_runner.rs:116` unnecessary u64 cast; existing CLI fetch_update
deprecation is explicitly allowed only for scoped verification, with no native edit.

## Scope admission correction before CLI transport

Independent review accepted cf84913's relation-copy boundary but found another P2:
a 256 KiB obligation ID and 64 short targets expand into >16 MiB gap strings in
assess_scope. Before tasks 2.4/2.5 CLI work, tighten FrozenObligations construction
with borrowed worst-case encoded gap accounting before validation collections or
formatting. Preserve all-or-error required scope and normal partial semantics;
prove actual allocation rejection with the independent fixture. Task 2.2 remains
partial and unchecked pending review. No CLI/native/PR31 changes in this correction.
Scope admission RED measured 50,596,694 cumulative allocated bytes in allocations
at least 256 KiB during constructor + assessment; GREEN measured zero such copies,
with constructor error before assessment. Additional escaping/small-scope controls
pass. Focused adapter/native contracts: 63 passed, 3 existing ignored; scoped clippy
with the previously documented no-deps/deprecated exclusions passed. Evidence:
external codeguard-scope-budget-{red,green,regression,clippy}.log. Caller-owned
input allocations and exact RSS are outside this admission policy guarantee.

## Qualification and boundary evidence slice

Prioritize task1.4: explicit zero-findings, WASM-only candidate, and resolved-task
negative controls; all registered profiles remain unqualified and successful
projection remains Partial/BLOCK. Expand qualification fixture with source-backed
legacy-task requirements and actual adapter-only evidence; no native-task status
changes. Task1.2: correct ADR to actual SDK dependency and reviewed GE revision,
record selected byte-only library entry and future CLI arguments, exercise repeated
projection and unselected native query under Linux syscall tracing. Release/pin
limitations must remain explicit. Tasks2.4/2.5 CLI are deferred for this small slice.
Root authorized task1.3 acceptance after independent review; register only that box.
Qualification controls passed for zero findings/exit0, WASM inventory and resolved
messages: each accepted report remains Partial/BLOCK; unsupported proof kinds and
non-run-report task previews reject. Fixture-completeness test first failed on the
missing explicit unverified status, then passed after source-backed legacy task
requirements were added. No runtime qualification behavior was changed.
Boundary test repeats ten byte-only SDK projections with identical output. Explicit
Linux strace run passed, recording no file/process/network syscalls in the marked
projection interval and no adapter/project access in native --version; sentinels
unchanged. Default focused suite: 69 passed, 4 ignored (three pinned Ruff cases and
the separately executed syscall test). Scoped clippy passed under documented
existing exclusions. Added explicit fixture export example and a native-only
profile rejection test for engine-backed consumers. Only independently accepted
1.3 is checked; 1.2/1.4 await review, and 2.1/2.2/CLI gaps remain unchanged.
