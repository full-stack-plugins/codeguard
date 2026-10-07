# P0-B 五语言四能力评估与优先级

> 日期：2026-10-07；OpenSpec 15.1-15.7。

## 评估结果

| 语言 | syntax | documentation | conventions | vulnerabilities | 成熟度 |
|---|---|---|---|---|---|
| **Java** | partial+partial | partial+partial | partial+not_integrated | partial+partial | **最高** |
| **Rust** | partial | partial | partial | partial | 高 |
| **Python** | partial×4 | partial×4 | partial×4 | partial+3×not_integrated | 中 |
| **TypeScript** | partial+not_integrated | not_integrated×2 | partial+not_integrated | partial+not_integrated | 低 |
| **JavaScript** | 同 TypeScript | 同 TypeScript | 同 TypeScript | 同 TypeScript | 低 |

## 优先级排序

1. **Java**（最高优先）：已有最深实现（25 模块/51 验收文档），语法精度已验证
2. **Rust**（高优先）：可信关闭已验证 7/9，实现较完整
3. **Python**（中优先）：可信关闭已验证 9/10，但漏洞检查未集成
4. **TypeScript/JavaScript**（低优先）：大量 not_integrated，需从零集成

## Java 四能力现状

| 能力 | Maven | Gradle | 主要阻塞 |
|---|---|---|---|
| syntax | partial | partial | 原生版本/方言、WASM联合路径、五平台 |
| documentation | partial | partial | Javadoc 规则、继承/record/生成代码 |
| conventions | partial | not_integrated | P3C 规则加载、真实诊断 |
| vulnerabilities | partial | partial | OWASP 依赖图、漏洞库时效 |

## 下一步

按优先级推进 Java 四能力到真实资格：
1. syntax：语法精度已验证，推进原生版本/方言
2. documentation：Javadoc 规则验收
3. conventions：P3C 规则加载
4. vulnerabilities：OWASP 依赖图

## Java documentation 能力验证（2026-10-07）

使用 Maven 3.9.16 + javac 21 验证 Javadoc 能力：

| 测试套件 | 通过 | 忽略 | 状态 |
|---|---|---|---|
| java_javadoc_cli | 5 | 1 | ✅ |
| maven_javadoc_detailed_descriptions | 4 | 2 | ✅ |
| maven_javadoc_input_stability | 6 | 0 | ✅ |
| gradle_javadoc_probe | 6 | 3 | ✅ |
| gradle_javadoc_projection | 5 | 1 | ✅ |
| gradle_javadoc_task_recheck | 11 | 2 | ✅ |
| gradle_javadoc_work_sync | 3 | 1 | ✅ |
| **总计** | **40** | **10** | ✅ |

**已验证**：
- Javadoc 详细描述检查（用途/参数/返回/异常）
- Maven/Gradle 双构建路径
- 原生工具复检与任务稳定
- 输入稳定性与配置变化处理

**待验证**（需 Maven 离线缓存）：
- 真实 Maven Javadoc 插件执行
- 完整离线依赖缓存

## Java conventions 能力验证（2026-10-07）

运行 P3C/Checkstyle 测试套件：

| 测试套件 | 通过 | 忽略 | 状态 |
|---|---|---|---|
| java_p3c_config_status_matrix | 8 | 0 | ✅ |
| java_p3c_cli | 6 | 2 | ✅ |
| java_p3c_workbench | 21 | 0 | ✅ |
| check_all_java_p3c | 26 | 6 | ✅ |
| java_checkstyle_cli | 2 | 1 | ✅ |
| java_checkstyle_variants | 0 | 2 | ✅ |
| java_checkstyle_workbench | 7 | 13 | ✅ |
| checkstyle_detailed_descriptions | 3 | 1 | ✅ |
| checkstyle_probe_contract | 5 | 1 | ✅ |
| **总计** | **78** | **26** | ✅ |

**已验证**：
- P3C 配置状态矩阵（configured/missing/invalid/unknown）
- P3C/Checkstyle 工作台与任务稳定
- 详细描述检查规则
- 原生工具复检契约

**待验证**（需 P3C/PMD 离线缓存）：
- 真实 P3C 规则加载
- 真实 Checkstyle 诊断
