# C11/C++17统一原生与WASM差分回放

对应 OpenSpec 14.17/14.19、15.2/15.7，接入开发期 Rust `replay_native_corpus` API。这不是新增公开 CLI 命令，也不是生产资格签发。

明确选定 c/cpp 和已有 Apple Clang21 原入口，共享独立公开入口的冻结stdin、固定标准、禁默认配置/头文件、私有cwd、有界SARIF及工具连续性。新增 `observe_with_cancellation` 让版本与源码阶段消费回放请求的原取消令牌；原公开入口及0.2反馈保持。版本更换别名/制品必须在源码执行前停止；完整工具链闭包与check-to-spawn隔离仍未验收。

先由公开Rust回放测试复现 `native_grammar_language_unsupported`，再接入两个观察器。C11/C++17不是自动推断项目标准。[Clang参数](https://clang.llvm.org/docs/ClangCommandLineReference.html)规定的fsyntax-only仍会产生语义诊断，因此不能把全部非零退出解释为grammar语法漏报。本批分类策略 `clang-parse-expression-only-v1` 仅审计原生 `clang.err_expected_expression`；任何其它error（含与该规则混合）保留unknown。警告仍存原报告但不算语法错误。完整解析诊断规则、其它标准/版本、项目预处理和扩展仍缺，不能宣称完整原生语法验收。

新封闭协议 `native_grammar_differential` 0.11保存标准与策略，历史0.1–0.10保持原字节和选择语义。未选择语言仍保留32库存；没有工作台写入或可信任务关闭，未授予独立holdout。源码、tool、grammar及program摘要保持，unknown不移出样本分母。

实际已有Clang对C/C++各6例执行公开服务与隔离WASM worker：正常声明、unused警告、井号字面量、缺表达式、未知类型、预处理上下文。每语言1TP/0FP/0FN/3TN及2unknown；两个unknown明确是未审计语义错误和预处理上下文，不算通过。12例仅开发回归，不是独立精度语料。证据 `evidence/c-family-native-replay/report.json`。

5个原生观察器单元目标通过，包含11语言版本/扫描运行中取消、11语言别名/制品变更反例和Clang警告/语义混合边界。受控错版本回放保留12个unknown与32库存；显式实际Clang条件用例另执行。控制替身不计实际原生资格。

逐语言四核心、完整版本/方言与源集、项目原生优先/缺工具WASM、所有32 grammar的独立语料、性能/资源/平台/宿主及可信修复闭环保持未完成，父任务不勾选、正式grammar资格0/32。

收尾：默认公开/计划11通过/2条件忽略；WASM公开/新回放8通过/3忽略，原差分7通过/1忽略；5观察器单元及显式实际Clang条件1通过分别记录。473schema定义、472历史原字节、新报告/六种伪造/旧消费者拒绝，双配置严格Clippy、分层/格式/OpenSpec通过。CI37502132499语料来源检出失败、MSRV通过；新提交CI未验证。
