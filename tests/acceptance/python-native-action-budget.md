# 原生语法修复动作与跨动作预算验收

对应 OpenSpec introduce-rust-codeguard-cli 的 9.9、9.10 和 14.10。父任务保持开放。

## 缺陷与修正

真实 Ruff 0.16.8 对原生首次及 WASM 首次 Python 确认任务报告 still_present，简报步骤要求修复源码，但受控 action_id 错误为 restore-checker-environment。定向 SDK 链路先 RED：目标任务简报的动作断言失败（29.00 秒）；修正后同一测试通过（160.44 秒），覆盖两类首次来源、可信关闭、幂等和 SDK/普通 CLI 复发。全局 next 可能优先选择独立环境任务，因此断言使用目标任务 read_task_brief，与 attempt start 消费相同入口，不跳过前置阻塞。

新增单位反例验证：源码或配置失效、未完成、无诊断和其他环境原因不被旧 Python 语法诊断授权；预算耗尽不改变动作身份或重新授予执行权。

纠正动作可能把旧失败历史从环境动作切换至源码动作，不能重置预算。纯历史聚合测试先 RED（旧逻辑仅计一次，期望三次）；修正后同一语法确认任务、同一输入的两种动作共享预算。原始追加事件不改；非确认任务不扩大关联，新输入和已核验进展保留既有规则。

## 执行证据

- 默认 CLI lib：54 passed / 0 failed / 3 ignored。
- WASM lib、next_command_contract、python_task_resolution_service、syntax_task_verify、task_lease_contract：96 passed / 0 failed / 6 ignored。原生条件测试不计入这里的通过数。
- 实际 Ruff SDK 两类首次来源链路：1 passed / 0 failed，160.44 秒。它单独显式执行，与普通回归的 ignored 分开。
- 增强既有持久化租约集成测试：两次源码修复 no-change 后原工具环境失败，动作改为 restore-checker-environment，但历史仍为二，第三次尝试被 no_progress_budget_exhausted 拒绝；1 passed / 0 failed，46.05 秒。这是受控工具协议夹具，不是 Zig 原生准确率验收。

默认与 WASM 两种构建的 CLI all-targets 严格 Clippy 均通过；OpenSpec strict、定向 rustfmt 与 git diff --check 通过。此批不修改旧 schema、WASM 资产、发行资格、npm 公开包或插件锁；不完成生产宿主信任根、全部语言修复闭环或交付门禁。Erlang/VB.NET grammar 重建仍待获准生成器。
