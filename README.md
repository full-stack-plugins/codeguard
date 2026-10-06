# Codeguard

[English](README.md) | [简体中文](README.zh-CN.md)

**A Rust CLI for native static checks and actionable repair workflows.**

Codeguard helps developers and coding agents discover existing quality configuration, run selected native checkers, and turn results into durable repair tasks. Rust owns orchestration and interpretation; Maven, P3C, Checkstyle, Javadoc, Ruff, Cargo, ESLint, and other native tools remain responsible for their checks.

> **Status:** early development, source `0.1.4`. Several native checks and local repair workflows work within explicitly bounded scopes. Full delivery gates, complete language coverage, automatic task closure, and host-plugin integration remain incomplete.
>
> **Baseline:** The current source and [implementation evidence](openspec/changes/introduce-rust-codeguard-cli/implementation-baseline.md) define callable behavior. Declared Rust minimum `1.85`, edition `2024`, Cargo resolver `2`. `@partme.ai/codeguard@0.1.4` is published for Apple Silicon macOS; no multi-platform binary release or crates.io availability is claimed.

```text
Project files + existing checker configuration
                    |
                    v
Codeguard: discover -> select -> native tools -> interpret
                    |                         |
                    v                         v
       Findings / environment blockers    Human / JSON / SARIF*
                    |
                    v
       Stable tasks -> agent repair -> original-tool recheck

* SARIF is supported by selected check output paths.
```

Source builds also support `codeguard task verify TASK_ID . --erl-tool /absolute/path/to/erl --format=json` for existing Erlang WASM confirmation tasks. Omitting the option uses the same invoking absolute-PATH selection as lint/check. `next` retains current native positions, concrete unresolved reasons and reusable tool argv; `repair_ready` shares the existing lease and attempt history. OTP 28 macro/preprocessor coverage remains unresolved, and zero diagnostics cannot close the task. These changes are absent from public npm 0.1.4. See [Erlang repair-loop acceptance](tests/acceptance/erlang-native-task-verification.md).

## 1. Purpose and boundaries

Source builds now detect a missing Go whole-file `package` declaration through a separate AST rule. `grammar probe go FILE`, `check go/all`, and confirmed save hooks preserve raw parser recoveries and reuse one native-confirmation task. Adding the declaration removes the candidate without closing the task. Native Go vet remains responsible for `lint go`; Go confirmation tasks support native rechecks and limited host-SDK resolution; default-host approval integration and grammar qualification remain incomplete. These changes are not in public npm0.1.4. See [acceptance](tests/acceptance/go-package-structure.md).

- Discover languages, build roots, declared versions, checker configuration, and statically observable module relationships.
- Unify code-style, documentation, dependency, vulnerability, security, and build checks as native adapters become available.
- Explain missing configuration and environment problems without misreporting them as source violations.
- Return findings and repair steps to the caller's conversation through human-readable or structured output.
- Preserve stable tasks and failed attempts; stop repeating actions that make no progress.
- Support precise false-positive dispositions. Current public allowlist commands inspect or propose decisions; they do not approve them.

The CLI contains no LLM or vector database. It does not reimplement P3C/Maven checks in Rust. Native tools may still require a JVM, Node.js, Python, Go, or Rust toolchain. Python reference fixtures are test material, not a second Codeguard runtime.

Source builds connect native P3C single-file lint to the nearest initialized workspace, or an explicit `--workspace ROOT`. Use `codeguard lint java FILE --checker p3c` with the existing explicit Maven/JDK/offline-repository arguments. Only the selected file and the nearest POM’s statically confirmed rule subset are checked; `check java` and `task verify` reuse the same stable task. Explicit P3C selection does not silently switch to WASM. This is local feedback, not complete P3C coverage or automatic task closure. See [workflow](docs/Codeguard-Native-Repair-Workflow.md#p3c-single-file-project-binding) and [acceptance](tests/acceptance/java-p3c-file-workbench.md). Public npm 0.1.4 is unchanged.

A failed P3C execution can still contribute validated, current diagnostics to stable source tasks while retaining its execution blocker. This does not increase completed-file counts or prove absence. See [partial-execution acceptance](tests/acceptance/java-p3c-partial-execution.md).

## 2. Capabilities and maturity

| Area | Current implementation | Boundary and evidence |
| :--- | :--- | :--- |
| Discovery / init | Multi-language observations, Maven/Cargo declarations, profiles, module graph, managed `AGENTS.md` | Static observations; architecture remains unknown without evidence. [Tests](crates/codeguard-cli/tests/init_command_contract.rs) |
| Python | Configuration-aware Ruff lint and `D###` documentation diagnostics; standard-pylock pip-audit observations | Partial scope/policy; distinguish simulated and real tool evidence. [Ruff](tests/acceptance/python-lint-scan.md), [CVE](tests/acceptance/python-cve-partial-native.md) |
| Rust | Clippy, Rustdoc, `cargo check`, cargo-audit; selected `check all` and repair integrations | Not every workspace/feature/target combination. [Clippy](tests/acceptance/check-all-partial-native.md), [Rustdoc](tests/acceptance/rustdoc-check-all.md), [build recheck](tests/acceptance/rust-build-task-verification.md) |
| Java | Maven configuration discovery; scoped P3C, JDK Javadoc, Checkstyle, dependency and OWASP observations | Explicit tool/configuration limits. [P3C](tests/acceptance/java-p3c-cli-native-local.md), [Javadoc](tests/acceptance/java-javadoc-cli-native-local.md), [Checkstyle](tests/acceptance/java-checkstyle-cli-local.md) |
| JavaScript / TypeScript | ESLint via `lint typescript` and source-built `check all`; npm audit via `cve typescript` and `check all` | Aggregate ESLint supports local 10.x and a unique flat config; not all package managers. [ESLint](tests/acceptance/eslint-public-lint-feedback.md), [npm](tests/acceptance/npm-partial-native.md) |
| Go | Scoped native Go vet observation | Not a full Go gate. [Evidence](tests/acceptance/go-vet-json-local-probe.md) |
| Repair workbench | Findings, task projections, sync, next, leases, attempts, selected rechecks | Formal task close/reopen remains pending. [Tests](crates/codeguard-cli/tests/task_verify_contract.rs) |
| Other languages | Capability inventory and explicit gaps | Registry presence is not an executable adapter. [Inventory](docs/CAPABILITIES.md) |

The six categories are `lint`, `comments`, `dependencies`, `cve`, `security`, and `build`. The registry defines 28 finer detection families; enumeration is not a coverage claim. See [capability dimensions](crates/codeguard-core/src/capability_dimensions.rs).

## 3. Architecture and layout

| Crate | Responsibility | Workspace dependencies |
| :--- | :--- | :--- |
| `codeguard-cli` | Binary, arguments, application orchestration, workspace files, feedback | core, runtime, adapters |
| `codeguard-core` | Findings, completion, obligations, gate calculation, allowlist matching, readiness | None |
| `codeguard-runtime` | Processes, snapshots, scheduling, cancellation, locks, artifact primitives | core |
| `codeguard-adapters` | Native command contracts, configuration observations, report parsing | core |

```text
codeguard/
├── Cargo.toml / Cargo.lock
├── crates/
│   ├── codeguard-cli/
│   ├── codeguard-core/
│   ├── codeguard-runtime/
│   └── codeguard-adapters/
├── rulepacks/          # preview candidates and legacy inventory
├── schemas/            # versioned data contracts
├── npm/                # thin Node command launcher
├── scripts/            # local npm packaging
├── docs/               # architecture, design, generated inventory
└── tests/              # acceptance records and shared fixtures
```

Repository: `codeguard`. Binary: `codeguard`. Cargo package: `codeguard-cli`. A checked project's generated `.codeguard/` data directory is separate from this source repository.

## 4. Build and first read-only run

Use a toolchain supporting Rust edition 2024. Declaring MSRV `1.85` does not prove every dependency/target combination on that version. Native execution and local coordination use Unix-specific code. Reviewed execution evidence is from macOS ARM64; the other candidate platforms are not a certified support matrix.

The WASM dependency is constrained to `tree-sitter-language=0.1.7` to preserve the declared Rust 1.85 baseline. The locked metadata regression and an explicit 1.85 CI check cover different evidence levels; full target/MSRV acceptance remains open. See [compatibility evidence](tests/acceptance/rust-msrv-dependency-compatibility.md).

S14.2 loader acceptance is complete: all32 fixed candidates load offline, malformed/hash/ABI negative cases pass, and actual Rust1.85 default/WASM checks succeeded for8ca3bb1. This does not approve language precision, asset release or the full platform matrix; see [loader verification](tests/acceptance/rust-wasm-loader-completion.md).

```bash
git clone https://github.com/full-stack-plugins/codeguard.git
cd codeguard
cargo build --locked -p codeguard-cli
./target/debug/codeguard --version --format json
./target/debug/codeguard capabilities java --format json
./target/debug/codeguard detect . --format json
./target/debug/codeguard init . --dry-run --format json
```

Version output includes `cli_version`, `target`, and protocol information. Discovery and init preview return observations and unknowns. Dry-run creates neither `AGENTS.md` nor a project data directory. Add Cargo `--offline` only when dependencies are cached.

Source builds now support `codeguard lint erlang FILE --timeout 10s --format=json`, automatically selecting `erl` from absolute PATH directories. `--erl-tool /absolute/path/to/erl` overrides discovery. The 0.2 feedback records the selection source and canonical executable; native failures never silently select another tool or WASM. OTP 28 scanning/parsing takes priority and catches the pinned WASM's missing-period case; macro/preprocessor inputs stay unresolved. If no executable can be resolved, the optional WASM candidate remains available and retains its known limit. This single-file forms observation remains incomplete for project delivery; published npm 0.1.4 does not include this new command. See the [native-first evidence](tests/acceptance/erlang-native-first.md).

The workspace requires `std`. The opt-in `wasm-precheck` build feature uses a bounded Rust worker with thirty-two pinned grammar candidates: ArkTS, C, C++, C#, Go, Java, JavaScript, Lua, Luau, Objective-C, Python, Rust, Solidity, TypeScript, TSX, Zig, Nix, Terraform, R, Ruby, PHP, Kotlin, Dart, Erlang, Pascal, CFML, CFQuery, CFScript, COBOL, Scala, Swift and VB.NET. Java and TypeScript/TSX have narrow single-file fallback paths; `lint python` can provide bounded suspected syntax positions when Ruff is unavailable or no project Ruff configuration is declared. Native Ruff results take priority. The published npm `0.1.3` package includes the candidate feature; `0.1.2` did not. No `no_std`, completed project-wide WASM fallback, zero-unsafe, or fastest-runtime claim is made. Runtime OS calls require safety review; a comprehensive security audit is not claimed.

The current source-built CLI's `codeguard grammar status --format=json` reports the coverage gap without loading grammars or running lint: the pinned CodeGraph source has 30 vendored WASM files plus two distinct grammars resolved through its dependency; CodeGuard has thirty-two unqualified asset candidates (ArkTS, C, C++, C#, Go, Java, JavaScript, Lua, Luau, Objective-C, Python, Rust, Solidity, TypeScript, TSX, Zig, Nix, Terraform, R, Ruby, PHP, Kotlin, Dart, Erlang, Pascal, CFML, CFQuery, CFScript, COBOL, Scala, Swift, VB.NET) and zero released syntax capabilities. C, C++, C#, Go, JavaScript, Lua, Luau and Rust are byte-pinned CodeGraph assets that load in the Rust worker but have no language-qualified standalone lint route or release qualification. ArkTS, Nix and Terraform are also pinned Rust-loadable candidates with narrow valid/invalid fixtures and no language-qualified standalone lint route or release qualification; see the [limited acceptance record](tests/acceptance/arkts-nix-terraform-grammar-candidates.md). The two dependency grammars retain their source bytes and use a byte-pinned legacy `dylink` metadata conversion for Rust loading; they have no language-qualified standalone lint route or release qualification. Zig uses a byte-pinned import adaptation so the corrected CodeGraph grammar can load in Rust; it has a narrow `lint zig` native-first candidate route but no release qualification. [Zig's limited acceptance record](tests/acceptance/zig-grammar-candidate.md) distinguishes this from released syntax support. Python has an unqualified CLI fallback path; its report remains incomplete and cannot approve delivery. R, Ruby, PHP and Kotlin have pinned CodeGraph bytes, licenses, measured Rust ABI and narrow worker samples; PHP covers mixed HTML/PHP, and all four remain unqualified. The original CodeGraph Dart WASM cannot be instantiated by Rust, but CodeGuard now pins a reproducible Zig rebuild with the real scanner and a byte-pinned WASI import adaptation. Its narrow Rust and worker fixtures pass, while public lint and release remain unqualified; see the [Dart rebuild record](tests/acceptance/dart-grammar-rebuild-candidate.md). Erlang has pinned CodeGraph bytes, the upstream 0.19 license, ABI 14 and Rust worker tests. Current source builds also have an OTP 28 native comparison, an explicit native-first `lint erlang --erl-tool` command and task verification; the original WASM still misses function terminators. Those native extensions are absent from public npm 0.1.4, and full language/release qualification remains open. See the [native comparison](tests/acceptance/erlang-native-differential.md) and [task verification](tests/acceptance/erlang-native-task-verification.md). Pascal now has pinned CodeGraph bytes and its original Isopod dependency commit and license; ABI 14 and narrow worker tests pass, but native comparison and public lint remain open. See the [Pascal candidate record](tests/acceptance/pascal-grammar-candidate.md). The remaining CFML/CFQuery/CFScript/COBOL/Scala/Swift/VB.NET assets also load in the Rust worker, bringing the candidate inventory to 32/32 with zero released syntax capabilities. CFQuery misses `SELECT FROM`, VB.NET misparses a valid unindented method, and COBOL has a high load cost; see the [seven-asset acceptance record](tests/acceptance/final-seven-grammar-candidates.md). The separate [coverage inventory](grammars/codegraph-coverage.json) is not an approved asset manifest. COBOL's 16.4 MB asset now fits the bounded 20 MiB loader input, but its cold-load and memory budgets remain unqualified.

