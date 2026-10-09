# CodeGuard / GitGuard candidate host (opt-in)

This is a separate, unpublished Rust1.90 workspace. The mature CodeGuard four-crate workspace remains Rust1.85 with its existing dependency-direction tests, lockfile, CLI and 57-language native surface unchanged. GitGuard requires Rust1.90 and must not become a reverse dependency of those native crates. The host lock selects libc0.2.189; GitGuard's own lock remains0.2.177 under its reviewed bounded compatibility declaration. Other host dependencies follow this separate lock; it is not a replacement for CodeGuard's native lock.

This implements a **local trusted-executor** candidate consumer for the already qualified one-file Ruff F401 profile. It is not an external authentication provider, complete CodeGuard gate, deployment, or write authorization. No CLI is installed or implicitly enabled. The caller/controller supplies a protected, independently fixed `gitguard.candidate/v1alpha2` snapshot, repository root, target and binary/tool pins.

## Flow

1. `PreparedCandidate::from_bytes(snapshot, root, target)` bounds the input, discovers the real local Git repository, validates full immutable candidate/base/member objects and GG source/tree digest, and requires clean index/worktree. It reads regular files from GG's verified private immutable object database. The selected Python source must be in frozen scope; committed pyproject.toml must exactly match the supported fixed Ruff configuration. Symlink/submodule trees are unsupported by this GG reader. Snapshot bytes are at most1MiB; selected source at most1MiB.
2. `capture(&ProducerPin, &AtomicBool)` checks actual executable bytes against the controller's CodeGuard SHA256 and fixed official Ruff SHA256. Files must be regular absolute paths, at most256MiB; hashing streams in64KiB chunks. It copies only the immutable selected file and exact configuration into a private temporary project and runs the original native CodeGuard command through its existing runtime `ProcessSpec`: literal argv, cleared environment, closed stdin,60-second shared deadline (including binary checks),1MiB combined stdout/stderr limit. The private capture type can only be created by this actual run; uploaded report/digest data cannot construct it. Native exit3 and raw stdout are preserved. Clean source binding is checked before and after invocation.
3. `CandidateCapture::project(current_expected_snapshot, mapping, contract)` compares the separately supplied current controller snapshot with the full captured GG identity, including repo/task/worktree/requirements/candidate/base/group/ordered members/policy/allowed scope/tree digest. It revalidates current cleanliness, then calls the existing Ruff reader/projection and FrozenRun machinery. Old PR-head or reordered queue evidence cannot be reused as M. Contract evaluation remains the SDK's narrow F401 profile; it does not generalize zero findings to other quality rules.

The resulting `CandidateProjection` retains the original CodeGuard **single-file source digest** in facts and envelope binding. It adds a separate `candidate.json` domain artifact reference to the host envelope, with complete GG snapshot/tree digest, invocation, native raw digest, and scoped file/config/tool provenance. The native domain bytes are unchanged. These are distinct digest domains; no invented equal common hash is used. Persist `envelope()`, contract/facts/report/domain bytes and `provenance_bytes()` together. The provenance artifact is hash-bound to the envelope, but its local caller identity is not authenticated by that hash.

An ALLOW is local technical observation only. This host never issues a grant, calls Git mutation commands, changes source refs/index/worktree, reads write credentials, or claims an external approved identity. Repository IDs are protected controller labels, not authenticated filesystem identities. Controller pins do not prove who signed a binary. This profile assumes owner-controlled, stable executable and repository filesystem ancestry; hostile same-UID mutation/TOCTOU is not solved. There is no network service or host plugin installation. Temporary project writes and existing GG private object writes are expected and isolated from the source repository.

## Verification

```
cargo test --manifest-path integrations/gg-candidate-host/Cargo.toml --locked --offline
```

Real native tests are explicitly ignored by default and must be deliberately enabled with all controller pins:

```
CODEGUARD_CANDIDATE_PRODUCER=/absolute/codeguard \
CODEGUARD_CANDIDATE_PRODUCER_SHA256=<protected-sha256> \
CODEGUARD_CANDIDATE_RUFF=/absolute/ruff \
cargo test --manifest-path integrations/gg-candidate-host/Cargo.toml --locked --offline -- --include-ignored
```

Optional `CODEGUARD_CANDIDATE_CAPTURE_DIR=/absolute/new-directory` exports raw native stdout, fixed source/config, exact expected GG snapshot, common artifacts, provenance and actual Git bundles. The directory must exist; case subdirectories must not already exist. No evidence output path is used by default. Fixtures are local identities, not production controller credentials.

The tests exercise actual SHA1/SHA256 objects, real native clean/bad/review results, HEAD≠queueM, wrong repository, staged/working dirty content, missing objects, committed disabled config, budget/pin rejection and exact pre/post source file states. Task3.1/4.1 acceptance remains subject to independent review and scoped to this local Ruff host profile.

Reviewed dependency/profile verification pins for this slice: GitGuard
`e2a20e208ec660fd694fe4949ffd3c270cf6838b`, GuardEngine
`6527e2a67cb55690330c95dd12b496dd878ed39b`. Real test producer was the preserved
CodeGuard `f5661d51dd1b2629f3b87b726fb09a9060ce7958` binary, SHA256
`b7576afa78f5ab6fdc8133650a1c75dc113357ac927e553b4f5c67731eea3a8c`.
Relative path dependencies require the controller to select matching local
checkouts; this unpublished host is not a signed/released distribution.
