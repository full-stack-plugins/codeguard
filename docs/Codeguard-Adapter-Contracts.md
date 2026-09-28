# Codeguard Native Adapter Contracts

> **Document version:** 1.2.0 · **Updated:** 2026-09-29. Current slices and target contracts are explicitly separated.

[简体中文](Codeguard-Adapter-Contracts.zh_CN.md) · [Documentation](README.md) · [Architecture](Codeguard-Architecture.md) · [Technical design](Codeguard-Technical-Design.md)

## 1. Native semantics are the boundary

Rust discovers configuration, prepares literal argv, controls processes and interprets native structured reports. It does not reimplement P3C, Maven, Javadoc, lint engines or vulnerability databases. The canonical six-category and 28-family portfolio lives in [technical design section 6](Codeguard-Technical-Design.md#6-static-detection-portfolio); registered families are not supported slots.

| Stage | Required contract |
|---|---|
| Discover | Declared/effective configuration, roots, language versions and unresolved conditions |
| Prepare | Exact tool/runtime/configuration identity, argv, working directory and input closure |
| Execute | Resource budget, cancellation, output bounds and raw exit |
| Parse | Tool-specific report and exit consistency, rule provenance and reversible locations |
| Normalize | Findings independent of completion, category-specific coverage and blockers |
| Recheck | Same obligation and target identities; no closure from empty or incomplete scope |

Do not grep generic words such as error, failed or warning to infer source violations. Report parse errors, missing files, crashes and unsupported versions are execution/evidence failures. Preserve qualified findings even when another part of execution is incomplete.

## 2. Configuration inspection and execution

The user needs to know whether a checker is configured and what it reports. Configuration inspection must not require running every checker merely to prove execution. Represent configured/missing/invalid/unknown separately from installed and executed. A configured declaration may still have an unresolved effective model.

Run only the relevant native contract. Output reports selected scope, configured source, actual execution status, findings, unresolved reasons and next action. Do not silently generate stricter native configuration, install tools or execute project scripts during static observation.

## 3. Ecosystem-specific obligations

| Ecosystem | Native families and essential boundaries |
|---|---|
| Java | Maven/Gradle effective configuration, P3C/PMD rule-set identity, Checkstyle, Javadoc/doclint, dependency and OWASP checks; source sets, JDK and classpath affect meaning |
| Rust | Cargo fmt/check/clippy/rustdoc/audit and declared combinations; `cargo check` is not a complete build or all-feature certification |
| Node | Project-local ESLint, package manager lock/workspace identity and npm audit; historical manifests must not broaden unrelated scope |
| Python | Configured lint/documentation/type/security/CVE tools with declared environments and dependencies; a partial probe is not ecosystem coverage |
| Go | Native project/module context and individual formatting/vet/build checks; archived compatibility does not establish full Rust-native coverage |
| Other registered languages | Explicit configuration, adapter and acceptance evidence required before a supported-slot claim |

These are tool families and boundaries, not an assertion that every listed checker is connected. The [Chinese current-slice matrix](Codeguard-Adapter-Contracts.zh_CN.md) links detailed source/tests and preserves retained Java/CVE/security design constraints.

## 4. Identity, security and dependencies

CVE results require precise component/resolved version, dependency graph and advisory database provenance/freshness. Distinguish direct/transitive/development/optional scopes and unresolved dependencies. Unknown database freshness is not safe. Redact private coordinates in public feedback without substituting redacted strings for trusted identity.

Security includes applicable source/configuration/secret/repository-entry checks. Check owned records before committing them; ignoring local generated output does not exempt arbitrary user code under `.codeguard/` or `codeguard/`. Current ordinary source discovery retains its dot-prefix policy, so `.codeguard/` is not indiscriminately rescanned as source. Owned records have separate schema/path/redaction checks and repository-entry security obligations. An ordinary `codeguard/` source directory remains in scope; its name grants no exclusion.

Adapters preserve original rule IDs and native severity separately from policy severity/disposition. Signed exception matching requires the exact current finding, configuration/tool/rule identities and independent approval; adapter output cannot self-approve.

## 5. WASM adapter — target only

A bundled grammar is a lightweight syntax/structure backend. It detects both `ERROR` and `MISSING`, respects grammar/dialect/version compatibility and groups recovery cascades. It supplies suspected syntax findings and parser completion, not type checking, cross-file semantics, CVE analysis or complete lint.

Select a ready native checker first. Missing native capability may trigger precheck; native failure/violation is never replaced by a clean parser result. Grammar load failure, unsupported syntax, zero checked files, timeout and cancellation remain incomplete/unsupported. Assets need immutable provenance, license, digest, ABI compatibility and bounded execution. See canonical [routing and examples](Codeguard-Technical-Design.md#53-native-first-selection-and-syntax-fallback--target-not-shipped).
