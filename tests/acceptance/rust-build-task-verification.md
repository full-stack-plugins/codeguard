# Rust 编译任务原工具复检

2026-09-28 局部验收。`codeguard build rust` 在已初始化工作区保存稳定编译任务后，`codeguard task verify CG-... . --cargo-tool ABS_PATH --format json` 持任务租约重跑原生 `cargo check --locked --offline --all-targets --message-format=json`。同一任务发现仍在时返回 `still_present`；原生零诊断返回 `candidate_absent_unverified_policy`，不会关闭任务或批准白名单。`next` 从任务事件和本地报告读取复检状态，并指向 `task verify`。

复检封套 `rust_build_task_recheck` 0.1 绑定任务、工作区、全部已发现 Rust 源码、根清单、锁与原工具摘要；当前任务反馈为 `task_verification_preview` 0.8。输入集合或字节变化时旧观察不能作为当前修复证据。原生工具故障、报告异常与零诊断均保持 `delivery_decision=not_evaluated`。只有类型检查执行，`test_execution=false`，没有完成项目策略或完整构建组合核验。

验证入口：`cargo test -p codeguard-cli --test rust_build_cli`；显式既有 Cargo 的真实复检：`CODEGUARD_CARGO_BIN=/Users/wandl/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo cargo test -p codeguard-cli --test rust_build_cli actual_cargo_task_verify_keeps_clean_type_check_as_unverified_candidate -- --ignored`。测试必须证明同一错误仍在、修复后的零诊断仅是候选、任务仍可由 `next` 获取，以及错误报告不能用白名单候选消除。

此验收不证明可信工具/策略批准、白名单实际放行、完整 workspace/features/targets、自动关闭和复发重开，也不证明所有语言类别与宿主门禁。
