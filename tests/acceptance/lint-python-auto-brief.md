# Python 扫描自动交付局部修复简报（2026-09-25）

已初始化项目的 `codeguard lint python` 在原生扫描后先保存 0.4 本地报告并同步，再调用与独立 `codeguard next` 相同的只读服务。对话反馈 0.5 新增 `repair_brief_status`、`repair_brief_reason`、`next`；JSON 给出结构化简报，human 直接显示任务、范围、步骤和复检 argv。缺配置会给 needs_decision，缺工具会给环境准备；真实 Ruff F401 在同一响应内同时显示原生 finding 和 actionable 简报。未初始化或同步失败时不伪造简报，仍保留原生诊断与 `backlog_update_failed` 等状态。简报读取失败单独标为 unavailable，不把已同步 backlog 错说成同步失败。

当前 `lint python` 与 `check all` 共用检查类 `--timeout <正整数ms|s|m|h>` 解析，默认 30m、上限 24h；非法值在启动原生进程前返回 2。CLI 从参数解析后建立一次截止时间并交给 Ruff 版本探测和逐文件检查，超时仅产生未完成观察，不能形成通过。本地同步、简报和清理尚未实现硬 I/O 截止时间，不能将此局部接线解释为完整总预算验收。普通预算用例在 `cargo test -p codeguard-cli --test lint_python_cli --offline` 中执行。

公开对话反馈 0.10 增加 `execution_budget.timeout_ms/source/enforcement`；`source` 当前是 `cli`、`registered_environment`、`project_default` 或 `builtin_default`。已登记 `CODEGUARD_TIMEOUT` 在未传 CLI 值时生效，非法值在启动原生工具前返回 2，显式 CLI 值优先。`enforcement=native_execution_only` 明确表示持久同步、简报及清理仍未受硬截止时间控制。工作区内的原始 0.8 扫描报告不因对话字段升级而改写。

公开反馈 0.11 修正取消出口：真实 SIGINT 发生于 Ruff 原生探测时，CLI 返回 130、`command_status=cancelled`、`exit_code=130`，保留 `request_cancelled` 原因和已取得的局部证据；子孙进程被清理，延迟写入标记不会出现。普通未完成仍退出 3。对话协议变化不改写工作区内的原始 0.8 扫描报告。

可选 `codeguard/runtime.json` 1.0 在无 CLI/登记环境预算时提供项目默认超时，公开来源为 `project_default`。`lint_python_cli` 验证它不会把缺 Ruff 配置改成质量通过，原生反馈仍为 `not_evaluated`。

公开反馈 schema 为 `python-lint-feedback:0.5.0`，`next` 嵌入 `repair_brief_preview:0.1.0` 且固定本地未验证、交付未评估。保存到 `codeguard/reports/` 的输入仍是 0.4，不包含反馈自身或简报，避免递归写入和改变同步摘要。`lint_python_cli` 8 项普通测试通过，包含简报失败但原生配置诊断和同步结果仍显示，以及 human 对话直接显示任务与复检；固定 Ruff 0.16.8 的 3 项显式原生测试通过，其中初始化项目的 F401 在同一 CLI 响应内返回修复简报。

这只是 Rust CLI 内的 Python/Ruff 局部 scan→sync→brief 链。插件 Hook/MCP 宿主、其它语言/类别、可信策略、attempt/租约及完整交付门禁尚未接线；OpenSpec 9.13 与 11.16 仍未完成。
