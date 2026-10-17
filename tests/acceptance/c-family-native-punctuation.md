# C11/C++17原生差分标点与扩展边界

对应14.17/14.19、15.2/15.7，沿用原生Clang原上下文观察，完整源码/规则/UTF-8位置和warning都保留，不改变公开lint0.2诊断或门禁。只改变开发期native/WASM语法统计的已审计分类策略。

旧expression-only策略将缺括号/分号/花括号等真实解析错误全部unknown。原生分类契约先RED，再逐条核对固定LLVM21定义与实际AppleClang21报告，新增七个精确规则ID（包含原缺表达式）；不使用err_expected前缀宽匹配，不把类型/名称/未知诊断归为语法无效。定义参考：[固定Parse诊断](https://github.com/llvm/llvm-project/blob/llvmorg-21.1.0/clang/include/clang/Basic/DiagnosticParseKinds.td)。

同时收紧警告：仅unused-variable/parameter的已审计诊断可保持valid；其它或混合扩展警告保持unknown。实际相同缺字段分号在C11是ext_expected_semi_decl_list警告、C++17是err_expected_semi_decl_list错误，分开统计，不将警告当作严格标准支持证明。未知类型与预处理上下文继续unknown；完整语言标准/扩展策略、全部解析诊断及独立holdout仍缺。

新native_grammar_differential0.12/clang-audited-punctuation-v1，历史0.11协议与12例报告原字节保留。26个开发回归在已有Clang/隔离worker实际执行，逐来源/工具/grammar/program身份、原规则及unknown分母保留；证据evidence/c-family-native-punctuation/report.json。不签发资格、可信任务关闭或新CLI入口。

CI源码依赖的独立问题与待确认动作见[远端可达性核验](ci-source-reachability.md)。完整57语言四核心/32grammar、平台/宿主/性能和修复闭环继续开放，父任务不勾选。

实际结果：C13例7TP/0FP/0FN/3TN/3unknown；C++13例8TP/0FP/0FN/3TN/2unknown。五个unknown全部保留在分母；与实际原生标签可比较的21例没有差异，仅开发回归。474schema定义、新实际报告及六类资格/标准/扩展/语义/宽前缀/旧协议伪造拒绝，旧0.11消费者拒绝新反馈。
