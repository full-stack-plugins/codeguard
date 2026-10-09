# Fixed local adapter compatibility

This is an explicit local source distribution and tested profile, not a public
release or activation. No semver negotiation or automatic N/N-1 fallback exists.
The original native workspace, all tracked native/grammar source and licenses,
and every Cargo manifest/lock are retained byte-for-byte.

## Exact matrix

| Layer | Fixed combination | Evidence and limit |
| --- | --- | --- |
| Adapter source | `594644d02b0de1a74f236340f9a0d301a0d7f58e`, package version 0.1.4 | Package version does not select report/wire version. Includes the new explicit matrix tests and previously accepted local trust/queue/host interfaces. |
| Engine | source `b0e4b445e1a20a9354d55ebb41bbedaf16ec8a74`; source tar SHA `6fa49f998a23803c4df4c47e9b7f55de32cb867e923364f711b6694980600d45` | A source bundle, **not** a released crate. Independently packaged adapter tests use exactly this source. |
| Native Ruff reader | native `lint_feedback` schema 0.13.0; one-file F401 profile `codeguard.ruff-f401/linux-x86_64/v1alpha1`; Ruff 0.16.8 | Original raw captures remain intact; complete only for exact target/config/source/tool scope. Original aggregate exit3 never becomes native success. Actual host pins below. |
| Legacy reader | package0.1.4, `codeguard.invocation/v1alpha1`, `run_report`1.0, lint `--format=json` | Unqualified/partial. The protocol fixture is not relabeled an actual native capture. Real `guard-project` CLI subprocess tests consume that fixture and retain BLOCK/null/error semantics. |
| Mapping | `codeguard.mapping/v1alpha1` | Existing exact F401/fixed-gap relations only. Unknown mapping version rejected by the actual parser. |
| Engine/wire | `guard.partme.ai/v1alpha1` contract; `guard.integration/v1alpha1` envelope | Independent namespaces; existing Ruff artifact tests recompute engine artifacts. No inference from crate semver or native schema. |
| Host | existing `codeguard-gg-candidate-host`0.1.0, optional local trusted-executor library | Task4.4 actual probe accepted with GG20f3a82 and GE68f3c1f. This source package separately includes GG20f3a82 and GEb0; its metadata resolves entirely inside the package. Host has two domain artifacts; SDK consumer has a separate single-domain contract. No MCP or deployment claim. |
| Platform/toolchain | Linux x86_64, Rust1.99.0 `b940084d7`, locked offline cached dependencies | Neither declared Rust1.85/1.90 minima nor other OS/architectures were executed. No all-platform claim. |

`guard_integration_version_matrix` checks native schema 0.13.0 positive and
0.12.0/0.14.0/0.13.1/1.0/unknown rejection, changed tool rejection or partial,
legacy package0.1.3/0.1.5 and schema/invocation mismatches, and unknown/adjacent
mapping versions. It calls existing readers and parsers; there is no new version
router. The old generic reader cannot qualify itself by claiming a newer schema.

Actual host tool inputs are external, controller-supplied executables, not bundled
runtime tools: preserved native CodeGuard `f5661d51dd1b2629f3b87b726fb09a9060ce7958`,
SHA `b7576afa78f5ab6fdc8133650a1c75dc113357ac927e553b4f5c67731eea3a8c`;
official Ruff0.16.8 SHA
`f3eb080f0173e1a5884fabcc2c836f5a0ebea35fccf252e718fc71b17f821ef7`.
The retained native tool/download catalogs are source, not proof that every listed
runtime executable is packaged or qualified. No57-language matrix rerun occurred.

## Source distribution

`integrations/local_package/prepare.py REVIEWED_GE_SOURCE_BUNDLE NEW_DIRECTORY`
uses fixed Git archives, never live worktree bytes. Layout:

```
NEW_DIRECTORY/
  codeguard/    # complete native + adapter source, unchanged manifests and locks
  guardengine/  # exact reviewed engine source archive
  gitguard/     # exact20f3a82 source, solely existing optional host dependency
  provenance.json
```

Only the archive's top-level engine directory is renamed. No source file,
manifest, lockfile, license, or old artifact is rewritten. Pre-existing output is
refused. Extraction rejects traversal, absolute paths, symlinks, special files,
noncanonical/duplicate names and excessive members/bytes. Engine SHA is fixed;
substitution of the older crate is rejected before extraction. Four standard
library Python tests cover safety rejection and exact file preservation.

Prepared bundle: `codeguard-local-source-594644d.tar.gz`, SHA256
`a7f2872c1029a12f1b5bbdd94c98e39800288129f5913b6e3781d3b70a9204bf`,
14,025,664 bytes. It contains source594644d, not the later packaging script/docs
commit. Provenance hashes the original input archives and all manifests/locks;
its `compatibility_verified:false` remains the immutable preparation fact. Later
review/test results are separate evidence, not an in-place authority upgrade.

Independent-path author verification: all3408 CG +577 GG +251 GE files compared
to their original fixed archives; both workspace metadata commands with
`--locked --offline --filter-platform x86_64-unknown-linux-gnu` succeed. Every local
package manifest/path dependency is under the bundle root; registry dependencies
use the existing Cargo cache. No external source sibling is needed. The registry
cache/toolchain and explicit native executables remain prerequisites.

From the packaged codeguard root, the targeted command was:

```sh
cargo test --locked --offline -p codeguard-cli \
  --test guard_integration_version_matrix \
  --test guard_integration_cli --test guard_integration_ruff
```

20 passed,0 failed,1 existing ignored actual-tool test. That ignored test is not
counted as an actual execution. Original workspace yielded the same counts; no
native behavior changed. Actual host execution has separate task4.4 evidence.
External logs: `codeguard-package-tests.log`, `codeguard-version-matrix-green.log`,
`codeguard-package-prepare-{red,green}.log`, metadata JSON/stderr and
`codeguard-package-source-comparison.json`.

Historical compatibility limits remain: earlier TG/GG package verification of
crate SHA `da5b30c22c28c9db444a0ca9c214b9441c3b7aebd14c223606298547452189bc`
lacked `validate_approval_record`; it is not interchangeable with this source
bundle and was not silently retried as the same artifact. Earlier all-platform
offline metadata hit an uncached `r-efi` dependency. This run explicitly limits
metadata to Linux, does not install missing packages and does not assert that
other-target resolution now works. Public distribution awaits applicable
GE-RELEASE gates; local compatibility acceptance does not deploy an issuer or
weaken required evidence checks.
