# Codeguard CLI

`@partme.ai/codeguard` runs the Rust Codeguard CLI through a small Node launcher. The npm package does not run installation scripts or download analyzers. Native tools such as Maven, P3C, ESLint, and language-specific linters remain separate prerequisites for their respective checks.

```bash
npx @partme.ai/codeguard --version --format json
npx @partme.ai/codeguard detect . --format json
npx @partme.ai/codeguard init . --apply
```

This initial npm package contains an **Apple Silicon macOS** executable only (`darwin-arm64`). npm rejects installation on other platforms. The CLI is an early implementation: individual checkers and end-to-end quality gates are incomplete. A successful command or an empty task list does not certify a project as clean.

The published `0.1.2` package does not include WASM syntax prechecks. Future packages that advertise the 32 pinned grammar candidates must pass a WASM worker and asset probe before packing. Candidate observations remain incomplete and do not replace native lint or certify a clean project.

The project workspace created by `init` is `.codeguard/`; existing user source under `codeguard/` stays in scope. See the [repository README](https://github.com/full-stack-plugins/codeguard#readme) for supported commands, current coverage, and limitations.

## 中文

此 npm 包只提供精简 Node 入口和 Rust 原生程序，不在安装时构建、下载或运行检测工具。首个 npm 版本仅支持 Apple Silicon macOS。`init` 创建的项目数据目录是 `.codeguard/`；部分检测器和完整质量门禁仍在开发中。具体能力和限制见[项目说明](https://github.com/full-stack-plugins/codeguard/blob/main/README.zh-CN.md)。

已发布的 `0.1.2` 不含 WASM 语法初检。未来声称包含 32 份固定 grammar 候选的包，须在打包前验证 worker 与资产；候选观察始终不代替原生 lint，也不证明项目质量通过。
