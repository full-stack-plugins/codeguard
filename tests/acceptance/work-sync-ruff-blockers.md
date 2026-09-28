# Ruff 环境阻塞归并切片（2026-09-25）

已初始化项目的 `codeguard lint python` 在原生 Ruff 缺失、配置缺失或执行不完整时，仍保存局部报告并自动同步。`work sync` 为每个 `incomplete` 文件读取有界结构化原因，按 `python.ruff + 构建根 + 原因 + 范围` 生成稳定 `CG-B-*` 身份。工具/配置原因在同一构建根归并多个受影响文件，源码可用性/变化原因限定具体文件；不同构建根不误合并。`findings/CG-B-*/finding.json` 的 `kind=blocker` 与源码违规的 `kind=finding` 分开，首次路径保存在事实中，后续每轮路径与报告摘要保存在默认忽略的 `state/observations/`；首次发现和复检后再次出现才新增 tracked observed 事件。`tasks/CG-B-*.md` 给出环境/配置准备步骤和复检命令，不建议修改无关源码。所有记录仍为 `local_unverified`、`not_evaluated`，同步预览协议升至 0.2 并报告 `new_blockers`。

原阶段 `work_sync_contract` 普通 7 项通过，包含真实 CLI 缺工具时两个文件归并为一张任务、缺配置生成准备任务，以及不同构建根不误合并。后续回归改为确认重复扫描任务及 tracked 事件均不变，本地证据逐轮保留。固定 Ruff 0.16.8 的 2 项忽略测试曾显式运行通过，验证原生发现仍可进入旧 finding 流程、记录故障时原始 F401 仍显示。

这只覆盖 Python/Ruff 的局部不完整原因。跨模块共享 JDK 等公共前置任务、正式义务依赖图、`next` RepairBrief、尝试历史、blocker 复检关闭、可信策略和完整交付门禁尚未实现；OpenSpec 9.3、9.6、9.7、9.10 均保持未完成。