The source-built `grammar status` now includes each candidate's bounded `known_limitations`, including the [VB.NET false-positive reproduction](tests/acceptance/vbnet-unindented-method-false-positive.md). This is risk context for the agent, not a confirmed violation or a native compiler result.

### npm installation and one-off use

WASM source builds now expose a separate duplicate direct JavaScript let/const binding candidate through `codeguard grammar probe javascript FILE --format=json`. [Probe0.6](schemas/grammar-probe-v0.6.schema.json) preserves raw parser recovery, declares the limited rule scope and requires native confirmation. Project checks, hooks and repair tasks have not yet adopted this rule; it is not included in published npm0.1.4. [Acceptance](tests/acceptance/javascript-binding-probe.md).

In a source build with `--features wasm-precheck`, `codeguard grammar probe <language> <file> --format=json` can explicitly run any of the 32 pinned candidates in an isolated worker. It always exits 3 with `status=incomplete`, `native.status=not_run`, and `delivery_decision=not_evaluated`; a completed parse also has `precheck.status=incomplete`. Input or worker failures use the same [closed JSON schema](schemas/grammar-probe-v0.1.schema.json). Recovery anchors are suspected observations only. This diagnostic command does not yet connect all grammars to language-qualified native-first `lint/check`; the published npm package includes it as an unqualified candidate.

Source-built `check all` now selects module-local ESLint 10 and a single flat config for discovered JS/TS/TSX files. It prefers `--node-tool`, otherwise resolving Node from PATH. Results appear under `native_results.node_lint`; initialized workspaces synchronize stable tasks and return `next`. Completed native files with matching source bytes skip duplicate WASM; missing configuration, ignored files and native failures retain their reasons and candidate fallback. The native path also works without the WASM feature. This wiring is included in npm 0.1.4 and does not cover pnpm symlink packages, older ESLint versions or every configuration combination. See [aggregate ESLint acceptance](tests/acceptance/check-all-eslint.md).

The same source build now runs a bounded candidate pass from `codeguard check all . --format=json` after existing native checks. Its `syntax_candidates` field in [check feedback 0.38.0](schemas/check-feedback-v0.38.schema.json) carries bounded known grammar limitations and distinguishes TSX, JavaScript, CFScript and explicitly embedded CFQuery, preserving native blockers and skipped work. Python files covered by a completed Ruff scan of identical source bytes skip duplicate WASM and count under `native_preferred_count`; files without matching native syntax evidence retain candidate prechecks. Four bounded integration fixtures collectively invoked all 32 pinned grammars through this command. Every observation remains unqualified and the command exits 3; 64 files/64 fragments/90 seconds bound the candidate pass. This is not yet a language-qualified native fallback; published npm `0.1.3` includes this candidate route, while the 0.35.0 known-limitation projection is included in 0.1.4. See [acceptance evidence](tests/acceptance/check-all-32-grammar-candidates.md). Narrow 13-case syntax differentials agree for [C11 against Apple Clang 21](tests/acceptance/c-native-differential.md), [Rust 2021 against rustfmt 1.9.0](tests/acceptance/rust-native-differential.md), and [Ruby 2.6.10](tests/acceptance/ruby-native-differential.md); none qualifies native lint. The [Swift 6.4 differential](tests/acceptance/swift-native-differential.md) now separates 12 decidable agreements from one hidden-error case reported as incomplete; Swift remains unqualified.

The source-build acceptance test also runs the 31-file mixed-language fixture as one project: at most two isolated candidate workers run concurrently, subject to `--jobs`, while report order, source rechecks, and the 90-second candidate deadline remain fixed. A local run observed all 32 candidates with no skipped fragments; the Linux WASM integration step for source commit `ff60184` passed the same one-project test; the complete CI run passed. This is still candidate coverage, not qualified lint.

Java now also has a narrow [javac 21 / Java 17 differential](tests/acceptance/java-native-differential.md): 8 valid and 5 invalid syntax samples agree with the pinned WASM. This does not qualify Java lint, other language versions, or project-wide native-first routing.

The [Kotlin 2.4.10 differential](tests/acceptance/kotlin-native-differential.md) now separates 11 decidable agreements from two hidden-error cases: native-rejected `fun f(x: ) = x` and native-valid `object C { val value = 1 }`. Both are incomplete rather than clean or confirmed violations; run the applicable native checker. Zero diagnostic counts cannot classify incomplete scans.

The source-built Unix CLI also has a narrow Zig route: `codeguard lint zig FILE --zig-tool /absolute/path/to/zig --format=json`. An explicitly supplied tool reporting Zig 0.16.0 and retaining the same byte digest runs native `ast-check` first; its source positions are retained without exposing source snippets. Without an explicit tool, the pinned Zig WASM provides an unqualified observation. Both paths remain incomplete because `ast-check` covers only local AST errors, not full lint, build or tests. See the [Zig feedback schema](schemas/zig-lint-feedback-v0.1.schema.json).

On Apple Silicon macOS, the published `0.1.4` candidate package was verified with a fresh npm cache:

```bash
npx --yes @partme.ai/codeguard@0.1.4 --version --format json
npx --yes @partme.ai/codeguard@0.1.4 grammar status --format=json
npx --yes @partme.ai/codeguard@0.1.4 check all . --format=json
```

For a project check, replace the arguments after the package name with the desired Codeguard command. The package is currently restricted to macOS arm64.

The Node entry point invokes the same Rust executable. From this checkout, build one host-native bundle and package it without npm lifecycle scripts:

```bash
cargo build --release --locked -p codeguard-cli
CODEGUARD_TARBALL="$(node scripts/pack-npm-local.mjs)"
npm install -g "$CODEGUARD_TARBALL" --ignore-scripts
codeguard --version --format json
```

For a one-off Node invocation without a global installation:

```bash
npm exec --yes --package "$CODEGUARD_TARBALL" -- codeguard detect . --format json
```

The packer currently supports macOS and Linux on x64/arm64 when a matching native binary is available. It requires Node 18+, npm, and an already built Rust binary for the current host. It checks the binary's reported target and version before packaging, and writes a platform-tagged tarball under ignored `release/npm/`. The default package is private and local-only. `node scripts/pack-npm-local.mjs --public` prepares the public `@partme.ai/codeguard` package with a README, full license, and host restrictions. Version `0.1.3` is published with all 32 WASM candidates; its registry-hosted `npx` entry and artifact hashes were verified on Apple Silicon macOS. The binary reports the candidate source commit, which is not a signed or reproducible-build attestation. Other platforms need their own verified binary packages. See the [distribution design](docs/Codeguard-Technical-Design.md) and [0.1.3 acceptance record](tests/acceptance/npm-0.1.3-wasm-candidate.md).

To prepare a **local** package with candidate WASM support, build with `cargo build --locked -p codeguard-cli --features wasm-precheck` and run `node scripts/pack-npm-local.mjs --require-wasm target/debug/codeguard`. The packer rejects a binary without the worker, checks all 32 pinned asset identities, and executes a bounded Zig candidate probe before writing the tarball. `--public` requires the same check and verified upstream licenses. The published `0.1.3` package carries all 32 candidates; local and registry installation do not qualify a language or certify a clean project.

## 5. Repair workflow

Make the built binary available as `codeguard` on `PATH`, or use its absolute path. In a target project:

```bash
codeguard init . --dry-run --format json
codeguard init . --apply --format json
codeguard check all . --format json
codeguard work sync . --format json
codeguard next . --format json
codeguard status . --format json
```

Current `init --apply` returns exit `3` and partial status even when files were created. `check all` also remains incomplete: inspect its findings and next actions. Do not convert exit `3` into CI success. Integrated checks already save/sync reports in initialized workspaces; explicit `work sync` supports recovery.

Set `TASK_ID` to an actual ID returned by `next`, then inspect and recheck after repair:

```bash
codeguard task show "$TASK_ID" . --format json
codeguard task verify "$TASK_ID" . --format json
```

Supply the original checker's required tool/configuration options to `task verify`. Missing tools require environment repair. Selected rechecks recognize native suppression or changed configuration; zero diagnostics alone do not close a task.

## 6. Command guide

`codeguard --help` provides current syntax. Source and acceptance records identify exact scopes; a few help descriptions remain conservative.

| Family | Value | Current boundary |
| :--- | :--- | :--- |
| `--version`, `capabilities`, `detect` | Binary, inventory, project observations | Current source `detect` 0.4.0 exposes local ESLint/Maven Wrapper candidates; no tool is run or approved |
| `init --dry-run / --apply` | Preview/create/refresh workbench | No implicit install, build, Hook takeover, or architecture certification |
| `config validate / explain`, `rules list` | Inspect configuration and rule origins | Candidates do not establish policy authority |
| `tools list / verify`, `doctor` | Inspect tools and limited environment probes | Explicit Ruff doctor probe; `tools install --apply` is blocked |
| `plan CATEGORY LANGUAGE` | Preview selections and gaps | Not a certified execution plan |
| `hook plan` | Route a versioned host event to a candidate check tier | Reads bounded JSON on stdin; exit 3, no check or host blocking |
| `hook execute` | Run read-only discovery, bounded Stop guidance, task-bound recheck, selected Python/Ruff, JS/TS/ESLint, Kotlin, Swift, Zig, Ruby, ShellCheck and optional WASM feedback, or a live pre-commit index safety preview | Explicit timeout; repair reuses `task verify` and never closes a task or approves delivery |
| `hook claude <session-start\|user-prompt-submit\|post-tool-use\|post-tool-use-failure\|stop>` | Map Claude Code lifecycle events to read-only discovery, constant prompt guidance, bounded edit feedback, no-check failure feedback, or local next-step guidance | Candidate soft Hooks; Stop offers one task continuation at most; default Hooks and delivery gates remain incomplete |
| `lint python / java / typescript / go` | Run selected native checks | Adapter-specific options and scope |
| `comments rust`, `build rust` | Documentation and type checking | Build does not run project tests |
| `cve rust / python / typescript` | Native advisory observations | Database identity/freshness and full coverage remain limited |
| `check all / <canonical-language-id>` | Aggregate integrated checks and repair feedback | Full project: `incomplete`; scoped request: `not_evaluated` |
| `work sync`, `status`, `next`, `task show` | Persist and inspect repair work | Task files are not a gate |
| `task claim / heartbeat / release`, `task attempt start / finish` | Local ownership and attempt history | Unix local coordination, not distributed locking |
| `task verify` | Repeat selected original checkers | Formal closure/reopening pending |
| `rules whitelist list / explain / propose` | Inspect/propose false-positive dispositions and corrections | No public approval or activation |
| `gate pre-commit` | Git-index path/object observation and narrow OpenSSH Ed25519 key detection | Incomplete preview; not a full content/security gate |

