# CodeGuard local adapter final acceptance index

This reconciles original task5.3, all12 requirements and their24 scenarios at
implemented source `9337cf2b8b8163c29abd7130c85529ec19bc0881`. Tasks1–5.2 have
independent local acceptance (25/26). Task5.3 remains unchecked until this final
matrix/index is independently reviewed. This document changes no native behavior.

The delivered scope is the explicitly selected local SDK/CLI adapter, one-file
Linux Ruff F401 qualification, controller-provided authenticated-reference ports,
local bounded artifact store/history, exact GG candidate host, and fixed local
source distribution. The generic run_report1.0 reader remains unqualified; the
whole check_feedback aggregate remains unsupported. Technical ALLOW never grants
merge/write authority. Old native task ownership and unverified languages/rules
remain unchanged.

## Exact scenario matrix

IDs follow the two original spec files in their original order. Full unchanged
scenario titles, spec paths, exact test functions/files and source SHA256 values
are recorded in [the machine-readable index](guard-integration-release-index.json).
Test target shorthand below means `crates/codeguard-cli/tests/guard_integration_*.rs`;
`host` means the separate optional host's `tests/candidate_binding.rs`. Historical
actual tool execution is not relabeled as a fresh run in this final reconciliation.

| ID | Original scenario | Concrete check / evidence |
| --- | --- | --- |
| R01S1 | Adapter is disabled | `compat::real_streams_and_source_side_effects_are_digest_bound`; original23 captures; current rollback actual ordinary native entry before/after unchanged. E1,E12. |
| R01S2 | Legacy exit semantics differ | `compat::native_exit_families_and_exact_explained_differences_stay_separate`; command-aware legacy CVE/Docker/check versus Rust0/1/2/3/4/130, not numeric engine aliases. E1. |
| R02S1 | Unknown schema or duplicate key arrives | `reader::rejects_stale_run_exit_conflict_unknown_schema_and_truncation` plus reader_matrix duplicate/budget controls; new version_matrix refuses adjacent/unknown versions. E2,E11. |
| R02S2 | Export failed with retained findings | `reader_matrix::actual_export_failure_keeps_native_f401_and_original_digest_but_aggregate_is_unsupported`;9406 original bytes and fresh independent export-failed capture, user file retained. No stale export substitution. E2. |
| R03S1 | Aggregate feedback remains incomplete | `cli::native_aggregate_three_remains_partial_block_not_approval`; normal native3 does not confer completeness/review authority. E3. |
| R03S2 | Narrow profile gains qualification | `ruff::genuine_bad_fixed_and_clean_have_only_narrow_f401_coverage`; true fixed-tool positive/negative native observations, frozen source/config/rule/tool/target. No other native scope upgraded. E2,E4. |
| R04S1 | A finding has no supported mapping | `relations::each_registered_finding_and_gap_has_positive_and_missing_mapping_counterexample`; every registered finding/gap has deletion/wrong-rule negatives. E2. |
| R04S2 | Partial facts are evaluable | `projection::partial_projection_uses_engine_for_every_enforcement_and_keeps_attachment`; partial yields real engine BLOCK/INDETERMINATE even under advise, original domain retained. E2,E4. |
| R05S1 | Native tool crashes after producing findings | `binding::bound_native_failure_and_cancellation_keep_null_decision_and_findings`; bound error preserves native bytes, null decision; real CLI after-binding errors return4. E3. |
| R05S2 | Native cancellation is projected | `cli::bound_native_errors_and_cancellation_preserve_original_exit_and_null_decision`; native130 retained, adapter cancelled/null4. Historical actual process-group cancellation remains separately captured; projection tests do not claim a fresh signal timing run. E1,E3. |
| R06S1 | Candidate is unresolved | `host::typed_prebinding_transport_rejects_all_four_failure_classes_without_envelopes`; actual GG roots/objects and candidate/base/scope errors, static private diagnostics, no fabricated binding. E5. |
| R06S2 | Engine requires review | `approval::approval_attachment_is_a_separate_borrowed_envelope`; review stays review, approval attachment does not rewrite original report/envelope. Actual CLI review3 is separate from original native3. E3,E6. |
| R07S1 | Approval is forged locally | `trust::baseline_queries_are_fresh_and_cannot_be_replaced_by_text_or_rehashed_issuer`; nonnull baseline needs independently frozen reference/purpose/policy plus GE authority port, not accepted=true/hash/issuer text. E7. |
| R07S2 | Approval service is unavailable | Same trust test includes provider-unavailable, expiry, revocation and wrong identity/scope. Technical results retained, no eligible result. E7. |
| R08S1 | Analyzer or baseline changes | `freshness::each_frozen_binding_mapping_run_domain_contract_analyzer_and_scope_change_invalidates`; all relevant frozen dimensions checked, not onlyHEAD. E6,E7. |
| R08S2 | Approval is revoked after evaluation | `freshness::every_call_requeries_authority_and_preserves_original_report`; prepared baseline and actual rollback re-enable query current authority again. E6,E7,E12. |
| R09S1 | Two requirements share extraction | `concurrency::two_tasks_two_worktrees_and_two_requirements_keep_attempts_separate`; isolated obligations/current tickets despite matching extraction; actual A/B barrier composition retains independent records. E8. |
| R09S2 | Old success arrives late | `concurrency::late_head1_success_cannot_overwrite_current_head2_block`; actual M1 delayed append remains history after M2 current. E8. |
| R10S1 | Queue candidate differs from PR head | `host::actual_queue_m_not_pr_head_and_changed_full_identity_rejects_old_evidence`; real native capture of exact syntheticM in SHA1/SHA256, not stitched PR reports. Historical native run retained. E5,E8. |
| R10S2 | Queue is regrouped | `joint_queue::real_queue_parallel_consumption_rechecks_baseline_and_late_current`; real bundle objects, M2 base/member order differs, old/wrong bindings rejected. Four original native captures are replayed unchanged; supplemental baseline projection/provider explicitly fixture. E8. |
| R11S1 | Input contains a malicious path | `storage::required_artifact_uris_are_never_paths_and_receipts_are_not_trusted_size_claims`; symlink/budget/quota/collision/retention controls, unchanged memory findings. E9. |
| R11S2 | Projection is retried without output request | `side_effects::repeated_consumption_leaves_actual_source_events_refs_and_index_unchanged`; prior actual source/event/ref/index and process/network trace, ten pure calls; redaction tests remove secrets/instructions. No implicit native/install/repair. E9. |
| R12S1 | Selected native slice is unfinished | `scope::zero_findings_and_native_exit_zero_cannot_qualify_complete`; unqualified/partial and original native-task gaps stay open; qualified F401 does not generalize. E2,E4. |
| R12S2 | Adapter is rolled back | `rollback::actual_default_shadow_optin_disable_preserves_native_and_required_policy`; current actual CLI/SDK same bytes, None refuses required evidence, retained files unchanged, re-enable revoked rejects; version_matrix rejects unknown/N-1. E10–E12. |

