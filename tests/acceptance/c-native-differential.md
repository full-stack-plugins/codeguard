# C grammar 与原生 Clang 的局部语法差分

日期：2026-10-03。对应 OpenSpec 14.4、14.17、14.19 的局部增量，不是 C lint 验收。

固定本机 Apple clang `21.0.0 (clang-2100.3.34.2)`，实际程序 `/Library/Developer/CommandLineTools/usr/bin/clang`，SHA-256 `1590ac950a3d627817d09ade5cb60b2115f17a72182a3141e010b4bcc482a0c9`。用 `clang -fsyntax-only -std=c11 -Wno-everything -x c FILE` 只读检查每份样例；CodeGuard 对同一源码调用公开 `grammar probe c FILE --format=json`，要求隔离 worker 返回候选观察、零授权和退出码 3。

共 13 例：8 份合法样例覆盖普通函数、指针、结构体、typedef、枚举、复合字面量、指定初始化与预处理宏；5 份破损样例覆盖缺失大括号、分号、初始化表达式、未闭合字符串与错误声明符。Clang 与候选的合法/破损分类全部一致。普通固定语料测试 1/1 通过；显式设置 `CODEGUARD_CLANG_BIN=/Library/Developer/CommandLineTools/usr/bin/clang` 运行忽略的真实原生差分测试 1/1 通过。测试核对 Clang 不修改源文件。

首次语料把 `int f(void) { return ; + 1; }` 误标为语法错误：Clang 因返回类型拒绝，但 Tree-sitter 对该语法不产生恢复节点。这是语义差异，不作为 grammar 漏检。改成 `int x = ;` 后两者同为语法拒绝。测试避免把编译器的类型/语义检查与语法恢复等同。

此对照只覆盖本机 Clang 版本及 C11 的 13 份小样例。`-fsyntax-only` 还会执行语义检查；语料标签已经人工核对为语法范围，但不代表 Clang 诊断均是 grammar 可检的内容。当前 grammar 仍 `grammar_qualified=false`，零恢复节点不表示原生 lint 或交付通过。尚无独立 holdout、系统误报/漏报率、跨平台版本、完整原生优先路由和宿主反馈，父任务不勾选。

## C11边界语料补充

当前语料扩展为19例，新增静态断言、泛型选择、变长数组、函数指针，以及破损静态断言/泛型选择。显式CODEGUARD_CLANG_BIN=/usr/bin/clang，offline/locked启用wasm-precheck并执行include-ignored：固定语料与实际原生差分2/2通过、无忽略；19例分类一致。此前13例为历史检查点。新增样例仍来自同一开发语料，不能替代独立holdout、误报率或全平台资格。编辑快检目前缺Clang标准上下文接线，此结果不证明自动调度完成。

逐例证据：evidence/c11-native-wasm-differential.json，由真实差分测试通过后写入；绑定测试源码、Clang二进制、Codeguard二进制和grammar清单SHA256，并保存19例输入摘要、原生退出码及WASM分类。标记independent_holdout=false与qualification=not_granted。可通过绝对CODEGUARD_C_DIFFERENTIAL_EVIDENCE路径重新采集。
