# Java Syntax P0-B 进度：grammar 资格与原生语法能力

> 日期：2026-10-07；OpenSpec 6.1 / 6.2 / 15.2。
> 状态：grammar 精度已验证，原生语法集成待完成。

## 已完成

### Grammar 精度验证 ✅

| 指标 | 值 |
|---|---|
| 语料 | 700 个 Java 语法样本（200 合法 + 500 违规） |
| TP | 250 |
| FP | **0**（零假阳性） |
| FN | 250 |
| TN | 200 |
| 精度 | **1.0** |
| Wilson 下界 (95%) | **0.9849** ≥ 0.98 |
| 证据 | `tests/acceptance/java-grammar-precision-2026-10-07.md` |

**结论**：Java WASM grammar 的语法解析精度满足 95% Wilson 下界 ≥ 0.98 门槛，
零假阳性，可作为语法初检的可靠基础。

### Grammar 资格路径 ✅

- `PrecisionValidation` 结构体：记录 TP/FP/FN/TN、Wilson 下界、证据路径
- 校验器支持 `release_status: validated` + 完整精度证据
- 证据生成函数 `generate_precision_validation()` 可复用

## 待完成（P0-B 剩余）

### 1. 原生语法版本/方言验收
- 需验证 Maven/Gradle 项目的 Java 版本（JDK 8/11/17/21）
- 需验证方言特性（record、sealed、pattern matching）
- 需验证源集范围（src/main、src/test、generated sources）

### 2. WASM 联合路径
- 需验证 grammar 初检 + 原生工具确认的完整链路
- 需验证"原生优先"路由（有原生工具时不用 WASM）
- 需验证 WASM 候选不被当作已确认违规

### 3. 五平台宿主反馈
- 需在 Codex/Claude/ZCode/Kimi/Gemini 五个宿主验证语法反馈
- 需验证反馈的可读性、准确性和可操作性

### 4. 可信关闭/复发重开
- 需验证 task verify 的证据绑定
- 需验证修复后原工具复检
- 需验证复发时重开同一父链

## 阻塞项（需外部资源）

1. **Maven/Gradle 原生工具**：需要 JDK + Maven/Gradle 环境运行真实编译
2. **五平台宿主**：需要五个宿主的安装和验证环境
3. **可信策略**：需要批准的规则策略和工具锁

## 下一步

1. 先完成 WASM 联合路径的代码集成（可在当前环境验证）
2. 再做原生语法版本/方言验收（需要 JDK 环境）
3. 最后做五平台宿主验证（需要宿主环境）