## Fixed evidence and actual commands

Independent reports, plus four recent verification JSONs and the fixed source
artifact, have exact SHA256 entries in the index (22 external ledger files).
References are explicit retained ledger basenames, not claims that a public URL
or release exists. Previous reports describe their historical pending tasks;
later accepted slices close those named local ports without rewriting old reports.
Counts below refer to each recorded command and must not be summed as unique
coverage: suites reuse fixture tests and actual capture sets.

| Evidence | Fixed source / executed scope / retained result |
| --- | --- |
| E1 | `25bac257d4c7e8767c391402833468fdeb5d83c4`, `codeguard-native-parity-independent-review.md`; actual23 native/legacy command captures and command-aware parity. Current `--test guard_integration_compat`:3passed,1optional recapture ignored. The ignored case is not fresh execution evidence. |
| E2 | `9202c9107103570f330558187495caf1cee5c77f`, `codeguard-reader-shadow-independent-review.md`;47 focused reader/scope/relation/binding/CLI/shadow tests0ignored plus independent exact-byte probe and fresh old pinned binary export-failure reproduction. |
| E3 | `72d80ae453c0380ad7061c10847595c5a4ac1736`, `codeguard-ruff-cli-independent-review.md`, plus earlier `codeguard-cli-independent-review.md`;92passed/4existingignored and27 actual CLI projections, typed0/2/3/4, actual unchanged nativecheck3. |
| E4 | `689a316cf87464d9ec267656e0a673a9516b45b0`, `codeguard-ruff-independent-review.md`;actual nine-case native Ruff qualification, explicit narrow policy and engine recomputation. Other native contracts unsupported. |
| E5 | `codeguard-candidate-host-independent-review.md` and `codeguard-host-transport-independent-review.md`; actual immutable Git candidate/native host and `ff96c179e0312c76a3ef127ac6fb0400a11a2557` typed prebinding11tests including independent probe. |
| E6 | `ff4efa9a2083ab1cbd5820970c996c2cce4b773a`, `codeguard-approval-attachment-independent-review.md`; immutable bounded attachment and fresh eligibility. This historical review did not yet accept baseline task3.2; E7 does. |
| E7 | `0d6e0ec49def46306f0d33ee074f9221415a46d5`, `codeguard-baseline-trust-independent-review.md`; `--test guard_integration_trust --test guard_integration_approval --test guard_integration_freshness`:19passed/0ignored on fixed GEb0. Generic caller-supplied authority port, not a deployed issuer. |
| E8 | `0514d298011ba9be1b85c1e5a15f1bd105381d6c` attempt CAS, then `a3dadb5332f56f2e942aab0c29c351200d7720b8` joint; `codeguard-attempt-cas-independent-review.md`, `codeguard-joint-independent-review.md`; fixed joint+concurrency18passed,38hashes/37original byte pairs. Current A/B/M1/M2 consumption uses real original captures and explicit fixture authority, no new native scan. |
| E9 | `f458ba31f5d2af7c2e017174e7202806b99e06dc` store and `e15f1b95052e8a2b1f9bfb8658bed65dd5c401dc` retention; `codeguard-store-independent-review.md`, `codeguard-retention-independent-review.md`, `codeguard-consumer-independent-review.md`. Local owner-controlled Linux/store-mediated access, retention and pure-consumption trace. Not hostile same-UID containment, durable CAS or power-loss proof. |
| E10 | `e04cc1b9b6ce8ff845b7c2f72c07d3d8d2958f79`, `codeguard-host-independent-review.md`; explicit exact ignored host probe1passed/0ignored, two actual nativeexit3 calls, pins and21artifacts verified, pre-cancel/reentry/env/source refs/index. Older GE68 run retained separately. |
| E11 | Packaging `7e2ef43348bd56c2b70c05fd68e61db9b7501c73`, package source594644d+GEb0+GG20f; `codeguard-package-independent-review.md`. Package tests20passed/1existingignored; root+optionalhost Linux metadata, all source bytes/locks/licenses and archive hashes checked. Independent package-host exact probe1passed/0ignored against GEb0 supplies actual later host compatibility. |
| E12 | `9337cf2b8b8163c29abd7130c85529ec19bc0881`, `codeguard-rollback-independent-review.md`; two new actual rollback tests2passed/0ignored;3413CG and18GE source files/9capture artifacts verified. Author related15passed/1existingignored, then final6passed capture. Actual CLI over retained raw reports, not new native scans. |

