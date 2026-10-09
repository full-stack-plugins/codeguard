# Narrow Ruff F401 SDK profile

`codeguard.ruff-f401/linux-x86_64/v1alpha1` consumes the actual
`python_lint_feedback`0.13.0 produced by `lint python ROOT --file TARGET
--format=json --ruff-tool ABSOLUTE_TOOL`. Native stdout stays byte-for-byte intact;
native exit3, command_status=incomplete and delivery_decision=not_evaluated remain
in the domain artifact. This is one-file unused-import coverage, not whole Python
quality, the aggregate CLI gate, task closure, or production authorization.

The SDK profile is separate from guard-project's existing run_report1.0 reader.
That CLI still cannot reach ALLOW0/REQUIRE_APPROVAL3. Unknown feedback versions,
WASM variants, workspace-bound variants, discovered-project scans, multiple files,
unknown/duplicate fields, truncated input and reports above1MiB are rejected.
No native producer or old profile qualification flag is changed.

The protected caller freezes target, source SHA256 and producer binary SHA256
before consuming evidence. The target is one bounded relative Python path. The
profile fixes the official PyPI Linux x86_64 Ruff0.16.8 executable digest and exact
pyproject.toml bytes: py311 plus `select=["F401"]`, no inheritance/overrides.
`RuffF401Policy` fields are private; there is no caller-provided qualification flag.
The scope identifier hashes the full profile/tool/config/producer/target/source
requirements, so differently frozen policies cannot publish through the same
FrozenRun binding. Caller capture provenance is an explicit trust prerequisite;
this in-memory SDK does not authenticate the caller, producer or approved policy.

Complete requires matching observed target, source/config/tool digests, exact
Ruff version, locally complete native scan, F401-enabled settings, no per-file
ignores, and a complete paired ignore-noqa audit with zero suppressed diagnostics.
Native `coverage_proven=false` describes the original settings-only observation;
the SDK does not rewrite it. The narrow profile combines that observation with
fixed configuration and native suppression evidence. Missing proof remains
Partial/BLOCK. Source/config mutation, tool absence and disabled rules cannot be
rescued by an unrelated successful file or zero findings.

A protected projection mapping must have exactly two sources (Ruff F401 and this
profile's incomplete-scope marker), both mapping to the sole contract rule. Extra
contract rules are rejected: this profile cannot prove their coverage. Exact
relations use shared FactBudget and bounded evaluation. The existing sealed
Projection drives envelope coverage; generic caller data cannot mark it complete.
Clean/fixed narrow scope can yield ALLOW; F401 yields BLOCK under enforce or
REQUIRE_APPROVAL under review; incomplete scope always yields BLOCK. Approval
consumption remains a separate authenticated controller concern.

Real evidence and wheel/binary provenance are recorded in the implementation
ledger. Tests keep original captured reports, including producer run IDs and
raw bytes. The explicit ignored test `real_native_ruff_profile_controls` runs
nine real cases using an existing tool selected by CODEGUARD_RUFF_F401_TOOL. Set
CODEGUARD_RUFF_CAPTURE_DIR to a new explicit directory to retain raw native/Ruff
stdout/stderr, commands, source/config, and actual SDK contract/facts/report/domain/
envelope artifacts. Without that option it only uses and removes its temporary
fixture directory. Direct Ruff exits0/1/2 and spawn failure are separate from the
CodeGuard aggregate exit3 and are never relabeled.
