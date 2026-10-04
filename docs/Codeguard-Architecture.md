# Codeguard Architecture

> **Purpose:** explain system ownership, component contracts, repair flow, and the gap between current implementation and target behavior.
>
> **Document version:** 1.2.4 · **Updated:** 2026-10-04 · **Source baseline:** current checkout and the [implementation evidence](../openspec/changes/introduce-rust-codeguard-cli/implementation-baseline.md); software version `0.1.4`.

[简体中文](Codeguard-Architecture.zh_CN.md) · [README](../README.md) · [Technical design](Codeguard-Technical-Design.md)

Topic owners: [commands](Codeguard-Command-Reference.md), [initialization](Codeguard-Project-Initialization.md), [remediation](Codeguard-Remediation-Workflow.md), [false positives](Codeguard-False-Positive-Governance.md), [adapters](Codeguard-Adapter-Contracts.md), [trust/distribution](Codeguard-Trust-and-Distribution.md), [acceptance](Codeguard-Validation-and-Rollout.md) and [legacy compatibility](Codeguard-Legacy-Compatibility.md). Detailed contracts live in these guides; OpenSpec owns requirements and tasks.

The source build wires ESLint into the `node.lint` task in `check all`, sharing native concurrency and the request deadline. Discovered JS/TS/TSX files select their nearest module manifest, local ESLint 10, unique flat config and Node without searching above the selected root. Execution collects reports; aggregation synchronizes the workbench serially and reuses stable tasks. Only completed native results for identical source bytes suppress duplicate WASM; ignored files, configuration errors and tool failures retain both their reasons and fallback. Protocols are `check_feedback` 0.36.0 and `check_aborted` 0.13.0, with older schemas archived. `native_results.node_lint` carries per-file feedback, unexecuted paths, synchronization status and next action. See [acceptance scope](../tests/acceptance/check-all-eslint.md). This does not qualify grammars or replace real host acceptance.


The source-built Erlang slice now applies native-first selection through `lint erlang FILE --erl-tool ABS_PATH`: fixed OTP 28 scanner/parser calls operate on stdin bytes, with a non-project cwd and project startup disabled. Native diagnostics take precedence; preprocessing/macro coverage remains unresolved. Without an explicit tool, `lint erlang` searches absolute PATH directories once and records the selected canonical executable in 0.2 feedback. Only absence of an executable enables the pinned WASM candidate; a selected tool failure remains unresolved. No task closure, complete project lint or release qualification follows from this local result; see [acceptance](../tests/acceptance/erlang-native-first.md).

## 1. Reading contract and evidence

This is the architecture of the Rust **Codeguard CLI repository**, not the implementation manual for the existing Python host plugin. It addresses adapter authors, CLI/runtime maintainers, agent integrators, and reviewers.

- **Implemented building block:** code and a bounded testable contract exist; this does not imply an end-to-end product feature.
- **Partial workflow:** a callable path exists with explicit scope and unresolved requirements.
- **Target:** agreed architectural intent whose complete integration or acceptance remains pending.
- **Unknown:** no sufficient observation; never replace this with an optimistic assumption.

The existing [OpenSpec change](../openspec/changes/introduce-rust-codeguard-cli/proposal.md) now lives in this Rust repository and is the sole specification and task authority. Implemented slices and remaining work are tracked separately; these documents do not maintain a second task ledger.

| Evidence | What it establishes |
| :--- | :--- |
| [Workspace manifest](../Cargo.toml) and each crate manifest | Names, dependency direction, version/toolchain declarations |
| [CLI dispatch](../crates/codeguard-cli/src/main.rs) | Callable command branches and platform guards |
| [Full-check orchestration](../crates/codeguard-cli/src/check_command.rs) | Current aggregation, native scheduling, and sync integrations |
| [Initialization](../crates/codeguard-cli/src/init_command.rs), [contract tests](../crates/codeguard-cli/tests/init_command_contract.rs) | Actual workspace artifacts and refresh behavior |
| [Core aggregation](../crates/codeguard-core/src/aggregate.rs), [delivery gate](../crates/codeguard-core/src/delivery_gate.rs) | Pure result semantics, not trusted public gate availability |
| [Acceptance records](../tests/acceptance) | Scoped observations, test method, and residual gaps |

WASM requirements and 19 implementation tasks are recorded in the existing change. An opt-in Rust worker and narrow Java/TypeScript/TSX single-file feedback paths and a Python Ruff-unavailable candidate path now exist. All thirty-two candidate grammars remain unqualified; ArkTS, C, C++, C#, Go, JavaScript, Lua, Luau, Nix, Rust, Terraform, Zig, Objective-C, Solidity, R, Ruby, PHP, Kotlin, Dart, Erlang, Pascal, CFML, CFQuery, CFScript, COBOL, Scala, Swift and VB.NET are pinned Rust-loadable candidates without language-qualified lint routes. Project-wide native-first routing, qualified grammar ranges, capability-matched task closure, and host integrations remain incomplete; design examples are not current command output.

A source build with `--features wasm-precheck` now exposes `codeguard grammar probe <language> <file> --format=json` for all 32 pinned candidates. It uses the isolated Rust worker and reports unqualified observations with exit 3, not a lint or delivery verdict. Successful observations and input failures for a valid language share the [closed 0.1.0 JSON schema](../schemas/grammar-probe-v0.1.schema.json) and remain incomplete. Native-first `lint/check`, language/dialect acceptance, task/host feedback and distribution remain separate open work; current discovery also groups JavaScript/TSX under TypeScript and has no separate CFQuery/CFScript source mapping.

The source-built `check all` now runs a bounded candidate pass after its existing native checks. A separate [grammar router](../crates/codeguard-cli/src/grammar_route.rs) distinguishes `.tsx`, JavaScript, and `.cfs`; complete `<cfquery>...</cfquery>` bodies are parsed with CFQuery, while ordinary `.sql` and ambiguous `.h` are not guessed. Shared `.m` requires an Objective-C line marker, and `.sc` is not guessed as Scala without project evidence; see the [ambiguity regression](../tests/acceptance/ambiguous-grammar-extensions.md). Four bounded real CLI fixtures collectively invoked all 32 pinned workers. [Check feedback 0.36.0](../schemas/check-feedback.schema.json) records selected dialect, source/grammar digests, file-relative recovery positions, bounded known limitations, skipped work and incomplete status; Python files with completed, source-matched Ruff scans skip duplicate WASM; style-only P3C does not confirm syntax. Candidate observations never become confirmed violations or delivery permission. The pass has a 64-file/64-fragment/90-second cap and was about 54 seconds for four bounded fixtures on this Mac; the prior sequential implementation observed 26/32 in one 31-file Linux run before its 90-second cap. It is a partial candidate route, not a qualified native fallback or task/host integration; npm 0.1.3 distributes it as an unqualified candidate. See [acceptance evidence](../tests/acceptance/check-all-32-grammar-candidates.md).

The source-built candidate scheduler now prepares bounded fragments in stable path order and runs up to `min(--jobs, 2)` isolated workers concurrently. It emits observations in that same order and rereads each source before accepting a worker result. A local 31-file, one-project fixture observed all 32 candidates within the unchanged 90-second pass; the Linux WASM integration step for source commit `ff60184` passed the same one-project test; the complete CI run passed. This changes candidate throughput, not its authority.

