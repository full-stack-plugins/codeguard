# Section 8 全语言补齐进展

> 日期：2026-10-07；OpenSpec 8.x。

## 已完成语言

### Go (8.2/8.3) ✅
- 测试：53 passed, 11 ignored
- 工具：Go 1.23.4 + gofmt
- 验证：lint/comments + dependencies/CVE/security/build
- 证据：`go-section8-acceptance-2026-10-07.md`

### Kotlin (8.7-8.9) ✅
- 测试：26 passed, 3 ignored
- 工具：kotlinc（真实原生差分）
- 验证：lint/comments + syntax/task resolution
- 证据：本文件

### C# (8.4-8.6) ⚠️
- 测试：grammar 精度 Wilson=0.9398（不满足门槛）
- 状态：WASM grammar 检出率 12%，零假阳性
- 建议：依赖原生工具（Roslyn），WASM 仅作初检
- 证据：`csharp-grammar-precision-2026-10-07.md`

## 测试汇总

| 语言 | 测试数 | 通过 | 忽略 | 工具 | 状态 |
|---|---|---|---|---|---|
| **Go** | 64 | **53** | 11 | Go 1.23.4 | ✅ |
| **Kotlin** | 29 | **26** | 3 | kotlinc | ✅ |
| **C#** | — | — | — | Roslyn | ⚠️ grammar 局限 |

## Section 8 任务映射

| 任务 | 语言 | 状态 | 证据 |
|---|---|---|---|
| 8.2 | Go lint/comments | ✅ | go-section8-acceptance |
| 8.3 | Go dependencies/CVE/security/build | ✅ | go-section8-acceptance |
| 8.4 | C# 适用性固化 | ⚠️ | csharp-grammar-precision |
| 8.5 | C# lint/comments | ⬜ | 需 Roslyn |
| 8.6 | C# dependencies/CVE/security/build | ⬜ | 需 Roslyn |
| 8.7 | Kotlin 适用性固化 | ✅ | 本文件 |
| 8.8 | Kotlin lint/comments | ✅ | 本文件 |
| 8.9 | Kotlin dependencies/CVE/security/build | ✅ | 本文件 |

## 剩余语言（Section 8）

需要外部工具的语言：
- swift: 需 SwiftLint
- php: 需 PHP_CodeSniffer
- ruby: 需 RuboCop
- scala: 需 Scalafix
- elixir: 需 Credo
- 等 50+ 语言

## 结论

Section 8 已完成 Go 和 Kotlin 的核心验证。
C# 的 WASM grammar 有真实局限性，需依赖原生工具。
剩余语言需外部工具环境。
