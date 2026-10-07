# Section 8 适配器完整汇总

> 日期：2026-10-07；OpenSpec 8.x。

## 已实现适配器（12 种，6 种语言）

| 语言 | lint/comments | dependencies/CVE/security/build | 测试 | 状态 |
|---|---|---|---|---|
| **PHP** | check_php_scan | php_dependency_scan | 6/6 | ✅ |
| **Scala** | check_scala_scan | scala_dependency_scan | 6/6 | ✅ |
| **Elixir** | check_elixir_scan | elixir_dependency_scan | 6/6 | ✅ |
| **Lua** | check_lua_scan | lua_dependency_scan | 5/5 | ✅ |
| **Dart** | check_dart_scan | dart_dependency_scan | 5/5 | ✅ |
| **R** | check_r_scan | r_dependency_scan | 5/5 | ✅ |

## 适配器功能

每种语言提供 2 个适配器，共 6 个类别全覆盖：
1. **lint/comments 适配器**：observe/refresh/prefers/applicability_profile
2. **dependency 适配器**：observe/refresh/prefers/applicability_profile（四类别）

## 统计

| 指标 | 值 |
|---|---|
| 已实现适配器 | 12 种 |
| 覆盖语言 | 6 种 |
| 测试总数 | 33/33 通过 |
| 适用性档案 | 5 种（Go/Ruby/PHP/Scala/Elixir） |
| Grammar 精度验证 | 17 种语言 |

## 结论

Section 8 适配器已完成 6 种语言的全部 6 个类别。
适配器模式已固化，可复制到剩余 40+ 语言。
