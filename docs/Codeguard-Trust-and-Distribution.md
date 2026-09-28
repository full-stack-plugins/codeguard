# Codeguard Trust and Distribution

> **Document version:** 1.2.0 · **Updated:** 2026-09-29. Current slices and target contracts are explicitly separated.

[简体中文](Codeguard-Trust-and-Distribution.zh_CN.md) · [Documentation](README.md) · [Architecture](Codeguard-Architecture.md) · [Technical design](Codeguard-Technical-Design.md)

## 1. Separate observation, candidates and authority

Native project configuration determines check semantics; runtime settings govern budgets; policy/rulepack/tool-lock candidates describe proposals; approval comes from protected host/CI context. Matching bytes do not establish their authority. Local flags, environment variables, candidate text and agent statements cannot approve themselves.

Current candidate validation/explanation keeps effective policy unavailable. Exact exclusions need a precise file/content identity, structured reason and finite lifetime. They are pre-scan scope decisions, unlike post-finding false-positive dispositions. Neither can replace the other.

Target rulepacks use `rulepacks/<pack-id>/<version>/` with manifest, rules, native tool configurations, compatibility and fixtures. Preserve provenance, licenses, digests, rule semantics and migration behavior. A candidate directory is not a complete released rulepack.

## 2. Snapshot and revision binding

Approval snapshot 1.0 binds policy revision, maximum validity and approved candidate ID/raw-byte SHA-256. Binding checks expected snapshot digest, membership, revision, validity and exact native finding identity. `BoundToPinnedSnapshot` proves input binding only; it does not prove that the supplied hash, clock or finding came from a protected source.

Later snapshot/replacement contracts preserve revocation and replacement ancestry. Multi-hop traversal is bounded (32 prior links), rejects cycles and requires exact predecessor identity. Candidate edits cannot inherit approval for previous bytes. Preserve historical valid decisions and current revocations separately; file timestamps cannot establish approval chronology.

## 3. Signatures and Git ancestry

Current Ed25519 verification uses ring and domain-separated input: message domain, NUL, key ID, NUL, exact UTF-8 payload bytes. Do not reserialize JSON before verifying. Bind workspace, policy revision, baseline, sequence, lifetime and snapshot digest. Host-pinned keys/time/minimum sequence remain external authority; there is no project self-signing approval CLI.

The current multi-hop entry verifies historical signatures/snapshots and ordering/time constraints before replacement/revocation checks. The joint Git entry additionally checks native ancestry, allowing equal commits. Missing objects, shallow history, unsupported safe Git options, cancellation or exhausted budget are incomplete, not permission to fall back to signature-only success.

The final domain gate compares host-frozen checker/tool/category/obligation mapping, primary target and complete observed identity. Duplicate findings or reused decisions are rejected before application. Missing identity, changed bytes/fingerprint/tool/adapter/rulepack/graph/advisory retains active findings and unresolved status.

Final matching needs a trusted current clock and independently frozen current policy revision and approval scope. Expiration is exclusive; an old observation timestamp cannot renew approval. Baselines are complete nonzero lowercase SHA-1/SHA-256 commit IDs, representing approval context rather than a temporary worktree HEAD. Old inputs remain readable but lack authority to waive findings when required scope is absent.

Verified previews are read-only bindings, not deserializable project approval tokens; their independent-approval marker stays false. Real host approval production, provenance freezing and final gate integration remain unfinished. See [verifier](../crates/codeguard-cli/src/signed_approval_verifier.rs).

## 4. Tool artifacts and publication

| Building block | Existing bounded contract | Remaining product boundary |
|---|---|---|
| Inventory/verify | Explicit lock, entry/runtime/tree digests and permissions; unapproved matches remain untrusted | Trusted required inventory |
| Install preview | Manifest/lock/full-layout binding; content verification not run; public apply blocked | Authorized apply and recovery |
| HTTPS | Fixed URL/size/hash/redirect authority, TLS, bounded bytes and shared cancellation/deadline | Real host source/network authorization |
| ZIP/tar.gz | Validate all members; reject traversal, links and sparse entries; bounded GNU/PAX support | Full format/platform qualification |
| Bundle/layout | Defined tree digest and root projection; preserve unconsumed members and complete install-tree digest | Runtime/destination integration |
| Raw/tree publication | Private staging, fixed directory handles where supported, no overwrite and revalidation | Public CLI orchestration and Windows equivalence |
| Distribution signature | Separate distribution domain and manifest/lock/publisher/channel/platform/sequence/lifetime | Real key/revocation/time sources and public installer integration |

Network publication verifies signature and exact site permission before transfer; download/publication share the budget. A local URL plus self-computed digest cannot create approval. Receipts certify only their stage, never complete install or code quality. Do not overwrite corrupted/link/FIFO/unsafe destinations. Cancellation cleans owned staging; post-publication failure may retain a complete artifact and retry must revalidate it. Ordinary same-user local execution is not a malicious-peer sandbox.

## 5. npm and WASM assets

The recorded npm `@partme.ai/codeguard@0.1.0` publication wraps an Apple Silicon macOS Rust binary. Node forwards argv/cwd/environment/streams/exit and is not another checker implementation. Current packaging has no automatic grammar download or install hook. Public installation verification is scoped in [npm acceptance](../tests/acceptance/npm-public-candidate.md); immutable source/tag and multi-platform qualification remain separate.

The repository now includes a [candidate asset manifest](../grammars/manifest.json), fixed-commit Java/TypeScript WASM bytes and three MIT licenses. The [Rust verifier](../crates/codeguard-adapters/src/grammar_asset_manifest.rs) checks provenance fields and exact bytes. Both assets measured ABI 14 under CodeGraph's runtime; CodeGuard Rust loading, dialect versions and syntax corpora remain unvalidated, so they are explicitly `candidate_unvalidated`. See [scoped acceptance](../tests/acceptance/grammar-asset-candidates.md). They are not yet packaged into public npm or selected by lint.

CodeGraph loads these WASM files with JavaScript `web-tree-sitter`, which is also how this asset candidate's ABI was observed. CodeGuard's previously agreed Rust detection runtime is to load the **same pinned bytes** through Tree-sitter `WasmStore` or an independently validated Rust equivalent. A successful Node load or manifest check does not prove Rust compatibility; actual loading is OpenSpec 14.2.

Full WASM distribution still requires pinned patches, compatibility ranges and corpora. Copying an active build directory is not a release contract. Users should not require tree-sitter-cli. The Rust parser host needs memory/time limits, safe loading, integrity verification and explicit unsupported/error status.

Retain release/grammar manifests for rollback and invalidate observations affected by changed identities. Preserve historical workbench records; rollback cannot erase findings or required native checks. Detailed retained snapshot and package edge cases are in the [Chinese companion](Codeguard-Trust-and-Distribution.zh_CN.md).
