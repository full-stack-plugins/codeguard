# Codeguard Documentation

[简体中文](README.zh_CN.md) · [Project README](../README.md)

This is the documentation entry for Rust Codeguard. The eleven former `rust-cli/` files have been integrated into the main documents and independent topic guides. There is one architecture and one technical design, each with a Chinese counterpart. Observations refer to HEAD plus the working tree reviewed on 2026-09-28; target designs are not shipped-feature claims.

## Reading order and ownership

1. [README](../README.md): installation, available scope, quick start and troubleshooting.
2. [Architecture](Codeguard-Architecture.md): boundaries, components, domain semantics, execution paths and decisions.
3. [Technical design](Codeguard-Technical-Design.md): shared contracts, detection families, canonical report examples and roadmap.
4. Topic guides below: detailed responsibilities, without a second task ledger.

| Topic | Responsibility |
|---|---|
| [Command contracts](Codeguard-Command-Reference.md) | C01–C36, parameters, budgets, exits and side effects |
| [Project initialization](Codeguard-Project-Initialization.md) | Static observations, module relations, owned files and AGENTS refresh |
| [Persistent remediation](Codeguard-Remediation-Workflow.md) | Stable findings, briefs, leases, attempts, verification and recurrence |
| [False-positive governance](Codeguard-False-Positive-Governance.md) | Triage, exact dispositions, lifecycle and feedback |
| [Native adapter contracts](Codeguard-Adapter-Contracts.md) | Native configuration/reports and Java/CVE/security boundaries |
| [Trust and distribution](Codeguard-Trust-and-Distribution.md) | Signatures, revision chains, tool packages, npm and grammar assets |
| [Validation and rollout](Codeguard-Validation-and-Rollout.md) | Language matrix, F01–F26, precision targets, host/release evidence |
| [Legacy compatibility](Codeguard-Legacy-Compatibility.md) | Old CLI/MCP/hook mapping and the dated 2026-09-24 audit |

## Current, target and historical evidence

Current slices are grounded in source, tests and scoped acceptance records; dispatch determines callable commands. WASM fallback, complete trusted gates, formal repair closure and automatic host workflows still have pending work. Generated [CAPABILITIES](CAPABILITIES.md) describes full slots; a gap does not negate an implemented partial native path.

C01–C36 are command-design IDs and F01–F26 are failure-acceptance IDs, not completion marks. English and Chinese guides share boundaries and conclusions. Detailed per-item tables, parameters and historical records are retained in the Chinese companions and explicitly linked from the English guides.

## Specification and evidence

The existing [OpenSpec proposal](../openspec/changes/introduce-rust-codeguard-cli/proposal.md), [specs](../openspec/changes/introduce-rust-codeguard-cli/specs), [tasks](../openspec/changes/introduce-rust-codeguard-cli/tasks.md) and [implementation coverage](../openspec/changes/introduce-rust-codeguard-cli/implementation-coverage.md) remain authoritative. See [acceptance records](../tests/acceptance) for bounded observations. Consolidation does not mark implementation tasks complete or archive the change.

The [consolidation record](../openspec/changes/introduce-rust-codeguard-cli/documentation-consolidation.md) maps old ownership and resolved conflicts. Original migration paths/digests remain historical provenance.
