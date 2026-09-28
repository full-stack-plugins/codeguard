# npm audit 未完成执行后的局部证据

对应 OpenSpec `introduce-rust-codeguard-cli` 的 F05、2.3、5.5 和 7.3。受控 Node 命令先返回预期 npm 11.16.0 版本，再输出结构完整、与本轮 `package-lock.json` 节点绑定的一条 high advisory，最后以异常码 2 退出。修正前公开 `cve typescript` 反馈的 `findings` 为空；目标测试先失败。

现在仅当完整 JSON 可由严格 npm 11 解析器接受、包含非空漏洞、锁节点绑定成功且执行前后输入摘要不变时，保留一条脱敏局部发现。反馈原因仍为 `npm_audit_execution_incomplete`，退出 3、`delivery_decision=not_evaluated`、`advisory_coverage=not_evaluated`；`local_coherent=true` 仅表示这份局部报告的结构与输入可核对，不能表示原生命令完成。`check all` 只在正常本地观察原因下把 npm 执行节点记为成功，因此该场景的执行节点保持 `native_incomplete`。已初始化工作区同步同一张稳定的 npm CVE 完整性任务，不批准漏洞身份、数据库覆盖或门禁通过。

同一测试再模拟两种故障：完整报告输出后进程继续运行至 2s 超时，以及完整报告输出后 stderr 超过共享 8 MiB 上限。两者均保留一条局部 finding，并分别标记 `deadline`、`npm_audit_output_limit`；超时后工作台反馈 `interrupted_not_persisted`，不越过截止时间写任务；输出超限时工作台仍可同步原有完整性任务，`check all` 执行节点保持 `native_incomplete`。运行中的工具文件变化使结果失效，截断 JSON 后异常退出或输出超限均为零 finding。取消和真实 npm 崩溃后的报告保留、全部原生故障优先级及完整 F05 矩阵仍未验收。受控命令是测试替身，不能代替真实 npm 的数据库及时效验收。

验证：`cargo test -p codeguard-cli --test npm_audit_cli --test check_all_npm --offline -q`、`cargo test --workspace --offline -q`、`cargo clippy --workspace --all-targets --offline -- -D warnings`、`cargo fmt --all --check` 均退出 0。全工作区 163 组、989 通过、0 失败、101 条件忽略；日志 `/tmp/codeguard-npm-f05-full-final-20260928.log`，SHA-256 `a008b5049c919c7cc9fa43d968e0be936e43d5631d9c6f5acd2c35c080eaa5f5`。独立 Draft 2020-12 对实际公开反馈、保存的工作台观察及 `check all` 报告均无 schema 错误。第一次全量失败于新增输出超限替身在并行负载中过早退出；第二次失败于既有 Rust CVE 替身的相同问题。两处替身在超限写出后保持运行、各自目标测试和最终全量通过，未改变 Rust CVE 产品逻辑。当前局部协议仍沿用 0.3 反馈与 0.1 工作台格式，后续需把执行完成与报告有效性拆为独立字段以减少调用方误读。
