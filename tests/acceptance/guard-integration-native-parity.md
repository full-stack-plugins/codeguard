# Command-aware native compatibility evidence

This slice freezes actual command results for adapter tasks 1.1/4.2. It changes no native implementation, existing test, native rule, formatter, command or report schema. Acceptance remains for an independent reviewer; no task is self-checked.

## Pinned inputs

- Mature PR31 preservation baseline: `96bae1381a84649420a74b7b6d772f3d992bf3d2`.
- Old Rust binary: `f5661d51dd1b2629f3b87b726fb09a9060ce7958`, SHA256 `b7576afa78f5ab6fdc8133650a1c75dc113357ac927e553b4f5c67731eea3a8c`. This retained binary already contains earlier adapter work; it is the pre-Ruff/native-consumer comparison point, **not an unmodified PR31 binary**. Original PR31 source preservation was separately reviewed.
- New Rust binary: `ff4efa9a2083ab1cbd5820970c996c2cce4b773a`, SHA256 `f54f299c79a6aa1ae923fe85fbe5aac3ce0e058cc35f875f7003ede6ff6108f0`. Built by the independent approval reviewer from a complete Git archive and GE `6527e2a67cb55690330c95dd12b496dd878ed39b`; retained as an immutable hardlink before later shared builds.
- Historical Python plugin: `9e4adb16f73d0d77d31e3b42ad08a5942503b2a3`, unchanged. This is an independent historical command family, not a claim that Python and Rust stdout are interchangeable.
- Real Ruff 0.16.8, official hadolint 2.12.0 and installed Node/npm. Full paths/hashes/versions are in [native-matrix.json](../fixtures/guard-integration/native-matrix.json). Hadolint's official upstream checksum matched `56de6d5e5ec427e17b74fa48d51271c7fc0d61244bf5c90e828aab8362d55010`; provenance/license record is retained in the cloud ledger `hadolint-tool-provenance.json`.

## Actual results

23 command captures, including ten old/new Rust pairs, plus three direct real Ruff captures (clean0/finding1/config-error2), were collected on Linux x86_64 on 2026-10-09. The manifest records exact argv, schema-bearing stdout, stderr hashes, exit and before/after file hashes. [native-captures/](../fixtures/guard-integration/native-captures/) retains raw bytes and generated fixture inputs.

| Command family | Real condition | Exit and meaning |
|---|---|---|
| Historical check | Actual Ruff F401 finding plus Elixir lacking required configuration | 2, FAIL takes precedence over UNVERIFIED |
| Historical CVE | Generated lodash 4.17.20 lock, actual npm audit high finding, missing pip-audit | 2, FAIL takes precedence over UNVERIFIED |
| Historical Dockerfile | Actual hadolint DL3007/DL3009 findings, missing trivy | 1, UNVERIFIED takes precedence over FAIL; findings remain |
| Rust format check | Actually formatted Python through Ruff | 0, formatting complete/allow, `authority=local_unverified` |
| Rust format check | Actually unformatted Python through Ruff | 1, formatting complete/deny; no source mutation |
| Rust version query | Actual version query | 0, query success without delivery decision |
| Rust lint | Unsupported argument | 2, usage error, stderr only |
| Rust lint | Actual clean/F401/missing Ruff/invalid Ruff config | 3 in all four cases; original local native findings and incompleteness remain distinct |
| Rust hook execute | Real directory FD as stdin gives read EISDIR | 4, input transport failure, empty stdout; no forged tool response |
| Rust lint | SIGINT after actual Ruff check exec observed | 130, cancelled/incomplete feedback, delivery not evaluated |

A formatter's native `allow` is limited to its original formatting command. It is not whole-project check approval or a GuardEngine qualification. No zero/one `run_report` was fabricated from the generic core Verdict enum. Query zero is separately tested. Historical exit2 findings, Rust exit2 usage, native exit3 and opt-in adapter exit3 are not aliased.

Cancellation uses an actual installed Ruff and generated Python, not a script emitting fake JSON. Raw process/signal strace is committed for both sides. A first `/proc/.../children` polling attempt could not observe children on this cloud kernel; the final harness observes real exec records instead. The traces show actual child termination and native exit130. This is local signal evidence, not certification of every host/platform cancellation path.

The CVE request transmitted only the generated public dependency list. Package-lock generation used `--package-lock-only --ignore-scripts --no-audit --no-fund`; no package scripts/install or user sources/credentials were sent. The subsequent **real** npm audit response is dated evidence, not a stable advisory database. Missing pip-audit/trivy are genuine missing tools, not simulated reports. Initial npm attempts without the environment proxy failed and are retained separately in the ledger, never counted as the positive capture.

## Exact comparison and side effects

All ten Rust pairs retain identical exits and byte-identical stderr. Parsed stdout is identical after removing only the fields actually changed in each pair:

- `run_id`: genuine per-invocation identity;
- `elapsed_ms`: actual formatter duration;
- `adapter_sha256`: genuine changed producer executable identity, present in the actual F401 record.

No finding, scope, config, tool, source digest, status, decision or diagnostic is normalized. Each comparison lists its exact variable fields; tests reject unexplained changes and reject normalizing fields that did not change. Raw stdout remains untouched.

Source/config/lock files stay unchanged. Historical check and native format check legitimately create `.ruff_cache`; these original native side effects are recorded, not described as zero I/O. Native pair captures start with empty fixture Ruff caches, and all changed paths must remain inside that known cache directory. No fix/apply/install command is introduced. Prior real check `--output` stdout/file parity and export-failure retention evidence is retained in cloud ledger `codeguard-ruff-independent-native/`, `codeguard-closure-independent-export/` and `codeguard-reader-shadow-independent-review.md`.

Native formatter/lint/hook implementations are byte-identical between the two Rust source pins. The main dispatch difference is the explicit `guard-project-ruff` arm; existing native arms are unchanged. Prior preservation bundles retain all original mature PR31 files, and this slice adds only compatibility evidence/tests. It does not claim a new 57-language/platform test pass.

## Verification and replay

`guard_integration_compat.rs` first failed three assertions because the old matrix had only source-backed declarations. After installing these actual captures, three maintained tests pass. A fourth explicit test independently executes real formatter0/1, query0, usage2 and native stdin fault4:

```sh
CARGO_INCREMENTAL=0 cargo test --locked --offline -p codeguard-cli --test guard_integration_compat
GUARD_PARITY_RUFF=/absolute/pinned/ruff cargo test --locked --offline -p codeguard-cli --test guard_integration_compat -- --ignored --exact actual_formatter_zero_one_query_usage_and_io_failure_keep_native_contracts
```

Cloud capture scripts are preserved alongside the fixtures. Their absolute paths identify this exact environment and retained binaries; outside this environment resolve the pinned tools/binaries explicitly before replay. Do not silently replay using whatever `codeguard` happens to be on PATH. The captured historical outputs can be verified offline without installed scanners or network.

Remaining boundaries: no external host authorization test (task4.4), Windows/macOS/WASM validation, production policy authority, general language completeness, current vulnerability guarantee, release or push. These are not inferred from this compatibility matrix.
