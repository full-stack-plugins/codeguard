# Windows platform candidate (2026-10-28)

This increment implements the existing S03 / 9.8 / 11.12 platform contracts. It does not release a package, change Hook trust, or certify all 57 registered languages. The candidate workflow binds its identity and tests to the checked-out commit.

## Implemented candidate behavior

- Native `.exe` discovery and `.venv/Scripts`, preserving Unix selection. Batch scripts are not silently interpreted by a shell.
- Protected current-user Windows DACLs for private creation; checked file handles, non-reparse pinned ancestors, file identity, exclusive locking, bounded state reads and handle-bound replacement.
- Suspended process creation, Job Object assignment before execution, no breakaway, shared output budget, bounded stdin, timeout/cancellation and descendant cleanup. Setup failures do not run user code. Handles have scoped ownership.
- Windows console cancellation and private process evidence. Existing CLI commands and language registration remain present.

## Required native evidence

`.github/workflows/platform-candidates.yml` runs version/57-registration identity, portable process contracts, platform filesystem contracts and CLI adapter contracts on Linux x64, macOS arm64 and Windows x64. Windows additionally runs private-log, resource-release, setup-failure and console-cancellation tests. The lease/attempt contract runs against the real candidate CLI. A Windows GNU cross-check only establishes compilation, never native runtime behavior.

The previous exact commit `4681e7e120a315f901aebc47d0a882f242567643` failed with 76 Windows WASM CLI compilation errors. The new shared code passed local Windows GNU compilation of the CLI/WASM and candidate test targets. Initial Linux regressions covered original process, private-log, ancestry, CLI and lease behavior. Final acceptance is the actual candidate CI result, not these intermediate checks.

## Boundaries retained

- Address-space sandboxing remains Linux-only; Windows requests explicitly return unsupported rather than pretending to enforce it.
- Private filesystem operations currently accept local disk paths and reject UNC/device roots and reparse ancestors. Batch launchers require a separately designed explicit interpreter contract.
- Windows file flush and write-through replacement do not claim POSIX parent-directory durability equivalence.
- Runtime safety tests are not validation of every native language tool, full workflow policy, a signed release, or installed Codex Hook execution.
- Existing public macOS-only runtime lock is unchanged. Portable manifest Hook discovery and any trust approval remain separate host acceptance work.

## First native candidate result and package follow-up

Candidate `7af093f99771ca189b7f168e79797957199e3c03`, run `40688039166`, built on all three native runners and preserved version/57-language identity. Linux completed successfully. Windows and macOS failed the process-contract step; available annotations contained exit codes only, and the job-log object requests returned HTTP 403. No failure cause is inferred from that limited evidence. A bounded diagnostic runner now preserves exact test commands and exit status while attaching the final 4 KiB of fresh test output to the next candidate check.

Private Windows x64 npm packaging now uses Node to invoke the installed npm JavaScript entry, never implicit `npm.cmd` shell parsing. The public darwin-arm64 release gate remains unchanged. The native candidate install test requires the original WASM/license/worker assertions, installs the tarball offline into a temporary profile, invokes its actual manifest entry, and checks identity and native bytes. This does not publish, globally install, or update the plugin runtime lock. Worker/probe working directories use the executable's absolute volume root on Windows rather than the invalid drive-relative `/` path.
