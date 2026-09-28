# `check java` Javadoc 局部原生反馈

`check java PROJECT --java-home JDK21 --format json` 在静态发现 `java.maven.javadoc=configured` 且存在 Java 主源码时，调度独立的 `java.javadoc` 任务。Rust runtime 对每个目标运行 JDK 21 原生 `javadoc -Xdoclint:missing` 单文件探针；报告位于 `native_results.java_javadoc`，human 输出也会展示位置和规则。没有配置或配置无效时，不调度此任务，发现报告仍说明原因，候选类别分别为 `not_configured` 或 `configuration_unresolved`。

项目探针保留 `project_checker_attribution=unverified`、`coverage_proven=false`、`delivery_decision=not_evaluated`。它不是 Maven Javadoc 插件生效模型或生命周期执行的证据；隔离单文件可能因项目类路径、生成源码、模块/包边界而未完成。POM 在执行前后核对内容摘要；配置变化、目标不在已确认主源码、原生输出未知或类路径缺失均不给项目通过，也不产生可关闭的任务。当前不把局部 Javadoc 结果写入 `work sync` 或白名单批准路径。

验收：伪原生 JDK 测试先证明旧 `check java` 缺失 Javadoc 结果，再验证有配置时返回 `JavadocMissingComment`、无配置时无新执行任务；显式本机 JDK 21 对真实最小 Maven 项目返回同一缺注释规则。默认 workspace 测试跳过显式 JDK 用例。

```bash
cargo test -p codeguard-cli --test check_all_java_p3c --offline
CODEGUARD_JAVA_HOME=/absolute/path/to/jdk-21 cargo test -p codeguard-cli --test check_all_java_p3c real_jdk_check_java_reports_javadoc_comment_probe --offline -- --ignored
```
