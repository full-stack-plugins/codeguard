# JDK Javadoc 局部检查验收

命令：`codeguard lint java FILE --checker javadoc --java-home ABS_PATH --format json`。Rust runtime 在私有目录复制单个 UTF-8 Java 文件，调用指定 JDK 21 的原生 `javadoc -Xdoclint:missing -quiet`，只解析已知英文诊断，并将位置映射回原文件。源码、JDK 入口和 `release` 文件在执行前后核对；未知输出、无文档产物、进程失败、类路径不完整都返回 `incomplete`，不给源码 finding。

单文件诊断始终返回退出码 3、`delivery_decision=not_evaluated`、`coverage_proven=false`。有诊断表示 `findings_observed_untrusted`；无诊断只表示 `clean_scope_unproven`。此切片没有绑定项目实际 Javadoc 配置、Maven/Gradle 模型、完整源集、批准的工具与规则身份，也没有接入 `check java`、任务同步和门禁。

本地测试包括解析器未知规则/越界反例、CLI 参数错误和伪原生工具反例。显式 JDK 21 验收覆盖缺少类注释、缺少参数/返回标签、已记录的类与构造器、`{@inheritDoc}`、record 参数注释，以及 Lombok 类路径缺失时的未完成结果。运行：

```bash
cargo test -p codeguard-adapters --test javadoc_output_contract --offline
cargo test -p codeguard-cli --test java_javadoc_cli --offline
CODEGUARD_JAVA_HOME=/absolute/path/to/jdk-21 cargo test -p codeguard-cli --test java_javadoc_cli --offline -- --ignored
```
