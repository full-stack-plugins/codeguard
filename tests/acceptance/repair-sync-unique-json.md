# 修复工作台写入拒绝歧义事实

日期：2026-10-06。延续 `introduce-rust-codeguard-cli` remediation-workflow 的稳定身份、历史完整性及失败反馈要求，9.3/9.7/12.5父任务保持开放。

前一修正使next/status/task show拒绝重复JSON字段，写入链路仍存在缺口：已有finding含冲突state时，work sync按最后一个值继续导入新报告并追加接受观察。新增反例实际先失败：failed_reports=0、imported_reports=1，而预期拒绝歧义事实。

现在work sync读取待导入报告、最新报告索引、既有finding/blocker事实，以及修复尝试读取验证事件/原生报告，复用现有有界递归唯一字段解析器。错误原因保持各入口原有finding_corrupt/blocker_fact_corrupt/report_invalid_json等语义；不修改租约或关闭政策。类型化AttemptStart/AttemptFinish和Lease仍由serde检查具名重复字段，没有把它们改成宽松Value。

实际回归覆盖两类任务：

1. 已有源码finding被加入冲突state后，新报告导入失败，不写该finding的新接受观察和成功消费收据，原始事实字节不变。
2. 已有缺Ruff工具的环境blocker出现同样冲突，新扫描仍保留检查未完成，但不追加接受观察或成功消费收据，不制造源码违规。
3. 恢复合法事实后，新报告复用原任务；同一源码finding或同一Ruff工具缺失任务仍只有1份，不通过修改旧失败收据假装旧报告已接受。失败报告仍是需要调查的历史事件。

这是局部记录完整性修复，不证明记录来源可信、整个导入批次原子性、并发/崩溃全部边界、宿主注入防护或完整质量门禁。完整目标不因这两个反例通过而勾选。

WASM首次回归的新blocker测试错误地要求全工作区仅一任务，实际还保留独立语法确认任务，因此20 passed / 1 failed / 2 ignored。仅修正测试为按checker/reason核对同一原生问题的任务唯一性，生产解析行为未调整；复跑work sync两目标22 passed / 0 failed / 2 ignored。默认work sync复跑21 passed / 0 failed / 2 ignored。该失败不是grammar误报，也不删除合法的语法确认任务。

受影响默认五目标最后有效结果合计60 passed / 0 failed / 12 ignored；WASM六目标最后有效结果合计78 passed / 0 failed / 12 ignored。结果由最初成功目标和修正后的work sync目标合并，各目标只计一次，两构建重叠不累加，条件忽略未执行。最终WASM CLI全目标严格Clippy、定向格式、分层、OpenSpec strict与diff检查通过。未运行完整工作区或真实宿主验收。

[证据索引](evidence/repair-sync-unique-json-2026-10-06.json)记录源码摘要、首次失败与最终日志身份，不将条件忽略或旧失败转换为通过。
