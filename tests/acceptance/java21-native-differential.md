# Java 原生差分截断判定与 Java21 增量样本

对应 introduce-rust-codeguard-cli 的14.17/14.19及12.11局部验收。Java差分测试此前只按recoveries是否为空判定合法，可能把隐藏/截断恢复误计为TN。测试分类器现在先读取truncated_files，非零保留unknown，不将空恢复转为干净；反例验证unknown结果。这是验收分类修正，产品仍保持未完成，不改grammar或发行资格。

显式使用本机Microsoft JDK21.0.12.1的javac，原13例语料继续通过原生对照（8合法、5语法错误，--release17）；新增独立登记的Java21版本组8例（--release21）包含模式switch、when guard、record pattern、sealed record、text block五个合法样本，以及缺guard表达式、缺record参数类型、缺switch箭头三个语法错误。实际javac与WASM可判定结果全部一致，新增组3TP/5TN/0FP/0FN/0unknown。两组不与固定358例或其它工具指标混合。

新增组由本轮作者编写，independent_holdout=false，不是独立第三方盲测。原生编译器字节在运行前后核对，报告绑定版本及摘要，每例绑定源码摘要和原grammar报告。不下载工具。新增组不含注解处理器或外部依赖，javac禁止注解处理；结果不覆盖完整Java语义或全部语言版本。

```bash
CODEGUARD_JAVAC_BIN=/absolute/jdk21/bin/javac CODEGUARD_JAVA21_REPORT=/private/local-report.json cargo test --offline --locked -p codeguard-cli --features wasm-precheck --test java_native_differential -- --include-ignored --test-threads=1
```

整个Java差分目标4项通过（基础worker、原13例原生、隐藏恢复分类、新8例原生）。没有运行用户Erlang草稿或全WASM套件，没有重放358例或重建grammar。所有32 grammar资格仍未完成，既有六项争议和独立holdout/真实宿主/多平台继续开放。
