# Windows focused projection harness

This unpublished standalone test package compiles the actual CodeGuard projection, run_report and check_request production source using path modules. It does not copy algorithms or claim the Unix CLI can run on Windows. Dependencies are real codeguard-core/adapters/runtime and guard-protocol/contract. Its independent lockfile and target support Rust 1.85 focused verification without compiling unrelated Unix-only CLI modules.

Use `cargo +1.85 test --locked --offline --manifest-path crates/codeguard-cli/tests/engineering_projection_harness/Cargo.toml`. Actual binary/provider verification is `engineering_cli_smoke` in early Ubuntu CI. See docs/engineering-guard/codeguard-projection.md for boundaries. Configure Cargo cache/TEMP/TMP/target to E: when running in the Windows development workspace.