Current source `config validate/explain` returns version 0.3 static native configuration observations per build root, including source hashes, unresolved conditions and preparation steps. It never runs JS configuration or native checkers; effective rules and suppressions remain unresolved. See [configuration acceptance](tests/acceptance/config-native-observation.md). This extension is not included in public npm 0.1.4.

For Python edit feedback, run `codeguard lint python . --file src/changed.py --format json`. `--file` is repeatable with a limit of eight distinct paths, 512 bytes per path and 2 KiB combined. The response declares `scan_scope=selected_files` and observes only those files; it is not imported as a full workspace scan and cannot approve delivery. Without `--file`, the existing command still scans discovered Python files and synchronizes its local report.

Generic `dependencies`, generic `security`, arbitrary category/language combinations, `fix`, `gate pre-push`, `gate ci`, `mcp serve`, and the legacy compatibility dispatcher are target-design surfaces, not available commands in this baseline.

### Native-first entry points and syntax fallback — design target

**The published `0.1.3` includes bounded WASM candidate routing but not qualified fallback or automatic conversation delivery.** The existing commands remain the unified entry points; native adapter-specific prerequisites still apply today:

An opt-in source build with `--features codeguard-cli/wasm-precheck` has narrow TypeScript and Java single-file candidate paths. For TypeScript, a visible local ESLint 10 entry and one flat config take priority: when an executable Node is resolved from `PATH`, the CLI runs its existing bounded native probe; without Node it reports the setup gap. Only when the project-local ESLint package is not observed can the bundled TypeScript/TSX candidate provide suspected syntax positions; an untrusted entry, package identity or config returns a specific setup blocker. The Java candidate likewise remains incomplete. Python `lint` first preserves Ruff results, then can attach suspected positions for missing Ruff or undeclared project Ruff configuration; a broken Ruff configuration stays a setup blocker. For one checked file in an initialized workspace, the source-only path syncs one stable native-confirmation task; repeat candidate scans do not close it. No bundled grammar has an accepted language/dialect range. Native observations and prechecks never approve delivery. Default and published binaries keep their previous native-context behavior. [TypeScript acceptance](tests/acceptance/typescript-syntax-fallback-candidate.md) · [native-first acceptance](tests/acceptance/native-first-eslint-candidate.md) · [Java acceptance](tests/acceptance/java-syntax-fallback-candidate.md).

With an initialized `--workspace`, that TypeScript/TSX fallback now returns a real, stable native-confirmation task ID after sync; repeated WASM scans never close it. A versioned local report now preserves bounded suspected positions with source and grammar identities; capability-matched native closure still needs implementation. [Workbench acceptance](tests/acceptance/typescript-syntax-confirmation-task.md).

```bash
codeguard lint java .
codeguard lint typescript .
codeguard check all .
```

Target behavior selects per module/language/configuration, so a ready frontend checker and a Java module missing its JDK can take different paths:

```mermaid
flowchart TD
    A[Unified lint / check entry] --> B{Native checker ready?}
    B -->|Yes| C[Native report and repair guidance]
    B -->|No| D[Bundled WASM syntax precheck]
    D --> E{Result}
    E -->|Clean checked scope| F[Precheck report; recommend native setup]
    E -->|Suspected issue| G[Precheck report; require native setup and confirmation]
    E -->|Incomplete or unsupported| H[Explain gap; restore checking capability]
    C --> I[Workbench sync and agent conversation brief]
    F --> I
    G --> I
    H --> I
    G --> J[Prepare applicable native checker]
    J --> C
```

Already-required native checks remain required even when precheck is clean. A native failure/violation cannot be erased by fallback. Installed tools with broken configuration need configuration repair. Suspected parser issues require native confirmation, not an automatic source edit; installation alone does not close a task. WASM syntax results do not cover type checks, style, Javadoc, CVE or security. See [architecture](docs/Codeguard-Architecture.md) section 8 and [technical design](docs/Codeguard-Technical-Design.md) sections 5.3–5.4.

## 7. Configuration and native tools

Native configuration controls native rule selection. Static discovery reports `configured`, `missing`, `invalid`, or `unknown`; it does not execute wrappers or JavaScript configuration to guess an effective build model.

Optional `.codeguard/runtime.json` controls scheduling only:

```json
{
  "schema_version": "1.1",
  "document_type": "codeguard_runtime_options",
  "timeout": "30m",
  "jobs": 4
}
```

For `check`: CLI → `CODEGUARD_TIMEOUT` / `CODEGUARD_JOBS` → project file → defaults. Default timeout is 30 minutes; default jobs is available CPU parallelism capped at four. Limits are 1 ms–24 hours and 1–64 jobs. Current hard deadlines cover native execution, not every discovery/persistence operation. Runtime options cannot disable rules or approve exceptions. See [schema](schemas/runtime-options-1.1.schema.json) and [selection code](crates/codeguard-cli/src/check_budget.rs).

Adapters use selected executables/configuration, not arbitrary command strings. Options include `--ruff-tool`, `--cargo-tool`, `--go-tool`, `--java-home`, explicit Maven repositories, and explicit Node/tool entry points. Prepare tools first; Codeguard does not silently install them. External checkers retain their own runtimes.

## 8. Results, exit codes, and false positives

| Code | Result-contract meaning |
| :--- | :--- |
| `0` | Query/operation success or a justified nonblocking result where implemented; not automatically delivery permission |
| `1` | Violations in a complete evaluated obligation set |
| `2` | Invalid/unsupported command or arguments |
| `3` | Incomplete configuration, execution, coverage, or policy context |
| `4` | Internal error |
| `130` | Cancellation |

These are [core semantics](crates/codeguard-core/src/verdict.rs), not a promise that partial scans produce all verdicts. Findings and completion are independent: timeout may retain valid findings; missing JDK is an environment blocker; damaged reports do not manufacture violations. Human/JSON are primary outputs; selected `check` paths support SARIF.

False-positive handling retains the native finding and matches a disposition to exact checker/rule/target/evidence identity. Expiry, revocation, and changed inputs require reevaluation. Public proposal/correction commands are partial workflows. Writing `approved: true` in a project file does not activate an exception. See [technical design](docs/Codeguard-Technical-Design.md).

### Conversation report examples — target presentation

These are illustrative report designs, not current CLI transcripts; the paths/task ID are synthetic, and English runtime localization is not claimed. Real messages must use actual scope, diagnostics and persisted task IDs.

```text
Codeguard: Java syntax precheck found 1 suspected issue

Method: bundled Tree-sitter WASM
Scope: 18 Java files selected and checked; 0 unresolved
Native check: not run; the project's required JDK is unavailable
Location: src/main/java/example/UserService.java:42
Observation: the parser recovered a missing syntax node here

Next steps (required):
1. Prepare the project's declared JDK and applicable native checker.
2. Run native verification covering this file and Java syntax.
3. Follow the native diagnostic, repair and recheck.

Task: CG-example-java-setup (illustrative; only after successful sync)
Recheck entry (after restoring original project tool/configuration context):
codeguard task verify CG-example-java-setup . --format json
This is not yet a confirmed code violation. Delivery is not evaluated.
```

A clean precheck with no existing native requirement should be shorter:

```text
Codeguard: no syntax anomaly observed in 18 checked Java files
Method: bundled Tree-sitter WASM; all 18 selected files checked.
Native lint: not run. Recommended: configure and prepare the native checker.
Not covered: types, project style rules and dependency security.
This is a syntax observation, not complete lint or delivery approval.
```

For a timeout, say “17 of 18 files checked; precheck incomplete”, identify the unresolved file and give the environment/parser recovery action. Do not say “all passed” or “source error”. For a native finding, show the actual checker/rule/location and original-tool recheck step. Do not guess missing tokens or promise a specific fix from a recovery node alone.

The plugin must deliver a bounded brief through the host's tool-result/context API; CLI JSON and files alone are not automatic conversation injection. Send initial results and meaningful changes, reuse stable tasks and avoid repeating unchanged installation requests. [Technical design](docs/Codeguard-Technical-Design.md) sections 7.3–7.4 include four complete scenarios, a proposed JSON example and review rules; none is a new supported report schema yet.

## 9. Data, security, and recovery

```text
.codeguard/                       # inside the checked project
├── .gitignore / README.md / workspace.json
├── project.json / module-graph.json / architecture.md
├── findings/                     # redacted facts and events
├── tasks/                        # readable repair projections
├── decisions/                    # intended decision references
├── reports/                      # ignored
├── runs/                         # ignored
├── cache/                        # ignored
├── worktrees/                    # ignored
└── state/                        # ignored
```

An existing `codeguard/src` remains user source and is still scanned. A previous `codeguard/workspace.json` is reported as a migration conflict; Codeguard does not move that directory automatically because it may also contain user code. Ignore rules are not a confidentiality boundary: do not commit secrets in records or share raw logs. Preserve the workspace before upgrading. Retry failed sync with `work sync`; changed inputs require a new scan.

Process groups, restricted environments, snapshots, and output limits are implemented building blocks, not a complete OS sandbox. Native builds can execute project plugins, scripts, and extensions. See [architecture](docs/Codeguard-Architecture.md).

## 10. Development and verification

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run --locked -p codeguard-cli --example gen_capability_docs -- --check
```

The 2026-09-28 local source baseline passed 163 groups: 990 tests passed, zero failed, 101 ignored; format and Clippy also passed. Ignored tests are not passes. This is a dated local baseline, not remote CI, cross-platform acceptance, a false-positive benchmark, or full native-tool coverage.

The native corpus example needs prepared tools. Replace these illustrative paths:

```bash
cargo run --locked -p codeguard-cli --example validate_corpus -- \
  --verify-ruff /path/to/ruff --verify-maven /path/to/mvn
