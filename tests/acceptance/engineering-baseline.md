# 工程基线验收（2026-09-24）

目标：OpenSpec `introduce-rust-codeguard-cli` 的 1.2、1.3。此记录只证明工程基线，不证明任一语言的原生检查或宿主接入。

- 本地独立工作目录：`full-stack-plugins-repositories/codeguard-cli/`；四个 Cargo package 与用户指定拓扑一致，`Cargo.lock` 已由 Cargo 离线解析生成。MSRV 声明为 Rust 1.85，当前执行工具链为 Rust/Cargo 1.98.1；MSRV 1.85 的实际编译仍需对应工具链验证。
- 旧语言快照 `rulepacks/legacy_languages.json` 与插件 `scripts/languages.json` 的 SHA-256 均为 `012370f9d83a2daeb3cc87fef292a7509e85b53540ce9adf1eaf171f0910d251`。54 stable / 3 planned 只作为迁移输入；查询输出全部标注 `check_capability=gap`。
- `cargo run --offline --bin codeguard -- --version` → 首行 `codeguard 0.1.0`，并列出目标平台、检查协议 major 及未验证的构建/规则包身份；`--format json` 使用 `schemas/version-report.schema.json`。
- `cargo run --offline --bin codeguard -- capabilities java --format json` → Java 旧状态 stable；六类别×五候选平台的新检查能力均为 gap。
- `cargo fmt --all -- --check`、`cargo test --workspace --all-targets --offline`、`cargo clippy --workspace --all-targets --offline -- -D warnings` 均通过；默认 Rust 单元与契约测试共 48 项，另有 4 项显式运行的 Ruff 0.16.8 真实工具复现（默认标记 ignored，不能混同默认结果）。核心聚合覆盖违规与未完成共存、取消/内部故障优先级、非阻断发现保留、UNKNOWN 严重度无法按策略判定时未完成；缺失或非法策略影响字段的反序列化拒绝。交付领域判定另见 `delivery-gate-baseline.md`，其中合成输入不构成真实交付 allow 证据。
- Rust `crates/codeguard-cli/tests/crate_boundaries.rs` 直接消费 Cargo metadata，拒绝 core→runtime、adapters→runtime、宿主 SDK→core、重命名/build/平台专属依赖。能力矩阵与语料校验、生成、真实 Ruff/Maven 回放均已迁为 Rust；新工作区的 `.py` 文件仅为 Python 语言检查用源码样本。

未完成：真实 adapter、检查命令、门禁、57 语言六类别能力、MSRV 目标工具链实测、跨平台和宿主验收；本工作目录尚未创建独立 Git 远程仓库。


## 2026-09-27 全工作区依赖边界失败与修复

上一全工作区测试已终止为exit101：52组结论累计271通过、1失败、23忽略，失败为crate_boundaries的真实Cargo metadata检查；不记录整轮通过。核对发现adapters两个原生测试调用runtime造成dev越层，且网络下载实现新增的runtime依赖和CLI测试异步依赖未登记。已将Checkstyle原生重放和Node字面参数原生测试移入CLI测试层，删除adapters→runtime dev依赖；保留adapters任何kind不得依赖runtime的红线。仅允许runtime normal reqwest/tokio/tokio-util/futures-util/idna_adapter与runtime dev rustls；CLI tokio只允许dev。新增对应正常/错误crate、build/dev/normal反例，现6项依赖边界与4项纯ESLint命令契约通过。迁移保留原生用例内容及ignored边界，不靠放宽生产依赖方向消除失败。

额外原生证据：已实时核验既有/opt/anaconda3/bin/ruff为0.16.8，check all部分结果1项、task verify8项、whitelist propose2项共11项真实Ruff用例通过；包含noqa、规则关闭、per-file-ignore、恢复环境不假关闭及纠错提案不自批。106份schema的静态合法性与本地引用检查通过，不能代替运行报告验收。MSRV1.85、Windows、三宿主仍未验收。修复后全工作区测试与all-targets Clippy重新运行，结果另附；13.2保持未完成。

补充终态：修复后all-targets Clippy -D warnings通过（29.62秒），fmt通过；迁移后的Node v24.18.0原生字面参数用例实际通过（1项），只证明argv安全转发，不冒充真实ESLint规则检查。Checkstyle重放迁移保留且本轮未运行。新的全工作区回归仍在原进程运行。
