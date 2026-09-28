# 运行时流与证据日志局部验收（2026-09-24）

Rust runtime 以字面 argv 启动原生工具，并发读取 stdout/stderr，两条流共享同一字节预算。`process_contract` 覆盖持续洪流、双流同时洪流、短命令退出后读线程才发现输出超限、总 deadline 超时、阻塞 stdin 及部分输出保留。超限或清理失败均不得解释为质量通过；原生退出码仍作为独立原始事实保留。`ruff_contract` 中非 UTF-8 原生 JSON 报告为 incomplete，而不是空报告通过。

读流的排空截止时间与请求 deadline 取较早值；即使直接子进程先退出，排空与清理跨过 deadline 也归为超时。内部状态测试覆盖退出码 0 后跨期与晚发现超限的优先级。Maven/Ruff 原生探测共享单一绝对 deadline，并在证据、工具和输入身份复核后再次检查，不允许复核阶段越时后返回局部完成。

`run_process_recorded` 在日志落盘后再核对 deadline；原生退出 0 但记录完成时已超时会返回 `DeadlineExceeded`，适配器保留为 `request_deadline_exceeded`，与实际日志写入失败的 `evidence_write_failed` 区分。此为完成状态的准确归类，尚未给磁盘同步 I/O 建立可抢占的硬时间上限。

私有日志保留原始字节，写入 0700 私有目录中的 0600 临时文件，先同步再以同目录 hard link 原子发布且不覆盖已有目标。`private_log_contract` 覆盖目标 symlink、目录末级和中间层 symlink、公开权限目录、已有目标及原生退出 0 但日志写入失败。中间层 symlink 反例在修复前实际写到了外部目录；现在逐级 `openat + O_NOFOLLOW` 拒绝该路径，外部文件保持未创建。

这是 Unix 局部运行时验收。macOS 中短命令完成后的进程组清理可能返回 `EPERM`；2026-09-29 修正后，等待直接子进程回收并再次核实组已不存在，才保留已经观察到的输出超限；组仍存在或不可确认时继续报告 `CleanupFailure`。两种结果均不能解释为原生规则违规或检查完成。Windows 等价进程控制、完整 CLI 门禁链和持久化 I/O 的硬截止时间仍未验收，OpenSpec 3.2/3.3 保持未完成。

2026-09-29 定向回归：新增 17 MiB stderr 对 16 MiB 共同预算的 8 次连续进程测试，修复前实际返回 `CleanupFailure`，诊断为 `killpg` 的 `EPERM` 且输出预算已耗尽；修复后该目标连续运行 30 轮通过。原 Python CVE 部分报告用例修复前 12 轮内复现同一终止误分类，修复后连续 15 轮通过。`cargo test --workspace --all-features -- --test-threads=1 -q`、全目标 Clippy 及 OpenSpec 严格校验均退出 0；远端 CI 验收仍未完成。

2026-09-25 增补：`process_contract` 在当前 macOS 环境真实启动后台子进程，超时与运行中取消后等待其预定的延迟写入时间，确认请求返回后没有后续文件副作用。CLI 入口安装 Unix SIGINT 处理器，只设置原子取消标记；执行循环据此杀死进程组并报告取消。`lint_python_cli::sigint_during_native_probe_reaps_descendants_and_returns_incomplete` 先在无信号桥接时失败（主进程被信号直接终止），接线后通过：CLI 退出 3，报告含 `request_cancelled`，后台子进程没有延迟写入。此证据不能替代 Windows Job Object、排队任务取消或完整门禁验收。

2026-09-28 排空稳定性补充：并行全量回归曾出现已成功退出的 `/bin/echo` 被 `ReadFailure` 误分类。运行时原实现只给 stdout/stderr 读线程固定 150ms 排空，系统负载可延迟读线程调度；现从子进程退出起继续在请求的绝对期限内等待读流完成，超过期限仍为 `TimedOut`，输出预算和取消优先级不变。另一个 ESLint 取消/期限测试原来只给包括探测、身份复核在内的整轮 1 秒，却要求实际扫描已启动；将伪扫描保持长于截止时间，同时扩大测试准备余量，避免并行负载使前提失效。目标 `process_contract` 15 项、`eslint_probe_contract` 2 项通过、2 项条件忽略；完整并行离线工作区 161 组、961 项通过、0 失败、97 项条件忽略。该轮只能证明当前 macOS/本次负载，不能替代跨平台或硬实时 I/O 验收。
