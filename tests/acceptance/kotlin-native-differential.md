# Kotlin 原生差分发现固定 grammar 漏检

日期：2026-10-03。对应 OpenSpec 14.4、14.17、14.19 的精度阻塞，Kotlin 仍为未验收候选。

固定 CodeGuard Kotlin WASM（SHA-256 `c80c88867a589a1a0959bcea89de84b7e9684b3693b2cdb2944812458e62ff48`）经隔离 `grammar probe kotlin` 解析 13 份源码：8 份合法、5 份故意破损。本机现有 `kotlinc-jvm 2.4.10` 分别编译同一输入，8 份合法通过、5 份非法拒绝；WASM 与原生仅 12/13 一致。

分歧样例是 `fun f(x: ) = x`。原生编译器在参数冒号后报告 `Type expected` 与 `Incomplete code`；固定 WASM 返回零恢复节点。这是**漏检**，不能因候选零恢复而宣称语法有效或建议跳过原生工具。常规语料测试明确要求仅该样例保留分歧，显式原生测试再次确认 12/13 和源码未变；两项均各 1/1 通过。`grammar status` 与 `check all` 的 `known_limitations` 现在显示具体漏检，项目样例还断言该候选零恢复、整体交付仍为 `incomplete`。Linux CI 运行无需 `kotlinc` 的固定回归；原生对照默认忽略，需显式提供 `CODEGUARD_KOTLINC_BIN` 且版本为 2.4.10。

后续需要修复或更新固定 grammar，保留原始与派生字节、许可证和来源，再以同一反例及更多语言版本/方言语料复测；还须完成统一原生优先路由、误报/漏报率、资源预算及发行包验收。只更新已知限制不能消除漏检。
