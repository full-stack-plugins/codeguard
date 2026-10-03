# Codeguard CLI

`@partme.ai/codeguard` runs the Rust Codeguard CLI through a small Node launcher. The npm package does not run installation scripts or download analyzers. Native tools such as Maven, P3C, ESLint, and language-specific linters remain separate prerequisites for their respective checks.

```bash
npx @partme.ai/codeguard --version --format json
npx @partme.ai/codeguard detect . --format json
npx @partme.ai/codeguard init . --apply
```

This initial npm package contains an **Apple Silicon macOS** executable only (`darwin-arm64`). npm rejects installation on other platforms. The CLI is an early implementation: individual checkers and end-to-end quality gates are incomplete. A successful command or an empty task list does not certify a project as clean.

Version `0.1.3` bundles 32 pinned Tree-sitter WASM grammar candidates. The Rust CLI can run them through `grammar probe` and bounded `check all` after native checks. Every candidate remains unqualified: observations return `incomplete` and do not replace native lint or certify a clean project. The package includes upstream grammar notices in `grammar-licenses/`.

```bash
npx @partme.ai/codeguard@0.1.3 grammar status --format=json
npx @partme.ai/codeguard@0.1.3 grammar probe zig ./sample.zig --format=json
npx @partme.ai/codeguard@0.1.3 check all . --format=json
```

Candidate commands return exit code `3` because the result is incomplete. `grammar status` reports `candidate_count: 32` and `released_count: 0` until language qualification is complete.

The project workspace created by `init` is `.codeguard/`; existing user source under `codeguard/` stays in scope. See the [repository README](https://github.com/full-stack-plugins/codeguard#readme) for supported commands, current coverage, and limitations.

## 中文

此 npm 包只提供精简 Node 入口和 Rust 原生程序，不在安装时构建、下载或运行检测工具。首个 npm 版本仅支持 Apple Silicon macOS。`init` 创建的项目数据目录是 `.codeguard/`；部分检测器和完整质量门禁仍在开发中。具体能力和限制见[项目说明](https://github.com/full-stack-plugins/codeguard/blob/main/README.zh-CN.md)。

`0.1.3` 包含 32 份固定 Tree-sitter WASM grammar 候选，可通过 `grammar probe` 和原生检查后的有界 `check all` 调用。候选观察仍返回 `incomplete`，不能代替原生 lint 或证明项目质量通过；上游许可证随包存放于 `grammar-licenses/`。
