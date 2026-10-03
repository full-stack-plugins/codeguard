# Java 17 与 javac 21 的窄范围语法差分验收

日期：2026-10-03。对应 OpenSpec 14.4、14.17、14.19 的局部证据，不代表 Java 语法能力已验收或发行。

固定 CodeGuard 清单中的 Java WASM（SHA-256 `181a6fbc34d7864a551d91c13882fc33007e923b3c81a4bdbe7fa47492090077`）由公开 `grammar probe java` 隔离 worker 解析同一组 13 份源码：8 份合法类、接口、枚举、泛型、record 和 lambda，5 份缺括号、分号、参数类型、表达式或未闭合字符串的破损源码。普通语料测试逐例要求恢复节点的有无与预期一致，仍要求退出码 3、`grammar_qualified=false`、`delivery_decision=not_evaluated`。

独立原生 oracle 使用本机现有 `/usr/bin/javac` 21.0.12.1，对每例运行 `--release 17 -proc:none -Xlint:none`，编译产物写入各自隔离目录；13/13 的原生接受/拒绝与候选 worker 分类一致，且源文件未被修改。常规候选测试 1/1、显式原生差分测试 1/1 通过。Linux CI 执行无需 javac 的普通语料回归；原生差分用例默认忽略，必须显式提供 `CODEGUARD_JAVAC_BIN` 且版本为 21.x 才运行。

此证据只覆盖 Java 17 的 13 个窄范围样例。其它语言版本、语义诊断、真实项目原生检查范围、系统误报/漏报率、`check all` 的 Java 原生优先路由、任务与宿主反馈、MSRV 及发行包精度均未验收；不能把一次差分一致说成完整 lint 或批准交付。
