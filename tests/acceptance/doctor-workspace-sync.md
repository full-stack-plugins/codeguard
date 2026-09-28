# Doctor 工作区报告与环境调查任务

范围：OpenSpec introduce-rust-codeguard-cli 的 5.6、9.4、9.7、9.26 局部实施。不是完整 PrerequisiteReport、可信准备判定或交付门禁验收。

## 可观察行为

- doctor 0.2 默认只发现静态配置；显式 `--ruff-tool ABS_PATH` 才诊断固定 Ruff 版本。
- 已初始化且绑定 workspace_id 的工作区，将不可变报告保存到 `codeguard/reports/<run_id>.json`，随后自动 work sync。落盘报告状态为 queued；对话输出另显示 synced_partial 或 backlog_update_failed，不覆盖已消费的报告字节。
- 同一工作区 Ruff 前置诊断失败形成一个 `python.ruff.doctor` 环境调查任务。失败原因变化仍归并到稳定 ID，每轮实际原因存入私有 observations；任务首次原因是历史证据，不声称当前原因不变。
- task/next 给出证据、依据、允许范围、恢复步骤、doctor 复检命令、历史观察位置及关闭条件，不要求修改无关源码。
- 未选择工具不等于必需工具缺失，不制造任务。未初始化、旧画像无绑定或无效画像不自动初始化、写入报告。
- 诊断恢复只更新报告，不自动关闭既有任务或产生 ready、质量通过、白名单批准。当前 task verify 尚无 doctor 专用关闭路径；可信必需前置映射及验证接线仍待实现。
- 导入拒绝错工作区、被改消费报告、自写批准、未知顶层/诊断字段、非法预算或质量/准备升级。报告目录链接不会被跟随；保存失败仍向对话报告 backlog_update_failed 并保留原生观察。
- 原检查器记录不新增空诊断字段；doctor 记录的扩展字段在公开 schema 中明确声明。0.1 doctor schema 独立保留，不能作为可导入的 0.2 报告。

## 本轮验证

先前入口缺失的四项同步行为测试已失败，补齐导入/保存/自动同步后通过。新增四项覆盖原因变化、未消费伪造报告、目录链接和显式真实 Ruff 0.16.8 恢复。

`CODEGUARD_TEST_RUFF=/opt/anaconda3/bin/ruff cargo test -p codeguard-cli --test doctor_work_sync --test doctor_cli --test work_sync_contract --test work_sync_cross_category --test next_command_contract --test tools_verify_cli --test task_verify_contract --offline -q`：54 项通过、10 项忽略；真实 doctor 版本用例显式运行，忽略的其它原生验收没有计为通过。

10 份实际对话/落盘报告和 finding/event/observation 通过 JSON Schema；6 项伪造批准/就绪/交付/质量/门禁反例被拒绝。schema 校验脚本仅用于验收，不是产品运行依赖。

最终 all-target Clippy（-D warnings）、格式、OpenSpec strict 与 diff 检查通过。

未运行全 workspace 测试、全语言原生工具或宿主端到端；完整 OpenSpec 任务保持未完成。