The [Java 17 native differential](../tests/acceptance/java-native-differential.md) adds a 13-case javac 21 oracle for one grammar. It is local precision evidence only; Java remains an unqualified candidate.

The [Kotlin native differential](../tests/acceptance/kotlin-native-differential.md) now records 11 decidable agreements and two unresolved hidden-error cases, one native-invalid and one native-valid. Swift similarly has 12 decidable agreements and one unresolved case. Incomplete scans cannot become valid/invalid labels merely because their recovery arrays are empty; unknown coverage is reported separately and both grammars remain unqualified.

```mermaid
flowchart LR
    A[check all discovery] --> B[Native adapters and blockers]
    B --> C[Source and scope recheck]
    C -->|stable| D[Dialect router]
    C -->|changed| G[Incomplete report]
    D --> E[Selected isolated WASM worker]
    E --> F[Unqualified observations]
    B --> G
    F --> G
```

A first narrow native-first route is now available for Zig: `lint zig FILE --zig-tool ABS_PATH --format=json` runs a Zig 0.16.0 `ast-check` executable with stable bytes on the selected source bytes before considering WASM. Native AST positions remain visible without source snippets; when no explicit tool is supplied, the pinned Zig grammar remains an unqualified fallback. Both outcomes stay incomplete because AST checking is narrower than full lint, build and tests. See the [0.1.0 feedback schema](../schemas/zig-lint-feedback-v0.1.schema.json).

## 2. Architectural drivers

The product exists because generic output matching and ambiguous failures produce false positives, while repeated errors encourage agents to suppress checks instead of fixing code.

| Driver | Architectural response | Acceptance intent |
| :--- | :--- | :--- |
| Preserve native rule meaning | Adapter-specific command/report contracts | Environment failures never become invented source violations |
| Low false positives | Configuration-aware scope, explicit unknowns, exact exceptions | Unsupported configuration remains unresolved; valid findings retain origin |
| Actionable repair | Stable facts, repair brief, attempt history, original-tool recheck | A finding yields an appropriate next action rather than repeated raw logs |
| Multi-language consistency | Six categories and versioned report semantics | Every advertised language/category has its own tested evidence |
| Predictable execution | Bounded processes, resource-aware scheduling, cancellation | Failure cannot silently erase other completed observations |
| Minimal onboarding | Discover existing configuration first | Initialization does not execute project scripts or fabricate architecture |

Rust is the orchestration language, selected for a deployable binary and explicit state/resource handling. No comparative performance result is available. Native analyzer runtime and database costs can dominate total scan time.

## 3. Scope and system context

```mermaid
flowchart LR
    Human[Developer] --> CLI[Codeguard Rust CLI]
    Agent[Coding agent] --> CLI
    Plugin[Host plugin: target integration] -.-> CLI
    Project[Source, manifests, locks, native configs] --> CLI
    CLI --> Native[Native checkers and their runtimes]
    Native --> Results[Diagnostics and execution status]
    Results --> CLI
    CLI --> Feedback[Conversation-ready feedback]
    CLI --> Workbench[Project-local repair files]
    Authority[Policy / exception authority: incomplete integration] -.-> CLI
```

Codeguard owns discovery, command selection, bounded invocation, interpretation, result presentation, and local repair coordination. Native tools own language semantics, rule execution, dependency resolution, and advisory lookup within their contracts. The agent owns proposed code/environment changes. A policy authority must own exception approval; writing a local proposal is not approval.

The CLI does not own a model provider, chat transport, IDE UI, RAG store, distributed scheduler, or SaaS tenant database. Plugins and skills remain separate integration/distribution layers. Their automatic check → sync → brief behavior is a target integration, not established by this repository's CLI tests.

## 4. Current state and target state

| Capability | Current state | Target / remaining work |
| :--- | :--- | :--- |
| Native checks | Partial Python, Rust, Java, Node and Go paths | Complete supported scopes and language/category matrix |
| Discovery | Static configuration and manifest observations | Effective models under explicit controlled resolution |
| Project architecture | Unknown plus static module declarations | Evidence-labelled observed/inferred/confirmed views and conflicts |
| Repair | Stable tasks, local attempts, selected native rechecks | Reliable verified closure, recurrence, and full event reconciliation |
| False-positive exceptions | Candidate inspection/proposals, exact matching and signature-related building blocks | Trusted approval source, lifecycle integration, and usable activation |
| Quality gates | Core algebra; public scans remain partial; index path safety | End-to-end checks against actual worktree/index/ref/CI content |
| Distribution | Source build and npm 0.1.3 for Apple Silicon macOS; artifact verification/download/install primitives | Supported binary releases, platform tests and host runtime bindings |

Current `check all` includes Rust build scheduling and persistence; [build recheck](../tests/acceptance/rust-build-task-verification.md) is also present. Older notes saying these integrations are wholly absent must not be used as the current status. Complete build combinations and formal closure remain missing.

## 5. Components and dependency direction

```mermaid
flowchart TB
    CLI[codeguard-cli: entry and application services] --> Core[codeguard-core: domain contracts]
    CLI --> Runtime[codeguard-runtime: execution and I/O]
    CLI --> Adapters[codeguard-adapters: native semantics]
    Runtime --> Core
    Adapters --> Core
    Runtime --> OS[OS / filesystem / native processes]
```

| Component | Input → output | State / failure responsibility |
| :--- | :--- | :--- |
| CLI | Arguments/configuration → selected operation and feedback | Owns workspace orchestration, command-specific errors, protocol projection |
| Core | Typed observations and scope → deterministic evaluation | No native process or filesystem access; does not establish trust from a boolean by itself |
| Runtime | Literal process specification / snapshot request → bounded observation | Process lifecycle, bytes, locks, resource cleanup, explicit I/O failures |
| Adapters | Native configuration/output → checker-specific observations | Rule identity, scope and exit/report coherence; unknown protocols remain unresolved |

The four crates are real compile-time boundaries. Application services currently live in `codeguard-cli`, including substantial scan/persistence logic. There is no separate application crate or universal runtime-loaded adapter ABI. Future extraction should follow cohesive responsibilities, not add empty crates to match a diagram.

### 5.1 Application services and extension boundaries

Discovery/Plan/Check/Work/Fix/Gate services in the earlier design are logical responsibilities, not a claim that corresponding public traits or separate crates exist. Argument parsing and application services currently live in the CLI. Scoped threads and bounded processes must not be described as an already implemented clap/Tokio scheduler. An adapter must declare configuration discovery, input scope, native report semantics and recheck capability; registering a language name is insufficient.

Project build scripts and checked code are untrusted inputs. The full CI gate is intended to isolate the validator, policy source and final credentials from project processes. Current local process controls and offline options do not establish network isolation or protection from a malicious process under the same user. See [trust and distribution](Codeguard-Trust-and-Distribution.md) for the detailed boundary.

## 6. Domain model and result semantics

