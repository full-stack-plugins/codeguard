# Codeguard CLI

`@partme.ai/codeguard` runs the Rust Codeguard CLI through a small Node launcher. The npm package does not run installation scripts or download analyzers. Native tools such as Maven, P3C, ESLint, and language-specific linters remain separate prerequisites for their respective checks.

```bash
npx @partme.ai/codeguard --version --format json
npx @partme.ai/codeguard detect . --format json
npx @partme.ai/codeguard init . --apply
```

This initial npm package contains an **Apple Silicon macOS** executable only (`darwin-arm64`). npm rejects installation on other platforms. The CLI is an early implementation: individual checkers and end-to-end quality gates are incomplete. A successful command or an empty task list does not certify a project as clean.

Version `0.1.4` bundles 32 pinned Tree-sitter WASM grammar candidates. The Rust CLI can run them through `grammar probe` and bounded `check all` after native checks. Every candidate remains unqualified: observations return `incomplete` and do not replace native lint or certify a clean project. The package includes upstream grammar notices in `grammar-licenses/`.

```bash
npx @partme.ai/codeguard@0.1.4 grammar status --format=json
npx @partme.ai/codeguard@0.1.4 grammar probe zig ./sample.zig --format=json
npx @partme.ai/codeguard@0.1.4 check all . --format=json
```

The 0.1.4 candidate adds bounded edit feedback and persistent syntax-confirmation tasks on initialized workspaces. Repeated observations reuse the task, `next` supplies evidence and repair guidance, and an explicit Zig 0.16.0 tool can perform native task rechecks. Native zero diagnostics without protected policy evidence do not close the task. The published Node command does not expose the protected-host resolution SDK as a self-approval option.

```bash
npx @partme.ai/codeguard@0.1.4 next . --format=json
# Replace TASK_ID and the absolute path with the task and installed Zig tool:
npx @partme.ai/codeguard@0.1.4 task verify TASK_ID . \
  --zig-tool /absolute/path/to/zig --timeout 30s --format=json
```

Plugin hosts still need to bind the Rust Hook entry; installing this package alone does not switch existing plugin Hooks.

Candidate commands return exit code `3` because the result is incomplete. `grammar status` reports `candidate_count: 32` and `released_count: 0` until language qualification is complete.

The project workspace created by `init` is `.codeguard/`; existing user source under `codeguard/` stays in scope. See the [repository README](https://github.com/full-stack-plugins/codeguard#readme) for supported commands, current coverage, and limitations.

## 中文

此 npm 包只提供精简 Node 入口和 Rust 原生程序，不在安装时构建、下载或运行检测工具。首个 npm 版本仅支持 Apple Silicon macOS。`init` 创建的项目数据目录是 `.codeguard/`；部分检测器和完整质量门禁仍在开发中。具体能力和限制见[项目说明](https://github.com/full-stack-plugins/codeguard/blob/main/README.zh-CN.md)。

`0.1.4` 包含 32 份固定 Tree-sitter WASM grammar 候选，可通过 `grammar probe` 和原生检查后的有界 `check all` 调用。候选观察仍返回 `incomplete`，不能代替原生 lint 或证明项目质量通过；上游许可证随包存放于 `grammar-licenses/`。

0.1.4 候选增加有界编辑反馈和持久的语法确认任务。已初始化工作区重复观察复用同一任务，`next` 提供证据与修复指引，显式 Zig 0.16.0 可复检原生语法。没有受保护策略时，原生零诊断仍不关闭任务。npm 命令没有自批准入口；安装此包也不会自动切换已有插件 Hook。
