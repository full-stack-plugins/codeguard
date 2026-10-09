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
