# Rust WASM 私有工作进程候选验收（2026-09-29）

`codeguard-cli` 的可选 `wasm-precheck` 构建特性启用同一二进制的私有 `__syntax-worker` 入口。父进程以字面 argv、空白名单环境和独立进程组调用它，stdin 最多 1 MiB，stdout/stderr 合计最多 64 KiB，并沿用共同截止时间与取消状态；子进程只读取 stdin 和编译进二进制的固定 grammar 字节。协议版本为 `syntax_worker_candidate` 1.0.0。父进程重新核对固定 manifest、grammar ABI/SHA-256、源码 SHA-256、响应字段、恢复节点数量、UTF-8 字节边界及行/字节列，然后通过 core 状态聚合返回候选观察。候选 grammar 一律 `grammar_qualified=false`，无异常源码也保持 `incomplete`。

目标测试先因 `syntax_worker_runner` 不存在而编译失败。实现后，Java 缺失 token 的 `MISSING` 位置、TypeScript 无恢复节点但未验收、过大输入、取消前置、超时工作进程、伪造输出和随后正常进程的独立性均通过测试。工具故障是未完成，不转换为源码违规；私有 worker 不从项目路径加载 grammar，不调用 Node、CodeGraph 或 tree-sitter-cli。

本候选不是完整 WASM fallback：TypeScript 显式单文件、缺原生上下文的 `lint` 已接入疑似观察与对话反馈，`check all`、任务、正式 grammar 验收及发行包尚未接入；尚无经各平台实测的内存上限或 worker 内文件系统/网络沙箱证明。Windows 进程组能力仍未验收。超时测试证明父进程对假 worker 的截止时间处理，尚不证明所有损坏 grammar 的隔离。OpenSpec 14.2/14.3/14.7 均保持未完成。

2026-09-29 增量：Linux 私有 worker 在 `exec` 前设置 `RLIMIT_AS=2 GiB`，设置失败即不能启动；Wasmtime 将默认 4 GiB 虚拟内存预留调为 64 MiB，并把 guard 调为 16 MiB。Linux 专用反例测试要求 3 GiB 的实际 `Vec::try_reserve_exact` 在子进程中失败，CI 新增启用 `wasm-precheck` 特性的 Linux 测试。当前本机为 macOS arm64，实测 `setrlimit(RLIMIT_AS)`、`RLIMIT_DATA`、`RLIMIT_RSS` 均返回 `EINVAL`；受限进程入口在 macOS 返回 `UnsupportedPlatform`，现有候选初检仍保持未限定内存的 `incomplete` 状态，不能据此声明内存隔离。Linux CI、真实 Linux grammar 运行、其它目标平台、损坏 grammar 与并发边界仍待验收，14.3 不勾选。

本机增量验证：`process_contract` 16 项、`syntax_worker_candidate` 4 项、`typescript_syntax_fallback_candidate` 5 项通过；启用 WASM 特性的全工作区离线回归 175 组、1048 通过、101 条条件忽略、零失败。CLI 特性 Clippy 与全工作区 Clippy `-D warnings`、受改文件 `rustfmt --check`、OpenSpec strict、分层检查及 `git diff --check` 均退出 0。Linux 专用分配负例被平台条件编译排除，需查看推送后 CI 的真实运行结果。

Linux 实机证据：[GitHub Actions run 36475894172](https://github.com/full-stack-plugins/codeguard/actions/runs/36475894172) 在提交 `dbac346ac1b2b2e02ef85d7ec2ee9a1b29b65163` 上通过。专项目标步骤真实运行 `linux_address_space_limit_rejects_a_larger_allocation`，runtime 18 项、Java/通用 worker 4 项、TypeScript fallback 5 项均通过；随后普通工作区构建、全量测试及分层检查也通过。首次 run `36475507492` 在专项步骤之前遇到未改动的 `native_version_observation` 30 ms 超时用例返回 `SpawnFailure`，第二轮全量通过；该偶发故障根因未独立定位，不纳入 Linux 内存证明。以上只证明当前 Ubuntu runner 上的进程地址空间限制及其测试路径，不证明 macOS/Windows 内存边界、内核级隔离或全部语法能力。

工作进程故障隔离增量：候选入口补充两个实际子进程反例。第一例让 worker 退出 9、留下拟在 1 秒后写文件的后台后代；父进程保留异常退出并清理进程组，标记文件没有出现，随后真实 Java WASM 观察仍成功。第二例让 worker 持续输出超过 64 KiB，再对另一个忙循环 worker 执行运行中取消；两个结果分别保留 `OutputLimit` 与 `Cancelled`，下一次真实观察仍成功。本机 6 项候选测试通过；这些证据仍不覆盖损坏 grammar 的真实加载、所有子进程跨平台行为或内存上限。

验证命令及结果在提交前记录：

```text
cargo test -p codeguard-cli --features wasm-precheck --test syntax_worker_candidate
cargo clippy -p codeguard-cli --features wasm-precheck --all-targets -- -D warnings
cargo test --workspace --features codeguard-cli/wasm-precheck --locked --quiet
openspec validate introduce-rust-codeguard-cli --strict
```

实际结果：私有 worker 4 项目标测试通过；CLI 启用特性的 Clippy `-D warnings` 通过；启用同一特性的完整 Rust workspace 回归退出码 0（外部原生工具条件用例按既有规则 ignored）；OpenSpec 严格校验和 `git diff --check` 通过。工作进程集成测试限定 Unix，其他平台不能借此认定通过。
