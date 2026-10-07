# Java Section 6 全链路验收状态

> 日期：2026-10-07；OpenSpec 6.2-6.8

## 任务状态总览

| 任务 | 描述 | 验收标准 | 证据 | 状态 |
|---|---|---|---|---|
| 6.1 | Maven/Gradle/JDK 观察 | 多模块/父POM/动态范围/错误JDK | java-mixed-build-roots.md + 3 测试 | ✅ |
| 6.2 | P3C/PMD/JDK 兼容组合 | 正反例证明规则加载 + 使用 P3C 名称 | ClassNamingShouldBeCamelRule 正反例 | ✅ |
| 6.3 | Checkstyle/Javadoc 注释 | 公共API/参数返回/inheritDoc/record/Lombok | MissingJavadocType 正例 | ⚠️ 部分 |
| 6.4 | Java 安全/CVE | 真实样本/坏报告/库过期与模糊匹配分开 | 9 CVE 测试 + OWASP 配置反馈 | ⚠️ 部分 |
| 6.5 | build 等级和测试执行 | 默认静态构建不谎称测试通过 | 无专用测试 | ❌ 缺 |
| 6.6 | 义务闭包 | verify未绑定/自定义echo/50-51文件/未修改调用方 | java-p3c-cli-native-local.md | ⚠️ 部分 |
| 6.7 | 修复前后对照 | 每条旧新差异人工裁定 | 无专用测试 | ❌ 缺 |
| 6.8 | 依赖治理/CVE配置 | 依赖图/版本/许可证/SBOM分开/动态版本不伪造 | Maven 依赖测试 6/6 | ⚠️ 部分 |

## 已验证能力

### 6.1 Maven/Gradle/JDK 观察 ✅
- 多模块、父 POM、动态未知范围、错误 JDK 真实样本齐备
- 3 测试通过（java_mixed_build_roots）

### 6.2 P3C/PMD/JDK 兼容组合 ✅
- P3C 2.1.1/PMD 6.15.0/Maven PMD 3.11.0 锁定
- ClassNamingShouldBeCamelRule 正反例验证规则实际加载
- 使用 P3C 名称，不用 Checkstyle 替代
- 配置状态矩阵反例（skip/management-only/profile 限定）

### 6.3 Checkstyle/Javadoc ⚠️ 部分
- Checkstyle 10.21.4/Java 21 真实执行
- MissingJavadocType 正例验证
- **缺**：参数返回/inheritDoc/record/Lombok/生成代码正反例

### 6.4 Java 安全/CVE ⚠️ 部分
- OWASP Dependency-Check Maven 配置反馈
- FindSecBugs 配置观察
- 9 CVE 测试通过（含坏报告反例）
- **缺**：库过期与模糊匹配分开记录

### 6.5 build 等级 ❌ 缺
- **缺**：默认静态构建不谎称测试通过验证
- **缺**：要求测试的策略不可自动跳过验证

### 6.6 义务闭包 ⚠️ 部分
- verify 未绑定质量任务验证
- 混合构建根保留双方检查器
- **缺**：自定义 echo/50-51文件/未修改调用方失败验证

### 6.7 修复前后对照 ❌ 缺
- **缺**：代表性 Java 项目 check java/check all/doctor 对照
- **缺**：每条旧新差异人工裁定

### 6.8 依赖治理 ⚠️ 部分
- Maven 依赖树原生探针（4 测试通过）
- 动态坐标拒绝反例
- **缺**：许可证/SBOM 分开记录
- **缺**：私服不可达不伪造清洁结果

## 测试覆盖

| 测试套件 | 通过 | 忽略 | 状态 |
|---|---|---|---|
| java_mixed_build_roots | 3 | 0 | ✅ |
| check_java_cve_native | 2 | 0 | ✅ |
| check_java_cve_attribution | 7 | 1 | ✅ |
| maven_dependency_pom_contract | 2 | 0 | ✅ |
| maven_dependency_tree_contract | 4 | 1 | ✅ |
| **总计** | **18** | **2** | ✅ |

## 剩余工作

1. **6.3 补全**：参数返回/inheritDoc/record/Lombok/生成代码正反例
2. **6.4 补全**：库过期与模糊匹配分开记录
3. **6.5 实现**：build 等级和测试执行声明
4. **6.6 补全**：自定义 echo/50-51文件/未修改调用方失败验证
5. **6.7 实现**：修复前后对照
6. **6.8 补全**：许可证/SBOM 分开记录 + 私服不可达验证

## 结论

Section 6 已完成 2/8 任务（6.1/6.2），4/8 部分完成（6.3/6.4/6.6/6.8），2/8 缺失（6.5/6.7）。
Java 全链路核心能力已验证，但完整验收仍需补全上述 6 项。
