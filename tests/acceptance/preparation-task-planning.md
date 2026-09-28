# 准备任务领域规划验收

范围：OpenSpec introduce-rust-codeguard-cli 9.25 的任务规划基础，不代表 CLI/宿主或完整修复工作台已经接通。

plan_preparation 使用同一 ReadinessInput 及 evaluate_readiness 的当前结论。有效必需 Missing/Incompatible/Conflict 分别生成恢复前置、复核兼容、冲突决策动作；unknown 只生成重新核验动作，不保留过期或错绑定的缺失诊断。可选、不适用、已满足条件不生成修复任务。要求/观察来源未核验或要求 ID 重复、非法、摘要无效时不生成可行动任务，原汇总诊断仍保留。

任务键 preparation:<prerequisite_id> 是工作区内逻辑键，不能直接用作路径；绑定/诊断变化更新任务而不改变键。顺序稳定、计划幂等。固定中文模板提供步骤和关闭条件，不拼接项目命令、原生消息或源码。automatic_execution_authorized 始终 false；规划不执行工具、安装依赖、覆盖配置、关闭 finding 或生成交付 allow。关闭必须重新取得本次绑定下的有效满足证据。

```bash
cargo test -p codeguard-cli --test preparation_plan_contract --test readiness_contract --test delivery_gate_contract --test check_session_contract --test init_command_contract --test crate_boundaries --offline
```

目标行为缺失时用例因类型/函数不存在失败。7 项规划用例覆盖具体动作、过期/错绑定/缺或重复观察、未解析适用性、可选与满足条件、未核验来源、非法/重复标识、零摘要、稳定键、顺序、部分清单、阻塞与未知并存，以及拒绝向计划注入 delivery_decision。

当前仅合成可信输入契约，不证明真实工具探测或批准来源。CLI init 仍 readiness=unknown；完整要求生成、证据关联、工作区任务持久化/尝试历史及 doctor/check/sync/next 接线仍缺，9.25 不勾选。完整计划保持进行中。
