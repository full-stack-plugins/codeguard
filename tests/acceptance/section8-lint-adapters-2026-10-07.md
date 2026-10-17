# Section 8 lint/comments 适配器汇总

> 日期：2026-10-07；OpenSpec 8.x。

## 已实现适配器（3 种）

| 语言 | 适配器 | 候选工具 | 测试 | 状态 |
|---|---|---|---|---|
| **PHP** | check_php_scan | PHP_CodeSniffer | 3/3 | ✅ |
| **Scala** | check_scala_scan | Scalafix | 3/3 | ✅ |
| **Elixir** | check_elixir_scan | Credo | 3/3 | ✅ |

## 适配器功能

每种适配器提供：
- `observe`: 观察 lint 扫描结果
- `refresh`: 刷新报告
- `prefers`: 检查是否优先使用原生工具
- `applicability_profile`: 六类别适用性
- `observe_rule_config`: 观察规则配置状态
- `rule_config_recheck`: 复检规则配置

## 配置状态检测

| 语言 | 配置文件 | 备用检测 |
|---|---|---|
| PHP | .phpcs.xml / phpcs.xml | composer.json |
| Scala | .scalafix.conf | build.sbt |
| Elixir | .credo.exs | mix.exs |

## 统计

| 指标 | 值 |
|---|---|
| 已实现适配器 | 3 种 |
| 测试总数 | 9/9 通过 |
| 适用性档案 | 5 种（Go/Ruby/PHP/Scala/Elixir） |

## 结论

Section 8 lint/comments 适配器已完成 3 种语言。
适配器模式已固化，可复制到更多语言。