```

Read the relevant [acceptance record](tests/acceptance) before enabling ignored native tests. Preserve distinctions between parser fixtures, simulated processes, actual native tools, and host integration.

## 11. Troubleshooting

| Symptom | Next action |
| :--- | :--- |
| `init --apply` exits `3` | Inspect created files and partial status; full readiness is pending |
| Findings plus incomplete result | Address both confirmed findings and listed blockers |
| Zero findings, still incomplete | Check configuration, scope, tools, policy, and advisory-data state |
| Managed `AGENTS.md` conflict | Preserve human changes and reconcile the marked block |
| Recurring task | Read current evidence; checkbox edits do not close it |
| `needs_decision` | Inspect attempts and change the repair approach or request a specific decision |
| Listed language cannot run | Consult adapter gaps; inventory is not implementation |
| Offline Cargo build fails | Populate the dependency cache or omit `--offline` |

## 12. Documentation, contribution, and license

The [documentation index](docs/README.md) assigns ownership and status to the architecture, technical design and eight topic guides. See the [command reference](docs/Codeguard-Command-Reference.md) for command contracts and [legacy compatibility](docs/Codeguard-Legacy-Compatibility.md) for historical plugin behavior, which does not describe current Rust availability.

| Document | English | 简体中文 |
| :--- | :--- | :--- |
| Architecture | [Architecture](docs/Codeguard-Architecture.md) | [架构设计](docs/Codeguard-Architecture.zh_CN.md) |
| Technical design | [Design and roadmap](docs/Codeguard-Technical-Design.md) | [技术方案与路线](docs/Codeguard-Technical-Design.zh_CN.md) |
| Capabilities | [Generated inventory](docs/CAPABILITIES.md) | Shared IDs and statuses |
| Evidence | [Acceptance records](tests/acceptance) | Each record states scope and date |

The existing [OpenSpec change](openspec/changes/introduce-rust-codeguard-cli/proposal.md) now lives in this Rust repository and is the sole specification and task authority. Implemented slices and remaining work are tracked separately; these documents do not maintain a second task ledger.

Contributions should preserve native rule meaning, add positive/negative cases, separate environment failure from findings, and update bilingual documents and affected schemas. Use [GitHub Issues](https://github.com/full-stack-plugins/codeguard/issues) for non-sensitive bugs. A dedicated security reporting policy is not yet present.

Cargo declares `Apache-2.0`; this working tree now includes a repository-level `LICENSE` and `NOTICE`, both included in the published npm package. No unverified crates.io or CI badges are provided.

Source builds now provide native-first edit feedback through `hook execute` / `hook claude post-tool-use`: only explicit ordinary files are selected; Python uses Ruff and JS/TS uses module-local ESLint 10. Coherent same-byte native results avoid duplicate parsing. Uncovered files may use pinned WASM candidates; mixed scopes retain native results, unwired native scopes and failures. Recovery nodes require native-tool setup/repair and confirmation; complete zero-recovery candidates only recommend native lint, never full acceptance. One deadline bounds at most eight files and two WASM workers; builds without WASM report that gap. The outer feedback is 0.7.0 with local `hook_fast_feedback` 0.2.0. Candidate task synchronization is connected; default plugin Hooks, capability-matched closure and real-host acceptance remain incomplete. See [edit-feedback acceptance](tests/acceptance/hook-fast-native-wasm.md).

Source edit feedback now imports recovery-bearing pinned WASM candidates into the existing `.codeguard/` workbench. Confirmation identities remain stable per workspace/file/language; Python and JS/TS reuse existing confirmation/preparation identities. Reports bind the grammar, source SHA-256, known limitations and suspected byte positions; imports reject mismatched identities/coordinates and duplicate JSON keys. Task IDs appear only after successful synchronization. Complete zero-recovery observations create no new mandatory task; unlocated incomplete scans create check-recovery tasks. Neither closes existing tasks. Dialogue includes `task show` / `task verify`; missing native confirmation adapters report an explicit capability gap. Outer Hook feedback is 0.7.0, local feedback is 0.2.0, and generic `next` briefs use 0.3.0 while existing checkers retain 0.1.0. Default plugin Hooks, capability-matched closure and actual-host acceptance remain incomplete.


### Native verification of a Zig confirmation task

Source builds can now verify a persisted Zig WASM confirmation task with `codeguard task verify TASK_ID . --zig-tool /absolute/path/to/zig --format=json`. The same explicit tool option is accepted by `hook execute` for `repair_ready`, using the existing lease and finished-attempt binding. `lint zig` can run the native tool without building the optional WASM feature; fallback without that feature reports its absence.

The pinned Zig 0.16.0 probe runs `version` and `ast-check --color off` against the exact source bytes under one deadline. A current native diagnostic changes the brief to source repair; missing tools, unsupported versions and execution failures retain environment/decision guidance. Fresh `next` briefs carry the unchanged tool path in their recheck argv. Changes to source or tool bytes invalidate old diagnostic guidance. Reports and attempts are retained under the existing workbench instead of a second task store.

The native observation is `syntax_task_recheck` 0.1.0, wrapped by `task_verification_preview` 0.12.0. Generic repair briefs are 0.3.0; old 0.2.0 briefs and 0.11.0 verification schemas remain available unchanged. Zero native AST diagnostics are `candidate_absent_unverified_policy`: they end the pending local verification step, but do not close the task or certify project lint, build or delivery. Other generic languages still lack native confirmation adapters. See [the acceptance record](tests/acceptance/syntax-native-task-verification.md).

Briefs also expose the latest native report reference/digest and current diagnostic positions; stale inputs suppress those positions. Only immutable grammars compiled into the binary reuse validated asset identities within a process; external manifests, source and native tools still require current-byte checks.


### Source SDK: task closure after native verification

`verify_zig_task_resolution` lets a protected host verify a task-specific signed policy, compare the original counterexample and current bytes with the same Zig 0.16.0, and append resolution, investigation or recurrence events. Ordinary `task verify` can record recurrence with the matching tool and reopen the same task; existing leases and attempt receipts remain in use.

The default plugin and public CLI still lack the trusted policy provider, and the npm launcher does not expose this protected API. Editing task state or reading a local closure file cannot close an issue or approve delivery. See the [technical design](docs/Codeguard-Technical-Design.md) for the API, execution graph and full report, and [lifecycle acceptance](tests/acceptance/task-resolution-lifecycle.md) for its current scope.

The current source also passes an offline npm package edit/task-recheck flow through the installed Node launcher; [acceptance evidence](tests/acceptance/npm-repair-local-package.md) distinguishes this from the published 0.1.3 package and actual host automation.


### Current public candidate: 0.1.4

`@partme.ai/codeguard@0.1.4` is published for Apple Silicon macOS from clean source `1cd458f6e01a44a74388243e964e3f45290ac18e`. It includes all 32 runnable, unqualified grammars, bounded edited-file checks, stable native-confirmation tasks, native rechecks and `next` guidance. Registry hashes, a fresh-cache npx invocation, the actual public-package repair loop with Zig 0.16.0, and the source commit's Linux CI passed. Ordinary CLI clean output cannot close a task without trusted policy. The protected Zig SDK is a source integration API; npm does not expose a self-approval command. Plugin activation, installed-host acceptance, full precision, other platforms and complete gates remain open. Earlier 0.1.3 evidence is historical. See [0.1.4 acceptance](tests/acceptance/npm-0.1.4-candidate.md).

### Aggregate Erlang native-first checking (current source)

```bash
codeguard check all . --format=json
codeguard check all . --erl-tool /absolute/path/to/erl --timeout 30s --jobs 2 --format=json
```

The `erlang.lint` task selects explicit Erlang or the first executable `erl` from absolute PATH directories, then uses the existing controlled OTP 28 scanner/parser. It observes at most 64 ordinary UTF-8 files, each at most 1 MiB, under the request deadline and scheduler concurrency. `native_results.erlang_lint` carries current source hashes, positions, tool selection, per-file next actions and original-tool recheck argv with an absolute source path, reusable outside the project directory. Files beyond the native budget are counted as unobserved.

Only complete, non-preprocessed forms observations for matching source and tool bytes skip duplicate WASM. Missing tools retain candidate observations; selected-tool failures stay visible alongside any supplementary candidate observation. Macro/preprocessor coverage remains unresolved. Source or tool changes withdraw affected positions and reusable argv. Project-scope changes set `scope_stable: false` and retain still-current single-file diagnostics; whole-scope completeness is withdrawn. SIGINT remains exit 130; JSON, human and conservative SARIF retain native findings without claiming project success.

Initialized workspaces now persist native-first observations directly, without a preceding WASM error: current `check_feedback` **0.38.0** embeds scan **0.2.0** with actual per-file `task_id` or `task_sync_reason`. `lint erlang FILE` binds the nearest existing workbench and emits **0.3.0**. Repeated scans and historical WASM evidence reuse one task; missing tools and preprocessing produce environment tasks. Unbound aggregate checks also use 0.38.0 and historical protocols remain available; `check_aborted` remains **0.13.0**, and prior schema bytes are unchanged. Default-host trusted closure and complete project checks remain open; the source SDK extension is described below. Public npm 0.1.4 does not contain this batch. See [native repair workflow](docs/Codeguard-Native-Repair-Workflow.md) and [acceptance](tests/acceptance/erlang-native-first-workbench.md).

### Development evaluation of all 32 grammars

The Rust `evaluate_grammars` example replays 358 fixed cases through the existing isolated workers across 32 languages and 35 language×source cohorts, including 150 Dart upstream cases in their own cohort. Reports retain per-cohort TP/FP/FN, Wilson intervals, unknowns, pending labels and cold-worker timings; mixed-source language summaries do not pool precision/recall. Corpus/source/program identities are bound, and pending COBOL/CFQuery labels stay outside metrics. Legacy 0.1 inputs and reports remain readable. This does not run native oracles or an independent holdout, qualify a grammar or approve a release. See [grammar evaluation](docs/Codeguard-Grammar-Evaluation.md).


### Erlang task resolution and recurrence (current source SDK)

`verify_erlang_task_resolution` reuses Zig's signed-policy verification, shared deadline, lease, attempt handoff and append-only parent chain. The protected host independently fixes the trust root, workspace, policy revision, baseline and trusted clock; project files cannot grant approval. OTP 28 scans/parses the original counterexample and current bytes separately. Only an original native diagnostic, changed source and a complete clean current result can record `code_fixed`. Macros/includes, empty forms, truncation and execution failures require verification; an originally valid sample requires false-positive investigation.

Both WASM-first and native-first tasks are supported. Erlang policy 1.1.0/evidence 0.2.0 stay separate from Zig 1.0.0/0.1.0. Native-first evidence requires `grammar_sha256=null` and the original tool identity. The internal rule identity uses the actual approved policy-byte digest instead of inventing a grammar digest. History checks bind the language, source and grammar to the first report; rehashing local files cannot switch languages. Ordinary `task verify --erl-tool` can append recurrence with the matching tool, but cannot close a task without trusted policy.

This remains a source SDK without the default plugin's trusted policy provider. Public npm 0.1.4 lacks this extension; syntax receipts do not certify complete lint, security or project delivery. The tool digest binds the launcher; the host must independently protect the OTP environment. See [Erlang lifecycle acceptance](tests/acceptance/erlang-task-resolution-lifecycle.md) for the execution path and actual results.

### Cargo input and proxy boundaries (current source)

Normal Clippy scans and same-rule `--force-warn` comparisons use `cargo clippy --locked --offline --all-targets --message-format=json`. A missing root Cargo.lock returns `cargo_lock_unavailable` before native startup and does not create a lock. Source fingerprints come from a bounded pre-run snapshot. Changes to observed sources, the manifest, lock, root Clippy/Cargo/toolchain configuration or selected tool withdraw the current Clippy findings and retain preparation/recheck work. Valid partial diagnostics under stable inputs remain visible without claiming a complete report.

For Cargo proxies such as rustup that dispatch by entry-point name, Clippy, rustdoc and build checks execute the selected Cargo path while validating the resolved bytes and post-run target identity. A separate sequence prevents private Clippy directories from colliding at identical timestamps. This protects observed inputs only; it does not prove the complete effective Cargo model, every build combination or a process sandbox. Local zero diagnostics still cannot close tasks automatically. Tests, failures and actual output are in the [Cargo input acceptance record](tests/acceptance/rust-clippy-input-stability.md). Public npm 0.1.4 does not include this batch.


After native aggregation, current source uses non-cached Clippy `compiler-artifact` records, the original root-manifest identity and pre-run source hashes to avoid duplicate WASM for byte-matching Rust target entry points. Coverage stays in the current process and cannot be restored from editable reports. Failures, changed inputs/tools, duplicate JSON keys or events after the finish record grant no coverage. `all-targets` does not establish that the entire directory was parsed: unproven modules and conditionally excluded files retain prechecks. Native warnings, stable tasks and complete delivery obligations remain. Diagnostics repeated across library/test targets at the same file/rule/line/column project one finding, preferring error over warning; distinct positions remain separate and historical duplicate tasks are not automatically closed. See [Rust native-first acceptance](tests/acceptance/check-all-native-preferred-rust.md). Public npm 0.1.4 does not include this batch.


### Existing Cargo discovery (current source)

`check all`, `comments rust`, `build rust` and their Cargo task rechecks accept an optional `--cargo-tool`. Without it, they select the first regular executable `cargo` in an absolute invoking-PATH directory. Empty/relative directories and non-executable entries are skipped. An explicit invalid tool or selected native failure remains a concrete blocker; it never triggers a search for another Cargo. Aggregate Rust checks share the selected entry.

The final `cargo` entry name is retained for rustup proxy dispatch; resolved bytes and target identity still undergo the existing checks. The child inherits `RUSTUP_TOOLCHAIN` and always receives `RUSTUP_AUTO_INSTALL=0`, even if the caller set it to `1`. Cargo `--locked --offline` alone does not prevent rustup from installing a missing toolchain; [rustup documents the separate control](https://rust-lang.github.io/rustup/environment-variables.html). A missing toolchain stays an environment failure, without a source violation or automatic installation. This does not qualify the entire toolchain, effective Cargo configuration, complete project coverage or trusted task closure.

Controlled proxy/no-fallback cases and actual installed/missing-toolchain observations are recorded in [Cargo discovery acceptance](tests/acceptance/cargo-native-discovery.md). Public npm 0.1.4 does not include this source change.


### Project-root Ruff discovery (current source)

For a configured Python scan, `lint python`, `check all`, selected-edit Hook execution and same-root task rechecks select an explicit `--ruff-tool` first, then the checked root's `.venv/bin/ruff`, then an executable in an absolute invoking-PATH directory. They do not activate a shell environment, enumerate installed packages or install Ruff. A root-local entry is version-probed and byte-bound through the existing native scan. Missing local directories/entry permit PATH discovery; linked/non-directory parents, broken or non-executable entries produce `ruff_local_tool_invalid` instead of silently switching to global Ruff. A chosen tool's version/execution failure never tries a different tool.

An invalid local environment projects one stable preparation task with `.venv/bin/ruff` evidence, bounded environment repair, original Codeguard recheck, history and closure conditions. No applicable Ruff configuration means no tool startup. Ordinary local parents may contain an executable symlink: the resolved tool bytes are fixed and rechecked. Local success and repaired-source zero diagnostics still do not approve policy or close tasks automatically. This selection concerns the explicitly checked root; per-module virtual environments, uv/Poetry/Conda resolution, real host integration and complete coverage remain separate work. See [root-local Ruff acceptance](tests/acceptance/ruff-local-discovery.md). Public npm 0.1.4 does not include this source change.


### Recovery tasks for unlocated syntax observations (current source)

`check all/java` and confirmed-write Hooks share syntax-task synchronization. Visible recovery nodes require native confirmation. A tree error with an incomplete recovery scan and no locatable nodes creates a check-capability recovery task: it retains an empty recovery array and `syntax_recovery_incomplete`, without a confirmed source violation or invented edit position. Complete zero-recovery observations recommend native checking without creating this task.

The workspace/file/language identity remains stable across repeat checks, Hooks and later visible recoveries. `next` and task Markdown require restoring the checker, language version or grammar; source edits are forbidden before native confirmation. `task verify` records a missing native adapter while retaining the open task. Zero recoveries, installation and checkbox edits cannot close existing tasks. Persistence failures retain observations and per-file reasons without fabricated task IDs.

Aggregate feedback uses `check_feedback` 0.38.0 with `syntax_tasks`; unlocated local confirmation reports use 0.3.0. Existing located 0.1.0 and native-first Erlang 0.2.0 reports and their schemas remain unchanged. Claude Hook command replay distinguishes recovery-node counts from incomplete recovery scans; actual-host and full-language acceptance remain open. Public npm 0.1.4 does not contain this batch. See [acceptance and actual reports](tests/acceptance/unlocated-syntax-recovery-tasks.md).


### Automatic native Erlang discovery for task rechecks (current source)

`codeguard task verify "$TASK_ID" . --format=json` (with `TASK_ID` set to the actual `task_id` returned by `next`) reuses lint/check selection for existing Erlang syntax tasks: explicit `--erl-tool` takes priority; otherwise the first executable erl in an absolute invoking-PATH directory is fixed. Rechecks verify OTP 28, current source and tool bytes while retaining leases, budgets, events and existing report versions. Both candidate-origin and native-first tasks can be rechecked; repair-ready Hooks use the same entry.

Only an absent tool produces `erlang_tool_not_found_on_path`; empty/relative PATH entries and non-executable files are ignored. An invalid explicit tool, unsupported selected version or execution failure never selects a later tool or installs one. After a valid observation, `next` supplies explicit recheck argv for the actual tool so later PATH changes cannot replace that entry. Local zero diagnostics still do not close tasks automatically; preprocessing, trusted policy and complete project capability remain separate obligations. See [recheck discovery acceptance](tests/acceptance/erlang-recheck-discovery.md). Public npm 0.1.4 does not contain this batch.

### Native confirmation of unlocated Swift observations (current source)

An existing Swift confirmation task now accepts `codeguard task verify "$TASK_ID" . --swift-tool /absolute/path/to/swiftc --format=json`, where `TASK_ID` comes from actual `next` output. `hook execute` accepts the same option for `repair_ready`. The caller supplies an installed Apple Swift 6.4 compiler; this path does not install tools or execute editable paths from historical reports. Missing tools receive an existing-compiler recovery action; unsupported versions and execution failures retain the task.

Rust reads and rechecks bounded source bytes, then runs `swiftc -frontend -parse -diagnostic-style llvm -no-color-diagnostics -` with frozen stdin, `/` as cwd, a cleared environment and the shared deadline. Only native error rules and positions enter agent guidance; raw diagnostic text is not treated as an instruction. Columns are UTF-8 bytes and must fall on character boundaries. Unknown output, exit/diagnostic contradictions, timeout, invalid positions and tool changes remain incomplete. The tool digest binds the launcher, not the entire Swift installation.

Native errors make the same task actionable for source repair. Source or tool changes withdraw old positions. A clean parse records `candidate_absent_unverified_policy` without automatically closing the task or satisfying project lint, type checking, macro/conditional-compilation context, build, security or delivery obligations. Existing no-progress budgets still apply. Swift grammar qualification and the 32-language precision conclusions are unchanged; public npm 0.1.4 does not contain this extension.

New protocols are `syntax_task_recheck` 0.4.0, `task_verification_preview` 0.15.0, `repair_brief_preview` 0.6.0 and Hook feedback 0.9.0 (task summary 0.3.0). Aggregate `check` uses 0.39.0 when `next` contains a native Swift brief; other paths retain 0.38.0. Historical schemas remain unchanged. See [Swift native-confirmation acceptance](tests/acceptance/swift-native-task-confirmation.md) for actual reports and limits.


### Actual Claude lifecycle acceptance and fresh-task guidance

Claude Code 2.1.273 now has actual session-local plugin-source evidence for confirmed saves, repeated saves, an execution-time filesystem write failure, native Zig 0.16.0 rechecks before/after repair, stable task identity and one Stop continuation followed by a reentry guard. The locked public runtime remains 0.1.4, independently of current development source. Native diagnostics changed from one to zero; the task stayed open and delivery remained not evaluated. Edit argument validation failed before execution and produced no failure Hook in this host; the separate EACCES write exercised PostToolUseFailure. This is not marketplace installation, trusted closure, complete native-first coverage or multi-host acceptance. See [actual host evidence](tests/acceptance/claude-host-prepared-runtime.md).

The actual session exposed incorrect first-task guidance claiming the Zig adapter was unavailable although its native recheck worked. Development `next` / `task show` now distinguish implemented Zig 0.16.0, OTP 28 and Apple Swift 6.4 adapters from unverified local tool readiness, using the bound original report and explicit tool arguments. Read-only guidance does not execute or install a checker, trust executable paths in editable task records, authorize source repair before native confirmation, or close a task. Unsupported languages retain a concrete capability decision; existing native history keeps precedence. This correction is not in the locked public 0.1.4.

Fresh native-confirmation guidance uses `repair-brief-preview` 0.7.0 to bind the language, supported tool version, tool-selection option, and `tool_readiness: not_evaluated`. Aggregate checks use `check-feedback` 0.40.0; `task show` preserves the same recheck arguments in its outer action. The agent must replace the placeholder with a verified installed tool path. Preparation does not establish readiness or native execution. Historical schema definitions remain unchanged.

### Native-first Kotlin single-file checking (development source)

`codeguard lint kotlin FILE.kt [--kotlinc-tool ABS_PATH] [--timeout 30s] --format=json` selects an explicit tool first, otherwise the first `kotlinc` in absolute invoking PATH entries. The current adapter supports Kotlin/JVM 2.4.10. Only an absent tool uses bundled WASM; a selected native failure remains visible. Frozen ordinary `.kt` files are compiled privately, without `.kts`, project build scripts or compiler plugins.

Only `[SYNTAX]` becomes `kotlin.syntax`; other diagnostics retain project-context limitations. Verified UTF-16 columns are also mapped to UTF-8 byte columns. Unknown output, redirected launchers, changed input and interrupted budgets cannot produce success. Missing backends or incomplete native checks require further native confirmation; an observable, complete zero-recovery WASM scan recommends native installation without granting project approval. The feedback schema is `kotlin-lint-feedback` 0.1.0.

Existing stable tasks now support task verify and repair_ready; aggregate feedback projects their current guidance. Initial check/file_changed now selects Kotlin native compilation first and synchronizes stable tasks. Complete Kotlin lint/comments, compiler JAR/JDK identity, version coverage and release acceptance remain open. Public npm 0.1.4 excludes this increment. See [single-file acceptance](tests/acceptance/kotlin-native-single-file.md).

Development source also supports existing Kotlin confirmation tasks through `codeguard task verify TASK_ID . --kotlinc-tool ABS_PATH --format=json`. `repair_ready` accepts the same option and preserves the stable task/history. Mixed observations separate actionable syntax positions from unresolved context. See [task-recheck acceptance](tests/acceptance/kotlin-native-task-confirmation.md).

### Kotlin first native observation (development source)

`check all . --kotlinc-tool ABS_PATH` and confirmed `file_changed` accept the same explicit tool; omission discovers invoking PATH. A selected native failure stays visible and does not switch to WASM. Only an absent compiler uses the existing candidate fallback. At most 64 ordinary `.kt` files share the request deadline; `.kts` is not compiled by this path. Repeated observations update one stable task, and native zero diagnostics does not automatically close it.

New reports use aggregate0.42.0, file-changed Hook0.11.0, scan0.1/0.2, native origin0.4.0, native-origin recheck0.6.0/task preview0.17.0 and first-native brief0.10.0. Existing WASM-origin rechecks retain their previous protocols. See [first-native acceptance](tests/acceptance/kotlin-native-first.md). Full project lint, compiler/JDK identity, trusted closure and public release remain open.

### Native-first Swift single-file feedback (development source)

`codeguard lint swift FILE.swift [--swift-tool ABS_PATH] [--timeout 30s] --format=json` selects an explicit compiler first, otherwise the first executable `swiftc` in absolute invoking PATH entries. The current adapter supports Apple Swift 6.4 frontend parsing of frozen stdin. Only tool absence enables the bundled WASM candidate; a selected compiler's failure stays visible. Verified positions use UTF-8 byte columns. Source changes or entry redirection withdraw diagnostics, and timeout/cancellation remain incomplete/cancelled rather than source violations.

The `swift-lint-feedback`0.1.0 report distinguishes a required native confirmation for incomplete or hidden recovery from a recommendation after complete zero recovery. It never grants full lint or delivery approval: SwiftLint, comments, type checking, dependencies/security and project builds remain obligations. Compiler identity covers the selected entry, not the entire toolchain. Project-wide Swift native observation is now connected as described below; full language/release qualification remains open; initialized-workspace task connection is described below. Public npm0.1.4 excludes this increment. See [single-file acceptance](tests/acceptance/swift-native-single-file.md).

### Swift project native observation (development source)

`check all . [--swift-tool ABS_PATH] --format=json` parses at most 64 ordinary Swift files under one shared deadline and reports unobserved files. Selected native failure never changes into a WASM result; only tool absence keeps candidate fallback. `check-feedback`0.43.0 / `swift-parse-scan`0.1.0 preserve current byte locations, selection and recheck argv. Source/tool changes withdraw positions; zero diagnostics do not prove SwiftLint, types or builds. This historical protocol describes an uninitialized workspace with `not_connected` task references; initialized-workspace task connection is described below. See [project observation acceptance](tests/acceptance/swift-native-project.md).

Confirmed saves through `hook execute . --swift-tool ABS_PATH` reuse the Swift scanner for selected paths only. Missing tools keep candidates; selected failures do not switch. `hook-execution-feedback`0.12.0 / `hook_fast_feedback`0.4.0 expose current native positions and the unconnected task state. The CLI Claude adapter emits bounded counts/byte positions without tool diagnostic text; mixed candidate errors still require native confirmation. Actual installed-host acceptance, stable native tasks and the full repair loop remain open. See [save feedback acceptance](tests/acceptance/swift-native-hook.md).

### Swift native-first repair workbench (development source)

In an initialized `.codeguard/` workspace, `check all` and a confirmed successful-save `hook execute file_changed` synchronize Swift native syntax findings or environment blockers into stable tasks. Repeated observations reuse the same task. `next` and `task show` expose evidence, rule basis, allowed scope, repair steps, recheck commands, history and closure conditions. An uninitialized workspace reports a disconnected workbench; synchronization failures remain blockers. Standalone `lint swift` does not create tasks yet.

`codeguard task verify TASK_ID . [--swift-tool ABS_PATH] --format=json` selects an explicit tool or discovers one in the invoking PATH for native-first tasks. Existing WASM-origin tasks retain their explicit-tool contract; editable historical paths are not executed. Diagnostics request source repair. Zero diagnostics records `candidate_absent_unverified_policy`; the same task stays open pending full lint, type, build and delivery checks.

Native-first protocols use observation 0.5, scan 0.2, check 0.44, initial brief 0.11, save Hook fast 0.5 / outer 0.13, and recheck inner 0.7 / outer 0.18. Existing verification briefs retain 0.6, and historical schemas remain unchanged. Actual Apple Swift 6.4 executions verified stable task references across repeated scans, saves and rechecks before and after repair. This is not installed-host or trusted-closure acceptance and is excluded from public npm 0.1.4.

### Recover missing task projections

`codeguard work sync . --format=json` now restores missing Markdown for committed facts under the workspace sync lock, imports new reports, and rechecks missing projections. Recovery reads structured facts, the current RepairBrief, original report hashes and consumption markers. It does not run checkers. Existing regular task files, including notes and checkmarks, retain their exact bytes. Symlinks, directory conflicts, invalid facts, changed origin hashes and uncommitted origins remain incomplete. Recovery examines at most 1,000 facts.

The readable task contains evidence, rule basis, allowed scope, steps, recheck argv, history and closure conditions. Local absolute paths in recheck arguments become verification placeholders; use `task show` for current instructions. Recovery never closes findings, changes original facts/events/consumption markers or attempt history, or grants delivery approval. Reports use `work_sync_preview` 0.3.0 with a positive `restored_task_projections` count only when recovery occurs; other runs retain 0.2.0.

`next` can continue with a physically independent source finding while retaining waiting or budget-exhausted findings and their query references. Prerequisite blockers and unproven independence preserve conservative handling; see [acceptance](tests/acceptance/next-independent-source-work.md).

### Scoped Swift syntax task lifecycle (development source)

A protected host can call `verify_swift_task_resolution` to compare the original counterexample and current source with Apple Swift 6.4. Policy 1.2.0 and evidence 0.3.0 remain separate from Zig/Erlang protocols; native-first tasks retain a null grammar identity. An original parse diagnostic, changed source, and a complete clean recheck with the same tool permit a scoped `code_fixed` event. Repeated verification is idempotent; an ordinary `task verify --swift-tool` recheck reopens the same parent chain on recurrence. A valid original sample requires false-positive review; tool failures or identity changes cannot close the task. The host supplies independent trust and signatures. Default plugin integration and public distribution remain incomplete, as do SwiftLint, type checking, project builds, and full delivery acceptance. See [lifecycle acceptance](tests/acceptance/swift-task-resolution-lifecycle.md).

### Scoped Kotlin resolution and context blockers (development source)

`verify_kotlin_task_resolution` reuses the host SDK with kotlinc-jvm 2.4.10. Policy 1.3.0 and evidence 0.4.0 are language-specific; native-first tasks keep a null grammar identity. The original diagnostic must have coherent UTF-16 and UTF-8 coordinates. Changed source and a complete clean recheck permit a scoped `code_fixed` event. Context-only errors remain pending verification. Mixed syntax and context errors retain the known finding and allow ordinary `task verify --kotlinc-tool` recurrence to reopen the same parent chain even when completion is incomplete. The host supplies independent trust. Tool identity currently covers the launcher, with full JAR/JDK/project identity, default plugin closure, lint/type coverage, and publication still pending. See [acceptance](tests/acceptance/kotlin-task-resolution-lifecycle.md).

Current development source accepts all 57 canonical registry IDs for `check`, reusing integrated native services and scoped WASM fallback. Missing adapters remain gaps. New scoped feedback 0.45 and aborted feedback 0.14 are separately versioned; Java/all retain compatibility. Public npm 0.1.4 and the plugin lock do not contain this increment. See [language-selection acceptance](tests/acceptance/check-language-selection.md).

Development-source Zig single-file lint and task verification now share explicit/PATH tool selection. Feedback 0.2 records provenance and current source; changed inputs withdraw stale positions. Actual Zig0.16.0 explicit/PATH positive and negative cases were replayed. Full aggregate Zig checking, lint/build and release remain incomplete; the public package is unchanged. See [acceptance](tests/acceptance/zig-native-discovery.md).


Development-source Zig native routing (2026-10-05): `check all`, `check zig` and selected-file editing now reuse the frozen Zig 0.16.0 AST probe. Explicit/PATH selection runs native first; selected-tool failures retain an incomplete observation without switching to WASM. Missing tools retain candidate fallback. Source or tool-entry changes withdraw old positions; at most 64 files are observed under the shared deadline, with remaining scope visible. Check feedback 0.46, aborted feedback 0.15 and Hook feedback 0.14 are separate protocols; reports without Zig keep previous versions. Claude feedback includes bounded native rule IDs, positions and the original recheck instruction. First-native Zig task creation is now connected as described below: native reports expose only actually synchronized task IDs, and clean AST observations do not close historical tasks or prove complete lint/build. Public npm 0.1.4 and the plugin lock are unchanged.

### Native-first Zig tasks (development source)

In initialized workspaces, `check zig`, `check all`, `lint zig` and edit Hooks synchronize one stable task per workspace/source scope. A newly clean file creates no repair task. Clean rechecks of existing tasks record `candidate_absent_unverified_policy` and retain the open task. `next` and `task show` expose bounded native positions and original-tool arguments; `task verify TASK_ID . [--zig-tool ABS_PATH] --format=json` and repair_ready record native rechecks. Native-first grammar identity is null; native diagnostics must not request grammar edits.

New protocols: observation 0.6, bound scan 0.2, aggregate 0.47, single-file 0.3, brief 0.12, task-show 0.2, recheck inner 0.8/outer 0.19, edit Hook 0.16 and recheck Hook 0.15. Historical schemas remain unchanged. Actual Zig 0.16.0 validated broken and repaired inputs. Trusted SDK closure for native-first Zig tasks is described below. Default installed-host integration, full lint/build and distribution remain incomplete. Public npm 0.1.4 is unchanged. See [acceptance](tests/acceptance/zig-native-first-workbench.md).

### Trusted native-first Zig resolution (development SDK)

A protected host can call `verify_zig_task_resolution` with independently signed policy 1.4.0 for native-first observation 0.6.0. Grammar must be null; legacy WASM-origin policy 1.0.0 retains its scope. An original same-tool diagnostic, changed source and a complete clean same-tool recheck permit scoped `code_fixed` with evidence 0.5.0. Repeated verification is idempotent; ordinary `task verify --zig-tool` positive recurrence reopens the same parent chain. Valid original samples require false-positive review. Unexpected output, invalid positions, changed tools or missing trusted context cannot close the task. The host independently supplies trust. Default plugin/public npm integration of that provider remains incomplete; local task records do not authorize delivery. See [acceptance](tests/acceptance/zig-native-first-resolution.md).

## Explicit native differential development replay

The Rust `evaluate_native_grammars` example selects samples for explicitly supplied Zig0.16.0, OTP28, Apple Swift6.4 and kotlinc-jvm2.4.10 from the same frozen 32-language corpus. It reuses native adapters and the existing WASM worker. All 32 languages remain in inventory; unselected tools, missing adapters, incomplete native observations and hidden WASM recovery stay unresolved. TP/FP/FN/TN include only jointly decidable syntax samples. Located Kotlin syntax diagnostics survive mixed context blockers while execution remains incomplete. Changed tools/entries or program bytes withdraw classifications. Reports remain incomplete with zero qualified grammars and create no tasks or whitelist approvals. Reused adapters, regression samples and entry-artifact hashes do not prove independent holdout or full toolchain identity. See [native differential acceptance](tests/acceptance/native-grammar-differential.md) for commands and actual evidence.


### Isolated Python native syntax comparison

Development replay now accepts explicit Ruff0.16.8 with the Python3.12 target. Frozen stdin, `--isolated --select E9 --ignore-noqa --no-cache` excludes project configuration and ordinary lint. Only consistent located `invalid-syntax` diagnostics classify syntax errors; F401, wrong paths/positions, changed versions or contradictory reports remain incomplete. Report0.2 preserves historical0.1 bytes and does not expand trusted task-closing authority. The18-case actual replay found two WASM false negatives: an empty function suite and incorrect indentation. Python remains unqualified; see [Python native differential acceptance](tests/acceptance/python-native-grammar-differential.md).

Python development differential 0.3 measures raw grammar separately from the parser-plus-structure candidate. Existing `comparison` and TP/FP/FN/TN stay intact; verified rule identity and source positions, `combined_candidate_comparison` and a separate denominator are added. Truncation, cancellation and program changes never become clean candidates; native identity changes withdraw both comparisons. Candidates still require native confirmation, historical 0.1/0.2 reports remain unchanged, and neither qualification nor delivery authority expands. See [layered acceptance](tests/acceptance/native-structure-differential.md).

JavaScript development differential now uses fixed Node 24.18.0 with isolated `--check --input-type=module`, a cleared environment, frozen stdin, a shared deadline and entry/artifact checks before and after execution. Report 0.4 records the explicit module goal and only verified source-line locations; unknown output or tool changes remain incomplete. It neither infers project CommonJS/ESM settings nor replaces ESLint. Node has a separate 128 MiB artifact budget; other checker budgets stay at 64 MiB. See [JavaScript native acceptance](tests/acceptance/javascript-isolated-native-differential.md). The actual 18-case module comparison retains 5 TP / 0 FP / 2 FN / 11 TN: top-level return and duplicate bindings remain missed, with 0/32 qualified grammars.

### Separate Python structure evidence and stable confirmation tasks

Source-built Python fallback now preserves `codeguard.python.required_suite` evidence separately from ERROR/MISSING. Uninitialized single-file lint uses0.16; initialized feedback uses0.17, with confirmation report0.2 binding source, fixed rule configuration and grammar digests. Rescans keep the same task and later valid candidate observations do not close it. Actual Ruff confirms empty suites and incorrect indentation; local zero findings still lack trusted closure authority. Historical raw-grammar false-negative reports, native priority, zero qualifications and unevaluated delivery remain intact. Trusted closure remains incomplete; aggregate integration is described below; see [acceptance](tests/acceptance/python-structure-lint-task.md).

`check python` / `check all` now preserve separate structure observations and actual counts in feedback0.48 and reuse the stable lint task. Generic confirmation0.7 validates fixed rule configuration and original byte coordinates. Valid candidate observations do not close earlier tasks; trusted closure remains incomplete. See [aggregate acceptance](tests/acceptance/check-python-structure.md).

Source-built edit Hook feedback0.17 (local0.8) also retains separate structure counts and requires native confirmation when present. The Claude-compatible summary includes rule and count; this is not installed-host acceptance or a change to published plugin defaults.

Python candidate-confirmation `task verify` now runs only the first report’s bound file through project Ruff configuration. Versioned feedback preserves the consumed original report and current input; it does not close the task or certify the project. [Scoped recheck acceptance](tests/acceptance/python-confirmation-scoped-recheck.md).


### Python native syntax repair feedback (0.19 / 0.21)

Matching `invalid-syntax` diagnostics in Ruff normal and ignore-noqa runs remain native syntax errors instead of suppression-audit failures. An EOF diagnostic can use the preceding nonempty source as its stable task anchor without changing native coordinates. Single-file confirmation feedback 0.19 and task preview 0.21 distinguish remaining syntax errors as `still_present` and provide source-repair guidance. A complete recheck without syntax errors only yields `candidate_absent_unverified_policy`; it cannot close a task. Historical 0.18 reports retain their event semantics, and source or configuration changes withdraw current conclusions. See [native syntax feedback acceptance](tests/acceptance/python-native-syntax-audit.md).


### Python resolution prerequisites: original source and target version

The read-only host API `validate_python_task_original_source(root, task_id, source)` checks both first-report families, consumed receipts, frozen bytes and recovery/structure positions. After repair, it still accepts the original bytes and rejects substituting current bytes. The isolated syntax probe can receive an explicit Python target and rejects invalid targets before tool resolution; the existing development differential retains py312. These prerequisites do not provide signed approval, project target provenance or trusted resolution. See [acceptance](tests/acceptance/python-resolution-prerequisites.md).


### Native Ruff lint target observation

`RuffSettingsObservation::explicit_python_target()` returns only an explicit lint target observed in the pinned Ruff settings. It requires one `linter.unresolved_target_version` and an empty `linter.per_file_target_version`; implicit none, missing/duplicate fields, unknown versions and unresolved per-file targets return specific reasons. Formatter/analyze targets are not substitutes. This internal observation shares the native tool/source/configuration checks, leaves the public settings shape unchanged and does not authorize resolution. See [acceptance](tests/acceptance/python-native-target-settings.md).


### Shared event commit for scoped tasks

Native syntax services now separate rechecks from `commit_resolution`. Before committing, domain evidence must match the redacted native pair for identity, original/current source, tool, adapter, grammar/approved policy and native-pair digest. Language entry points still verify signatures and execute native rechecks. Shared commit logic retains idempotency, policy-change reconciliation, parent-chain conflicts and recurrence, without granting project delivery. This prepares Python integration; Python trusted resolution is not implemented yet. See [acceptance](tests/acceptance/task-resolution-commit-boundary.md).


Python syntax confirmation tasks now support scoped host SDK resolution. Independently signed policy 1.5 binds original source, explicit lint target, project configuration and the Ruff artifact. Complete original/current native comparison and stable inputs are required for a code-fix event. Ordinary `task verify` can record recurrence under the same bindings, but cannot authorize closure from local history. This does not certify production host integration, all languages or project delivery gates; see the [scoped acceptance record](tests/acceptance/python-task-resolution-lifecycle.md).

Python native counterevidence now reaches `next`: preserve valid source and investigate grammar/version differences. Current source or configuration changes require fresh verification before historical counterevidence can be used. See [counterevidence acceptance](tests/acceptance/python-template-string-counterevidence.md).


### Syntax confirmation actions and retry budgets

A Python syntax-confirmation task uses `repair-source` when its current native observation is `still_present` and source/configuration bindings remain valid. Missing tools, incomplete observations and stale inputs do not authorize source edits from historical positions. For the same syntax-confirmation task and unchanged input, `restore-checker-environment` and `repair-source` share the existing no-progress budget. Switching actions cannot erase failed attempts. Append-only events retain their original action and fingerprint; other task families keep their existing accounting. New inputs and verified progress follow existing reset rules. Exhaustion requires a concrete diagnosis or decision and grants neither closure nor gate acceptance.


### Unified scope for TypeScript module sources

`detect`, `check all` / `check typescript` and `file_changed` hooks now include `.mts/.cts` and `.d.mts/.d.cts` in the existing TypeScript scope. They use the pinned TypeScript grammar, never TSX. Complete same-file native ESLint observations retain priority; files in another build root without native context keep independent candidate fallback. Repeated suspected results update the same confirmation task; valid declarations create no new syntax blocker. Edit hooks check only confirmed changed files. Extension recognition does not certify module resolution, type checking, grammar qualification or delivery. See [module-extension acceptance](tests/acceptance/typescript-module-extension-routing.md).


### Explicit R and C++ source suffixes

Project discovery and WASM candidate routing include `.R`/`.r` and C++ `.C`/`.cp`/`.CPP`/`.c++`/`.cxx`/`.hxx`, retaining existing suffixes. Case remains significant: `.C` uses C++, while shared `.h` gets no speculative C++ grammar route. Missing native tools still produce actual candidate observations and `incomplete` delivery; suffix coverage does not qualify a grammar or grant acceptance. See [suffix acceptance](tests/acceptance/r-cpp-extension-routing.md).


Specific grammar compatibility limits appear in bounded terminal and Claude summaries, including older Python grammar behavior for 3.14 template strings. Feedback still requires native confirmation; source text is omitted and limitations are not allowlist decisions. See [conversation acceptance](tests/acceptance/grammar-limitation-conversation-feedback.md).

Go candidate tasks can now run `codeguard task verify <task-id> . --go-tool /absolute/sdk/bin/go --format=json`. This performs a frozen whole-file native syntax recheck and records an attempt; it keeps the task open until approved closure requirements are met. See [acceptance](tests/acceptance/go-package-structure.md).

### Native-first Go lint with missing-tool candidates

Source-built `codeguard lint go . --format json` prefers explicit `--go-tool`, then executable Go from absolute caller PATH entries. Selected tool/version/execution failures remain native failures. When no native tool exists, the built-in WASM produces bounded whole-file recovery and structure candidates. Candidates or incomplete prechecks require a project-appropriate native tool; zero candidates in the completely observed bounded scope only recommend preparation. Native obligations remain incomplete and exit3 is retained. Builds without WASM report that capability gap. Repeated lint/check reuse the confirmation task; adding a package declaration does not close it. Public npm0.1.4 is unchanged. See [limited acceptance](tests/acceptance/go-lint-fallback.md).


### Limited Go task resolution by a protected host

The Unix SDK exposes `verify_go_task_resolution(&GoTaskResolutionRequest)`. Independently signed [policy1.6.0](schemas/task-resolution-policy-v1.6.schema.json) binds Go1.23.4, same-SDK gofmt bytes and canonical companion-path digest. Original native diagnostics, current zero diagnostics and stable inputs are all required to close the same task; an ordinary original-tool recheck records recurrence and reopens it. Companion changes, native counterevidence and missing/tampered history cannot close it. [Evidence0.7.0](schemas/task-resolution-evidence-v0.7.schema.json) never grants project delivery. The real Go pair was locally verified using a test host trust fixture; default production host integration remains open. See [acceptance](tests/acceptance/go-task-resolution-lifecycle.md).


### Read-only current-source command support catalogue

`codeguard help --format json` queries C01–C36 and additional public entries. `codeguard help task verify --format json` selects an exact command prefix. The command_help0.3 report separates implemented/partial/planned/unavailable_build and executable; executable means an entry exists, not that checking or installation completed. Grammar probe is unavailable without the WASM feature and remains unqualified with it. Unimplemented standalone MCP and other operations are explicitly planned.

```bash
codeguard help --format json
codeguard help lint --format json
codeguard task verify --help
```

Help does not inspect projects or start tools. Exit0 means the query completed, with delivery_decision still not_evaluated. Exact-prefix trailing help is supported; full language/path/tool-argument contextual help and complete parameter generation remain open. See [actual reports and acceptance](tests/acceptance/command-help-current-support.md). Public npm0.1.4 does not contain this source increment.


### Native-first standalone Rust lint

The current source supports `codeguard lint rust . --cargo-tool /absolute/cargo --format json`. It reuses the aggregate check's locked/offline all-targets Clippy observation, input validation and workbench synchronization, executing only lint. Initialized workspaces retain one stable task per finding and return the next action. Recheck with `codeguard task verify TASK_ID . --cargo-tool /absolute/cargo --format json`; zero diagnostics still require rule/policy review and do not automatically close the task.

When no explicit Cargo is selected and no executable is found on absolute PATH entries, WASM builds provide bounded candidate prechecks. Candidates or incomplete observations require native tool preparation; zero candidates across the complete bounded scope recommend it. Explicit tool errors or native failures do not trigger fallback or installation. The JSON protocol is `rust_lint_feedback` 0.1 with delivery unevaluated; help 0.2 adds Rust while preserving historical 0.1 schemas. Source integration does not imply public npm availability or complete Rust qualification.

## Ruby single-file native syntax and task rechecks

Source builds add `codeguard lint ruby FILE.rb --ruby-tool /absolute/path/to/ruby --format=json`. This only observes fixed Ruby2.6.10p210 `ruby -c` with gems disabled and frozen UTF-8 stdin; user source is never executed. Explicit tool failures and unsupported versions do not trigger WASM fallback. Missing native selection uses candidate prechecks in WASM builds; candidates/incomplete checks require native preparation, while zero candidates recommend it. The `ruby_lint_feedback`0.1 report preserves native line numbers without fabricated columns, leaves project-version compatibility unverified and requires RuboCop, documentation, security and full project checks. Help0.3 adds Ruby without changing historical help0.1/0.2 schemas. Initialized workspaces reuse one stable confirmation task across WASM candidates and native observations. `next` retains native line positions and the verified `--ruby-tool` recheck argument; `task verify` records attempts and leaves zero-diagnostic tasks open pending policy/coverage review. Ruby syntax feedback0.3, native history0.9, recheck0.10, task preview0.23 and repair brief0.15 preserve old protocol versions. `codeguard check ruby ROOT --ruby-tool /absolute/path/to/ruby --format=json` and `check all` now reuse the bounded native syntax scan and stable task connector; selected failures do not switch to WASM. The shared deadline and64-file cap retain incomplete scope; native observations withdraw positions when source/tool identity changes. Aggregate feedback0.51 supports Ruby scan0.1/0.2 and brief0.15 without modifying old schemas. Full RuboCop/project lint and trusted closure remain pending. Public npm0.1.4 is unchanged.


Ruby edit feedback: source builds of `hook execute` / `hook claude post-tool-use` accept `--ruby-tool ABS_PATH` or select Ruby from the caller's absolute PATH entries. Only confirmed edited files are parsed under the event deadline. Selected failures do not fall back; absence retains WASM candidates. Bounded dialogue carries current line positions and real stable tasks without source/tool-message echoes or invented columns, and asks agents to verify project Ruby compatibility first. `repair_ready` rechecks using the selected tool and retains real evidence references; zero diagnostics do not close tasks. New closed protocols are edit feedback 0.19 (inner 0.10) and recheck feedback 0.20 (inner 0.6); old schemas are preserved. Actual installed-host triggering, complete RuboCop coverage, and public npm release remain unverified.

Ruby version declaration guard: source builds read the nearest `.ruby-version` between the source and project root, bounded to 64 ancestor levels and 4096 bytes. Only `2.6.10`, `2.6.10p210` and their `ruby-` forms match the fixed parser. Other explicit versions yield `ruby_project_version_mismatch`; aliases/ambiguity yield `ruby_project_version_unresolved`; unsafe links/read failures yield `ruby_project_version_unreadable`. None starts the old Ruby or falls back to WASM. Declaration bytes and nearer missing declarations are rechecked after parsing; changes withdraw diagnostics. Saved positions are also withdrawn when a new declaration is incompatible. Standalone lint locates a `.codeguard` workspace before a Git root or Gemfile, so nested Gemfiles cannot hide the workspace pin; the nearest module version declaration remains authoritative. Absence retains an unapproved preliminary observation. Gemfile constraints, JRuby/RVM, complete project profiling and release remain pending. See [version guard acceptance](tests/acceptance/ruby-project-version.md).

The Ruby six-category candidate profile now records runtime dialects, project-lock version selection and conditional tool scopes. Native rules, full project coverage and platform qualification remain unverified. See [profile acceptance](tests/acceptance/ruby-candidate-baseline.md).

## Native ShellCheck single-file inspection (partial capability)

`codeguard lint shell app.sh --dialect bash --shellcheck-tool /absolute/shellcheck --timeout 30s --format json` invokes ShellCheck 0.11.0 `json1` through Rust, preserving native SC rules, severities and ranges. Explicit sh/bash/dash/ksh/busybox are supported; zsh/fish declarations, shebangs and known filenames remain capability gaps. Missing tools produce setup guidance. No Shell WASM asset is available, so fallback remains explicitly unavailable.

An explicit absolute `--shellcheck-config` or the nearest ancestor rc is frozen in a private run directory (automatic discovery is bounded to 64 ancestors). HOME/XDG global configuration is not loaded. Project configuration status is independent of native execution with built-in rules. Frozen stdin avoids executing the inspected script; source, entry bytes, rc bytes and nearer missing rc witnesses are rechecked across phases. Linked rc, invalid encoding and external-sources enablement are rejected. No fix is applied. Tool dependency closure and full platform isolation remain open.

SC1071/1090/1091/1092/1134/1144/1145 represent environment/dependency blockers and may coexist with retained local SC2086 diagnostics. `json1` columns count Unicode scalar characters, with each tab counting one character. Invalid/partial reports cannot claim completeness. Free-text native messages and replacement payloads are not forwarded into repair instructions.

Without an initialized workbench, version 0.1.0 `shell_lint_feedback` includes seven-part repair guidance, original-tool recheck argv and official rule links. `task_workflow_status=not_integrated` and empty attempt history explicitly identify the missing persistence integration. Incomplete evidence yields investigation guidance without authorized source paths. Even native zero diagnostics leaves the overall command incomplete/exit 3 and delivery not evaluated. Project coverage, Dockerfile/IaC, security, trusted exceptions and task closure require independent implementation and acceptance. See the [partial acceptance record](tests/acceptance/shellcheck-native-baseline.md).

## Shell native rule groups in the repair workbench

In an initialized `.codeguard` workspace, single-file `lint shell` now saves a separate 0.1.0 `shellcheck_workbench_observation` through existing report consumption, fact, append-only event and Markdown projection mechanisms. Feedback becomes 0.2.0 with actual synchronization status and stable task IDs. Uninitialized workspaces retain 0.1.0 local feedback and are not initialized automatically. Persistence/import failures remain explicit without erasing native results or inventing task references.

The stable unit is a workspace-relative file, explicit dialect and native SC rule group. All locations remain in the report. Different files/dialects stay separate; a group does not claim multiple occurrences are one semantic defect. Moving positions or adding another occurrence does not create another group. Tool, dialect, rc and source-dependency blockers share a per-file/per-dialect environment recovery task, with concrete reasons in each original report. Stale input becomes historical/environment evidence, not fresh actionable source findings.

`next` supplies 0.17.0 Shell guidance and `task show` supplies 0.3.0. Both preserve the Shell tool parameter in task verify argv bound to the task ID and absolute workspace. The initial report binds dialect and explicit rc; the tool entry must be revalidated. Source or original rc changes withdraw direct repair instructions and require a native rescan. Suppression, synchronization and zero diagnostics cannot close existing tasks. Shell-specific `task verify CG-… . --shellcheck-tool /absolute/shellcheck --format json` now provides a 0.24.0 local recheck through existing leases and failed-attempt history, binding the initial dialect, explicit rc and SC rule group. Outcomes distinguish still_present, rule_coverage_requires_review after a configuration change, suppression_requires_review for a possible disable comment, and candidate_absent_unverified_policy after a clean repair. Comment observation does not prove effective suppression. Trusted closure, recurrence and project-wide coverage remain unaccepted under 7.4. Editing or deleting Markdown cannot remove persisted facts.

Shell attempts use task claim/attempt/verify. Two unchanged attempts with the original rule still present yield needs_decision and reject a third same-action attempt; the finding remains. Existing Markdown is preserved; task show/next provide current guidance. See [acceptance evidence](tests/acceptance/shellcheck-task-recheck-baseline.md).

## Native checks over discovered project Shell files

`codeguard check shell . --shellcheck-tool /absolute/shellcheck --format json` and `check all` now run ShellCheck 0.11.0 over discovered Shell files under one deadline. Feedback 0.52.0 exposes `native_results.shell_lint`, a 0.1.0 shell_native_scan with per-file source hashes, dialects, original rc, native SC rules/scalar positions, input stability and workbench status. Human output shows native positions; SARIF projects current observations while keeping locations and native messages private.

Shebangs and explicit filename conventions supply dialect evidence. `--shell-dialect bash` supplies a default for undeclared files and cannot override zsh/fish declarations. Unknown or unsupported dialects stay incomplete; next requests a concrete dialect/checker decision rather than repeating an unsuitable installation. At most 64 files are checked; unobserved_count exposes overflow. local_check_complete describes these isolated native observations and proves no project source-dependency, category or policy coverage.

An initialized scan binds its requested root despite nested workbenches; an uninitialized scan creates no workspace. Repeated file/dialect/SC rule observations update stable tasks. Configuration or environment failures remain blockers. Source, scope, rc or tool changes withdraw current location authority. No Shell WASM fallback is fabricated. check shell still returns 3/not_evaluated, and check all remains incomplete. Security, CVE, sourced dependencies, trusted closure/recurrence, dedicated zsh/fish tools, Dockerfile/IaC and platform acceptance remain open.

```mermaid
flowchart LR
    A[Discovered Shell files] --> B[Per-file dialect and rc]
    B --> C[Native ShellCheck and shared deadline]
    C --> D[Recheck source scope config and tool]
    D --> E[Stable tasks bound to requested root]
    E --> F[next and task verify]
    C --> G[Per-file diagnostics and blockers]
    G --> H[Partial human JSON and SARIF feedback]
