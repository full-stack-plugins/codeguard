# 宿主事件局部执行验收（2026-09-29）

`codeguard hook execute PATH --timeout DURATION --format=json [--ruff-tool ABS_PATH]` 从 stdin 读取有界 `hook_trigger_request` 1.0.0，复用 `hook plan` 的严格解析与 core 路由。超时必须显式给出，至多 120s。返回 `hook_execution_feedback` 0.1，固定 `delivery_decision=not_evaluated`、`host_blocking_verified=false`、`soft_result_reused=false`；普通候选观察退出 3，取消 130，内部故障 4，非法输入 2。插件尚未自动调用它，这些 CLI 退出码不能直接照搬为宿主动作。

现有两个真实动作：`session_start` 仅静态发现语言和检查器配置，最多展示 32 条配置，不执行源码检查；确认成功且 core 规划的目标全是 `.py` 的编辑调用既有有界 Ruff 文件快检，内嵌 `python_lint_feedback` 0.13。此局部报告不导入完整工作台。失败写入不检查；未知/缺范围要求重新确定；超过逐文件预算不扫描子集；混合语言不偷偷只扫 Python；修复、提交、推送、CI 等未接线动作明确 `not_run`。实际 Git 范围和宿主阻断能力没有被验证。

目标测试先因 `hook execute` 命令不存在而失败。实现后常规事件目标测试 7 项通过，原生 Ruff 0.16.8 单独实跑 1 项通过，确认只返回目标文件 F401。独立 Draft 2020-12 校验器用实际 CLI 输出覆盖启动、成功编辑、失败编辑和提交事件；四类输出通过新增封闭 schema，内嵌 Python 报告通过 0.13 schema，伪造 `delivery_decision=allow` 被拒。无效路径、超 120s 预算在执行前拒绝；Clippy `-D warnings` 和 OpenSpec 严格校验通过。

后续必须接入插件的真实宿主事件与受信二进制、逐语言/批量计划、软反馈等价身份缓存、任务原工具复检、真实 Git/CI 门禁及跨平台验收。OpenSpec 11.17 不勾选。