| Concept | Meaning | Important distinction |
| :--- | :--- | :--- |
| Configuration observation | `configured / missing / invalid / unknown` | Declaration is not effective-model resolution or execution |
| Check obligation | Selected checker/category/target/rule scope | A planned inventory row is not an executed obligation |
| Finding | Native rule, tool, location, severity, identity | Native severity and policy gate impact are separate |
| Completion | `complete / incomplete / not_applicable` | Independent of finding count |
| Blocker | Environment, configuration, scope, or evidence issue | Not a source-code violation |
| Repair task | Actionable projection of a persistent problem | Checking a Markdown box is not resolution |
| Disposition | Exact false-positive or exception decision | Does not delete the raw finding or rewrite tool output |

```mermaid
flowchart LR
    O[Native observation] --> F[Findings]
    O --> C[Completion]
    F --> A[Request aggregation]
    C --> A
    A --> R[Result + unresolved reasons]
    R --> G[Delivery evaluation if trusted full context exists]
```

Core aggregation priority is cancellation → internal error → incomplete → violations → not applicable/passed. It retains findings from partial runs. The core delivery calculator separately checks scope, obligations, coverage and exception bindings. It can model `allow`, `allow_with_exceptions`, `deny`, `incomplete`, `not_applicable`, and `not_evaluated`; current public partial checks do not supply the complete trusted context needed to claim delivery success.

The user-facing contract is practical: show what is configured, what was found, what failed to run or resolve, and what to do next. Internal identity checks prevent stale or mismatched observations; they should not turn ordinary usage into a separate exercise in manually proving execution.

## 7. Initialization and project knowledge

```mermaid
flowchart LR
    Root[Explicit project root] --> Observe[Read manifests, locks, configs and source paths]
    Observe --> Profile[Project profile]
    Observe --> Graph[Typed module declarations]
    Profile --> Preview[Dry-run plan and conversation summary]
    Graph --> Preview
    Preview --> Apply[Explicit apply with ownership checks]
    Apply --> Files[Workspace and managed AGENTS block]
```

`init` defaults to dry-run. Static observation does not execute wrapper scripts, Maven extensions, JavaScript configs, or service startup. Declared package versions, language targets, resolved dependency versions, and installed runtimes are distinct; unknown values stay unknown.

Maven and Cargo support selected direct module/dependency declarations. Aggregation, containment and build dependency are distinct edges. Variables, inherited models, optional/target conditions, missing targets and ambiguous identities are unresolved. An incomplete graph cannot justify a narrower scan or prove independence.

`AGENTS.md` contains bounded summaries of languages, roots, versions, checkers and graph observations, with links and content digests. Human text outside the managed markers is retained. Duplicate/missing markers, modified managed content and detected concurrent edits produce conflicts. Current write checks do not establish perfect exclusion against an uncooperative external editor. Source strings are escaped as data; unknown MVC/DDD architecture does not activate a blocking rule.

## 8. Scan, feedback, and repair flow

```mermaid
sequenceDiagram
    participant A as Agent / developer
    participant C as CLI application
    participant R as Runtime
    participant N as Native checker
    participant W as Local workbench
    A->>C: check with project and selected tool context
    C->>C: discover configuration and freeze relevant input
    C->>R: task graph + common deadline
    R->>N: literal arguments, selected environment
    N-->>R: output + termination state
    R-->>C: bounded observation
    C->>C: adapter interpretation and input recheck
    C->>W: save eligible local report and sync
    W-->>C: stable tasks / sync failure
    C-->>A: findings + blockers + next action
    A->>A: repair within task scope
    A->>C: task verify with original checker
    C-->>A: still present / blocked / candidate absent
```

Local saving and sync are implemented for selected paths. Uninitialized projects can still receive feedback; they do not silently receive a persistent workbench. A sync error must remain visible without erasing native findings. Malformed or stale reports cannot close unrelated tasks.

The desired complete loop is:

```mermaid
flowchart LR
    N[Native checks] --> M[Merge findings and environment blockers]
    M --> T[Stable repair tasks]
    T --> Next[Next action]
    Next --> Fix[Repair code or environment]
    Fix --> V[Original-tool recheck]
    V -->|Still present| T
    V -->|No progress| D[Specific diagnosis / decision]
    V -->|Verified resolution: target| Close[Record resolution]
    Close --> Full[Full delivery checks: target]
```

The final two nodes are target behavior. Current candidate-absent observations do not automatically close tasks. Each task should carry evidence, rule basis, allowed scope, repair steps, recheck command, attempt history, and closure conditions.

### 8.1 Native-first routing with bundled WASM precheck — target

**Target contract, partly implemented in the published `0.1.3` candidate package.** Keep a single entry point; callers select a language or project, not a parser backend:

```bash
codeguard lint java .
codeguard lint typescript .
codeguard check all .
```

These entry points already exist with bounded native adapters and adapter-specific prerequisites. Automatic fallback described below is new target behavior. Resolve checker availability per module, language/version, dialect and effective configuration. An ESLint-ready frontend and a Java module missing its JDK take different paths in the same run.

```mermaid
flowchart TD
    A[Observe modules, language versions and checker configuration] --> B{Native checker ready?}
    B -->|Available with valid configuration| C[Run native checker]
    C --> D[Native findings, completion and repair guidance]
    B -->|Tool missing or configuration unavailable| E[Run bundled WASM syntax precheck]
    E --> F{Precheck result}
    F -->|No anomaly in checked scope| G[Return precheck report; recommend native setup]
    F -->|Suspected syntax issue| H[Return locations; require native setup and confirmation]
    F -->|Incomplete or unsupported| I[Explain coverage gap; restore valid checking capability]
    G --> J[Sync eligible workbench observations and return agent brief]
    H --> J
    I --> J
    D --> J
    H --> K[Install or restore the applicable native checker]
    K --> C
```

A completed native violation never selects fallback to obtain a clean result. A native execution failure may receive supplementary precheck feedback, but the original failure remains visible. WASM covers syntax only: it does not replace configured style rules, Javadoc, type checking, dependency/CVE analysis or security checks. Already-required native obligations remain required even after a clean precheck.

| Situation | User-facing conclusion | Next action |
| :--- | :--- | :--- |
| Native checker ready | Native report with actual scope and completion | Follow native diagnostics |
| Native tool missing; precheck clean | No syntax anomaly in checked scope; native lint not run | Recommend installation unless the project already requires it |
| Native tool missing; suspected syntax issue | Suspected issue awaiting native confirmation, not a confirmed violation | Require installation/restoration and an applicable native recheck |
| Tool installed; configuration invalid | Configuration blocker plus scoped precheck | Restore configuration; do not reinstall an already available tool |
| Grammar incompatible, worker timeout or load failure | Precheck incomplete/unsupported | Restore native checking or repair parser support; do not blame source |
| No files checked or partial embedded-language coverage | Empty/incomplete scope, never an all-clear | Resolve scope or show exactly which regions remain unchecked |

“Required” controls verification and task closure; it does not authorize installation outside the user's existing scope or freeze unrelated work. If installation is unavailable, keep one environment task with a concrete reason. A grammar anomaly can be false positive, but still requires valid native confirmation before the escalated task is resolved.

### 8.2 Grammar assets and Rust ownership — target