```

See [project Shell acceptance](tests/acceptance/shellcheck-project-baseline.md).

### Shell edit and repair events

Rust `hook execute` and the Claude-shaped adapter route confirmed Shell edits to the same per-file ShellCheck path, accepting `--shellcheck-tool /absolute/path`. Only selected files run within the event deadline. Tasks retain their identity across `lint shell`, `check shell`, and edit events; an uninitialized workspace is not created automatically.

```mermaid
flowchart LR
    A[Confirmed Shell edit] --> B[Selected files and observed dialect]
    B --> C[Native ShellCheck and input revalidation]
    C --> D[Stable tasks and bounded dialogue]
    D --> E[Agent repairs]
    E --> F[Task-bound repair_ready]
    F --> G[Original-rule task verify]
    G --> H[Persist observation and retain closure requirements]
```

Edit feedback uses outer protocol 0.21 and inner 0.11, preserving earlier schemas. Dialogue includes current SC rules, Unicode scalar positions, persisted task IDs, and recheck commands; source and free-form tool messages are excluded. Missing tools, unsupported dialects, selected-tool failures, and persistence failures remain incomplete. No bundled Shell WASM fallback is claimed. Failed writes run no checker; repair-ready events call the existing task verifier, and zero diagnostics never close a task.

The [acceptance record](tests/acceptance/shellcheck-hook-baseline.md) distinguishes real ShellCheck output, controlled fixtures, offline npm installation, and live host sessions. Claude-shaped replay does not prove acceptance in a real installed host or a complete project delivery gate.

CFQuery SQL comparisons require a database dialect. PostgreSQL accepts an empty SELECT list but rejects its DISTINCT variant; the fixed WASM reports zero recoveries for both. The isolated native evidence does not promote generic pending labels. See [dialect evidence](docs/CFQuery-SQL-Dialect-Evidence.md); project SQL adaptation and grammar repair remain open.

Failed-write routing also accepts registered Go/Cargo/Maven and other checker configuration options, returning `not_run/write_failed` without starting tools or creating a workbench. Ownership/lease, unknown and malformed arguments remain rejected. This exception only applies to failed writes and does not silently enable unwired tools on confirmed edits. See [acceptance](tests/acceptance/hook-failed-write-options.md).

## Go native syntax feedback after edits (local acceptance)

`hook execute . --go-tool /absolute/path/go --timeout 30s --format=json` prefers the companion gofmt from the fixed Go1.23.4 SDK for selected confirmed edits. It checks frozen stdin and revalidates both artifacts and source bytes without running source, installing dependencies or executing project-wide go vet. Missing tools retain WASM candidates; selected failures, missing companions, unverified versions and `//line` position remapping remain incomplete without switching checkers. Project language versions and all build conditions remain unaccepted.

