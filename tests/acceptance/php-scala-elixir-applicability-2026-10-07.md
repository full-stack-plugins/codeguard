# PHP/Scala/Elixir 六类别适用性固化

> 日期：2026-10-07；OpenSpec 8.13/8.19/8.22。

## 适用性档案

### PHP (8.13) ✅
| 类别 | 候选工具 | 状态 | 来源 |
|---|---|---|---|
| lint | PHP_CodeSniffer (PSR12) | gap | squizlabs/PHP_CodeSniffer |
| comments | PHP_CodeSniffer (Commenting) | gap | squizlabs/PHP_CodeSniffer |
| dependencies | composer show --direct | gap | getcomposer.org |
| cve | composer audit | gap | Roave/SecurityAdvisories |
| security | Psalm | gap | psalm.dev |
| build | composer install --dry-run | gap | getcomposer.org |

### Scala (8.19) ✅
| 类别 | 候选工具 | 状态 | 来源 |
|---|---|---|---|
| lint | Scalafix | gap | scalacenter.github.io/scalafix |
| comments | Scalafix (DisableSyntax) | gap | scalacenter.github.io/scalafix |
| dependencies | sbt-dependency-check | gap | github.com/sbt/sbt-dependency-check |
| cve | sbt-dependency-check | gap | github.com/sbt/sbt-dependency-check |
| security | sbt-dependency-check | gap | github.com/sbt/sbt-dependency-check |
| build | sbt compile | gap | scala-sbt.org |

### Elixir (8.22) ✅
| 类别 | 候选工具 | 状态 | 来源 |
|---|---|---|---|
| lint | Credo | gap | hexdocs.pm/credo |
| comments | Credo (Readability) | gap | hexdocs.pm/credo |
| dependencies | mix deps.tree | gap | hexdocs.pm/mix |
| cve | mix deps.audit | gap | hexdocs.pm/mix |
| security | mix deps.audit | gap | hexdocs.pm/mix |
| build | mix compile --warnings-as-errors | gap | hexdocs.pm/mix |

## 档案文件

- `rulepacks/php_static_candidate_v1.json`
- `rulepacks/scala_static_candidate_v1.json`
- `rulepacks/elixir_static_candidate_v1.json`

## 结论

三种语言的六类别适用性已固化，全部标记为 `applicable/gap`。
候选工具、方言、版本和缺口已记录，为后续原生工具集成奠定基础。
