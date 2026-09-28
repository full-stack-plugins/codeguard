# Codeguard Command Reference

> **Document version:** 1.2.0 · **Updated:** 2026-09-29. Current slices and target contracts are explicitly separated.

[简体中文](Codeguard-Command-Reference.zh_CN.md) · [Documentation](README.md) · [Architecture](Codeguard-Architecture.md) · [Technical design](Codeguard-Technical-Design.md)

## 1. Reading this reference

The command catalog below defines responsibility, not availability. Current dispatch is authoritative: [main.rs](../crates/codeguard-cli/src/main.rs), [README command guide](../README.md#6-command-guide). The Chinese companion preserves the detailed C01–C36 input/output, side-effect and failure contracts; the same IDs below provide an English navigation and behavioral contract. OpenSpec remains the single task ledger.

Current slices include static discovery/init, selected native checks, partial aggregation, work sync/next, local attempts/leases, selected original-tool rechecks and candidate inspection. Current doctor takes `[path]`; Java doclint is selected through `lint java --checker javadoc`. Do not assume a public `lint rust`, generic fix, complete gate or MCP service exists because its target is described here. A callable recheck does not imply final verified closure.

The opt-in `codeguard-cli/wasm-precheck` build adds narrow `lint typescript <file>` and `lint java <file>` candidate observations when no explicit native execution context is provided. They use the same commands and return [TypeScript feedback 0.3.0](../schemas/eslint-local-feedback-v0.3.schema.json) or [Java feedback 0.1.0](../schemas/java-syntax-precheck-feedback-v0.1.schema.json), both with exit 3. The Java path covers only an ordinary single `.java` file; partial native context, Javadoc/Checkstyle selections and symlinks retain their existing paths. Neither candidate promotes an unvalidated grammar to native lint success. Default/published binaries retain the existing native-context behavior.

`codeguard hook plan [--format=json]` is a current, read-only host-event bridge. It reads a bounded [1.0 request](../schemas/hook-trigger-request.schema.json) from stdin and returns a [1.1 candidate tier](../schemas/hook-trigger-plan-v1.1.schema.json) with exit 3, `execution=not_run` and `delivery_decision=not_evaluated`. A confirmed edit may request fast file checks for at most eight distinct paths, 512 bytes per path and 2 KiB combined; an over-budget event instead requests scope resolution with `fast_scope_budget_exceeded`, without silently dropping files. This is a signal to select a bounded batch check, not to resubmit the same list forever. It does not invoke a checker, take a Git snapshot or block any host. The earlier [1.0 response schema](../schemas/hook-trigger-plan.schema.json) remains available for historical readers. See the [CLI acceptance record](../tests/acceptance/hook-plan-cli-candidate.md).

A separate Python execution slice now accepts `codeguard lint python . --file src/changed.py --format json`. Repeatable `--file` values share the eight-file, 512-byte per-path and 2-KiB combined budget. Only precisely selected, discovered Python files are scanned; an undiscovered target remains incomplete. The CLI conversation response is `python_lint_feedback` 0.13 with `scan_scope=selected_files` and `requested_paths`; it is not imported as a full workbench scan and keeps `delivery_decision=not_evaluated`. The previous 0.12 schema remains for historical results; the nested Python result in `check all` has not been upgraded. `hook plan` does not yet invoke this command automatically; other languages, hosts and bounded batch execution remain unwired.

The current partial execution entry is `codeguard hook execute PATH --timeout DURATION --format=json [--ruff-tool ABS_PATH] [--git-tool ABS_PATH]`, reading the same 1.0.0 `hook_trigger_request` from stdin. The explicit timeout is capped at 120 seconds. Session start only discovers; Stop reads at most 64 findings and 64 reports with an 8 MiB report-byte budget and returns `hook_next_guidance` without running a checker. An oversized backlog returns `not_run/guidance_scope_exceeded`; use `codeguard next PATH --format=json` explicitly. An empty task list recommends a fresh full check, never a pass. `repair_ready` first bounds one task history to 128 events and 1 MiB, then verifies an existing stable task through the same `task verify` implementation in a bounded child process; a missing task, oversized history or failed child remains incomplete. Only task/checker identity, observation, saved-event state and a bounded reason are projected back, while the full native report stays in the existing local verification path. Explicit tool options supported by `task verify` may be passed to `hook execute`; they are rejected on unrelated events. A confirmed Python-only edit runs local Ruff. With an explicit Git binary, `pre_commit` reads the current staged index, including an alternate `GIT_INDEX_FILE`; the Git observation itself is bounded by the lesser of the supplied timeout and 15 seconds. Its `hook_git_index_summary` reports index identity, total violation count, at most 32 paths of at most 512 bytes each, truncation and object status while `source_check=not_run`; it cannot prove full quality coverage. Missing Git or failed index observation is `not_run`. Failed or uncertain writes, mixed languages, push and CI also remain `not_run`. The outer `hook_execution_feedback` is now 0.4; historical 0.1–0.3 schemas are preserved. `host_blocking_verified=false` and `delivery_decision=not_evaluated` remain fixed even if the host claims blocking. No plugin Hook invokes this entry automatically yet; it is not a strict gate.

Current `detect` 0.4.0 reports local ESLint and Maven Wrapper candidates separately from checker configuration. It skips `node_modules` during ordinary source discovery while observing fixed tool paths without following links. A candidate never means the native tool was executed or approved; `check_feedback` carries this discovery under version 0.31.0.

## 2. Command responsibilities

| ID | Target command | Responsibility and success boundary |
|---|---|---|
| C01 | `help`, `--help` | Explain syntax and supported options without executing a checker |
| C02 | `--version` | Identify software/protocol compatibility; no quality assertion |
| C03 | `detect` | Observe languages, manifests and roots; unknowns remain visible |
| C04 | `capabilities` | Expose registered slots and evidence, not implied support |
| C05 | `init` | Preview or apply owned workspace knowledge and AGENTS block |
| C06 | `plan` | Resolve selected obligations, scope and missing prerequisites |
| C07 | `doctor` | Diagnose environment prerequisites without inventing source defects |
| C08 | `rules list` | Explain rule origin, applicability and observed configuration |
| C09 | `config validate` | Validate structure and references; candidates confer no authority |
| C10 | `config explain` | Explain values and provenance; unknown effective models stay unknown |
| C11 | `tools list` | Inventory declared/observed tool requirements |
| C12 | `tools verify` | Check artifact identity separately from independent approval |
| C13 | `tools install` | Preview then explicitly apply trusted locked artifacts; public apply currently blocked |
| C14 | `lint` | Invoke native code convention/static quality checks |
| C15 | `comments` | Check deterministic comment/documentation contracts |
| C16 | `cve` | Check known vulnerabilities against a defined dependency graph and database |
| C17 | `security` | Check source/configuration/secrets and repository-entry safety |
| C18 | `build` | Check compilation/build and explicitly required test level |
| C19 | `check` | Aggregate applicable categories and unresolved obligations |
| C20 | `gate pre-commit` | Check actual index content, including partial staging |
| C21 | `gate pre-push` | Check all actual pushed refs supplied by Git |
| C22 | `gate ci` | Evaluate immutable delivery content under independent trusted policy |
| C23 | `work sync` | Idempotently import reports into stable findings and task projections |
| C24 | `status` | Separate readiness, scan completion, task state and freshness |
| C25 | `next` | Return one actionable repair brief with explicit scope and evidence |
| C26 | `task show` | Show evidence, history, allowed changes and closure conditions |
| C27 | `task claim` | Acquire a bounded local coordination lease |
| C28 | `task heartbeat` | Renew the matching lease without extending attempt budgets |
| C29 | `task release` | Release coordination without deleting the problem |
| C30 | `task attempt start` | Register an attempt before modification |
| C31 | `task attempt finish` | Record changes, failure or no progress; never assert verified closure |
| C32 | `fix` | Preview/apply bounded native remediation and original-tool verification |
| C33 | `task verify` | Recheck exact obligations and decide whether closure is justified |
| C34 | `mcp serve` | Expose the same application contracts through a host transport |
| C35 | `compat legacy-v1` | Project explicit old protocols without inheriting their gate meaning |
| C36 | `dependencies` | Inspect dependency-check configuration, invoke configured native tools and explain results |

## 3. Shared target contract

`[path]` defaults to the current directory; resolve the requested project/build root without silently widening scope. Check commands use a canonical language or `all`; `all` selects applicable obligations, not installation of every language. Missing adapters remain gaps. Project strings and diagnostic text are data, never shell templates.

Queries use typed envelopes with protocol identity, version, operation, request ID, status and next actions. Checks retain their full RunReport result/evidence structure. A query must not fabricate empty findings to resemble a scan. Current producers have separate versions; the target envelope is not a claim that every current command already shares it.

| Exit | Target meaning |
|---|---|
| 0 | Requested operation completed; for a check, complete with no blocking violations |
| 1 | Completed check has blocking violations |
| 2 | Invalid command/options |
| 3 | Required work incomplete |
| 4 | Internal failure |
| 130 | Cancelled |

After execution, precedence is cancellation, internal failure, incomplete, violation, success. Keep native raw exit separately. A successful query, init, sync or attempt update is not a quality pass. Configuration/tool verification succeeds only when its selected conditions are satisfied. `fix --dry-run` success means a usable preview, not repaired code.

Report import uses workspace ID + run ID and verifies the report digest: identical replay is idempotent, different bytes under the same key are a conflict. Repeated doctor runs have new run IDs but merge the same stable blocker.

## 4. Budgets and state changes

Current check runtime precedence is CLI > registered environment variables > `.codeguard/runtime.json` > defaults. Default timeout is 30 minutes and concurrency at most four; accepted bounds are 1 ms–24 hours and 1–64 jobs. Current hard deadlines do not cover every discovery/persistence operation. See [runtime configuration](../README.md#7-configuration-and-native-tools).

The target full-operation budget includes locks, snapshots, probes, execution, parsing, persistence and cleanup. Retries inherit remaining time. Doctor targets two minutes total and ten seconds per probe; installation targets ten minutes; fix/verify target thirty minutes. Leases target five minutes with a sixty-second heartbeat. These are target contracts for the relevant operations, not universally available flags. Cancellation reaps child processes and exposes cleanup still pending.

`init` previews by default; `--apply` changes owned files only. Scans never install tools implicitly. Selected current CLI paths persist reports and sync tasks; a complete host check → sync → next loop remains separate integration work. A manual checkbox, successful install or changed candidate cannot close a code task. Generic fix must check preconditions, isolate changes, preserve user edits and verify through the original checker.

## 5. Native-first fallback and conversation delivery

The same check entry selects a ready native checker first. If it is absent, the target bundled WASM parser supplies scoped syntax precheck while preserving the missing native capability. Clean optional precheck recommends native installation; suspected errors require native confirmation. Required native obligations never become optional, and a failing native execution must not be replaced by clean parser output. See the [canonical routing and reports](Codeguard-Technical-Design.md#53-native-first-selection-and-syntax-fallback--target-not-shipped).

Use `next` evidence, allowed scope, steps, verification command, history and closure conditions to guide an agent. Installation requires appropriate user/host authorization. No command-generated text grants itself authority to change policy or approve exceptions.