```mermaid
flowchart LR
    E[Confirmed Go edit] --> S{SDK selected?}
    S -->|Yes| G[Same SDK gofmt on frozen stdin]
    G -->|Diagnostics| T[Update one stable syntax task]
    G -->|Failure| B[Keep environment blocker and diagnosis]
    S -->|No| W[Bundled WASM precheck]
    W -->|Candidate or incomplete| R[Require native confirmation]
    W -->|Complete zero recovery| I[Recommend native lint preparation]
    T --> V[task verify --go-tool original SDK]
    V --> O[Record attempt and evidence; unapproved task stays open]
```

Edit feedback uses outer0.22/inner0.12 and native-first observations0.10; historical schemas remain intact. Native and candidate observations share workspace/path/language task identity. Claude-shaped feedback exposes current Go rules, UTF-8 byte positions and actual task recheck guidance without source or free-text messages. Zero native diagnostics do not authorize closure: project go vet, types, dependencies, security and complete checks still apply. See [Go Hook acceptance](tests/acceptance/go-native-hook.md).

Go native-first task guidance uses `repair_brief_preview` 0.18 with a `syntax-confirm-` observation reference. Actual task-recheck guidance retains 0.14 with a `syntax-native-` reference; existing schemas are unchanged. Final affected regressions: WASM 78 passed / 5 conditionally ignored, default 15 passed / 0 ignored. The earlier 1,461-pass full default run predates this final protocol correction.

