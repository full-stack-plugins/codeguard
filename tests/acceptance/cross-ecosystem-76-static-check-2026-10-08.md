# 跨生态 7.6 静态检查验收

> 日期：2026-10-08；OpenSpec 7.6

## 验收标准覆盖

| 标准 | 测试 | 状态 |
|---|---|---|
| 注释/API 文档检查器 | comments_checker_config_recognized | ✅ |
| 依赖治理检查器 | dependency_checker_config_recognized | ✅ |
| SAST 检查器 | sast_checker_config_recognized | ✅ |
| 秘密检查器 | secrets_checker_config_recognized | ✅ |
| IaC 检查器 | iac_checker_config_recognized | ✅ |
| 容器检查器 | container_checker_config_recognized | ✅ |
| 未配置/无效/不可用分开 | unconfigured_invalid_unavailable_separate | ✅ |
| 修复后原工具复检 | recheck_with_original_tool_after_fix | ✅ |
| 跨生态完整报告 | cross_ecosystem_complete_report | ✅ |

## 测试结果

| 测试 | 说明 | 状态 |
|---|---|---|
| comments_checker_config_recognized | 注释检查器配置识别 | ✅ |
| dependency_checker_config_recognized | 依赖检查器配置识别 | ✅ |
| sast_checker_config_recognized | SAST 检查器配置识别 | ✅ |
| secrets_checker_config_recognized | 秘密检查器配置识别 | ✅ |
| iac_checker_config_recognized | IaC 检查器配置识别 | ✅ |
| container_checker_config_recognized | 容器检查器配置识别 | ✅ |
| unconfigured_invalid_unavailable_separate | 三种状态分开 | ✅ |
| recheck_with_original_tool_after_fix | 原工具复检 | ✅ |
| cross_ecosystem_complete_report | 完整报告 | ✅ |
| **总计** | | **9/9** |

## 验证明细

### 检查器配置识别 ✅
- 注释: checkstyle（MissingJavadocType）
- 依赖: maven_dependency（outdated）
- SAST: spotbugs（SQL_INJECTION）
- 秘密: gitleaks（api_key_exposed）
- IaC: terraform_validate（open_security_group）
- 容器: trivy（CVE-2024-1234）

### 未配置/无效/不可用分开 ✅
- not_configured: checker_not_declared
- invalid: config_syntax_error
- unavailable: tool_not_found

### 修复后原工具复检 ✅
- recheck_tool 与原 tool 一致
- findings 清空后 status=complete

## 结论

7.6 跨生态静态检查验收标准全部满足。
注释/依赖/SAST/秘密/IaC/容器检查器配置识别与结果归类正确。
