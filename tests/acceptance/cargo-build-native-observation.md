# Cargo 原生构建观察前置验收

2026-09-27；OpenSpec introduce-rust-codeguard-cli，7.1未完成。

独立Rust解析Cargo check stdout及退出契约，结构化保留原生E编码/主定位/包/清单/目标；原生自由文本不进入结构化诊断。坏流、重复键、结束不完整或矛盾、环境失败无可归属诊断均未完成。多主定位、非E错误及其它无法归属情形保守未完成，未来执行层还须核对真实输入和目标。

RED：新入口未实现导致unresolved import。GREEN：新协议8、adapter库6、Rustdoc7、边界6共27项普通通过；真实既有Cargo原生1项通过（0.94秒），覆盖类型错误E0308/成功check但panic测试未执行/build.rs失败；临时目录别名首轮路径断言失败保留，改为实际路径归属比较后通过。相关Clippy -D warnings通过（10.63秒）。

复跑：cargo test -p codeguard-adapters --lib --test cargo_build_contract --test cargo_rustdoc_contract；cargo test -p codeguard-cli --test crate_boundaries；显式CODEGUARD_CARGO_BIN后cargo test -p codeguard-cli --test cargo_build_native -- --ignored。原生测试在CLI crate，adapter只保留纯解析/模型，不新增进程依赖。

尚未接入build rust、共享runtime的进程/预算/取消/快照、源范围重建与工具信任、持久任务/verify/check all；完整workspace/features/targets及测试等级策略待实现。纯解析无issue不代表完整项目构建、测试通过或交付允许。未重跑全工作区；旧910项基线在本变更之前。
