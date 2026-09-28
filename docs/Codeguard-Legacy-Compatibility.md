# Codeguard Legacy Compatibility

> **Document version:** 1.2.0 · **Updated:** 2026-09-29. Current slices and target contracts are explicitly separated.

[简体中文](Codeguard-Legacy-Compatibility.zh_CN.md) · [Documentation](README.md) · [Architecture](Codeguard-Architecture.md) · [Technical design](Codeguard-Technical-Design.md)

## 1. Historical scope

This guide describes the old Python plugin protocol audited on 2026-09-24 and its Rust compatibility boundary. It is not the current Rust CLI command reference. The [Chinese protocol table](Codeguard-Legacy-Compatibility.zh_CN.md) retains each exact old entry, exit precedence, source location and test reference; its dated audit appendix is historical evidence.

A legacy CLI exit zero, MCP tool success or hook allowing an action is not new delivery approval. The compatibility projection keeps `new_delivery_decision=not_evaluated`. Rust computes its own 0/1/2/3/4/130 protocol; never forward old numeric meaning blindly.

## 2. Old CLI mapping

| Old entry | Historical result semantics | Migration consequence |
|---|---|---|
| Default / `check` | FAIL→2 outranks UNVERIFIED/PLANNED→1; only PASS/SKIPPED→0; no language→1 | Zero may contain only skipped checks |
| `fix` | Failure/planned/unverified/no language→1; successful/skipped/dry-run/no applicable changes may→0 | Formatter success or empty results cannot prove repair |
| `cve` | FAIL→2, then UNVERIFIED→1; empty→1; parameters/ecosystem/severity error→3 | Old 2 is not Rust usage error |
| `dockerfile` | UNVERIFIED or no target→1 even with failures; otherwise FAIL→2 | Preserve qualified failures when aggregate is incomplete |
| `detect` | Normal query, including empty roots→0; configuration error→1 | Empty discovery is not an empty-project quality certificate |
| `init` | Guidance and zero without writing project files | Not equivalent to Rust apply or a completed check |
| `java-plan` | UNVERIFIED→1; other plans→0 | Read-only planning, not native execution |

Unknown commands, argparse failures and MCP-specific envelopes require explicit per-entry handling. The detailed source mapping is retained rather than consolidated into a universal incorrect exit table.

## 3. MCP and hooks

MCP transport success only means the request was handled under that old tool's protocol. Structured result states still need interpretation; a success envelope must not override an incomplete native observation. Hook pass/fail-open behavior and host process exits likewise describe legacy transport/control behavior, not a trusted Rust gate.

Compatibility stays behind explicit `compat legacy-v1`; do not silently reinterpret native CLI commands. Preserve caller-visible fields and branch distinctions while exposing the new delivery decision as unevaluated. Golden fixtures need each old entry's positive, negative, mixed, empty and malformed behavior.

## 4. Dated audit and migration

The [2026-09-24 audit](Codeguard-Legacy-Compatibility.zh_CN.md#legacy-audit-20260924) records differences from an earlier plugin design snapshot. Statements that adapters/gates were absent refer to that historical scope and must not override current Rust implementation evidence.

The Rust repository now owns Rust architecture, contracts and OpenSpec implementation planning. The plugin repository owns its live host packaging/hooks and references Rust documentation. Historic plugin paths in the audit remain provenance; they are not missing Rust source files or an instruction to move Python code into Rust.

Migration requires preserving old consumer behavior where promised, making incomplete/unknown boundaries explicit, and validating actual host delivery. It does not authorize replacing native analyzers, weakening policy or claiming full coverage from compatibility tests.
