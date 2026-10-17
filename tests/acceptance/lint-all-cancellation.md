# lint all 真实取消与兄弟结果保留

对应现有2.3/2.8，不创建新变更。复用同一受控Ruff/Cargo进程夹具，分别公开调用check all与lint all，等待两个原生节点实际启动、Clippy输出准备完成后发送真实SIGINT。断言退出130/command_status=cancelled，保留Clippy needless_return诊断，取消Python节点可见、发现结果保留，并等待后核对原生子孙没有晚写入。lint入口另核对0.59协议、requested_categories只含lint、任务ID仅python.lint/rust.lint、候选类别只含lint。

默认两个相关目标23通过、0失败、3条件忽略；WASM两项取消目标通过，不与默认重复累计。默认与WASM各一份实际取消报告通过0.59 schema；每份报告伪造退出3或incomplete状态被拒绝。CLI全目标严格Clippy、分层、OpenSpec strict与diff通过。测试使用受控原生命令，不表示实际Ruff/Clippy准确率或安装宿主验收。没有运行用户Erlang草稿，摘要保持不变。

此批补齐lint模式公开取消证据；内部异常、完整义务聚合、全部命令取消/平台/真实宿主仍缺，2.3/2.8保持未完成。中英文文档修正此前取消未验收的说明，上一批验收记录保留当时历史状态。日志及实际报告摘要见evidence/lint-all-cancellation-2026-10-06.json。
