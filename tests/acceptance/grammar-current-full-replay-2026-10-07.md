# 当前32种grammar原始解析回放

对应introduce-rust-codeguard-cli的12.11/14.17/14.19/15.2。现有358例、32语言、35来源组，输入显式绑定当前manifest；未修改历史语料标签，未执行原生oracle，不是独立holdout。

实际offline/locked启用wasm-precheck，显式执行grammar_evaluation/replay_expanded_cohorts_and_archive_current_evidence：1通过、0失败、0忽略，252.79秒。358例全部attempted，program_stable=true，manifest摘要独立重算相符；原始报告通过0.2 schema。

|原始分类|数量|
|---|---:|
|TP|73|
|FP|1|
|FN|10|
|TN|269|
|unknown|3|
|pending|2|

仅盘点计数，不汇总跨来源准确率。结果incomplete/fails_fixture_threshold、grammar_qualified_count=0，不能因为测试退出0宣称语法质量通过。Erlang十项原始漏检仍存在；历史组合结构规则另有测量，不能混为原始解析通过。VB.NET未缩进方法为回归标签下的疑似误报；Kotlin缺参数类型/object和Swift缺参数类型为unknown；CFQuery/COBOL待裁定。

[本轮完整报告](evidence/grammar-current-manifest-2026-10-07.json)保存程序/清单/语料/逐例源码及grammar摘要和分来源混淆分母。程序SHA256为b48bb60b1dcb48b29c8d016033f8eb75641628ecd8997123a09b1bee0a6d3457。报告对应当前本机开发构建，不证明平台、宿主、发行、完整版本范围、原生对照或生产门禁。
