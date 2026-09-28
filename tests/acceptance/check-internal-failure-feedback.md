# 检查任务内部故障的局部反馈

当 `check all` 的任务图已完成一个原生检查并获得 finding，另一任务发生内部故障时，CLI 必须退出 4，并在结构化反馈中保留已取得的原生结果。故障不应吞掉 finding，也不能转成白名单放行或普通通过。

定向单元契约 `check_command::tests::internal_task_failure_keeps_sibling_native_findings_visible` 构造 Python F401 与 Rust 内部故障并核对局部中止报告：退出码 4、交付 incomplete、失败任务 ID、两项任务状态及原始 F401 均保留。该测试在反馈构造函数缺失时先编译失败，接线后通过。后续 `check_aborted` 0.2 把同轮取消优先于内部故障：取消与内部故障同时存在时，退出 130，原 F401 和故障任务 ID 都保留。`schemas/check-aborted.schema.json` 固定这两种中止状态的关系。

范围限制：两项单元测试验证报告构造与 CLI 早退接线的代码路径，不是对真实 Ruff/Clippy 同轮内部异常注入的端到端验收。真实 SIGINT 与并行原生 finding 的 CLI 用例见 `check-all-partial-native.md`。完整义务账本、调度器自身失败及可信白名单门禁仍未验收；OpenSpec 2.3 继续未完成。
