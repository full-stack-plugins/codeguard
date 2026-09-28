# Python 编辑范围快检局部验收（2026-09-29）

`codeguard lint python [path] --file REL_PATH [--file REL_PATH...] --format json` 复用现有 Ruff 原生探测，但只对明确选中的已发现 Python 文件读取源码并执行逐文件检查。请求最多 8 个不同路径，单路径不超过 512 字节、路径总计不超过 2 KiB；绝对路径、`..`、空组件、反斜杠和控制字符在启动检查前拒绝。重复路径去重。找不到的目标以 `target_not_discovered:<path>` 报未完成，不回退扫描其它源码。

对话报告升级为 `python_lint_feedback` 0.13，标明 `scan_scope=selected_files` 与 `requested_paths`，始终保持 `delivery_decision=not_evaluated`、退出码 3。局部结果的 `backlog_status=not_synced_scoped`；本轮不会把不完整范围导入 `.codeguard/reports`、伪造成完整工作台扫描或关闭旧任务。不传 `--file` 的原有项目扫描和同步流程仍保持。原 0.12 schema 保存在 `schemas/python-lint-feedback-v0.12.schema.json`，`check all` 的嵌套 Python 结果仍用原 0.12 语义。人工修复和正式交付仍需原任务复检及完整门禁。

先写的 CLI 目标测试因 `--file` 尚不支持而失败。实现后，选定文件、未发现目标、越界路径和 9 文件超预算测试通过；目标无 Ruff 配置而另一模块有配置时不启动原生工具。使用本机 `ruff 0.16.8` 的显式绝对路径执行独立 native 测试：两份 Python 源码中仅所选文件产生 F401 反馈，初始化工作区的报告数量没有增加。`lint_python_cli` 常规 16 项通过、5 项需外部 Ruff 的测试 ignored；`python_lint_scan_contract` 常规 7 项通过、5 项需外部 Ruff 的测试 ignored；新 native 定向测试另行显式执行并通过。

独立 Draft 2020-12 校验使用本机已有 `jsonschema 4.25.1`：项目扫描、选中文件及未发现目标的三种实际 CLI 输出均通过新 0.13 schema，均不通过保留的 0.12 schema。Python 只用于独立协议验收，不属于 Rust 产品运行路径。目标测试、CLI 全 targets Clippy `-D warnings` 通过。

本切片尚未接入 `hook plan` 或 Codex/ZCode/Kimi 的真实编辑事件；没有软反馈缓存身份、跨语言局部调度、批量超预算执行和严格 Git/CI 门禁。OpenSpec 11.17 仍未完成。