Go's fixed 1.23.4 syntax SDK now statically checks the nearest `go.mod` and nearest `go.work` before editing or original-tool task verification. A newer minimum/suggested toolchain, ambiguous or unreadable declarations produce an environment observation without running that SDK or falling back to WASM. Changes during observation withdraw diagnostics; incompatible current declarations withdraw saved repair positions. This is a bounded compatibility guard, not general project toolchain or language-version acceptance. Evidence: [Go project version](tests/acceptance/go-project-version.md).

### CFQuery static DISTINCT projection candidate

The fixed CFQuery grammar tokenizes SQL without validating every SQL clause. Codeguard now reports adjacent AST `SELECT DISTINCT FROM` keywords as an independent candidate, preserving raw ERROR/MISSING observations. String/identifier literals and CFML interpolation interrupt the sequence; comments can be ignored. `SELECT FROM users` remains unflagged by this rule because PostgreSQL permits an empty projection without DISTINCT.

```mermaid
flowchart LR
    A[CFQuery SQL fragment] --> B[Fixed WASM AST]
    B --> C[Raw ERROR / MISSING]
    B --> D[Adjacent SELECT DISTINCT FROM keywords]
    D --> E[Independent unqualified candidate]
    E --> F[Whole-file identity + fragment identity + file positions]
    F --> G[One stable confirmation task]
    G --> H[Resolve datasource, dialect, version and template context]
    H --> I[Applicable native SQL confirmation required]
```

