# Kotlin 原生差分与隐藏错误覆盖

初始日期：2026-10-03；更新：2026-10-04。对应 OpenSpec 14.4、14.17、14.19 的精度阻塞，Kotlin 仍为未验收候选。

当前差分统计为 **11 例可判定且与原生一致、2 例隐藏错误导致的未知**。原生非法的缺类型与原生合法的 object 样例都被 Rust 标为扫描未完成；它们仍在 13 例总分母内，不按零诊断计作通过、一致、误报或漏报。清单与中英文主文档已纠正为此统计。下文 12/13 是扫描器完整性纠正前的历史记录。

## 历史测量和纠正证据

固定 CodeGuard Kotlin WASM（SHA-256 `c80c88867a589a1a0959bcea89de84b7e9684b3693b2cdb2944812458e62ff48`）经隔离 `grammar probe kotlin` 解析 13 份源码：8 份合法、5 份故意破损。本机现有 `kotlinc-jvm 2.4.10` 分别编译同一输入，8 份合法通过、5 份非法拒绝；WASM 与原生仅 12/13 一致。

分歧样例是 `fun f(x: ) = x`。原生编译器在参数冒号后报告 `Type expected` 与 `Incomplete code`；固定 WASM 返回零恢复节点。这是**漏检**，不能因候选零恢复而宣称语法有效或建议跳过原生工具。常规语料测试明确要求仅该样例保留分歧，显式原生测试再次确认 12/13 和源码未变；两项均各 1/1 通过。`grammar status` 与 `check all` 的 `known_limitations` 现在显示具体漏检，项目样例还断言该候选零恢复、整体交付仍为 `incomplete`。Linux CI 运行无需 `kotlinc` 的固定回归；原生对照默认忽略，需显式提供 `CODEGUARD_KOTLINC_BIN` 且版本为 2.4.10。

后续需要修复或更新固定 grammar，保留原始与派生字节、许可证和来源，再以同一反例及更多语言版本/方言语料复测；还须完成统一原生优先路由、误报/漏报率、资源预算及发行包验收。只更新已知限制不能消除漏检。

[Linux CI 运行](https://github.com/full-stack-plugins/codeguard/actions/runs/37115529650) 对提交 `1623cf9` 的 Kotlin 固定回归、项目已知限制、单项目 32/32、npm 包与完整工作区测试均通过。WASM 集成步骤中的候选测试文件 8/8 通过，用时约 174 秒；这不是单个 Kotlin 文件的性能测量。原生 kotlinc 对照仍只在上述本机显式执行。

后续定位发现更具体的扫描器缺口：旧 grammar 对上述缺类型样例及合法 `object C { val value = 1 }` 均令根节点 `has_error=true`，S-expression 含 `MISSING`，但 Rust 树遍历看不到该缺失 token，原扫描器返回零恢复。现已让这类无法定位的错误标记 `truncated=true`、初检 `incomplete`；项目 JSON 候选观察给出 `syntax_recovery_incomplete`，终端文本提示运行原生工具。差分测试也不再将零恢复自动解释为有效：13 例中这两例明确列为未解析，其余 11 例须与原生分类一致。运行时、隔离 CLI 和项目反馈均有回归；这只防止“零恢复”误导智能体，不等于修好 Kotlin grammar。`object` 是潜在误报，缺类型是漏检，两者均需原生确认。

评估了 [Kotlin LSP 随附 WASM](https://github.com/Kotlin/kotlin-lsp/tree/e89317e303708ad4d90ab4829a98ce63d8f9ec67/vscode-language-kotlin/grammars)（SHA-256 `7bf452758684c40d84304db3eb51ae4ad61f22465d6130aa964cb683e2fea5a0`，ABI 14；[来源说明](https://github.com/Kotlin/kotlin-lsp/blob/e89317e303708ad4d90ab4829a98ce63d8f9ec67/vscode-language-kotlin/grammars/README.md) 指向 exoego fork）。该版本在原 13 份窄语料上无分歧，但扩展样例中仍将原生可编译的 `fun interface`、嵌套类报错，且对原生拒绝的无名顶层函数未给出解析错误。以上三个分歧已由本机 `kotlinc-jvm 2.4.10` 确认。候选未替换仓内资产，也未进入发行；仍需独立、可重复的更大语料和 grammar 修复。
