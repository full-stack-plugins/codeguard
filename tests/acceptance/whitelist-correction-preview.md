# 白名单纠错预览与本地事件验收

`rules whitelist propose <finding-id> [path] --correct-decision FILE --correction-reason CODE --verification-run RUN_ID [--replacement FILE] [--record] --format json` 生成 `whitelist_correction_preview` 0.2；仅显式 `--record` 会在复核后写入本地提案事件。它从稳定任务读取原生检查器/规则/目标，核对旧候选，并要求传入的 run ID 等于本地 `task verify` 已保存、事件和报告摘要均可复核的最新原生复检。输出始终 `command_status=incomplete`、退出 3、`authority=unverified`、`gate_effect=none`、`delivery_decision=not_evaluated`，不写 `codeguard/decisions/`。

误批、范围过宽或证据失效且原 finding 仍存在时，预览包含旧决策 ID、同一稳定 finding、结构化原因、本轮复检引用和待撤销 ID。`root_cause_fixed` 只在原工具未再检出问题、目标源码未在复检后改变时给出仅撤销预览，不制造替代候选。替代文件必须用 1.1 引用旧 ID、保持同一 finding/规则/目标，并与本轮已观察的源码、工具和发现指纹一致；由于批准的适配器和 rulepack 身份仍缺，替代状态为 `unverified_draft`，整体为 `evidence_incomplete`。环境 blocker、缺复检、错 run、扩大目标、复检后源码再次变化都没有提案。

加 `--record` 后，事件以提案内容 SHA-256 为文件名存入同一 finding 的 `events/`，事件含稳定 task/finding ID、复检引用及待补批准证据。相同提案重放保持同一路径与字节；环境 blocker 或复检后源码变化无事件。事件不是批准，不能关闭任务或改变门禁。`next` 仅在记录绑定的复检 run/报告摘要仍为当前任务观察时，把事件引用呈现在同一修复简报，给出独立评审动作；过期记录不驱动本轮任务。

普通测试：`cargo test -p codeguard-cli --test whitelist_propose_contract --offline`。真实 Ruff 测试显式运行：`CODEGUARD_RUFF_BIN=/opt/anaconda3/bin/ruff cargo test -p codeguard-cli --test whitelist_propose_contract correction_preview_requires_current_original_checker_receipt_and_never_self_approves --offline -- --ignored`。复检后源码再变化的反例也由 `task_verify_contract` 的原生样本覆盖。全工作区与静态检查结果以插件 OpenSpec `verification.md` 为准。

未完成：旧决策是否来自受保护批准、本轮完整适配器/批准 rulepack 身份、自动生成可批准替代候选、批准/驳回/撤销全事件链与完整任务投影协调、可信发布与真实门禁。

## 可读纠错任务附件

显式 --record 在事件保存后生成 `codeguard/tasks/corrections/<finding-id>/<event-sha256>.md`，作为同一稳定任务的历史附件，不创建新 finding。附件包含问题证据、规则依据、允许范围、处理步骤、复检命令、历史尝试与关闭条件；明示待独立评审，引用不可变事件及本轮报告摘要。候选文本中的 Markdown/HTML 标点实体化，不能生成链接、图片或指令。主任务和用户编辑不覆盖。

事件为事实源，附件写入失败仍返回 record_ref 及 projection_status=failed、受限原因与恢复动作；重复 --record 能重建被删除附件。相同路径有不同内容时拒绝覆盖，事件保留。CLI 输出以持锁重新核对的提案为准，不把锁前旧预览与锁后记录混合。next 只在复检观察仍当前，且附件字节与事件重建内容相同时展示 correction_task_refs；附件被用户修改时不展示为当前投影，但事件继续驱动待评审步骤，不改变门禁。

RED：显式原生纠错测试在旧实现缺 projection_status 时失败。GREEN 验证七节内容、幂等引用、用户编辑保留、投影冲突不丢事件、删除后重建及 next 引用；普通转义反例禁止用户候选形成 Markdown/HTML 动作。协议保留读取 0.1，0.2 的投影状态字段必填且 recorded/failed 分别要求文件引用/失败原因。独立批准、可信发布和正式任务关闭仍未完成。

## 复检后配置身份失效

原生 RED：修改 Ruff F401 配置为 E501 后，next 仍显示原纠错提案及附件；新增与旧 ruff.toml 字节相同的高优先级 .ruff.toml 后，普通 propose 仍展示旧原生身份。现在 next 与本地 finding 观察复用只读发现核对：目标仍须在 Python 发现范围内，配置须为 configured，当前选中的路径和字节摘要都要与报告一致；发现不完整或旧报告缺身份时不能沿用观察。

旧纠错事件和历史附件继续保留，但 current next 不引用它们，提案要求 verification_required，普通 propose 返回 latest_configuration_changed；恢复原配置后可以重新调查，不能靠删事件重置问题。已未检出候选之后再修改配置同样要求复检。此复核没有执行项目脚本，也不认证独立批准或完整规则覆盖。
