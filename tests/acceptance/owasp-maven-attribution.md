# OWASP 与 Maven 依赖图精确归属候选

Rust 适配器只在同一 OWASP 漏洞观察含唯一可无损解释的 Maven PURL，且该 PURL 的 groupId、artifactId、version、type、classifier 与原生 Maven 依赖图中唯一非根节点完全相同时，返回 `ExactCoordinateCandidate` 和节点索引。它不靠文件名、漏洞描述或模糊版本推断。报告中原生抑制的漏洞仍保留 suppression 标志。

缺 PURL、未知限定符、URL 编码、私服 `repository_url`、不同 PURL 冲突、图中缺节点或同坐标多节点分别返回未归属状态。第二阶段从传入的本次 OWASP 报告和 Maven 图重新计算归属，再按节点索引比较 OWASP 原生 `sha256` 和独立观察的 Maven 仓库制品字节摘要；接口不接收可能来自旧报告的归属列表。缺任一摘要、摘要格式无效或字节不同都不会得到 `ExactDigestCandidate`。即使两个摘要一致，也尚未认证扫描报告、仓库来源或数据库身份，不能签发漏洞结论。

验证：`cargo test -p codeguard-adapters --test owasp_maven_attribution_contract --offline -q`，4 项通过。测试包含精确匹配、原生 suppression、错误版本/组、缺身份、私服限定符、冲突标识、type/classifier 差异、重复图坐标，以及相同坐标下的摘要一致/冲突/缺失/损坏。尚未接入真实 OWASP 执行或 `check java`，所以 OpenSpec 6.4/6.8 未完成。
