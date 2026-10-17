# P0-B 可信关闭/复发重开进展汇总

> 日期：2026-10-07；OpenSpec 9.10 / 15.6。

## 各语言可信关闭测试状态

| 语言 | 测试数 | 通过 | 忽略 | 工具 | 状态 |
|---|---|---|---|---|---|
| **Python** | 10 | **9** | 0 | Ruff 0.16.8 | ✅ 核心机制已验证 |
| **Rust** | 9 | **7** | 2 | Rustfmt 1.9.0 | ✅ 核心机制已验证 |
| Java | — | — | — | JDK+Maven | ⬜ 待集成 |
| TypeScript | — | — | — | ESLint | ⬜ 待集成 |
| JavaScript | — | — | — | ESLint | ⬜ 待集成 |

## Python 已验证的可信关闭行为（9/10）

1. noqa 不等于修复 ✅
2. 禁用规则不等于修复 ✅
3. 原工具复检清除待处理尝试但不关闭任务 ✅
4. per-file ignore 需要审查 ✅
5. 借用租约验证 ✅
6. 源码变化后重新可操作 ✅
7. 原始发现缺失仅是候选 ✅
8. D100 稳定任务与复检 ✅
9. D101 任务解释与复检 ✅

## Rust 已验证的可信关闭行为（7/9）

1. 签名配对关闭幂等、原生复发重开 ✅
2. WASM-first 任务确认/解决/重开（绑定原始 grammar）✅
3. 变更 edition 和伪造上下文不能关闭 ✅
4. 信任失败被拒绝（无执行/事件）✅
5. WASM-first grammar 来源和 edition 在原生执行前绑定 ✅
6. WASM-first 原生反例需要 grammar 审查 ✅
7. 原生反例和中途上下文变更永不解决 ✅

## 发现的功能缺口

**next 命令任务选择**：verify 后 `next` 应优先显示新发现（actionable），
但当前总是优先显示 blocker。需要上下文感知排序：
- 环境正常时优先 finding
- 环境退化时优先 blocker

## 结论

Python 和 Rust 的可信关闭核心机制已验证。
剩余工作：
1. 修复 next 任务选择缺口
2. 复制流程到 Java/TypeScript/JavaScript
3. Java 原生工具集成（需 JDK 环境）
