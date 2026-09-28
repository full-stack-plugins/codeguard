# 原生进程执行层阶段验收（2026-09-24）

本记录仅覆盖 Rust `codeguard-runtime` 中的 Unix 进程调用原型；它尚未接入 `codeguard check`、任何语言 adapter、私有证据日志或交付门禁。

- `ProcessSpec` 明确指定绝对可执行路径、字面 argv、绝对 cwd、完整白名单 env、关闭或显式写入 stdin、共享 `Instant` 截止时间和 stdout/stderr 合计保留预算。执行层不解释 P3C、Maven、Ruff 等原生工具的规则语义。
- macOS 上的实现通过 `std::process::Command` 启动工具，以独立 Unix 进程组回收同组子进程；stdout/stderr 和 stdin 管道并发、非阻塞处理。取消、超时、输出超限与原生退出码分开返回；超时保留已读到的部分字节。
- `cargo test --offline -p codeguard-runtime --test process_contract`：11/11 通过，覆盖 shell 元字符仍为字面参数、cwd/env/stdin、输出洪流预算、截止时间、启动前及运行中取消、同组子进程、部分输出和不读 stdin 的阻塞写入。
- `write_private_log` 在已存在的私有目录中使用 `openat(O_EXCL|O_NOFOLLOW)`、同目录 hard link 原子发布、文件与目录同步；不会覆盖已有目标。记录采用版本化二进制前缀，保留非 UTF-8 原始字节。`run_process_recorded` 将写入失败作为独立错误返回，携带真实原生进程结果，不把退出 0 变为检查通过。
- `cargo test --offline -p codeguard-runtime --test private_log_contract`：5/5 通过，覆盖私有权限、非 UTF-8、目标和目录 symlink、外部文件保持不变、非法名称、已存在目标，以及工具退出 0 但证据失败的边界。
- 本层完成时，`cargo fmt --all -- --check`、`cargo test --workspace --all-targets --offline`（当时 64 项默认测试通过、4 项真实 Ruff 测试按设计忽略）、`cargo clippy --workspace --all-targets --offline -- -D warnings` 通过；OpenSpec strict validate 通过。后续试运行测试见 `native-ruff-baseline.md`。

仍未完成：证据索引/保留策略、磁盘故障注入验收、日志预检查与命名并发压力测试、脱离进程组的子孙进程处理、Windows Job Object、Ctrl-C/排队取消真实平台测试、总请求中排队/探测/解析预算、资源锁和 adapter 接入。因此 OpenSpec 3.1–3.3 保持未勾选，不能用本原型声称生产级进程树回收或完整质量扫描。
