# Ruff 本地报告与修复任务同步切片（2026-09-25）

已初始化且具有 `workspace_id` 的项目运行 `codeguard lint python` 时，CLI 在私有 `codeguard/reports/` 原子保存本轮脱敏局部报告，随后自动调用同一 Rust 同步服务。对话 JSON/human 保留原生配置和诊断，并给出 `backlog_status`；保存或同步失败显示 `backlog_update_failed`，仍保留本轮发现、固定退出 3 和 `delivery_decision=not_evaluated`。未初始化项目不创建工作区文件。

`codeguard work sync [path] --format json` 消费该工作区报告队列，不只取最新一份。它要求报告的 workspace ID、run ID、项目根、Ruff 复检 argv 与当前工作区一致；只把原生完成且目标源码字节摘要仍相同的发现写成 `findings/CG-*/finding.json`、首次 observed event 与 `tasks/CG-*.md`。过时报告计为历史，不生成当前修复任务；不完整文件不会借“未出现发现”关闭旧问题。任务仅包含规则 ID、允许路径、静态修复步骤和复检 argv，不写入原生自由文本。此处没有工具/策略批准，记录明确 `local_unverified`。

同一 finding 的后续报告保留稳定任务，同时把每轮报告摘要、源码摘要、行号和 run ID 关联到默认忽略的 `state/observations/CG-*/<run_id>.json`；相同内容不反复改 tracked finding/task/event。`state/consumed/<run_id>.json` 在观察、事件与投影之后写入。写入先在 Git 忽略的 `state/` 暂存并同步，再硬链接创建目标；标记丢失后重试不重复生成任务或观察。相同 run ID 不同报告摘要报冲突，一份坏报告不阻止其它有效报告导入，整体仍返回 3。本地观察协议固定 `authority=local_unverified`，不能作为原检查已获批准或可交付的证明。

`work_sync_contract` 的 4 项普通测试和固定 Ruff 0.16.8 的 2 项真实测试通过：重复扫描只有一张任务，标记丢失可恢复，过时报告只作历史，错误工作区报告与有效报告并存仍导入有效者，错误报告摘要被拒绝；报告目录不可写时原生 F401 仍在 CLI 反馈中。完整检查器义务、环境 blocker、跨进程锁、重命名协调、attempt/lease、状态机、`next` RepairBrief、原生复检关闭及真正 gate 尚未实现，OpenSpec 9.3–9.14、11.16 仍需后续工作。

后续回归把十轮相同 finding 加入目标测试：只有一份 fact/task 和首次 tracked event，本地观察保留十个不同 run，消费标记丢失重试仍幂等。模拟复检事件后的下一轮重新检出会增加一条恢复待处理的观察事件，再后续相同扫描只更新本地观察。`work_sync_contract` 当前 12 项普通测试与两项固定 Ruff 0.16.8 原生测试通过；全工作区回归和门禁验收以最新 verification 记录为准。