Reuse pinned CodeGraph grammar WASM artifacts and applicable upstream licenses, source references, patches and corpus cases. Do not copy CodeGraph's graph extraction or source-blanking heuristics into a syntax verdict. CodeGraph's `src/extraction/grammars.ts` loads language artifacts through `web-tree-sitter`; successful loading there does not prove compatibility with Codeguard's Rust runtime. The local source review on 2026-09-28 found modified grammar artifacts and about 65 MB of grammar files, so a live directory is not a release input.

The read-only `codeguard grammar status` command projects a [pinned CodeGraph coverage inventory](../grammars/codegraph-coverage.json): 30 vendored WASM files and two dependency-provided grammars. CodeGuard currently pins 32 Rust-loadable candidates, including ArkTS, Nix and Terraform; none is qualified for release. ArkTS, Nix and Terraform provenance and worker evidence are recorded in the [candidate acceptance](../tests/acceptance/arkts-nix-terraform-grammar-candidates.md). ArkTS, Nix and Terraform have pinned bytes, licenses and narrow Rust worker evidence, but no language-qualified standalone lint route or release qualification; see the [limited acceptance record](../tests/acceptance/arkts-nix-terraform-grammar-candidates.md). The corrected Zig source WASM requires a pinned import adaptation to load in Rust; language-qualified `lint zig` remains open. Python has a narrow unqualified fallback for unavailable or undeclared Ruff, with native results taking priority and no delivery approval. An initialized single-file scope now syncs one stable native-confirmation task; candidate zero recoveries do not close it. This inventory is deliberately separate from the pinned [candidate asset manifest](../grammars/manifest.json). COBOL now loads under a bounded 20 MiB input limit, but its cold-start and memory budget remain unqualified. Inventory rows never trigger loading or count as supported parser capability.

R, Ruby, PHP and Kotlin now also have pinned CodeGraph bytes, licenses, measured Rust ABI and narrow positive/negative worker samples; the PHP fixture covers mixed HTML/PHP. They remain unqualified candidates. The original CodeGraph Dart WASM remains incompatible with Rust, but CodeGuard now has a byte-reproducible Zig rebuild that compiles the real external scanner and uses a pinned WASI import adaptation. Rust loader and isolated-worker fixtures pass; Dart remains unqualified for public lint and release. See the [Dart rebuild record](../tests/acceptance/dart-grammar-rebuild-candidate.md). Erlang has pinned CodeGraph bytes, the upstream 0.19 license and ABI 14, Rust worker samples and an OTP 28 native comparison. Current source builds include explicit native-first lint and task verification; the original WASM still misses function terminators, while full language and public release qualification remain open. See the [native comparison](../tests/acceptance/erlang-native-differential.md) and [task verification](../tests/acceptance/erlang-native-task-verification.md). Pascal now has pinned CodeGraph bytes and its original Isopod dependency commit and license; ABI 14 and narrow worker tests pass, but native comparison and public lint remain open. See the [Pascal candidate record](../tests/acceptance/pascal-grammar-candidate.md). The remaining CFML/CFQuery/CFScript/COBOL/Scala/Swift/VB.NET assets also load in the Rust worker, bringing the candidate inventory to 32/32 with zero released syntax capabilities. CFQuery misses `SELECT FROM`, VB.NET misparses a valid unindented method, and COBOL has a high load cost; see the [seven-asset acceptance record](../tests/acceptance/final-seven-grammar-candidates.md).

```text
grammars/                         # proposed release assets, not project state
├── manifest.json                 # immutable source, digest, ABI and capability map
├── java/
│   ├── parser.wasm
│   └── LICENSE
└── typescript/
    ├── parser.wasm
    └── LICENSE
```

For each artifact, record language/dialect, upstream commit and patch identity, WASM SHA-256, Tree-sitter ABI/runtime compatibility, tested language versions, known limitations, license and corpus reference. File presence, load compatibility and syntax-check acceptance are three separate capability states. Bundled grammars must work offline; load only the languages needed. Additional language-pack updates require a versioned manifest and compatibility check.

| Owner | Responsibility added by this design |
| :--- | :--- |
| `codeguard-core` | Backend selection policy, precheck outcomes, required/recommended setup actions and reconciliation |
| `codeguard-runtime` | Rust Tree-sitter WASM host, bounded parser worker, asset validation, caches and cancellation |
| `codeguard-adapters` | Language/dialect interpretation, syntax observations and native confirmation capability mapping |
| `codeguard-cli` | Compose adapters with runtime; commands, persistence and human/JSON brief |
| Separate `codeguard-plugin` | Host triggers, bounded conversation delivery and feedback deduplication |

Retain the four-crate dependency direction: adapters interpret observations; CLI composes them with runtime. WASM workers receive bounded source bytes without general filesystem/network imports. Parent-owned deadlines, process termination, memory limits and output bounds must be verified on each supported host; “WASM” alone does not prove resource isolation. Runtime and grammar versions must also fit the declared Rust MSRV or follow an explicit compatibility change. Rust's [Tree-sitter WasmStore](https://docs.rs/tree-sitter/latest/tree_sitter/struct.WasmStore.html) provides a loading API, not a guarantee that every copied grammar is compatible (reference checked 2026-09-28).

### 8.3 Agent feedback and verification lifecycle — target

