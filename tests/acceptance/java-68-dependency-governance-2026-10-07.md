# Java 6.8 依赖治理验收

> 日期：2026-10-07；OpenSpec 6.8

## 验收标准覆盖

| 标准 | 测试 | 状态 |
|---|---|---|
| 依赖图/版本/许可证/SBOM 分开记录 | dependency_categories_recorded_separately | ✅ |
| 未配置时给智能体具体建议 | unconfigured_gives_specific_suggestions | ✅ |
| 动态版本不伪造清洁结果 | dynamic_version_never_fakes_clean | ✅ |
| 父 POM 不伪造清洁结果 | parent_pom_never_fakes_clean | ✅ |
| 私服不可达不伪造清洁结果 | private_repo_unreachable_never_fakes_clean | ✅ |
| 依赖治理完整报告 | dependency_governance_complete_report | ✅ |
| 依赖图/版本分开记录 | dependency_graph_and_version_recorded_separately | ✅ |

## 测试结果

| 测试 | 说明 | 状态 |
|---|---|---|
| dependency_categories_recorded_separately | 各类别独立记录 | ✅ |
| unconfigured_gives_specific_suggestions | 未配置时给具体建议 | ✅ |
| dynamic_version_never_fakes_clean | 动态版本不伪造清洁 | ✅ |
| parent_pom_never_fakes_clean | 父 POM 不伪造清洁 | ✅ |
| private_repo_unreachable_never_fakes_clean | 私服不可达不伪造清洁 | ✅ |
| dependency_governance_complete_report | 完整报告结构 | ✅ |
| dependency_graph_and_version_recorded_separately | 依赖图/版本分开 | ✅ |
| **总计** | | **7/7** |

## 验证明细

### 依赖图/版本/许可证/SBOM 分开记录 ✅
- dependency_graph: nodes/edges 独立记录
- version_check: outdated 独立记录
- license_check: violations 独立记录
- sbom_check: components 独立记录
- vulnerability_check: advisories 独立记录

### 未配置时给智能体具体建议 ✅
- dependency_graph: configure_maven_dependency_plugin
- version_check: configure_versions_maven_plugin
- license_check: configure_license_maven_plugin
- sbom_check: configure_cyclonedx_maven_plugin
- vulnerability_check: configure_owasp_dependency_check

### 动态版本不伪造清洁结果 ✅
- status: "incomplete"
- reason: "dynamic_version_detected"
- clean: false

### 父 POM 不伪造清洁结果 ✅
- status: "incomplete"
- reason: "parent_pom_not_resolved"
- clean: false

### 私服不可达不伪造清洁结果 ✅
- status: "incomplete"
- reason: "private_repo_unreachable"
- clean: false

## 结论

6.8 依赖治理验收标准全部满足。
依赖图/版本/许可证/SBOM 分开记录，未配置时给具体建议，动态版本/父POM/私服不可达不伪造清洁结果。
