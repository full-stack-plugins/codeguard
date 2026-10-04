# 缺失任务投影恢复验收

对应 OpenSpec：introduce-rust-codeguard-cli / remediation-workflow 的 Missing task projections SHALL be recovered without changing facts，任务9.5。完整事件父关系、状态机和入库安全仍未完成，不勾选父任务。

```mermaid
flowchart LR
 A[持有 work sync 锁] --> B[核验已消费事实和来源]
 B --> C[恢复缺失 Markdown]
 C --> D[导入新报告]
 D --> E[复核剩余缺失投影]
 E --> F[当前 RepairBrief 及原工具复检]
 F --> G[保持开放 等待完整关闭条件]
```

实现共用 next 的当前事实/历史校验，另核对唯一字段JSON、原报告摘要、工作区/run身份及消费标记。全部待恢复项先验证再创建文件，原子不可覆盖写入保留编辑器竞争下的现有内容；最多1000条事实及每份128 KiB任务投影。已有普通Markdown含备注与勾选原字节保留，链接/目录/坏事实/来源变化明确失败。恢复不运行工具，不修改原事实、事件、消费标记、租约或尝试预算。

恢复报告0.3要求正整数restored_task_projections和not_evaluated；无恢复报告保持0.2，旧schema原件不改。只有已消费来源先恢复，避免同一轮新扫描因为投影缺失而被误判导入失败；未提交中间态仍需原导入事务有效重放，不能凭半份记录生成修复授权。

实际 Apple Swift 6.4 检查创建任务后，删除Markdown、work sync恢复、task show、手工备注/勾选、再次sync及next均执行。同一任务仍open，原事实/事件/消费标记SHA一致，原有备注不覆盖。证据：[task-projection-recovery-2026-10-05.json](evidence/task-projection-recovery-2026-10-05.json)，SHA256 `24fef33be4d0e1900d23b2e13005c3786d3c547da94df1f53af1a1f0cbc839ef`。该证据基线0a3e6a3，投影代码为基线上的未提交增量，记录实际二进制SHA；不是实际安装宿主或可信关闭验收。

当前相关 WASM 五目标（lint_python_cli、work_sync_contract、task_lease_contract、next_command_contract、swift_native_workbench）共63 passed、0 failed、7 ignored。活跃租约、open attempt、budget history和state/findings JSON逐字节保留，next仍waiting，普通release仍拒绝未结束attempt。Ruff工具阻塞恢复仍是blocker，不建议移除源码导入。四项开发协议验收覆盖实际报告、旧消费者拒绝、不可变记录、假通过/零恢复计数拒绝和七项可读内容。

237 schema元定义有效，236历史schema字节保留；OpenSpec strict、分层和diff检查通过。当前 `cargo test --offline --workspace` 已退出0：229组、1283 passed、0 failed、113 ignored；示例目标另以 `cargo test --offline --workspace --examples` 验证，1 passed、0 failed。示例与此前all-targets范围单独对齐，不将新增doc-test空组计算为额外功能。此前0a3e6a3完整默认230组1278 passed/0 failed/113 ignored及对应Linux CI成功均属于本次改动之前，不替代当前源码验收。没有重跑全部32语料或更改grammar资格、公共包、插件锁；完整跨平台故障矩阵和可信策略仍缺。

已修复的回归：投影恢复曾把 `invalid-record` 非任务目录当作有效任务事实，导致独立原生报告导入受阻。恢复现仅处理合法CG-/CG-B-稳定ID；异常目录继续由next/status诊断。修正后默认 lint_python_cli/next/task_lease/work_sync 共60 passed、0 failed、7 ignored。历史WASM十一目标95通过及后补租约17通过与最新测试重叠，不累加成更大的覆盖数。

## 重跑

```bash
cargo test -p codeguard-cli --features wasm-precheck --test work_sync_contract --test next_command_contract --test task_lease_contract
cargo test --offline --workspace
python3 tests/task_projection_feedback_schema.py
```

Python仅用于开发协议验收，需jsonschema；产品路径为Rust。

## 日志摘要

- `/tmp/codeguard-projection-recovery-red.log`：`f010db8a26f9412b04d0f67f982ec4484ce26709fd09e7a6e0ceae6d34923781`。
- `/tmp/codeguard-projection-new-scan-red.log`：`7d5b53667671ee3d17868adfe3d2c0a0297606bf49634d9edd345c7a870449ac`。
- `/tmp/codeguard-projection-wasm-regression.log`：`19de571faf7d5fdf0bd390d6b6e4aac74ac4b4beb2dc937d9c0e01f27e5088b6`。
- `/tmp/codeguard-projection-lease-wasm.log`：`3ffbd02ef4873795437f8a3e87411cee702eb905014f5e7dd669592f6e6d19df`。
- `/tmp/codeguard-projection-lease-default.log`：`6f8b80ccd4167bb8f37a77a6094c995f400691f99106e670ddeac5a33d52814c`。
- `/tmp/codeguard-projection-recovery-actual-final.log`：`f2243249b03c9fe6a74fbb1d69339fb150429d17a616b7574d666fffd6ee28a7`。

## 取消测试诊断与夹具修正

原WASM取消测试经Cargo启动失败，父测试在2秒内看不到就绪标记，CLI最终返回3及ruff_version_unavailable。同一测试二进制直接运行曾通过。脚本trace与失败后文件检查证明，原生脚本实际执行过，ready_after_exit及late_after_exit均为true；探针Exited(0)、耗时1.175秒。原来的“原生进程未开始”判断不成立：父测试等待包含CLI启动，而原生预算只覆盖执行，夹具还另启/usr/bin/touch发布信号，造成同步窗口错位。

同一夹具现在通过shell内建重定向发布启动信号，后台shell仍在外部sleep后迟写。新增清空环境、无取消的正对照，确认同一脚本真实产生迟写。取消测试仍保持2秒预算、退出130、request_cancelled与650毫秒后无迟写断言；失败时回收CLI并输出实际报告。没有预热工具、跳过测试或放宽预算。临时产品诊断、环境修改及文件同步试验均已删除。WASM lint_python_cli整目标18 passed、0 failed、5 ignored，包含上述正负对照；这些计入前述63通过，不重复相加。

这项夹具修正不能证明整体CLI性能达标；冷启动、端到端deadline、多平台进程清理和正式发布仍按原OpenSpec任务验收。

- `/tmp/codeguard-probe-original-marker-after-exit.log` SHA256：`308e7fa8f33eacf367a236c8c69d880f43c5268c38e778f76b8d4d620c633c22`。

- `/tmp/codeguard-cancellation-fixture-green-wasm.log` SHA256：`e46b273c6776cf3a0b665b08333195de0c78f24912074d3c60d8691d36282b8f`。

- `/tmp/codeguard-projection-cancellation-final-wasm.log` SHA256：`d6cc78b068d3e0ceb9efc137232975f53ebae296f5873f5e7bd833e78f56cf11`。

完整默认日志SHA256：`3613238534ae14fe107c1e86eddd64265db1ebd97a2ea021a59cfe0cb7b34f32`；示例日志SHA256：`bde7d359dc6d3d314e894af60ec514bdcf48fca474ef6ab4d5ad5e8dbec94278`。

提交前最终检查：默认及WASM工作区all-targets Clippy `-D warnings` 均退出0；fmt、diff、分层、OpenSpec strict与781条本地文档链接检查通过。236份历史schema保持原字节，新增0.3报告的4项协议/状态不变量开发验收通过。
