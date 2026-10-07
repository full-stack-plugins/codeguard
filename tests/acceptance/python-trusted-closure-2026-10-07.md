# Python 可信关闭/复发重开验收

> 日期：2026-10-07；工具：Ruff 0.16.8（/opt/anaconda3/bin/ruff）。
> OpenSpec 9.10 / 15.6。

## 测试结果

使用真实 Ruff 0.16.8 运行 10 项可信关闭测试：

| 测试 | 结果 |
|---|---|
| adding_noqa_does_not_look_like_a_repaired_finding | ✅ PASS |
| native_recheck_of_a_repaired_finding_clears_pending_attempt_without_closing_the_task | ✅ PASS |
| per_file_ignore_requires_review_before_original_task_can_close | ✅ PASS |
| disabling_original_rule_does_not_look_like_a_repaired_finding | ✅ PASS |
| native_ruff_verify_respects_the_borrowed_token_before_scanning | ✅ PASS |
| changed_source_with_the_same_native_finding_becomes_actionable_after_recheck | ✅ PASS |
| absent_original_finding_is_only_a_candidate_until_policy_and_coverage_are_verified | ✅ PASS |
| d100_check_all_creates_a_stable_task_and_rechecks_with_native_ruff | ✅ PASS |
| d101_task_explains_class_docstring_and_requires_native_recheck | ✅ PASS |
| recovered_environment_is_observed_but_old_blocker_is_not_falsely_closed | ❌ FAIL |

**总计：9/10 通过**

## 已验证的可信关闭行为

1. **noqa 不等于修复**：添加 `# noqa` 不会被当作已修复 ✅
2. **原工具复检清除待处理尝试**：修复后原工具复检通过，但不关闭任务 ✅
3. **per-file ignore 需要审查**：忽略规则后原任务不能直接关闭 ✅
4. **禁用规则不等于修复**：禁用原规则不会被当作已修复 ✅
5. **借用租约验证**：Ruff 验证尊重借用的租约令牌 ✅
6. **源码变化后重新可操作**：同一发现的源码变化后变为 actionable ✅
7. **原始发现缺失仅是候选**：缺少原始发现时仅标记为候选 ✅
8. **D100 稳定任务与复检**：创建稳定任务并用原生 Ruff 复检 ✅
9. **D101 任务解释与复检**：任务解释类文档字符串并要求原生复检 ✅

## 发现的功能缺口

**测试 10 失败**：`recovered_environment_is_observed_but_old_blocker_is_not_falsely_closed`

- **预期行为**：verify 后 `next` 应显示新发现（`native_rule_id: F401`）并创建新任务
- **实际行为**：`next` 显示原始任务（`project_ruff_config_not_found`），缺少 `native_rule_id`
- **根因**：verify 步骤检测到 F401 发现，但 `next` 命令的 repair_brief 未关联新发现
- **影响**：环境恢复后，新发现不能通过 `next` 引导修复

## 结论

可信关闭/复发重开的核心机制已验证（9/10）。
剩余 1 项是 `next` 命令与 verify 发现的关联问题，属于 P0-B 的待修复项。
