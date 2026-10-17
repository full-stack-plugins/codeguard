# JavaScript 原生差分与项目绑定规则对齐

日期：2026-10-06。基础提交 e328f62，延续 OpenSpec 12.11/14.17/14.19；父任务保持未完成。

此前开发差分调用通用 worker，组合指标未计入项目已经使用的重复直接 let/const 绑定规则。本次 JavaScript 差分改用项目绑定候选 worker：原始 ERROR/MISSING 分类保持独立，结构事实只影响组合分类；其他语言仍使用原入口。新报告 0.9.0 要求选中 JavaScript，并按语言及固定规则摘要约束结构证据，旧协议文件不变。

受控回归先因目标行为缺失失败（原始 false_negative，组合仍 false_negative），实现后通过。初始测试夹具只保留 JavaScript，导致全语言库存范围校验失败；改为保留未选择语言库存，未弱化产品校验。四个相关目标合计15通过/0失败/6条件忽略；其中真实 Node 条件目标另显式执行1通过/0失败/0忽略。Node24.18.0 的18个样例：原始5TP/11TN/0FP/2FN；组合6TP/11TN/0FP/1FN。duplicate_binding 组合成为 true_positive，module_return 仍为 false_negative。此为额外 module 语料，不改写固定358例统计。

实际报告通过0.9 schema；错误规则摘要、跨语言结构和伪造资格被拒绝，旧0.4消费者拒绝新版本。严格 WASM CLI all-targets Clippy 初次发现版本表达式的重复条件，移除后退出0；不隐藏初次失败日志。报告/输入使用新文件归档，原历史文件保持。日志摘要及输入/报告摘要见 [证据](evidence/javascript-native-project-binding-2026-10-06.json)。

这次修正评测入口，没有新增 grammar 资产或宣告语言通过。明确 module 的顶层 return 仍需带模式条件的方案，不能无条件将 CommonJS 或未知项目模式判错。独立 holdout=false、正式资格0/32；项目全量原生能力、可信闭环、平台/宿主/发行及远端 CI 仍开放。
