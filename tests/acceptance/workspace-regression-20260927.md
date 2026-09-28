# Rustdoc集成后的工作区回归基线

2026-09-27；对应 OpenSpec introduce-rust-codeguard-cli。

当前源码 cargo test --workspace 已退出0：154组、910通过、0失败、93忽略；全部doc-tests已结束，日志SHA256 8d647e591bac5d2d247e33a3e648922d102da3639c72909a0513ecb3b91c5ff8。日志 /tmp/codeguard-workspace-current-20260927.log。

cargo clippy --workspace --all-targets -- -D warnings 已退出0，日志SHA256 ba479f2fe8cb4e988013460eed8c16cd6d506830b65530f8804107cf89c7695d。基于既有Rust1.98.1，不声明MSRV1.85或Windows。

独立真实Cargo3项、反馈0.27协议、插件unittest621项与模拟142项，以及schema静态/OpenSpec/vendor验收见插件verification.md。忽略项不计通过，夹具不计真实原生，普通回归不证明批准、完整覆盖或宿主门禁。

完整计划仍未完成：全部语言与类别、可信策略及完整构建组合、关闭复发、跨平台宿主及发布。此基线不签发产品完整验收，不勾选13.2。后续源变更须按影响检查，不能借用旧基线覆盖新行为。
