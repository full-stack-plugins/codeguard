# Maven Javadoc 原生输出与复检边界

本机 Maven 3.9.16、JDK 21、离线缓存的 Maven Javadoc Plugin 3.12.0，在独立临时 Maven 项目中运行插件的原生 `javadoc` goal。源码 `public class Bad {}` 有缺类注释和默认构造器注释两条真实警告。`failOnWarnings=false` 时 Maven 输出 `BUILD SUCCESS` 但仍有两条 `[WARNING]` 诊断；`failOnWarnings=true` 时输出 `BUILD FAILURE` 与同一组诊断。可复跑的显式原生测试见 `crates/codeguard-adapters/tests/maven_javadoc_output_contract.rs`。

Rust 解析器只接受已知插件版本、单一 goal 标记、完整警告块/数量、与本轮源码字节匹配的位置和已知 JDK 规则。失败退出必须只含 Javadoc 警告失败签名；额外错误、未知规则、源码行不符和异常计数均返回 `Incomplete` 且不保留部分 finding。它只解析原生输出，还不是项目执行器或门禁。

同一临时项目目录连续执行时，第二轮可能复用 `target/site/apidocs` 而不重放警告。更关键的是，已完整注释的项目首轮与同目录重跑均可返回 `BUILD SUCCESS`，且日志没有可区分“重新生成”和“复用旧产物”的标记。真实测试分别覆盖独立目录中的警告成功/失败，以及同目录的干净首轮/重跑。解析器对此只返回 `CleanLogUnverified`、零诊断和 `javadoc_fresh_execution_unverified`，不能据此关闭任务或签发项目通过；未知警告与失败日志仍为 `Incomplete`。正式执行器仍需隔离快照、验证本轮目标/产物与配置身份。

```bash
cargo test -p codeguard-adapters --test maven_javadoc_output_contract --offline
CODEGUARD_MAVEN_BIN=/absolute/path/to/mvn CODEGUARD_JAVA_HOME=/absolute/path/to/jdk-21 cargo test -p codeguard-adapters --test maven_javadoc_output_contract --offline -- --ignored
```
