# Rust WASM 私有工作进程候选验收（2026-09-29）

`codeguard-cli` 的可选 `wasm-precheck` 构建特性启用同一二进制的私有 `__syntax-worker` 入口。父进程以字面 argv、空白名单环境和独立进程组调用它，stdin 最多 1 MiB，stdout/stderr 合计最多 64 KiB，并沿用共同截止时间与取消状态；子进程只读取 stdin 和编译进二进制的固定 grammar 字节。协议版本为 `syntax_worker_candidate` 1.0.0。父进程重新核对固定 manifest、grammar ABI/SHA-256、源码 SHA-256、响应字段、恢复节点数量、UTF-8 字节边界及行/字节列，然后通过 core 状态聚合返回候选观察。候选 grammar 一律 `grammar_qualified=false`，无异常源码也保持 `incomplete`。

目标测试先因 `syntax_worker_runner` 不存在而编译失败。实现后，Java 缺失 token 的 `MISSING` 位置、TypeScript 无恢复节点但未验收、过大输入、取消前置、超时工作进程、伪造输出和随后正常进程的独立性均通过测试。工具故障是未完成，不转换为源码违规；私有 worker 不从项目路径加载 grammar，不调用 Node、CodeGraph 或 tree-sitter-cli。

本候选不是正式 WASM fallback：未设置经各平台实测的内存上限，尚无 worker 内的文件系统/网络沙箱证明，也未接入 `lint/check`、任务、对话反馈或发行包；Windows 进程组能力仍未验收。超时测试证明父进程对假 worker 的截止时间处理，尚不证明所有损坏 grammar 的隔离。OpenSpec 14.2/14.3/14.7 均保持未完成。

验证命令及结果在提交前记录：

```text
cargo test -p codeguard-cli --features wasm-precheck --test syntax_worker_candidate
cargo clippy -p codeguard-cli --features wasm-precheck --all-targets -- -D warnings
cargo test --workspace --features codeguard-cli/wasm-precheck --locked --quiet
openspec validate introduce-rust-codeguard-cli --strict
```

实际结果：私有 worker 4 项目标测试通过；CLI 启用特性的 Clippy `-D warnings` 通过；启用同一特性的完整 Rust workspace 回归退出码 0（外部原生工具条件用例按既有规则 ignored）；OpenSpec 严格校验和 `git diff --check` 通过。工作进程集成测试限定 Unix，其他平台不能借此认定通过。
