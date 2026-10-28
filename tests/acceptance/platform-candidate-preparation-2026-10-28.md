# Three-platform candidate evidence preparation

This preparation reuses `introduce-rust-codeguard-cli` tasks **11.1, 11.8,
11.10, 11.12, 11.13 and 13.4**. It does not replace the 57-language compatibility
inventory or mark those tasks complete. The selected scope is Windows x64,
macOS arm64, Linux x64 and Codex; separate operating-system and real-host evidence
is required. Existing task definitions remain the acceptance source.

## Current boundary

- Source baseline: `b499f13922647d4bf1e2344eaa5c37c450e302b5`; CLI version 0.1.4.
- The public npm package and plugin runtime lock cover macOS arm64 only. The
  public package is not the current source binary.
- `scripts/pack-npm-local.mjs` can make private native Linux/macOS packages.
  `--require-wasm` checks all 32 pinned assets and licenses and executes a real
  Zig worker probe. These remain unqualified candidates, not 57 native checks.
- Windows is not in that packer's host map. More importantly,
  `crates/codeguard-runtime/src/process_runner.rs` returns `UnsupportedPlatform`
  on non-Unix systems. Merely adding a packaging target would not provide the
  Job Object, process-tree cancellation and native-tool guarantees of task 11.12.
- The plugin's current lock and runtime validation require macOS arm64; Linux
  private tarballs cannot silently replace that lock. A raw CLI smoke or an
  npm package smoke does not prove the default plugin or Codex loads that binary.

## Prepared CI, not executed acceptance

`.github/workflows/platform-candidates.yml` is a candidate workflow using
Ubuntu 22.04 x64, macOS 14 arm64 and Windows Server 2022 x64 native runners. It
builds the WASM-feature candidate, runs the existing version CLI tests, verifies
the actual target and build SHA, and retains binary identity metadata. It also
requires the portable native-process safety tests on every target; the current
Windows runtime cannot satisfy them. UnsupportedPlatform is not a passing or
ignored test result. Unix
jobs additionally run the existing private packer after the process tests. The
Windows job does not attempt npm packaging or certify functional runtime
support. A build or process-contract failure remains a failure, not a skipped
success. No workflow step publishes a package, release or tag.