Commands are reproducible per the linked task-specific acceptance documents and
retained reports. Ordinary tests use `source /workspace/guard-toolchain/env.sh`,
`CARGO_INCREMENTAL=0`, jobs1, `cargo test --locked --offline -p codeguard-cli` with
the named `--test` targets. Use only an assigned existing target. The optional
host uses its separate manifest and explicit native/Ruff pins with `--ignored
--exact`; pure-consumption strace/native recapture tests also need their declared
inputs. Do not count their default skip as a pass. This final documentation task
runs the index verifier only, not the full native or adapter matrix again.

## Artifact and preservation boundaries

Fixed source artifact `codeguard-local-source-594644d.tar.gz` SHA256
`a7f2872c1029a12f1b5bbdd94c98e39800288129f5913b6e3781d3b70a9204bf` contains complete
CG594644d+GEb0 source bundle+GG20f for the optional host; every manifest/lock is
unchanged. It does **not** contain the later9337cf2 required-consumption API or its
later documentation. This final acceptance does not silently repackage/replace it.
Toolchain/cache and fixed native executables remain external controlled inputs,
not allegedly included runtime binaries. Full pins are in compatibility.md and
index.package; the older incompatible GE crate is not substituted for GEb0.

The recent baseline32b8444→implemented9337cf2 diff contains exactly58 changed
paths. The index includes every path, not just a count; all are within the adapter,
its tests/docs/fixtures and optional integration directories. No mature native
source/dispatch, root/crate Cargo manifest/lock, original native tasks, or57-language
surface changed in this interval. Final task registration/index docs are additional
explicit documentation commits, not included in the frozen58-path assertion.