The worker uses 1.3, explicit probe 0.4, project feedback 0.53, editing feedback 0.23/0.13, and saved candidate observation 0.11. Historical schemas and grammar assets remain unchanged. Embedded observations carry whole-file `source_sha256`, separate `fragment_source_sha256`, and restored file positions. Repeat scans share a task; candidate disappearance does not close it. `next` explicitly asks for datasource, dialect/version and dynamic-template/schema context: the CFQuery native task-verification adapter is still unavailable, and no project database is contacted automatically.

```bash
codeguard grammar probe cfquery query.sql --format=json
codeguard check all . --format=json
codeguard next . --format=json
```

Report example (selected fields, not a complete schema):

```json
{"language":"cfquery","recoveries":[],"structural_observations":[{"basis":"codeguard_structure_rule","rule_id":"codeguard.cfquery.distinct_projection","rule_version":"1.0.0","parent_syntax_kind":"program"}],"grammar_qualified":false,"status":"incomplete","delivery_decision":"not_evaluated","next_action":"confirm_candidate_structure_with_applicable_native_tool"}
```

This addresses one fixed native-counterexample at the candidate layer. It does not qualify the grammar, measure an independent holdout, infer every SQL dialect, validate injection safety or prove native adapter/release acceptance. Evidence: [CFQuery candidate acceptance](tests/acceptance/cfquery-structure.md).


### Rust formatter parser differential (development-only)

The Rust-owned developer replay now supports explicitly selected Rustfmt1.9.0-stable on frozen stdin with private edition2024 configuration, empty environment, shared budgets and artifact/config continuity checks. Valid unformatted source is not a syntax finding; `--check` is not used. Only recognized, bounded stdin diagnostics are retained, including the measured EOF and E0765/exit101 cases; crashes and unlocated output remain incomplete. This fixed edition does not infer project edition, analyze external modules, replace Clippy/build checks, close tasks or implement Rust editing Hook routing. The captured 16-case comparison has 5TP/11TN/0FP/0FN/0unknown on this small non-holdout corpus; language qualification remains0/32. See [scoped acceptance](tests/acceptance/rustfmt-controlled-native-differential.md) for the actual execution path, protocol and reproduction commands.


Rust selected-file parser preparation now resolves Cargo edition from bounded package/workspace declarations and rechecks declaration/source continuity between native calls. The scoped Unix library service has actual2015/2021/2024 counterexample evidence; selected-file edit Hook, stable confirmation tasks and original-tool rechecks are now connected; complete project and real-host acceptance remain open. See [project edition contract](docs/Rust-Project-Edition-Syntax.md).

Rust edit feedback now reports safe lines, stable tasks and original Rustfmt recheck guidance while retaining Clippy/type/build obligations. See [scoped chain acceptance](tests/acceptance/rust-native-hook.md).

Rust edit dialogue now provides an executable post-batch Clippy command and explicitly states that project lint did not run during editing. Original-task repair-ready retains current rule/line feedback and withdraws changed-input guidance; this is not a background queue or authoritative closure. See [project lint follow-up](docs/Rust-Project-Lint-Followup.md).

Native-first and WASM-first Rust syntax tasks now share a protected-host SDK confirmation, scoped resolution and same-tool recurrence path. Native-first policy1.7/evidence0.8 retain grammar=null; WASM-first policy1.8/evidence0.9 retain the original grammar digest. Both bind the original counterexample and approved Cargo edition provenance; a native counterexample requests investigation, and incomplete or changed inputs cannot resolve. Production host approval integration remains pending; this does not qualify Clippy or project delivery. See [scoped acceptance](tests/acceptance/rust-task-resolution.md).

See [WASM-first acceptance](tests/acceptance/rust-wasm-task-resolution.md) for the actual execution path, protocols and counterexample routing.

JavaScript fallback candidates now reach project checks and confirmed edit hooks: duplicate direct simple let/const bindings retain separate structural evidence, reuse one ESLint confirmation task, and produce native-check guidance through `next`. The standalone `lint typescript` entry also selects the JavaScript grammar for `.js`, `.mjs`, `.cjs` and `.jsx`, with native ESLint first. Candidate-free observations recommend native lint only when no prior confirmation task remains. Actual host acceptance and grammar qualification remain pending. See [acceptance](tests/acceptance/javascript-binding-workbench.md).

These additions describe the current source candidate; the public npm release is unchanged.
