# Rust project edition parser preparation — 2026-10-06

Scope: `CargoEditionDeclaration`, `RustProjectEdition` and public Unix `rust_project_syntax::observe` library service. Complete editing Hook, task ingestion/recheck and project edition discovery remain open. Existing fixed2024 developer differential is unchanged in meaning.

RED: declaration adapter did not exist, followed by missing project edition/syntax exports. A test-source delimiter typo was fixed before the missing-API RED was confirmed; that typo is not feature evidence. GREEN: static declaration/provider parsing, bounded filesystem lookup and context-bound native observation implemented.

Actual native evidence: [three edition reports](evidence/rust-project-edition-2026-10-06.json) bind the same source digest and installed fixed Rustfmt artifact, with default2015 diagnostics and declared2021/2024 no diagnostics. This is an edition applicability counterexample, not an independent precision corpus. Reports retain `coverage_proven=false`/`delivery_decision=not_evaluated`, source digests and relative manifests; no source/raw diagnostics or host absolute paths appear.

Reproduction using the already installed explicit tool:

```bash
cargo test --locked -p codeguard-adapters --test cargo_edition_declaration --test cargo_module_model_contract
cargo test --locked -p codeguard-cli --test rust_project_syntax
CODEGUARD_RUSTFMT_BIN=/absolute/path/to/rustfmt \
CODEGUARD_RUST_PROJECT_REPORT=/absolute/path/to/reports.json \
cargo test --locked -p codeguard-cli --test rust_project_syntax \
  real_parser_uses_project_edition_instead_of_fixed_2024 -- --ignored --exact --nocapture
python3 tests/rust_project_edition_schema.py
```

Development Python requires the existing jsonschema environment and does not run in the product. No dependencies/SDKs were installed.

Validated: adapter/model targets6 passed; project targets5 passed/1 explicit-tool conditional ignore, actual tool separately1 passed; WASM affected Rust5 targets39 passed/0 failed/7 conditional ignores. Project and historical differential schema tests each2 passed. Controlled cases cover default vs explicit vs inherited edition, malformed/duplicate/unknown declarations, missing provider/no startup, symlink refusal, changed provider, new nearer manifest and version-phase manifest mutation preventing a second invocation. Layering and OpenSpec strict passed. Final Clippy/default regression results are recorded below after execution.

Limitations: static declaration parsing does not prove full manifest validity, workspace membership, source target/module/macro inclusion context or entire project semantics. Explicit workspace locator outside the requested root is unresolved. Host/release/whole-language qualification is unchanged. No complete WASM workspace suite or protected Erlang draft execution is claimed. Details and Cargo primary references: [English design](../../docs/Rust-Project-Edition-Syntax.md), [中文设计](../../docs/Rust-Project-Edition-Syntax.zh_CN.md).

Final default Rust4-target regression:39 passed/0 failed/6 conditional ignores. After two additional scope/budget counterexamples, WASM project syntax target7 passed/0 failed/1 conditional ignore; the other previously measured targets are unchanged. Default parser units4 passed. Default/WASM workspace all-targets strict Clippy passed before those test-only additions; final test-target Clippy checks are recorded by the delivery commands. Local document link checks, layering, OpenSpec strict and diff checks passed. Remote CI is independently pending.

Final default/WASM project test-target strict Clippy both passed; no Rust source changes followed these checks.