This local acceptance does not claim a public release, production issuer/service,
unknown platform, declared MSRV validation, unknown or default N/N-1 support,
all57-language runtime qualification, arbitrary native scope, native sandbox,
Git write authority or generic SDK acceptance of the host's two-domain artifacts.
Older native tasks remain owned by their original change and are not closed here.
Historical all-platform offline `r-efi` and older GE crate API incompatibilities
remain recorded in compatibility.md; no install or full-suite workaround occurred.

## Read-only machine verification

Run from this repository with the retained ledger available. Missing artifacts,
changed refs/bytes or missing scenarios fail instead of being silently skipped:

```sh
python3 - <<'PY'
import hashlib, json, pathlib, re, subprocess
index = json.loads(pathlib.Path('tests/acceptance/guard-integration-release-index.json').read_text())
pin = index['implemented_source_pin']
def git(*args): return subprocess.check_output(['git', *args])
def sha(raw): return hashlib.sha256(raw).hexdigest()
for path, expected in index['source_file_sha256'].items():
    assert sha(git('show', pin + ':' + path)) == expected, path
ledger = pathlib.Path('/workspace/guard-implementation-ledger')
for path, expected in index['external_ledger_sha256'].items():
    assert sha((ledger / path).read_bytes()) == expected, path
actual = set()
for requirement in index['requirements']:
    text = git('show', pin + ':' + requirement['spec_path']).decode()
    section = text.split('### Requirement: ' + requirement['requirement'] + '\n')[1].split('### Requirement: ')[0]
    expected = set(re.findall(r'^#### Scenario: (.+)$', section, re.M))
    assert expected == {s['scenario'] for s in requirement['scenarios']}
    for scenario in requirement['scenarios']:
        test = scenario['primary_test']
        assert ('fn ' + test['function'] + '(').encode() in git('show', pin + ':' + test['file'])
        actual.add(scenario['id'])
assert len(index['requirements']) == 12 and len(actual) == 24
preserve = index['recent_preservation']
paths = git('diff', '--name-only', preserve['baseline'], preserve['through']).decode().splitlines()
assert paths == preserve['changed_paths'] and len(paths) == 58
assert all(p.startswith(tuple(preserve['allowed_prefixes'])) for p in paths)
print('verified: 12 requirements, 24 scenarios, source/ledger hashes, 58 scoped changes')
PY
```

Hashes bind the evidence being reviewed; they do not authenticate an issuer or
replace substantive independent review. No new false-ALLOW finding is known in
the accepted local slices; untested native/platform scopes stay unqualified.
