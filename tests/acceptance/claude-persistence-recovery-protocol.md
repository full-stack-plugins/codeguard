# Claude 保存反馈：持久化失败后的恢复与稳定任务

日期：2026-10-06。对应 `introduce-rust-codeguard-cli` 的 11.17、14.14、14.19，以及插件 `2026-10-04-rust-lifecycle-default` 的 2.4。本项是当前源码构建程序消费 Claude 事件协议的集成回归，**不是实际 Claude 宿主或市场安装验收**；父任务保持开放。

新增 `hook_syntax_tasks::claude_feedback_survives_persistence_failure_then_reuses_recovered_task`：初始化私有 Zig 项目，禁用 PATH 原生工具发现，使用带有不可信指令注释的坏源码。将 `.codeguard/reports` 从空目录替换为普通文件，制造真实记录失败。连续两次成功保存事件都必须返回候选任务同步未完成、交付未评估；不返回虚构的任务引用，不创建任务，不将源码注释回显到 `additionalContext`。Hook 退出 0 仅说明非阻断反馈成功，不能说明源码检查通过。

恢复原报告目录后，同一源码的两次事件必须生成并复用唯一稳定任务；重复事件不覆盖已有可读任务文件，finding 仍为 open。整个过程不修改源码以规避检查，不安装工具，不修改 grammar、发行锁或关闭策略。

```mermaid
flowchart LR
    A[保存成功 源码含不可信注释] --> B[报告目录故障]
    B --> C[同步失败反馈 无虚构任务]
    C --> D[恢复报告目录]
    D --> E[生成唯一任务]
    E --> F[重复事件复用身份]
    F --> G[任务保持开放 等待原生复检]
```

显式目标测试通过：1 passed / 0 failed / 0 ignored，3.88秒。日志保留在本机 `/private/tmp/cg-claude-persistence-regression.log`。受影响协议与工作台回归的最终结果记入本 change 的 verification。

## 真实宿主尝试与限制

实际安装 Claude 仍为 2.1.273。但旧宿主验收所用私有 cache 缺少 `active.json`；固定 manager 的 `verify` 实际退出 3，报告该文件不存在。新夹具的 init 因此未返回有效报告，测试在启动宿主前终止。该尝试不计为实际宿主运行或失败恢复验收，也不复用旧记录声称最新程序已经运行。没有下载、安装或改动全局配置。

仍缺真实宿主中报告写入故障与恢复、诊断注入防护、权限恢复矩阵、其它宿主，以及当前发行制品的相应验证。CLI 不回显注释只证明协议摘要边界；不证明模型在读取源码后抵御了所有提示注入。

受影响两目标最终合计 **30 passed / 0 failed / 1 ignored**；显式单目标与其中一项重叠，不累加。条件忽略未执行。两目标 WASM 严格 Clippy、定向格式、分层、OpenSpec strict 和 diff 检查通过。[证据索引](evidence/claude-persistence-recovery-protocol-2026-10-06.json)记录测试源码及日志摘要。未运行完整工作区回归或全语料精度回放，本次只新增验收测试，不改变产品行为。
