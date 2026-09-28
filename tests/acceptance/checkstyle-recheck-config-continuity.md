# Checkstyle 复检原始配置连续性

## 验收目标

修改原生规则的名称排除、范围或 token 后，即使检查类及规则 ID 不变、复检零诊断，也不得显示为源码修复候选。首次配置依据从摘要绑定的首次报告恢复，不被后续扫描或复检覆盖。

## 实际验证

Java 21 与固定 Checkstyle 10.21.4，测试 java_checkstyle_workbench::changed_native_field_exclusion_cannot_become_a_repair_candidate：

1. 原配置 JavadocVariable scope=public 检出 public 字段缺注释，生成稳定任务。
2. 增加 ignoreNamePattern=value，不修改源码；原生零诊断。目标测试先失败，旧结果是 candidate_absent_unverified_policy。
3. 修改分类后，连续两次同配置复检均为 rule_coverage_requires_review，事件保存。
4. 恢复原配置，在字段前新增独立行 Javadoc；原工具零诊断，观察为 candidate_absent_unverified_policy。事实仍 open。

最终四轮原生执行通过，耗时 40.99 秒。修复简报增加可选 original_configuration_input，保留原报告的配置路径与 SHA-256；旧简报仍可读，缺首次配置身份时不形成缺失候选。当前以整个原配置字节为边界，因此仅改 XML 空白或其它规则也会要求复核；这会要求额外配置调查，但不会生成源码违规，也不会自动放行。将来经验证的有效规则模型可以细化等价性判定，不能凭同一检查类推断覆盖相同。

## 边界

已有 still_present 诊断仍按实际发现反馈；本验收不证明原配置已经由可信策略批准，不实现语义等价配置识别或完整项目覆盖。正式任务关闭与门禁仍待批准接线。
