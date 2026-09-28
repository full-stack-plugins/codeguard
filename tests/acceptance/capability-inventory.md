# 能力矩阵基线（2026-09-24）

目标：OpenSpec `introduce-rust-codeguard-cli` 1.5。`schemas/capability-inventory.schema.json` 定义 language×category×platform 结构，Rust `codeguard-cli/examples/gen_capability_docs.rs` 从同一发行注册表生成并核对 `docs/CAPABILITIES.md`。

实测 `target/debug/codeguard capabilities --format json` 返回 57 条：54 stable、3 planned；每条有 5 个候选平台，每个平台有 lint/comments/dependencies/cve/security/build 6 个类别，共 1710 个显式单元，现阶段全部为 `gap`。完整报告包含发行版本。Julia 和 Pascal 的旧清单仅有 formatter、没有 lint；Rust 正反例拒绝把 formatter 作为 lint 能力。`implemented` 或 `not_applicable` 需要证据引用；未来真实实现需逐单元提供工具身份和验收证据。

`capabilities metal --platform=macos_arm64 --category=lint --format=json` 返回 `capability_selection` 单元，旧状态仍为 planned，新能力仍为 gap，已验证组合为空。未知语言/平台/类别为 exit 2，注入损坏注册表为 exit 4。筛选只读取内置发行注册表，不触发项目探测、安装或原生工具执行；筛选报告有独立严格 JSON schema。

验证：`cargo test --workspace --offline`、`cargo run --offline -p codeguard-cli --example gen_capability_docs -- --check` 均通过。此处只证明登记完整、缺口诚实，并未完成任何原生工具验收。
