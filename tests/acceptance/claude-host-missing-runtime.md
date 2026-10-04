# Claude Code 实际宿主：缺运行时反馈验收

日期：2026-10-04。对应既有 OpenSpec S11.2、S11.4、S11.17、S14.19 以及 `hook-protocol` 的宿主事件要求。插件侧对应 `2026-10-04-rust-lifecycle-default` 的 2.4。这些父任务保持开放，本次只补真实宿主的缺运行时分支。

## 实际执行范围

使用本机已安装 Claude Code 2.1.273，在私有临时空项目中通过 `--plugin-dir` 会话内加载插件源码 `dec5f9d`。这是实际宿主自动调度，不是手工把 JSON 送给 Hook 脚本；同时它也不是从市场安装后的验收。

插件固定四份输入（canonical hooks、Rust dispatcher、runtime manager、0.1.4 lock）的摘要在[脱敏原始观察投影](evidence/claude-host-missing-runtime-2026-10-04.json)中逐项记录。加载的是插件源码，不是当前 CodeGuard 开发二进制；本次刻意保持运行时缓存为空，因此 Rust 检查进程未执行。

关闭内置工具、skills、MCP 与会话持久化；禁用 user/project/local 设置来源，使用空插件启用设置，实际 init 事件确认只有该会话内插件且 tools=[]。第二次执行仅在子进程环境中保留用户已有的认证环境，不复制或公开凭据、不修改全局设置、不新增插件安装。

## 两次终态

| 观察 | 宿主退出 | 自动 Hook | 对话与状态 |
|---|---:|---|---|
| 初次完全隔离设置 | 1 | SessionStart、UserPromptSubmit 均返回 0/success | 宿主因未登录结束，没有有效模型对话；不能计为对话验收 |
| 保留已有认证环境 | 0 | SessionStart、UserPromptSubmit、Stop 均返回 0/success | 一轮实际对话明确引用 `runtime_unavailable`，说明源码未检查、交付未评估，没有声称质量通过 |

第二次耗时 2776 ms，第一次 1081 ms；是单次空项目调用，不是冷启动分布或 p95。宿主报告的第二次费用为约 0.02383 USD，首次为 0。两次项目和私有缓存均保持空目录，工具列表为空；没有下载、原生扫描、任务初始化或源码修改。

三个成功反馈由真实 Claude Code 的 `hook_response` 事件提供：记录事件名、逐字节 stdout、空 stderr、exit_code=0 和 outcome=success。宿主调试记录还确认 SessionStart 的 additionalContext 已被接收；有效模型回复直接说明收到 SessionStart/UserPromptSubmit 中的未完成原因。Stop 的成功事件证明该次结束事件被触发，不证明恢复后的完整任务循环。

## 证据边界及剩余验收

公开 JSON 仅保留上述允许字段、原始 stdout/stderr 文件摘要及实际模型最终回复，不公开完整调试文件、init 的机器路径、认证设置、环境值或模型思考内容。公开文件是脱敏投影，不冒充未改写的完整 stream-json；完整原件留在本地私有临时目录，摘要可核对。

仍需真实宿主的已准备运行时、成功与失败保存、重复事件、原生发现、修复复检、任务关闭授权、Stop 重入、权限拒绝/恢复和市场安装证据；还需要 Codex/ZCode/Kimi 的各自实测以及真实 Git/CI 门禁。不能以 Claude 的三个缺运行时事件替代这些范围，不能勾选 S11.17/S14.19 或插件 2.4。

本次没有修改插件源码、版本、用户安装、受管技能或公开 npm 制品；主目标继续未完成。
