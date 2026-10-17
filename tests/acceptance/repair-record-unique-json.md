# 修复指引拒绝重复 JSON 字段

日期：2026-10-06。对应 `introduce-rust-codeguard-cli` 的 remediation-workflow、9.x/12.5/12.9；不替代完整命令矩阵或可信关闭验收。

问题：`next` 与指定任务查询曾直接用 `serde_json::from_slice<Value>` 读取本地事实。JSON 对象的重复字段会取最后一个值，导致包含互相冲突 `state` 的事实仍返回正常修复简报。新增集成反例先失败：`next` 返回0和完整简报，而预期为3且不提供简报。失败日志保存于本机 `cg-fact-duplicate-red.log`，不是可接受的运行证据。

修正：下一步视图的事实、验证事件、原生报告、消费和导入失败收据，以及白名单纠错引用统一复用现有有界 `parse_unique_json`；递归拒绝重复字段。原128KiB/4KiB等读取上限保持，报告解析上限仍为16MiB。既有合法报告、历史版本和错误原因保持；不修改门禁或关闭政策，不从任务Markdown读取指令。纠错引用的无效事件保持原有过滤行为，不作为可用决策引用。

实际回归覆盖：

- 顶层冲突state及嵌套重复instruction使next/status/task show退出3，不提供修复简报，不回显指令。
- 实际task verify产生的事件被加入冲突state_after后，next拒绝该观察；修复回原事件字节后恢复查询。
- 实际扫描的消费收据被加入冲突workspace_id后，next拒绝该收据，不用末值掩盖冲突；恢复后查询正常。
- 所有故障查询都保留原件，不擅自删除或修改历史；恢复查询仍为交付未评估。

这些是原始字节反例；JSON Schema验证已解析对象不能发现解析阶段丢失的重复字段，不能替代本项。此修正不证明本地记录具有可信来源、不存在并发变更、全宿主注入防护或完整工作台验收。测试最终结果与日志身份列入本change的verification。

最终定向回归：默认三目标24 passed / 0 failed / 10 ignored；WASM四目标42 passed / 0 failed / 10 ignored，两构建有重叠，不相加，忽略项未执行。WASM CLI全目标Clippy `-D warnings`通过；所改Rust文件定向格式、分层、OpenSpec strict与diff检查通过。[证据索引](evidence/repair-record-unique-json-2026-10-06.json)保存源码和日志身份。未执行完整工作区或完整原生/宿主矩阵。
