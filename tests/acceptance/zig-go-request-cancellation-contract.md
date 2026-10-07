# Zig/Go 原生检查的请求取消契约验收

日期：2026-10-07。沿用 introduce-rust-codeguard-cli 的 unified-cli-contract（退出 130、`command_status=cancelled`）、execution-kernel（真实终止原因）与 syntax-precheck（取消保持取消语义、不缓存 clean）；对应 8.2/8.101 的取消/故障恢复反例与 15.2/15.6 的取消恢复要素。本记录是局部实现证据，父任务尚未完整验收。

## 发现的缺口（先 RED）

1. `zig_syntax_probe`：版本探测与 ast-check 被请求级终止（SIGINT 取消、共享预算耗尽）打断时，原因被改写成 `zig_version_unverified_or_unsupported` / `zig_ast_check_incomplete`，把请求取消伪装成工具缺陷；版本 stdout 非 UTF-8 时整个观察返回 `None`，被上层解释为 `zig_tool_unavailable_or_untrusted`（工具缺失），同样失真。
2. `lint zig`：取消后继续落入 WASM 候选初检（取消后仍启动新检查），退出码固定 3，报告无 `command_status`，违反 unified-cli-contract 的取消优先级与 130 契约。
3. `go_syntax_probe`：版本/伴生版本/gofmt 扫描的请求级终止被改写为 `go_syntax_version_unverified` / `go_syntax_execution_incomplete`。
4. `lint go`：`observe` 已产生 `reason=request_cancelled`，但公开反馈保持 `command_status=incomplete`、`exit_code=3`；缺工具回退路径在取消后仍运行 WASM 候选并固定退出 3。

RED 证据（修复前实际运行）：`zig_syntax_probe::request_tests` 3 项、`go_syntax_probe::tests` 新增 2 项全部失败，失败输出显示被改写的原因；`zig_lint_cli::cancelled_native_zig_check_returns_130_without_running_wasm_fallback` 失败于 `Some(3) != Some(130)` 且 `syntax_precheck` 已在取消后运行；`go_lint_cli::cancelled_go_vet_scan_returns_130_with_cancelled_status` 失败于 `Some(3) != Some(130)`、`command_status=incomplete`。

## 实现边界

- 两个探针新增 `request_reason`：`Termination::Cancelled → request_cancelled`、`TimedOut|DeadlineBeforeStart → request_deadline_exceeded`，在版本判定与执行判定之前保留请求级原因；Zig 非 UTF-8 版本输出改为结构化 `zig_version_unverified_or_unsupported`，不再冒充工具缺失。
- `lint zig`（反馈 0.2.0→0.3.0，新增 `command_status`/`exit_code`）：入口预检 SIGINT、原生阶段返回 `request_cancelled` 或全局 SIGINT 置位时不落入 WASM，输出 `status/command_status=cancelled`、`exit_code=130` 并退出 130；WASM 后的取消同样返回 130。取消不缓存 clean、不签发通过。
- `lint go`（持久化 schema 保持 0.6.0 不变）：`persist_and_sync` 之后才把公开反馈标记为 `cancelled`/130，持久化报告保持 work_sync 可导入的 `incomplete`/3 身份；`emit` 按反馈 `exit_code` 返回。缺工具回退（外层 0.7.0）在取消时跳过 WASM 候选并返回 130，候选阶段中的取消同样标记 130。
- 不改变：原生发现与定位的有效性判定、工具字节/输入前后复核、`observe_for_check` 供 `check all` 的签名与语义、Zig 0.16.0 与 Go 1.23.4 版本锁定。

## 验证

```bash
cargo test --locked -p codeguard-cli --features wasm-precheck --lib          # 144 通过/0 失败/6 条件忽略
cargo test --locked -p codeguard-cli --features wasm-precheck \
  --test zig_lint_cli --test go_lint_cli                                    # 5+6 通过/1 条件忽略（真实 Go 1.23.4）
cargo test --locked -p codeguard-cli --features wasm-precheck \
  --test check_all_go --test check_all_native_preferred_go --test check_all_zig \
  --test go_finding_identity --test go_native_hook --test go_package_structure \
  --test check_go_package_structure --test go_syntax_fallback_candidate \
  --test go_task_resolution_service --test go_work_sync --test go_native_differential \
  --test zig_native_cli --test zig_native_discovery --test zig_native_first_resolution \
  --test zig_native_hook --test zig_native_workbench --test work_sync_cross_category
rustfmt --edition 2024 --check <本轮修改的 7 个文件>
# 真实原生（本机已装，未自行安装）：
CODEGUARD_ZIG_BIN=/opt/homebrew/bin/zig cargo test --locked -p codeguard-cli \
  --features wasm-precheck --test zig_native_differential -- --ignored        # 1 通过（9.07s，Zig 0.16.0）
CODEGUARD_GO_TOOL=/usr/local/go/bin/go cargo test --locked -p codeguard-cli \
  --features wasm-precheck --test go_lint_cli -- --ignored                    # 1 通过（16.05s，Go 1.23.4）
CODEGUARD_ZIG_BIN=/opt/homebrew/bin/zig cargo test --locked -p codeguard-cli \
  --features wasm-precheck --test zig_native_discovery --test zig_native_first_resolution \
  --test zig_native_workbench --test check_all_zig -- --ignored               # 4 通过（真实 Zig 0.16.0 全链路）
```

受控进程取消用带标记文件的阻塞 shell 工具触发真实 SIGINT（`/bin/kill -INT`），断言原生阶段先启动、取消后秒级返回、退出 130、取消原因不被改写、取消后 WASM 未运行。真实原生差分/CLI 复跑（Zig 0.16.0 ast-check 语料对照、Go 1.23.4 vet 违规/干净/编译错误/多模块/部分失败）在本机已装工具上通过，证明取消语义改动未破坏正常原生路径。受保护的 `tests/erlang_native_differential.rs` 未修改、未运行、未提交（SHA256 与基线一致）；未运行任何 workspace 级 clippy/test。

## 未完成

Go comments 类别适配（gofmt 不能冒充注释检查）仍未实现，需要共享入口文件授权；Zig lint 的 `zig fmt --check` 规范适配与 Zig/Go dependencies/CVE/security/build 类别、取消任务的持久化事件、可信关闭/复发、15.x 四能力生产资格不变，父任务 7.1/8.2–8.3/8.100–8.102/15.x 均不勾选。
