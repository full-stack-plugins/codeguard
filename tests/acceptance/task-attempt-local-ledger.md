# 本地修复尝试记录验收边界

状态：Unix 局部实现；不代表 OpenSpec 9.8/9.9/9.27 全项完成。

`cargo test -p codeguard-cli --test task_lease_contract --offline` 覆盖：错误 token 不能开始；同一任务只接受受控动作 ID；未结束 attempt 阻止 release；重复 finish 同结果幂等、冲突结果拒绝；`no-change` 与实际源码变化矛盾时拒绝；两次同输入无进展后 `next` 要求决策并拒绝第三次 start；过期接管先写 `abandoned` 再换 generation，旧 token 不能改写；环境 blocker 即使不改项目文件也能 `ready-to-verify`，不会被误计为 no-change。

后续增加与原工具复检绑定的反例：旧复检不能解除新 attempt 的等待；同一 `ready-to-verify` 未复检前再次 start 返回 `verification_required_before_retry`；复检结果仍为 `incomplete` 时累计无进展并在两次后拒绝重试。验证事件 0.3 记录对应 attempt-id；历史私有报告丢失时旧事件不充当有效复检，重新运行原工具后才可继续。真实 Ruff 0.16.8 的修复样本证实：F401 修复后的本轮复检清除待验证状态，但 finding 仍保持 open、门禁仍未评估。

开始与结束事件保存在 `codeguard/findings/<task-id>/events/attempt-<attempt-id>-{start,finish}.json`。事件只含受控代码、摘要、owner 与租约 generation/token 摘要，不含原始源码或自由诊断文字。报告与 `next` 始终声明 `local_unverified` 和 `not_evaluated`，`ready-to-verify` 不关闭任务。

剩余：任务输入摘要尚不等于完整 patch 或环境身份；verify/fix 未携租约；Windows 任务锁与同等语义未实现；可信策略、完整原生复检、门禁及任务关闭仍需独立验收。
