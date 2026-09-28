# Rustdoc 完整检查调度验收

2026-09-27；OpenSpec introduce-rust-codeguard-cli；7.1/9.7 保持未完成。

check all 对 Rust 项目调度 rust.comments/rust.lint，共享截止时间与 Cargo 资源互斥，独立保留原生状态、结果和准备任务。复用 comments rust 固定锁定离线库目标探针，初始化后自动保存并同步稳定文档 finding/任务。取消标记贯通原生服务；原生零诊断不授予规则覆盖或交付权威。

新增契约先因缺 Rustdoc 结果失败；接线后更新实际两节点/混合三节点预期、自包含 schema 和独立准备任务的 next 选择。普通相关67项通过，12项忽略不计通过；显式既有 Cargo 原生3项通过，包含 check all 真实 missing_docs 断言与文档修复/抑制对照。相关 Clippy -D warnings 通过。

协议 check_feedback 0.27 和 check_aborted 0.8，旧版本 byte-preserved。实际未初始化及初始化反馈通过 Draft202012，初始化文档 backlog_status=synced；异常中止协议只静态验证，不声称本轮运行验收。独立 Python 验证不属于 Rust 产品路径。

复跑：cargo test -p codeguard-cli --test check_all_go --test check_all_java_p3c --test check_all_npm --test check_all_partial_contract --test check_all_rust_native --test rust_comments_cli。显式 CODEGUARD_CARGO_BIN 后，两个 Rust 测试目标加 -- --ignored 运行真实工具。

未完成：原项目有效策略、全部 workspace/features/targets、可信工具/批准来源、正式关闭重开、跨平台宿主和其余语言类别；全工作区未重跑，完整目标继续。
