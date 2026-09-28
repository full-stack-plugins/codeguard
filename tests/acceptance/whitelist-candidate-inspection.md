# 误报白名单候选只读查询切片

`codeguard rules whitelist list --candidate FILE [--candidate FILE...] --format json` 与 `codeguard rules whitelist explain ID --candidate FILE [--candidate FILE...] --format json` 只读取显式文件，不扫描项目脚本、不写入策略。命令调用既有严格候选解析器，限制每个文件 128 KiB、每次最多 100 个文件；坏文件和重复决策 ID 报 incomplete/退出 3。有效候选的查询退出 0 仅表示**只读查询成功**，公开 `authority=unverified`、`gate_effect=none` 和 `candidate_unverified`，不能签发质量通过或误报豁免。

输出不回显自由文本 rationale、原生消息或批准引用，防止把候选中的指令性文本展示给智能体。`explain` 必须精确命中 ID；缺失 ID 返回 incomplete。公开协议 `whitelist-candidate-inspection.schema.json` 固定无批准或 allow 状态。

目标测试先因命令不存在失败，随后 `whitelist_command_contract` 4 项通过，覆盖有效候选、精确 ID、恶意自由文本不回显、非法本地 `approved=true`、重复 ID 和 schema 边界。该切片不自动发现候选，不生成 `propose`，不验证可信批准快照、时钟、本次原生 finding 或受保护 CI；OpenSpec 4.9 与 12.12 保持未完成。

## 2026-09-25 精确身份失配解释

`explain ID --candidate FILE --observed-identity FILE` 可把显式提供的发现身份与候选作严格比较：同一身份输出 `identity_matched`，内容摘要、规则或工具身份变化输出 `identity_mismatch`。协议 0.3.0 的 `mismatch_fields` 只列字段名，不回显观察值；`next_action` 提醒复检、修复候选或核验独立批准。观察文件非法、候选集有坏文件、同 ID 冲突或不同 ID 重复处置同一精确身份时不报告匹配。观察文件也只是本地输入，不能证明来自本轮原生检查；始终保持 `authority=unverified`、`gate_effect=none`，匹配成功也不批准白名单。命令只读，不显示候选自由文本。目标契约测试 8 项通过；可信原生身份、审批来源、期限与门禁绑定仍未实现。
# 同一 finding ID 的候选冲突

即使两条候选的源码目标或内容不同，只要复用同一个稳定 finding ID，`rules whitelist list/explain` 就将双方标为冲突；带观察身份的 `explain` 不会选中其中一条输出 `identity_matched`。纯候选筛选器同样在进入独立批准核验前返回 `ConflictingCandidates`。回归位于 `whitelist_command_contract::one_finding_id_with_different_targets_is_a_conflict` 与 `false_positive_decision_contract::same_finding_id_with_different_target_cannot_reach_authority_check`。此检查仍不提供可信批准或门禁放行。
