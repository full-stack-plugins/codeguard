# 宿主事件档位路由候选验收（2026-09-29）

本记录只证明 Rust core 的纯事件决策，不能证明插件已调用二进制、检查器已运行、Git 内容面准确或任一宿主已具备严格阻断。

TDD 首先因 `HookEvent`、`HookTriggerAction`、`HookTriggerInput` 和 `plan_hook_trigger` 不存在而编译失败；区分确认失败与未知写入的第二个目标测试也先因新契约缺失而失败。实现后，启动只请求发现，停止只请求摘要；确认写入且有有效相对路径才请求局部快检，重复路径归并。确认写入失败不启动源码检查；结果未知或缺目标须重新确定范围。修复事件需要合法稳定任务 ID 并请求复检。提交、推送及 CI 事件不接受宿主给出的变更路径作为交付范围，前两者要求重新获取 Git 快照；无可靠阻断能力时不声明硬门禁。所有路由计划均固定 `may_claim_delivery=false`，快检的 `soft_result_reuse_candidate` 只表示可进一步核验的候选，不是缓存命中或检查已完成。

补充反例：用户提示即使提到提交也只请求非阻断性意图建议，不能代替真实 Git 操作前的严格门禁；该测试先因事件/动作变体缺失编译失败，再在路由实现后通过。

验证命令：

```text
CARGO_NET_OFFLINE=true cargo test --locked -p codeguard-core --test hook_trigger_plan
CARGO_NET_OFFLINE=true cargo clippy --locked -p codeguard-core --all-targets -- -D warnings
openspec validate introduce-rust-codeguard-cli --strict
```

`host_blocking_claimed` 仅回显宿主声明，不证明宿主真的能阻断；即使声明为真，计划也不能签发交付通过。后续须在 CLI 把档位绑定到真实 `detect`、`lint/check`、`task verify`、Git/CI 入口，核对工作区、源码和工具身份，并建立快反馈节流、跨进程租约和资源预算。三宿主应分别验证实际事件、对话反馈、重复事件及无法阻断时的说明。仅靠本纯函数不能勾选 OpenSpec 11.4/11.5/11.17。
