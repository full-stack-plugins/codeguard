# 七份 CodeGraph grammar 候选局部验收

日期：2026-09-29。对应 OpenSpec 14.1、14.2、14.4、14.19 的局部进展；完整任务均未完成。

固定 CodeGraph `1072f82ce24db3d133258d30165cef6b74d108b2` 中 CFML、CFQuery、CFScript、COBOL、Scala、Swift、VB.NET 七份 WASM，逐份与提交中的原始字节核对。许可证、上游提交或发行包及必要补丁分别见 `grammars/<language>/README.md`；清单和 Rust 适配器同时固定摘要、字节长度、ABI 与候选状态。`grammar status` 现列出 32 种来源、32 份可由 Rust worker 加载的候选、0 项已发行，仍以退出码 3 明示未完成。

Rust 离线加载实测 CFML/CFQuery/CFScript/Scala/Swift/VB.NET 为 ABI 15，COBOL 为 ABI 14；隔离 worker 对七种基本正例返回 `incomplete`、`grammar_qualified=false`。COBOL 曾被原 8 MiB 上限及小写导出名校验挡住：RED 用例失败为“无效的 grammar 名称”；将受限输入上限调整为 20 MiB、允许固定的 `COBOL` 导出名后，固定格式程序及 worker 正例通过。本机一次冷加载约 20 秒、峰值常驻内存约 458 MB，后续 worker 样例约 12 秒；这只是单机观察，不是生产快检预算或 Linux 资源验收。

验收暴露的精度缺口必须保留：CFQuery 将 `SELECT FROM` 解析为无恢复节点，因而不能代替 SQL 语法/语义检查；未闭合的 `#` 插值才触发当前反例。VB.NET 将合法的未缩进类方法体解析出 `MISSING ":"`，属于已知误报；本轮仅用缩进后的正例证明资产可加载。CFML 三种 grammar 之间的嵌入片段切换也没有在 CodeGuard 中实现。不能将上述样例通过或候选数等同 32 种语言的 lint 可用。

源码构建新增 `grammar probe <language> <file> --format=json`，显式调用同一隔离 worker；Python、TSX 干净样例与 Java 缺失括号样例的 CLI 测试均保持 `incomplete`、原生未运行、交付未评估。完整工作区测试日志包含 183 个结果段、1156 通过、0 失败、106 忽略；此次数值不把被忽略的原生工具验收算作通过。Clippy、默认构建检查、OpenSpec 严格验证及清单 JSON Schema 验证通过。

公开候选诊断补齐 [0.1.0 封闭报告 Schema](../../schemas/grammar-probe-v0.1.schema.json)：32 种语言枚举与资产清单由测试对齐；干净 Python 样例及缺失文件样例的真实输出均通过 Draft 2020-12 验证，伪造 `clean` 或额外字段被拒。输入失败也返回版本化 JSON 与退出码 3，不把缺失文件作为源码违规。此 Schema 只服务显式单文件诊断，仍不具备原生 lint 或交付权威。

剩余工作包括逐语言版本/方言独立语料、原生工具对照、误报漏报评估、原生优先统一 `lint/check` 路由、任务/对话反馈、COBOL 成本优化、发行包离线实装及宿主验证。未满足这些条件前，32 份候选只能提供显式诊断或局部受控观察，不能签发通过或关闭原生检查义务。
