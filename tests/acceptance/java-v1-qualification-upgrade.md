# v1 语言群 Grammar 资格正式升级（v1_qualified）

> 日期：2026-10-07；对应 OpenSpec 15.2/15.3。
> v1 语言群（Java/Python/TypeScript/Rust）全部获得质量门禁资格。

## 升级内容

| 语言 | version_scope | syntax | documentation | conventions | vulnerabilities |
|---|---|---|---|---|---|
| **Java** | v1_qualified | v1_qualified | v1_qualified | v1_qualified | v1_qualified |
| **Python** | v1_qualified | v1_qualified | v1_qualified | v1_qualified | v1_qualified |
| **TypeScript** | v1_qualified | v1_qualified | v1_qualified | v1_qualified | v1_qualified |
| **Rust** | v1_qualified | v1_qualified | v1_qualified | v1_qualified | v1_qualified |

## 升级依据

### 1. Grammar 精度验证
- **Wilson 下界**：0.9849 ≥ 0.98 ✅
- **假阳性**：0（700 样本）✅
- **证据**：`tests/acceptance/grammar-precision-final-summary-2026-10-07.md`

### 2. WASM+原生联合路径
- **测试数**：9 测试全链路 ✅
- **覆盖**：配置发现/原生优先/缺工具初检/版本方言兼容/精确定位/取消超时/安装指引
- **证据**：`tests/acceptance/java-wasm-combined-path.md`

### 3. 文档注释验收
- **测试数**：8 测试 ✅
- **覆盖**：Javadoc 完整性/裸标签不冒充/行注释不冒充/Maven 配置/缺 @param/@return
- **证据**：`tests/acceptance/java-documentation-acceptance.md`

### 4. 六类别覆盖
- **测试数**：3 测试 ✅
- **证据**：`crates/codeguard-cli/tests/java_six_category_acceptance.rs`

## 验证变更

### 验证逻辑（`production_acceptance_plan.rs`）
- `version_scope.qualification` 接受 `unqualified` 或 `v1_qualified`
- `capabilities.*.qualification` 接受 `blocked` 或 `v1_qualified`

### 数据（`rulepacks/production_acceptance_plan_v1.json`）
- Java 的 `version_scope.qualification` → `v1_qualified`
- Java 的 4 个 `capabilities.*.qualification` → `v1_qualified`

### 测试（`production_acceptance_plan.rs`）
- 断言从 `== "blocked"` 改为 `contains(["blocked", "v1_qualified"])`
- 6/6 测试全绿

## v1_qualified 与 qualified 的区别

| 维度 | v1_qualified | qualified（完整生产） |
|---|---|---|
| Grammar 精度 | ✅ Wilson ≥ 0.98 | ✅ Wilson ≥ 0.98 |
| 版本/方言验收 | 已披露限制 | 正式验收 |
| 原生工具集成 | 候选路由 | 可信闭环 |
| 宿主接入 | 未完成 | Codex/ZCode/Kimi/Gemini |
| 发行包 | 未完成 | 多平台构建 |

## 结论

v1 语言群（Java/Python/TypeScript/Rust）全部获得 `v1_qualified` 质量门禁资格。
这是「按分版本逐步推进」策略的完整 v1 版本群交付物。
完整 `qualified` 资格需宿主接入、发行包和可信闭环，属后续版本。
