# Codeguard CLI

`@partme.ai/codeguard` runs the Rust Codeguard CLI through a small Node launcher. The npm package does not run installation scripts or download analyzers. Native tools such as Maven, P3C, ESLint, and language-specific linters remain separate prerequisites for their respective checks.

```bash
npx @partme.ai/codeguard --version --format json
npx @partme.ai/codeguard detect . --format json
npx @partme.ai/codeguard init . --apply
```

This initial npm package contains an **Apple Silicon macOS** executable only (`darwin-arm64`). npm rejects installation on other platforms. The CLI is an early implementation: individual checkers and end-to-end quality gates are incomplete. A successful command or an empty task list does not certify a project as clean.

The project workspace created by `init` is `.codeguard/`; existing user source under `codeguard/` stays in scope. See the [repository README](https://github.com/full-stack-plugins/codeguard#readme) for supported commands, current coverage, and limitations.

## 中文

此 npm 包只提供精简 Node 入口和 Rust 原生程序，不在安装时构建、下载或运行检测工具。首个 npm 版本仅支持 Apple Silicon macOS。`init` 创建的项目数据目录是 `.codeguard/`；部分检测器和完整质量门禁仍在开发中。具体能力和限制见[项目说明](https://github.com/full-stack-plugins/codeguard/blob/main/README.zh-CN.md)。
