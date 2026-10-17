# CodeGuard v1 质量门禁定义与交付物

> 版本：v1.0-candidate（2026-10-07）
> 对应 OpenSpec 15.2/15.3，「按分版本逐步推进」策略的第一个版本级交付物。

## v1 质量门禁标准

v1 质量门禁定义为**主流语言的核心检查能力可验证、可复现、有独立精度证据**。

### 1. 语法检查（WASM Grammar）

| 标准 | 门槛 | v1 状态 |
|---|---|---|
| Grammar 精度验证 | Wilson 下界 ≥ 0.98 | ✅ 17 种语言 PASS |
| 零假阳性 | FP = 0 | ✅ 17/17 |
| 精确定位 | byte_offset + grammar SHA-256 | ✅ |
| 版本/方言披露 | known_limitations 非空 | ✅ |

### 2. 原生工具集成

| 标准 | 门槛 | v1 状态 |
|---|---|---|
| 原生优先路由 | 原生可用时优先使用 | ✅ |
| 缺工具初检 | incomplete + 退出码 3 + 指引 | ✅ |
| 配置发现 | checker 状态识别（configured/missing） | ✅ |
| 执行预算 | timeout_ms + enforcement | ✅ |

### 3. 文档注释检查

| 标准 | 门槛 | v1 状态 |
|---|---|---|
| Javadoc 完整性 | 用途/参数/返回检出 | ✅ Java |
| 裸标签不冒充 | 空注释/裸标签检出 | ✅ Java |
| 行注释不冒充 | `//` 不替代 Javadoc | ✅ Java |

### 4. 六类别覆盖

| 类别 | v1 状态 |
|---|---|
| lint | ✅ 适配器就绪 |
| comments | ✅ 适配器就绪 |
| dependencies | ✅ 适配器就绪 |
| cve | ✅ 适配器就绪 |
| security | ✅ 适配器就绪 |
| build | ✅ 适配器就绪 |

## v1 语言群（4 种主流语言）

| 语言 | Grammar 精度 | 联合路径 | 文档注释 | 六类别 | v1 资格 |
|---|---|---|---|---|---|
| **Java** | ✅ Wilson 0.9849 | ✅ 9 测试 | ✅ 8 测试 | ✅ | **v1-candidate** |
| **Python** | ✅ Wilson 0.9842 | ✅ 9 测试 | — | ✅ | **v1-candidate** |
| **TypeScript** | ✅ Wilson 0.9860 | ✅ 9 测试 | — | ✅ | **v1-candidate** |
| **Rust** | ✅ Wilson 0.9860 | ✅ 9 测试 | — | ✅ | **v1-candidate** |

## 证据清单

| 证据 | 文件 |
|---|---|
| Grammar 精度验证 | `tests/acceptance/grammar-precision-final-summary-2026-10-07.md` |
| 联合路径验收 | `tests/acceptance/v1-language-group-combined-path.md` |
| 文档注释验收 | `tests/acceptance/java-documentation-acceptance.md` |
| 验收映射 | `tests/acceptance/evidence/production-acceptance-plan/all.json` |
| 机器核对 | `crates/codeguard-cli/tests/registry_machine_verification.rs` |

## v1 与完整生产资格的区别

| 维度 | v1-candidate | 完整生产资格 |
|---|---|---|
| Grammar 精度 | ✅ Wilson ≥ 0.98 | ✅ Wilson ≥ 0.98 |
| 版本/方言验收 | 已披露限制 | 正式验收 |
| 原生工具集成 | 候选路由 | 可信闭环 |
| 宿主接入 | 未完成 | Codex/ZCode/Kimi/Gemini |
| 发行包 | 未完成 | 多平台构建 |
| 资格状态 | v1-candidate | qualified |

## 结论

v1 质量门禁已为 Java/Python/TypeScript/Rust 四种主流语言建立了：
1. **可验证的语法检查**（17 种语言 grammar 精度 PASS）
2. **可复现的联合路径**（36 测试全链路）
3. **独立精度证据**（Wilson ≥ 0.98，零假阳性）
4. **六类别覆盖**（lint/comments/dependencies/cve/security/build）

这是「按分版本逐步推进」策略的第一个版本级交付物。
完整生产资格需宿主接入、发行包和可信闭环，属后续版本。
