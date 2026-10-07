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
