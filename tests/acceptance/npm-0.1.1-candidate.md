# npm 0.1.1 单平台候选来源绑定

对应 OpenSpec `introduce-rust-codeguard-cli` 的 S11.1、S13.4 与 `binary-distribution`。

2026-09-29：`version_cli` 先在显式 `CODEGUARD_BUILD_SHA` 下因 `build_identity=null` 失败；实现后 2 项通过。源码版本与锁同步到 0.1.1。公开 npm 打包器新增干净 checkout 和候选构建提交等于 Git HEAD 的要求：当前工作树未提交时，公开打包在写出候选包之前拒绝。普通本地包仍允许无构建提交声明，不能借此获得公开发行身份。

本次源码完整工作区 `cargo test --workspace --all-features -- --test-threads=1 -q` 退出 0；无效构建提交 `CODEGUARD_BUILD_SHA=not-a-commit` 的版本目标测试 2 项通过且报告 null。全目标 Clippy `-D warnings`、改动 Rust 文件的 rustfmt、Node 语法、OpenSpec strict 与 diff 检查通过。先前脏工作树中制作的本地 0.1.1 预演包只用于本机安装和 `PostToolUseFailure` 命令验证，不是正式候选包。

候选构建提交来自编译环境，本身可被伪造；干净 checkout 和自报匹配也不能证明可复现构建、发行签名、平台矩阵或宿主实际绑定。正式发布与验收证据在打包、注册表和新缓存运行完成后补记；完成前不勾选 S11.1 或 S13.4。
