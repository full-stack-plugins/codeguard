# Checkstyle 准备任务原工具复检验收

状态：局部实现与本机验收；不代表完整 Java 义务、可信批准或任务正式关闭。

## 行为

`task verify` 支持 `java.checkstyle.preparation`，复用任务租约、尝试与统一截止时间。显式提供 Java、Checkstyle JAR 和原配置，不从可编辑历史记录选择可执行工具。缺前置时保存 `still_blocked`；完整局部原生观察为 `environment_restored_unverified_policy`。复检报告与事件保留具体原因，任务始终 open。

正常观察保存冻结输入并复核字节；其中源码 finding 使用既有稳定身份同步，`next` 能引导对应源码修复。准备报告、源码指导、历史事件及尝试读取均识别新容器；已消费收据不因今日输入变化被当成新的坏报告。输入变化后不能沿用局部恢复线索。

新增 `checkstyle-preparation-recheck` 与 `checkstyle-preparation-verification-preview` schema，更新准备简报复检 ID、观察及原因；授权固定 local_unverified、覆盖 false、交付 not_evaluated。

## 验证

- TDD：缺前置测试在实现前得到 task_checker_unsupported/incomplete，目标 still_blocked 断言失败；实现后通过。
- 五个受影响集共 53 项普通测试通过，14 项需要原生工具的测试未在该普通运行执行。
- 显式 Java 21 和 Checkstyle 10.21.4 的恢复回放通过（8.36 秒）：一个原生 MissingJavadocType finding 转入源码任务，准备任务保持 open，next 指向源码任务。
- 错租约 token 返回 lease_token_mismatch，无原生报告、无复检事件，报告数量不变。
- 全 workspace/all-targets Clippy（拒绝警告）、格式检查、OpenSpec 严格校验与插件差异空白检查通过。
- 两轮实际 blocked/recovered 报告、命令反馈与 next 简报通过专用 schema；伪造 coverage_proven=true 被拒。
- 新增身份校验时曾要求所有读取简报含命令临时 evidence_workspace_id，合法历史事件因而被拒；修正为在该显式字段存在时核对，历史读取仍独立核对事件/事实工作区，回归与实际产物通过。

可复现命令：

```bash
cargo test -p codeguard-cli --test java_checkstyle_workbench --test task_verify_contract --test task_lease_contract --test next_command_contract --test work_sync_contract
CODEGUARD_JAVA_BIN=/absolute/java CODEGUARD_CHECKSTYLE_JAR=/absolute/checkstyle-10.21.4-all.jar cargo test -p codeguard-cli --test java_checkstyle_workbench preparation_verify_native_recovery -- --ignored
```

## 未完成

模块/JDK 粒度准备义务、完整生效配置与规则集、可信工具和审批来源、全项目覆盖、正式关闭与重开、宿主 Hook 门禁仍须完成。本次复检成功不能替代这些条件；OpenSpec 9.7 保持未勾选。

## 准备任务无进展预算补充验收

新增准备任务专用完整回放：claim → attempt start → finish ready-to-verify → 拒绝提前重试 → task verify still_blocked → 事件绑定 attempt → next 历史累计。两轮均计为无进展，第三次相同动作被 no_progress_budget_exhausted 拒绝，next 为 needs_decision，任务仍 open。

TDD 首次暴露第二轮开始被 task_not_actionable 拒绝：失败复检后准备简报固定 verification_required，使环境修复无法继续。现对原观察源码仍有效的失败复检恢复 actionable，随后由已有尝试预算控制；源码已变化或不可用仍保持 verification_required，新增反例通过。没有新增忽略规则、人工关闭或预算重置入口。

本轮三个相关普通测试集共 29 项通过，四项原生用例在普通运行中未执行；另显式运行真实 Java/Checkstyle 环境恢复测试通过（9.28 秒）。这是准备任务局部链路验收，不代表完整项目覆盖、批准或正式关闭。

## 恢复尝试关联与工具变化补充验收

真实恢复回放新增 claim/start/finish ready-to-verify：原工具恢复事件精确关联该 attempt，task show 的 awaiting_verification=false、no_progress_count=0；源码 finding 继续转入修复任务，环境任务保持 open。使用项目内复制 JAR，避免改动验证工具原件。

将复制 JAR 的内容改动后，旧 task show 仍显示 environment_restored_unverified_policy，真实反例首次失败。新增反馈阶段 Java/JAR 的绝对路径、规范路径、普通文件有界读取与内容摘要核对；变化或不可用时，next/task show 要求重新核对工具复检，不附旧恢复或缺失观察。处理同时适用于准备与源码 Checkstyle 任务，不授予新工具批准权威。

扩展后的真实回放通过（10.28 秒）。首次测试错误读取 task show 的 repair_brief 字段已改为实际 task 字段；随后确认红灯确实来自旧恢复结论，而非测试字段缺失。

后续六轮原生源码 task verify 回归通过（45.25 秒），覆盖仍存在、文档修复、规则身份变化、缺 JAR 与工具字节变化；39 项普通回归、CLI 全目标 Clippy、格式与 OpenSpec 严格校验通过。

## 失效原因与状态摘要补充验收

新增 `verification_invalidated_reason` 区分 tool_inputs_changed_or_unavailable、source_input_changed_or_unavailable、configuration_input_changed_or_unavailable；它与历史原工具 `verification_reason` 分开，不改写原收据。status 摘要保留这两类原因。专用 next schema 声明精确失效原因；status schema 对任务摘要的字段、开放状态、本地权威与处置值进行校验，不能在该投影伪称关闭或批准。

TDD 暴露 status 丢失 prerequisites_missing，产品修正后通过。17 项普通相关回归通过。真实恢复回放新增 JAR 字节改动后删除的反例，task show/status 均显示工具失效，不沿用恢复，回放通过（9.77 秒）。实际报告、简报与状态经过 schema 校验；伪造覆盖仍被拒。完整宿主批准与正式门禁不在本次验收范围。
