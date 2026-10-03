# CFQuery 候选路由的标记边界纠错

日期：2026-10-03。对应既有 OpenSpec 14.4、14.17、14.19 的局部误报治理，完整任务仍未完成。

旧路由直接在 `.cfm/.cfc` 源码字节中搜索 `<cfquery>`，所以被服务器移除的 CFML 注释内示例标签也会变成候选；真实 `check all` 可能把其中破损插值回显为源码疑似问题。RED 用例中 HTML 注释、CFML 注释与正文各一段查询产生 3 条路由，正确预期是保留 HTML 注释与正文中的 2 条。另一个 RED 用例表明 `<cfquery name="a>b">` 的属性值会让旧逻辑在引号内的 `>` 截断标签，污染查询体字节与位置。

[Adobe 官方注释说明](https://helpx.adobe.com/coldfusion/developing-applications/the-cfml-programming-language/elements-of-cfml/comments.html)明确 CFML 注释使用三条短横线、在服务端移除、允许嵌套，也允许置于开标签内；普通 HTML 注释不会提供同等的 CFML 注释语义。因此路由现仅跳过闭合且可嵌套的 CFML 注释，并保留 HTML 注释中的 CFML 标签。普通标签属性和明确的 CFScript 体里的假标签不单独路由；开标签的 `>` 按引号及嵌套 CFML 注释边界定位。未闭合 CFML 注释或不明确的标记尾部不猜测后续 CFQuery。真实标签的原始字节切片与偏移不变，仍调用固定 CFQuery worker 且保持未验收状态。

验收：`grammar_route` 5/5 通过，覆盖嵌套 CFML 注释被跳过、HTML 注释中的查询被保留、CFScript 字符串和属性值中的假标签、开标签内嵌套注释，以及真实标签的字节偏移；`check_all_grammar_candidates` 4/4 通过，包括实际 worker 路径中 HTML 注释加正文报告 2 条 CFQuery 候选、CFML 注释中的查询不生成观察，以及四个有界项目仍调用全部 32 份候选。受影响测试的特性 Clippy `-D warnings`、格式、OpenSpec strict 与空白检查通过。

这不是完整 CFML 词法或 SQL 语义解析。CFQuery grammar 对 `SELECT FROM` 的已知漏检仍在，VB.NET 的已知误报也未改变；没有原生 ColdFusion/SQL 对照和误报率统计，因此 S14.4、14.17、14.19 保持未完成。
