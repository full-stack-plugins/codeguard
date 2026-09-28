# Doctor 环境调查任务的原生复检

对应 introduce-rust-codeguard-cli/remediation-workflow、5.6/9.7/9.26 局部实施。

## 行为与边界

`codeguard task verify <doctor-task-id> . --ruff-tool ABS_PATH --format json` 复用 doctor 的固定 Ruff 0.16.8 版本诊断、工具前后摘要、私有运行目录及 task verify 剩余预算。不得改为源码 lint 或隐式选择 PATH 工具。

- 失败为 still_blocked，保留原生诊断原因；脚本入口启动前拒绝。
- 未选择工具为 incomplete，不制造恢复、缺失必需项或新任务。
- 真实原生版本恢复为 environment_restored_unverified_policy，不关闭任务、不返回 ready、质量通过或白名单批准。
- 报告仍是 doctor 0.2 queued 观察，进入同一同步器；复检事件绑定 run_id、报告 SHA-256、workspace_id 和现有修复尝试。沿用租约获取/释放和写入前尝试一致性校验。
- next 读取完整且摘要相符的收据，给出 verification_required 和“仅版本观察，核验批准前置与工具锁并执行原受阻质量检查”的下一步。
- next、同步和尝试账本统一识别 doctor 运行身份；保留严格报告、事件、工作区及摘要校验。

## 验收

两个目标用例最初因 checker 不支持失败；接线后查询暴露尝试账本不识别 doctor run_id，修正协议分支后通过，未放宽任意报告接收。

显式设置 CODEGUARD_TEST_RUFF=/opt/anaconda3/bin/ruff：doctor 工作流 12 项、doctor CLI 6 项、next 6 项、租约 16 项、task verify 10 项、work sync 15 项，共 65 项通过、10 项 ignored。新增四项分别覆盖诊断仍失败、真实恢复及 next 反馈、未选择、脚本拒绝无副作用。增加记录 reason=null 断言后定向复跑。

12 份实际复检反馈、原生报告、落盘报告和事件通过公开 JSON Schema；交付 allow 篡改反例拒绝。all-target Clippy（-D warnings）、格式、OpenSpec strict/diff 检查通过。

可信义务、完整工具兼容诊断、当前证据时效和正式关闭/重开仍未接通。未运行全工作区测试、其它原生 ignored 用例或宿主端到端；完整任务不勾选，目标继续。