Runner labels and architecture were checked against the
[official GitHub runner table](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
on 2026-10-28. Actual runner image/toolchain logs must be retained when run.

This local workflow has **not been pushed or run**. Its push trigger is restricted
to `fix/codeguard-platform-evidence-20261028`, so a later authorized push of that
branch can collect evidence without merging main. Manual dispatch is also
declared, but a new `workflow_dispatch` workflow generally must be present on
the default branch to be dispatchable. Neither a local workflow file nor a
successful push is evidence that the jobs completed. No push, merge or dispatch
is part of this preparation.

## Minimal Linux evidence using existing implementation

Run in a trusted source checkout, with a dedicated Cargo/toolchain and temporary
project/npm cache. Set `CARGO_BUILD_JOBS=2`, `CARGO_PROFILE_DEV_DEBUG=0` and
`CARGO_PROFILE_TEST_DEBUG=0` to bound parallelism and artifact sizes. Rust 1.85+
and a working C/C++ build toolchain including CMake are required by dependencies.

```bash
CODEGUARD_BUILD_SHA="$(git rev-parse HEAD)" \
  cargo build --locked -p codeguard-cli --features wasm-precheck
target/debug/codeguard --version --format json
node scripts/pack-npm-local.mjs --require-wasm target/debug/codeguard
CODEGUARD_WASM_BIN="$PWD/target/debug/codeguard" \
  CODEGUARD_NPM_REPAIR_ARTIFACT=/tmp/codeguard-linux-npm-repair.json \
  node --test tests/npm_pack_repair.test.mjs
```

The last test really installs the private tarball in an isolated npm cache and
exercises edit tasks, evidence invalidation and rechecks; its controlled Zig
tool proves protocol behavior, not genuine Zig lint or a Codex session. Expected
candidate exit code 3 must remain incomplete. Do not run package tests in
parallel because they share `release/npm` output paths.

For broader package-specific evidence, reuse `tests/npm_pack_wasm.test.mjs`
(requires **both** `CODEGUARD_NO_WASM_BIN` and `CODEGUARD_WASM_BIN`), or select
its real Zig/Dart offline-package case by Node's `--test-name-pattern`. No need
to rerun every language to establish package entry-point identity.

## Still required before production claims

Actual Windows native runtime/Job Object behavior, target-specific path and
permission behavior, Linux libc/minimum OS qualification, macOS distribution
identity, immutable package/plugin-lock alignment, and installed Codex command,
MCP, save-feedback, failure/recovery and Git-boundary observations remain open.
All candidate identities, default-plugin failures and real-host outcomes must
be reported separately. No source-language logic or public package support was
changed by this preparation.

## Native process safety checkpoint

`crates/codeguard-runtime/tests/process_contract_portable.rs` uses its own real
native test executable as a child, without shell scripts. It verifies literal
argument handling, explicit cwd/environment/stdin, closed absent stdin, blocked
stdin bounded by the request deadline, a shared
stdout/stderr output budget, pre-start cancellation/deadline rejection, running
cancellation and timeout cleanup, and cleanup when the direct parent exits
before a descendant. Descendant tests first require a marker proving that the
spawn actually happened, then check that the delayed side-effect marker never
appears. Tests only write their own disposable directories.

Linux x64: the new target ran **9 tests passing** (one is the child-fixture entry)
alongside **19 passing** existing Unix process contracts. These preserve the
existing Unix implementation; no Windows runtime implementation was added.
Local log: `/workspace/guard-update-20261028/codeguard-portable-process-linux.log`.
The Windows target has **not run**, so it has no observed RED or GREEN result.
Its current source returns `UnsupportedPlatform`; the new candidate workflow
will expose that gap on a real Windows runner rather than skipping these tests.

An attempted Linux-to-Windows type check used
`cargo check --locked -p codeguard-runtime --test process_contract_portable --target x86_64-pc-windows-gnu`.
The Rust target component installed, but dependency `ring` could not build
because `x86_64-w64-mingw32-gcc` is absent. Exit 101 is a toolchain blocker, not
evidence that the Rust target compiles or runs. Log:
`/workspace/guard-update-20261028/codeguard-portable-process-windows-check.log`.

### Windows implementation boundary

The retained historical `process_cancellation.rs` contains only a Windows TODO,
not a reusable Job Object implementation. The existing S03/11.12 contract still
requires whole controlled-process-tree cleanup, literal argv and bounded I/O.
Starting a runnable child and attaching a job later introduces an escape race
and is not an acceptable shortcut.

A candidate implementation should use explicit Win32 handles and either a
suspended child attached before `ResumeThread`, or an atomic job-list process
creation attribute. The latter requires Windows 10 / Server 2016 or newer; the
current candidate runner is Windows Server 2022. Older or unavailable APIs must
fail closed. `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` should limit inherited handles;
`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` and `TerminateJobObject` must be paired with
observed cleanup, not merely handle drops. Pipe reads/writes must remain bounded
and cancellable under the existing absolute deadline. Windows C-runtime argument
quoting, explicit environment, Ctrl-C handling and all failure cleanup paths need
their own real-run evidence before support is claimed. No unverified FFI or
weakened process guarantees were introduced in this checkpoint.

Official API references checked on 2026-10-28:
[Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects),
[process/thread attributes](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute),
the [Microsoft API source including Job List minimum versions](https://github.com/MicrosoftDocs/sdk-api/blob/docs/sdk-api-src/content/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute.md),
and [Rust Windows CommandExt](https://doc.rust-lang.org/std/os/windows/process/trait.CommandExt.html).
The Rust 1.85 minimum must remain intact; a newer or nightly-only process API
must not be silently substituted for the declared toolchain contract.
