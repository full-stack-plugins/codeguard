# ShellCheck 原规则 SDK 修复闭环验收

日期：2026-10-06；对应现有 OpenSpec S09 / S10 / S12。仅是源码 SDK 切片，父任务继续开放。

## 行为与协议

新增 `ShellTaskResolutionRequest` / `verify_shell_task_resolution`；策略 1.10.0、证据 0.11.0、收据 0.2.0。普通 Shell finding 使用 `CG-…` 与 `shell.shellcheck`，不会伪造 WASM grammar 或借用 `CG-B-…` 确认任务身份。旧收据 0.1.0 不改；新收据只用于此版本 Shell 证据。

同一固定 ShellCheck 0.11.0 对原样本及当前样本复检，绑定原规则、方言、配置、工具和适配器。完整检查不再含原规则即可关闭原任务；另一条规则仍有诊断时保留其独立任务。重复关闭幂等，同配置同工具的普通 task verify 检测到原规则复发时沿父链重开。修改项目 rc、禁用指令或已有目标规则禁用指令缺乏作用域证明时不能冒充源码修复。支持借用租约，消耗待核验尝试而不释放调用方租约。

## 实际验证

- 初始 RED：缺少 SDK 导出；后续回归暴露 Shell finding 使用 path 而共用生命周期按 scope 读取，已修复该绑定。
- 真实输出协议 RED：旧收据 0.1.0 只接受 CG-B 确认任务，普通问题 ID 被拒绝；新增 0.2.0 后，真实/受控报告的 schema 三项通过。旧 schema 仍拒绝新收据；跨版本、跨 checker、伪批准和未完成关闭反例被拒绝。
- 14 个共享生命周期及 Shell 目标：85 passed / 0 failed / 14 ignored；忽略的真实工具用例不计通过。日志 `/private/tmp/codeguard-shell-receipt-shared.log`。
- 新借用租约及尝试消费用例单独实际执行：1 passed / 0 failed / 0 ignored；日志 `/private/tmp/codeguard-shell-receipt-lease.log`。
- 已有本机 `/opt/homebrew/bin/shellcheck` 0.11.0 显式执行两个真实用例：2 passed / 0 failed / 0 ignored，三个实际结果归档。SC2086 修复并复发；SC2086 已解决而 SC2154 仍保留时原任务关闭。日志 `/private/tmp/codeguard-shell-receipt-real.log`。
- 默认/WASM CLI all-targets 严格 Clippy、分层、OpenSpec strict、所改 Rust 文件格式和 diff 检查通过；三个新 schema 元定义有效。
- [真实报告](evidence/shell-task-resolution-real-2026-10-06.json) 与 [受控报告](evidence/shell-task-resolution-controlled-2026-10-06.json) 包含实际策略、收据、证据；schema 校验脚本 `tests/shell_task_resolution_schema.py`。

## 验收边界

签名密钥、时钟和信任根是测试宿主夹具；真实 ShellCheck 执行不证明默认宿主可信关闭或所有 Shell 规则均可修复。不放宽普通 CLI 权限，不签发项目 allow，不改变 grammar 资格，不声明 npm 已发布该能力。真实宿主、跨平台、完整项目/依赖/安全、发布及 S09 全契约仍缺。用户维护的 Erlang 原生差分草稿未修改、暂存或执行。
