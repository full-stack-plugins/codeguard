# Rust 构建公开CLI探针验收

2026-09-27；OpenSpec introduce-rust-codeguard-cli；7.1/9.7未完成。

build rust复用共享runtime、预算/取消、SourceSnapshot与私有target，调用固定锁定离线all-targets类型检查，不执行测试。前后核验源集合/清单/锁/工具，原生目标与源范围归属有效才给限定修复简报；变化/歧义/取消/环境失败只调查，取消130优先。包身份摘要与目标类别保留，报告原生解析后的个人简报历史仍标记not_integrated；已初始化工作区的报告同步和任务见后续 rust-build-workbench-sync.md，不生成批准/关闭。

反馈rust_build_local_observation 0.1严格schema，真实失败及成功输出通过Draft202012，六个伪造授权/覆盖/状态变体被拒。TDD缺入口先失败；最终普通build6/Rustdoc14/边界6共26通过，两个默认忽略不计通过。真实公开Cargo独立1项通过（7.15秒）：类型错误E0308、补齐后成功、panic测试未运行、build.rs环境失败；最终相关Clippy通过（5.63秒）。一次输入变化测试意外发现未完成，原子序列隔离fixture后全组通过，底层原因未完全归属，失败保留。

复跑：cargo test -p codeguard-cli --test rust_build_cli --test rust_comments_cli --test crate_boundaries。显式CODEGUARD_CARGO_BIN后cargo test -p codeguard-cli --test rust_build_cli -- --ignored。

未完成：原工具task verify/关闭复发/check all、原配置闭包、完整workspace/features/targets、可信工具/策略、跨平台宿主。新源没有全工作区终态；旧910项基线不替代新增行为验收。
