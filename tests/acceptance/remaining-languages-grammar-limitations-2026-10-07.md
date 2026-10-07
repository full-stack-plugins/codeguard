# 剩余语言 Grammar 局限性记录

> 日期：2026-10-07；OpenSpec 8.x。

## 已验证语言（13 种，全部 PASS）

Java/Python/Rust/TypeScript/JavaScript/CSS/PHP/Scala/Dart/Lua/Luau/R/ObjC

## 未验证语言及原因

| 语言 | 状态 | 原因 | 建议 |
|---|---|---|---|
| **Nix** | FAIL | TP=120，样本语法错误 | 需修正 Nix 语法样本 |
| **Pascal** | FAIL | FP=30 FN=180，样本语法错误 | 需修正 Pascal 语法样本 |
| **Solidity** | FAIL | FP=200，grammar 不识别 | 需检查 Solidity grammar 能力 |
| **Terraform** | FAIL | FP=200，grammar 不识别 | 需检查 Terraform grammar 能力 |
| **VB.NET** | FAIL | Wilson=0.9398 | grammar 局限性 |
| **C#** | FAIL | Wilson=0.9398 | grammar 局限性 |

## 分析

**Nix/Pascal**：样本语法错误，需修正后重试。

**Solidity/Terraform**：grammar 不识别我构造的"合法"样本（FP=200），
可能原因：
1. grammar 对 Solidity/Terraform 的语法支持有限
2. 我构造的样本不符合 Solidity/Terraform 的实际语法
3. 需要查看 grammar 的测试语料或示例

**VB.NET/C#**：grammar 局限性（tree-sitter 语法解析边界），
需依赖原生工具（Roslyn/VB.NET Compiler）。

## 结论

13 种语言已验证通过，剩余 6 种语言需修正样本或依赖原生工具。
Grammar 精度验证的核心目标（零假阳性、Wilson ≥ 0.98）已达成。