Report precheck status separately from native execution and delivery. `clean` means only no observed syntax anomaly; `suspected_issue` needs confirmation; `incomplete` and `unsupported` identify missing coverage. Detect both Tree-sitter `ERROR` and `MISSING` recovery, group related recovery nodes and preserve source spans. Version/dialect uncertainty is diagnostic context, not proof of a source defect. See [Tree-sitter query syntax](https://tree-sitter.github.io/tree-sitter/using-parsers/queries/1-syntax.html).

The plugin sends a short report containing method, checked/uncovered scope, observation, native-tool state, next action and an actual task reference when persistence succeeded. The CLI alone cannot inject messages into arbitrary hosts. `AGENTS.md` stores standing workflow guidance, not repeated run logs. Repository text and checker output remain data; installation actions come from reviewed adapter recipes, not copied diagnostic shell strings.

Illustrative target brief, not captured output (path and task ID are synthetic):

```text
Codeguard: Java syntax precheck found 1 suspected issue
Method/scope: bundled WASM; 18 of 18 selected Java files checked.
Native check: not run; the project's required JDK is unavailable.
Location: src/main/java/example/UserService.java:42; MISSING recovery node.
Next step (required): prepare the declared JDK and applicable native checker,
then confirm this file's syntax before repairing according to native diagnostics.
Task: CG-example-java-setup (only emit an actual persisted task in real use).
This is not yet a confirmed code violation; delivery is not evaluated.
```

```mermaid
sequenceDiagram
    participant A as Agent host
    participant P as Codeguard plugin
    participant C as Rust CLI
    participant W as WASM worker
    participant T as Workbench
    A->>P: Check changed module
    P->>C: lint with project context
    C->>C: Native checker unavailable
    C->>W: Parse bounded source with pinned grammar
    W-->>C: Syntax observations and coverage
    C->>T: Sync if initialized; reuse setup/confirmation task
    T-->>C: Actual task reference or sync error
    C-->>P: Precheck + native state + next action
    P-->>A: Bounded brief, with changed results only
    A->>P: After authorized native setup, request verification
    P->>C: task verify with original project context
    C-->>P: Native confirmation, contradiction or unresolved coverage
    P-->>A: Repair action or resolved verification outcome
```

For required setup, create one setup/confirmation task per module/checker obligation and attach suspect source observations to it. Optional setup remains a recommendation, not a blocking repair task; `next` must not repeatedly select it. Installing a tool alone does not close that task. Native confirmation must cover the same syntax capability, file/dialect and current input: Java style-only output is not compiler-level syntax proof. Confirmed issues become native repair work; a sufficiently scoped native contradiction records a parser false-positive candidate. Ambiguous coverage stays unresolved. A later clean WASM run alone cannot satisfy a previously required native confirmation.

Allowlist dispositions bind rule, source target, grammar identity and reason; retain observations and reevaluate after relevant changes. They cannot remove required native checking or fabricate coverage. Repeated runs update stable tasks and attempt history. Emit the initial brief, then meaningful changes; preserve a status view instead of repeating unchanged installation requests. See the technical design's **7.3–7.4** for bilingual report examples and the proposed data contract.

### 8.4 Host events and check tiers — target

Host hooks pass events, workspace identity and known targets. Rust core selects the check tier; the formal CheckPlan then chooses native checkers and resource budgets. An agent cannot change delivery obligations through a prompt, an old soft cache entry or a project-local `skipGate`. The [pure domain router](../crates/codeguard-core/src/hook_trigger_planner.rs) now bounds per-file feedback selection; bulk edits above that budget request a batch-scope decision with a distinct reason. The read-only `codeguard hook plan` CLI returns only a candidate tier with exit 3; it does not execute a checker or connect the plugin, real Git/CI or any verified automatic host trigger. The complete event-driven execution remains a target design.

Separately, `lint python . --file REL_PATH` can invoke native Ruff within the same path budget and report only the selected files. `hook plan` does not yet invoke it automatically, and this local result is neither a real Git-scope check nor a full workbench scan.

`hook execute PATH --timeout DURATION --format=json` consumes the same versioned host event and core route. Session start performs read-only discovery; Stop reads a bounded local next-step view without invoking a checker; a confirmed bounded edit runs selected Python/Ruff and JS/TS/ESLint native checks, then optional WASM candidates for uncovered files. Stop examines at most 64 findings and 64 reports, with an 8 MiB total report-byte budget; exceeding either returns `guidance_scope_exceeded` and leaves the check unrun. `repair_ready` bounds one task history to 128 events and 1 MiB, resolves the stable task fact, then invokes the existing `task verify` path in a bounded child process; its response contains a small summary of the original checker observation and whether the local event was saved. It never closes the task or approves delivery. With an explicit `--git-tool ABS_PATH`, `pre_commit` observes the live staged index (including `GIT_INDEX_FILE`) under the event deadline, reports path violations and verified-object status, and never reads an old edit result as a gate. Missing tools or unstable index observations remain incomplete. Uncertain or failed writes, push and CI remain `not_run`. `hook_execution_feedback` 0.6 remains candidate evidence; historical 0.1–0.5 schemas are retained. Prompt submission now returns constant non-blocking timing guidance without interpreting prompt text or running a checker; this does not establish delivery intent or a Git gate. Plugin Hooks, cache reuse and complete Git/CI quality gates are still unwired.

The Rust CLI also has candidate Claude Code `SessionStart`, `PostToolUse`, `PostToolUseFailure`, and `Stop` soft entries. Session start delegates to read-only project discovery. A successful file edit checks the absolute target against the project root and regular-file boundary, then calls the same executor. A failed edit invokes the no-check route without reading the tool error as instructions. Stop reads bounded local task facts without invoking a checker; only a stable task on an initial Stop yields one `additionalContext` continuation, while `stop_hook_active=true` and an empty backlog yield a user-visible message without another continuation. All outputs are bounded and omit source text, raw errors, and editable task Markdown. Invalid or over-budget events say the check was not run. Host exit 0 means only soft feedback, never quality or delivery approval. The separate `codeguard-plugin` does not yet bind or invoke this binary; see the [candidate acceptance record](../tests/acceptance/claude-post-tool-hook-candidate.md).

```mermaid
flowchart LR
    H[Host event] --> R[Pure Rust event routing]
    R -->|Session start| D[Read-only discovery]
    R -->|Confirmed edit| F[Changed-file feedback]
    R -->|Repair ready| V[Original task and checker recheck]
    R -->|Commit / push| G[Acquire current Git content scope]
    R -->|CI| A[Full project obligations]
    F --> P[Unified report / task / next action]
    V --> P
    G --> P
    A --> P
```

| Event | Requested tier | Required boundary |
| :--- | :--- | :--- |
| Start/resume, prompt, stop | Read-only discovery, nonblocking intent guidance, session summary | Prompt text cannot replace a real Git gate or claim delivery |
| Confirmed write | Affected-file fast check | Reuse a complete soft result only when content and all configuration identities match; queue project-wide work |
| Failed/unknown write | No source check / resolve scope | Unknown cannot mean unchanged or checked |
| Repair attempt completed | Original `task verify` | Installing a tool, ticking a task or WASM clean alone does not close it |
| Commit, push, CI | Strict commit scope, actual pushed refs, full project obligations | Ignore host-supplied path guesses and old save feedback; disclose missing enforcement support |

Coalescing applies only to duplicate fast feedback for the same content snapshot; it cannot swallow new input, failed rechecks or delivery events. Cache identity must cover source and dependency closure, tool, rules, configuration, platform and vulnerability database freshness. Strict gates revalidate full obligations and identities, then recompute or report incomplete when proof is missing. Measure latency percentiles, repeat execution, invalidations, missed findings and false positives per tier before fixing budgets.


## 9. Persistence and ownership

```text
checked-project/
├── AGENTS.md                       # only Codeguard's marked block is managed
└── .codeguard/
    ├── .gitignore / README.md
    ├── workspace.json              # local workspace identity and managed digests
    ├── project.json                # static observations
    ├── module-graph.json            # typed relations plus unresolved conditions
    ├── architecture.md             # observation projection
    ├── findings/                   # redacted facts and append-oriented events
    ├── tasks/                      # readable projections and correction attachments
    ├── decisions/                  # references; no local self-approval
    ├── reports/                    # ignored local observations
    ├── runs/                       # ignored run data
    ├── cache/                      # ignored cached data
    ├── worktrees/                  # ignored working copies
    └── state/                      # ignored receipts, observations and leases
```

The architecture distinguishes original checker output, normalized observations, persistent problem facts, and human-readable projections. These are not interchangeable authority sources. Empty directories may not appear in Git until populated.

`work sync` binds reports to a workspace/run and consumes matching reports idempotently. Repeated findings retain stable tasks; repeated observations should avoid tracked-file noise. A bad report does not justify discarding other valid reports. Multi-file updates are not claimed to be a fully atomic database transaction; recovery tests cover selected intermediate states and process exits.

The dot-prefix rule skips `.codeguard/` during ordinary source discovery; Codeguard still validates its own records. User source such as `codeguard/src` remains in scope. An older `codeguard/workspace.json` causes a migration conflict; moving the entire directory could hide user source. Tracked records must still obey repository security checks; completing that security gate remains work. Deleting tasks does not supply evidence that the source is clean.

## 10. Execution, concurrency, and resource limits

[ProcessSpec](../crates/codeguard-runtime/src/process_spec.rs) records an absolute executable, literal arguments, explicit cwd, a selected environment, stdin policy, common deadline and combined stdout/stderr limit. [Process execution](../crates/codeguard-runtime/src/process_runner.rs) handles capture, termination and cleanup. Unix process-group control and filesystem calls use isolated unsafe code; the workspace does not promise zero unsafe.

[Task scheduling](../crates/codeguard-runtime/src/task_scheduler.rs) runs a validated DAG with dependency and shared-resource exclusion. Failed dependencies do not start their children. Cancellation and deadline expiry distinguish queued from running tasks. Arbitrary Rust callbacks that ignore cancellation cannot be forcibly preempted by the scheduler.

| Budget | Current value / scope |
| :--- | :--- |
| Check timeout | Default 30 minutes; accepted 1 ms–24 hours |
| Check jobs | Default min(available parallelism, 4); explicit range 1–64 |
| Check source snapshot | Current aggregate path requests at most 4,096 files, 16 MiB/file, 256 MiB total |
| Process output | Per-process specification; many current probes use 16 MiB combined capture |
| Task lease | Local token/generation ownership and expiry; not cross-machine coordination |

These limits are implementation budgets, not measured latency or throughput guarantees. Different probes may impose smaller limits. No end-to-end CPU, memory or network sandbox is certified. Build extensions and native tool behavior remain part of the threat model.

## 11. Task lifecycle and no-progress control

```mermaid
stateDiagram-v2
    [*] --> Open
    Open --> Claimed: valid local lease
    Claimed --> Attempt: record start
    Attempt --> Recheck: finish with change
    Attempt --> Decision: repeated failure or no change
    Recheck --> Open: still present / incomplete
    Recheck --> Candidate: absent but policy unresolved
    Candidate --> Open: current input changes
    Candidate --> Resolved: target - verified full conditions
    Resolved --> Open: target - recurrence
```

This is a conceptual lifecycle, not a claim that every named state is stored verbatim. Current local facts remain open while recheck events distinguish `still_present`, `still_blocked`, suppression/configuration review, and candidate absence. Attempts, leases and verification receipts are separate structures.

A lease cannot be replaced by an owner string alone: tokens, generation and expiry matter. Failed/no-change/abandoned attempts contribute to bounded no-progress handling. `next` reads structured facts and verified local observations instead of executing instructions embedded in task Markdown. Complete parent-event reconciliation, cross-machine coordination and reliable formal closure remain pending.

## 12. False-positive rules and trust boundaries

| Boundary | Required behavior | Current limit |
| :--- | :--- | :--- |
| Native diagnostics → findings | Preserve exact rule/tool/location and unsupported states | Parsers implement selected protocols and rule subsets |
| Project text → agent context | Escape data; no instructions promoted to policy | General prompt-injection immunity is not claimed |
| Candidate → approved exception | Match exact current identity, expiry and approval scope | Public CLI has no complete approval/activation path |
| Native suppression → repair result | Detect changed coverage before claiming a fix | Counterchecks exist only for selected adapters |
| Report → current observation | Verify workspace, run, input and tool bindings | A digest establishes byte consistency, not independent trust |
| Task projection → source status | Recheck with original tool | Checkbox/delete operations cannot establish resolution |

False-positive handling is an explicit product capability, not a broad skip switch. Exact matching and signature verification building blocks exist, but their existence is not proof that key distribution, approval governance, current clocks and gate integration are complete. See [false-positive design](Codeguard-Technical-Design.md).

## 13. Protocols and configuration

The CLI emits command-specific versioned JSON, human-readable projections and selected SARIF. [Schemas](../schemas) include current and older protocol shapes. `RunReport` is a target/common contract alongside multiple current local observation protocols; integrations must not assume every command already emits one identical envelope.

Configuration layers have different authority:

1. Native tool configuration selects native checks.
2. Project runtime options select time/concurrency, not policy.
3. Rulepack/tool-lock/policy candidates describe intended bindings but are not independently approved by their location.
4. Approved policy and exception authority require an explicit trustworthy source; full integration remains pending.

The runtime precedence and exact schema are documented in [README](../README.md). Unknown fields and unsupported versions must be handled according to the relevant parser, not universally claimed to share one parser policy. Protocol compatibility and legacy exit semantics need explicit adapters.

## 14. Reliability and operational recovery

| Failure | Output and repair action |
| :--- | :--- |
| Missing JDK/tool/configuration | Preparation blocker; restore the needed environment or configuration |
| Native timeout or output limit | Incomplete; preserve independently parseable current findings |
| Truncated JSON/XML or unsupported protocol | Concrete parse/protocol issue; do not invent a finding |
| Input changed during/after scan | Observation becomes unusable for current resolution; rescan |
| Sync conflict or interrupted persistence | Preserve original report, report sync state, retry/reconcile |
| Changed managed `AGENTS.md` | Preserve human content and surface ownership conflict |
| Repeated no progress | Stop the same repair action and provide a specific decision requirement |
| Advisory database unknown/stale | Display available scoped observations and unresolved CVE completeness |

A clean native result should not trigger endless identical rescans because an unrelated policy prerequisite remains unresolved. The target experience is to surface the missing prerequisite once as a stable actionable item. Current partial authority integration is a product gap, not a reason to claim a fix has failed.

There is no daemon uptime SLO, multi-region RPO/RTO, production metrics backend or proven benchmark in this repository. Diagnosis currently relies on structured feedback, run IDs, local observations, and acceptance evidence. Raw logs are local and potentially sensitive even when Git-ignored.

## 15. Deployment, upgrades, and integration

Current deployment is a locally built binary plus independently prepared native toolchains. The `@partme.ai/codeguard@0.1.3` npm package bundles an Apple Silicon macOS binary behind a Node command entry; the Node layer only forwards arguments, cwd, environment, streams and exit status. Publication, a fresh-cache `npx` version check, and registry/local artifact byte comparison passed on that host. The binary reports a candidate source commit, but this does not establish signed provenance, a reproducible build, a multi-platform binary release, or completed quality gates. `tools install` public apply is blocked while internal package verification/download/install primitives have tests.

```mermaid
flowchart LR
    Build[Explicit Rust build] --> Verify[Check binary version and host target]
    Verify --> Wasm{WASM package requested?}
    Wasm -->|yes| Probe[Match 32 pinned identities and run Zig worker]
    Wasm -->|no, local private only| Pack[Local platform-tagged npm tarball]
    Probe --> Licenses[Verify pinned upstream licenses]
    Licenses --> Pack
    Pack --> Node[Node bin: codeguard]
    Node --> Rust[Bundled Rust executable]
    Rust --> Native[Selected native analyzers]
```

The default local package is marked private and contains no npm install hook. Its `bin` entry is [npm/codeguard.cjs](../npm/codeguard.cjs); [scripts/pack-npm-local.mjs](../scripts/pack-npm-local.mjs) builds it from an existing binary. The `--public` packaging mode produced the published `@partme.ai/codeguard@0.1.3`, restricted to Apple Silicon macOS. Fresh-cache registry execution and matching package/binary hashes are recorded in [npm 0.1.3 acceptance evidence](../tests/acceptance/npm-0.1.3-wasm-candidate.md). Broad registry distribution still needs approved platform coverage, a trusted artifact manifest, matching versions and hashes, and a release process. CodeGraph's Node entry and platform package layout informed this design; its optional network fallback is not part of Codeguard's current installation path.

For the published 0.1.3 candidate-WASM package, `--require-wasm` and `--public` both reject a binary lacking the worker. The packer compares its inventory against the 32 pinned manifest identities and executes a Zig worker probe before writing a tarball; [local offline npm acceptance](../tests/acceptance/npm-wasm-local-package.md) additionally runs Zig and Dart from the package. The packer also verifies and includes all pinned upstream licenses; the public 0.1.3 tarball matches the registry bytes. This does not retroactively add WASM to 0.1.2 or qualify any language for lint; see [public candidate acceptance](../tests/acceptance/npm-0.1.3-wasm-candidate.md).

For upgrades: record binary/schema versions, preserve workbench data, preview managed changes, rerun a bounded check, and validate report consumption before adopting the new binary. Downgrade must respect supported schemas; never rewrite historical files into an older shape merely to make parsing pass.

Target plugin integration should map host invocation to the same CLI semantics, return result/next action to the conversation, and keep transport failure distinct from quality status. Host-specific fail-open policy must not erase unresolved findings. No MCP server or host binding is provided by this baseline's command dispatcher.

## 16. Architecture decisions

| Decision | Choice and rationale | Revisit when |
| :--- | :--- | :--- |
| A01 Language | Rust orchestration; preserve native analyzers to retain ecosystem semantics | A measured platform constraint requires a bounded adapter change |
| A02 Result model | Findings and completion are separate | New category needs additional dimensions without erasing either channel |
| A03 Persistence | Reviewable files and local receipts | Measured scale/concurrency requires a store migration with compatible projections |
| A04 Agent boundary | Deterministic checks and transitions; agent proposes repairs | A model-assisted advisory feature has independent evaluation and cannot grant authority |
| A05 Exceptions | Precise identity, approval and lifecycle instead of broad bypass | Valid project cases require richer identity without expanding unintended scope |
| A06 Architecture discovery | Evidence-labelled unknowns and direct declarations | Effective model/source graph resolution has controlled execution and tested provenance |

These decisions document current intent and constraints, not retroactive approval of missing features. The technical design gives implementation detail, phase dependencies and observable acceptance criteria.

## 17. Acceptance and unresolved questions

Required acceptance dimensions are native semantic correctness, false-positive and false-negative measurement, fault handling, stable task identity, bounded retries, configuration/suppression changes, input freshness, platform behavior, and actual host delivery. Passing unit tests or having a schema file proves none of these in aggregate.

The dated local baseline is 990 passing tests with 101 ignored. No production false-positive rate, complete native matrix, MSRV matrix, full-platform support or remote CI result is inferred from it.

Open decisions for production include approval-source ownership, practical exception review UX, tool distribution/upgrade ownership, complete cross-platform process isolation, schema retention, private vulnerability reporting, and release packaging. They do not prevent using explicitly scoped local observations, but they do prevent declaring a completed production quality gate.

---

**Document version:** 1.2.0 · **Created:** 2026-09-28 · **Updated:** 2026-10-04 · **Status:** ready for review; implementation remains partial.

Source builds now provide native-first edit feedback through `hook execute` / `hook claude post-tool-use`: only explicit ordinary files are selected; Python uses Ruff and JS/TS uses module-local ESLint 10. Coherent same-byte native results avoid duplicate parsing. Uncovered files may use pinned WASM candidates; mixed scopes retain native results, unwired native scopes and failures. Recovery nodes require native-tool setup/repair and confirmation; complete zero-recovery candidates only recommend native lint, never full acceptance. One deadline bounds at most eight files and two WASM workers; builds without WASM report that gap. The outer feedback is 0.7.0 with local `hook_fast_feedback` 0.2.0. Candidate task synchronization is connected; default plugin Hooks, capability-matched closure and real-host acceptance remain incomplete. See [edit-feedback acceptance](../tests/acceptance/hook-fast-native-wasm.md).

Source edit feedback now imports recovery-bearing pinned WASM candidates into the existing `.codeguard/` workbench. Confirmation identities remain stable per workspace/file/language; Python and JS/TS reuse existing confirmation/preparation identities. Reports bind the grammar, source SHA-256, known limitations and suspected byte positions; imports reject mismatched identities/coordinates and duplicate JSON keys. Task IDs appear only after successful synchronization. Zero-recovery observations create no new mandatory task and cannot close existing tasks. Dialogue includes `task show` / `task verify`; missing native confirmation adapters report an explicit capability gap. Outer Hook feedback is 0.7.0, local feedback is 0.2.0, and generic `next` briefs use 0.3.0 while existing checkers retain 0.1.0. Default plugin Hooks, capability-matched closure and actual-host acceptance remain incomplete.


### Native verification of a Zig confirmation task

Source builds can now verify a persisted Zig WASM confirmation task with `codeguard task verify TASK_ID . --zig-tool /absolute/path/to/zig --format=json`. The same explicit tool option is accepted by `hook execute` for `repair_ready`, using the existing lease and finished-attempt binding. `lint zig` can run the native tool without building the optional WASM feature; fallback without that feature reports its absence.

The pinned Zig 0.16.0 probe runs `version` and `ast-check --color off` against the exact source bytes under one deadline. A current native diagnostic changes the brief to source repair; missing tools, unsupported versions and execution failures retain environment/decision guidance. Fresh `next` briefs carry the unchanged tool path in their recheck argv. Changes to source or tool bytes invalidate old diagnostic guidance. Reports and attempts are retained under the existing workbench instead of a second task store.

The native observation is `syntax_task_recheck` 0.1.0, wrapped by `task_verification_preview` 0.12.0. Generic repair briefs are 0.3.0; old 0.2.0 briefs and 0.11.0 verification schemas remain available unchanged. Zero native AST diagnostics are `candidate_absent_unverified_policy`: they end the pending local verification step, but do not close the task or certify project lint, build or delivery. Erlang now has the same workbench integration through explicit OTP 28; [Erlang task acceptance](../tests/acceptance/erlang-native-task-verification.md) records its separate protocol versions. Remaining generic languages still lack native confirmation adapters. See [the acceptance record](../tests/acceptance/syntax-native-task-verification.md).

Briefs also expose the latest native report reference/digest and current diagnostic positions; stale inputs suppress those positions. Only immutable grammars compiled into the binary reuse validated asset identities within a process; external manifests, source and native tools still require current-byte checks.


### Verified closure and recurrence for one task (source SDK)

The source exposes `verify_zig_task_resolution` for a protected host to verify a **Zig native syntax-confirmation task**. The host independently pins its verification key, workspace, policy revision, code baseline, trusted time and rollback floor, and supplies the hash-bound original counterexample. Project-selected keys, policy candidates and task Markdown cannot provide that authority. The default plugin and public CLI still lack a trusted policy provider; this API is absent from published npm 0.1.3.

Under the existing task lease, the handler runs the same approved Zig 0.16.0 against the original bytes and current file. Both runs share the request deadline, capped by approval expiry. A `code_fixed` event requires original native diagnostics, changed current source with no diagnostics, and matching tool, host artifact, grammar, task and policy identities. A native-clean original becomes a false-positive investigation; environment failure or changing input requires verification. This proves syntax for the specified task, not full lint, types, security or CVE coverage.

Events replay through explicit parent links. Repeating the same verified result preserves its existing event. Ordinary `task verify` can append `reopened` when matching native diagnostics recur on current input, retaining the original fact and repair history and restoring native repair guidance. Missing parents, forks, duplicate identities, absent evidence or changed hashes require reconciliation. The handler also writes the existing native attempt receipt, clears matching `awaiting_verification`, and preserves a borrowed lease.

Sanitized events live in `.codeguard/findings/<id>/events/lifecycle-*.json`; comparison evidence is ignored under `.codeguard/state/resolution_evidence/`. The first `finding.json` is immutable. Local `next/task show/status` have no trusted policy context and cannot elevate historical claims into current closure or delivery approval. All host receipts retain `delivery_decision=not_evaluated`.

Other checker closures, environment/dependency/target/policy dispositions, actual host trust providers, cross-machine evidence recovery and the delivery gate remain open. See [task lifecycle acceptance](../tests/acceptance/task-resolution-lifecycle.md).


```mermaid
flowchart TD
    A[Protected host pins policy and trust context] --> B[Verify signature and original task identity]
    B --> C[Acquire or borrow existing lease]
    C --> D[Same Zig checks original counterexample]
    D --> E[Check current bytes and revalidate identities]
    E --> F{Native result and input}
    F -->|Original invalid and current repaired| G[Append resolution evidence and parent event]
    F -->|Original also native-clean| H[False-positive investigation]
    F -->|Tool failed or input changed| I[Preserve failed observation]
    G --> J[Ordinary task verify native recheck]
    J -->|Issue recurs| K[Reopen same task with history]
    G --> L[Independent complete delivery check]
    K --> J
```


### Current public candidate: 0.1.4

`@partme.ai/codeguard@0.1.4` is published for Apple Silicon macOS from clean source `1cd458f6e01a44a74388243e964e3f45290ac18e`. It includes all 32 runnable, unqualified grammars, bounded edited-file checks, stable native-confirmation tasks, native rechecks and `next` guidance. Registry hashes, a fresh-cache npx invocation, the actual public-package repair loop with Zig 0.16.0, and the source commit's Linux CI passed. Ordinary CLI clean output cannot close a task without trusted policy. The protected Zig SDK is a source integration API; npm does not expose a self-approval command. Plugin activation, installed-host acceptance, full precision, other platforms and complete gates remain open. Earlier 0.1.3 evidence is historical. See [0.1.4 acceptance](../tests/acceptance/npm-0.1.4-candidate.md).


### Erlang confirmation tasks and native repair guidance (current source)

`codeguard task verify TASK_ID . --erl-tool /absolute/path/to/erl --format=json` and the `repair_ready` event of `hook execute . --erl-tool /absolute/path/to/erl` now reuse the existing `.codeguard/` task, lease, finished attempt and shared deadline. They call the same OTP 28 forms parser as `lint erlang`. Task language and tool options are checked before acquiring a lease or launching a tool; project source, macro expansion and parse transforms are not executed.

```mermaid
flowchart LR
    A[Suspected edited syntax] --> B[Stable workspace/file/language task]
    B --> C[task verify / repair_ready]
    C --> D[Explicit native OTP 28 parser]
    D -->|Current diagnostics| E[next positions and repair argv]
    E --> F[Repair and record attempt]
    F --> C
    D -->|Preprocessing or tool fault| G[next concrete blocking reason]
    G --> H[Restore project context or matched tool]
    H --> C
    D -->|Zero diagnostics| I[Retain evidence pending closure verification]
```

Erlang observations use `syntax_task_recheck` 0.2.0, verification feedback 0.13.0 and post-verification `next` briefs 0.4.0. Zig and historical schemas remain unchanged. `native_confirmation_reason` identifies missing tools or preprocessing gaps; guidance names OTP 28 preparation or project-native preprocessing/compilation. Source changes invalidate diagnostic guidance; changed tools lose their stored argv. Only a byte-current tool already observed as OTP 28 can be reused as ready.

Zero diagnostics remain `candidate_absent_unverified_policy`: they consume the linked pending verification, but cannot close the task or approve delivery. Two no-progress attempts use the existing decision budget. The protected closure API still supports only Zig; full Erlang project lint/preprocessing, native-finding lifecycle, automatic tool discovery and installed-host releases remain open. Public npm 0.1.4 does not include this extension. [Acceptance and protocols](../tests/acceptance/erlang-native-task-verification.md).

Erlang `repair_ready` feedback uses Hook 0.8.0 / local summary 0.2.0 to include current native positions, the concrete unresolved reason, Unicode scalar column units and an existing report reference. Other Hook versions remain unchanged; installed-host delivery is not certified.

### Aggregate Erlang native-first checking (current source)

```bash
codeguard check all . --format=json
codeguard check all . --erl-tool /absolute/path/to/erl --timeout 30s --jobs 2 --format=json
```

The `erlang.lint` task selects explicit Erlang or the first executable `erl` from absolute PATH directories, then uses the existing controlled OTP 28 scanner/parser. It observes at most 64 ordinary UTF-8 files, each at most 1 MiB, under the request deadline and scheduler concurrency. `native_results.erlang_lint` carries current source hashes, positions, tool selection, per-file next actions and original-tool recheck argv with an absolute source path, reusable outside the project directory. Files beyond the native budget are counted as unobserved.

Only complete, non-preprocessed forms observations for matching source and tool bytes skip duplicate WASM. Missing tools retain candidate observations; selected-tool failures stay visible alongside any supplementary candidate observation. Macro/preprocessor coverage remains unresolved. Source or tool changes withdraw affected positions and reusable argv. Project-scope changes set `scope_stable: false` and retain still-current single-file diagnostics; whole-scope completeness is withdrawn. SIGINT remains exit 130; JSON, human and conservative SARIF retain native findings without claiming project success.

Protocols are `check_feedback` **0.36.0**, `check_aborted` **0.13.0**, and the embedded `erlang_forms_scan` **0.1.0**. Earlier aggregate schemas are preserved byte-for-byte. Forms completeness is distinct from full lint/build/test coverage. Native-finding task persistence and trusted closure are still unimplemented; the report exposes `task_id: null` rather than inventing a task. Existing WASM-origin Erlang confirmation tasks retain their separate `task verify` workflow. Public npm 0.1.4 does not include this new aggregate path. See [acceptance](../tests/acceptance/check-all-erlang.md).

```mermaid
flowchart TD
    A[check all: Erlang files] --> B{Explicit tool or PATH erl}
    B -->|Available| C[OTP 28 version and tool identity]
    C --> D[Bounded per-file forms parsing]
    D --> E{Current and complete without preprocessing?}
    E -->|Yes| F[Retain native diagnostics and recheck argv]
    F --> G[Skip duplicate WASM for matching bytes]
    E -->|No| H[Retain native blockers and any candidate observation]
    B -->|Absent| H
    G --> I[Project lint/build/tests remain incomplete]
    H --> I
    I --> J[Human / JSON / conservative SARIF]
```
