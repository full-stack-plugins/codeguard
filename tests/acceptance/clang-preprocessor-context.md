# C/C++ 独立原生入口预处理上下文精度修复

对应 OpenSpec 8.26/8.29、14.5/14.6、15.2。仅 Apple Clang 21.0.0 C11/C++17 显式单文件入口；不授予生产资格。

旧入口只检查源码是否包含井号，合法字符串、字符、注释与 C++ 原始字符串会被误标上下文未解析；同时漏过 `%:`、C11 trigraph 与续行组合。公开反例先失败后修复。Rust 有界扫描负责阻止未知预处理上下文，原生 Clang 仍负责诊断；原生失败不回退 WASM。

新扫描保留原始偏移，最多 1MiB；C11 trigraph、两语言续行、Clang 水平空白续行扩展、初始 BOM、注释与字面量分开处理。C++ 原始字符串内部必须用原字节找闭合，不能将内部的反斜杠换行先拼接。依据：[C++ 翻译阶段](https://eel.is/c%2B%2Bdraft/lex.phases)、[LLVM 21 Unicode 字符集](https://github.com/llvm/llvm-project/blob/llvmorg-21.1.0/clang/lib/Lex/UnicodeCharSets.h)。

未解析的行首非 ASCII 可能受编译器恢复影响，暂时保守保留上下文未知，不生成源码违规。实测 U+200B 是标识符而非空白，该保守限制仍可能导致未完成；没有宣称完整编译器词法兼容。后续应基于固定版本原生词法契约进一步减少此限制，不能将其当作已验收。

验证证据：6 个有界扫描单元用例；公开 CLI 7 通过、2 条原生条件默认忽略；显式已有 Clang 实际条件用例另 1 通过，生成 15 份公开报告。另 18 次直接原生 `#error` 合成样例验证 BOM、Unicode、空白续行和续行注释的实际行为，包含 U+200B 非指令反例。直接原生 oracle 与公开报告分别保存，不累计为独立精度样本。

- `evidence/clang-preprocessor-context/native.json`：公开工具反馈，沿用 syntax_lint_feedback 0.2。
- `evidence/clang-preprocessor-context/compiler-oracle.json`：仅合成源码与原生实验诊断，产品反馈不采用其自由文本。

项目编译数据库/include/宏、完整标准与方言、32 WASM 联合路径、独立精度、五平台、稳定任务可信关闭和宿主发行仍未完成。文档/开发规范/CVE 三核心不由此语法修复代替；父任务保持开放，正式 grammar 资格仍 0/32。
