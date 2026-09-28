# 语料与 oracle 基线（2026-09-24）

目标：OpenSpec `introduce-rust-codeguard-cli` 1.4。`schemas/corpus-case.schema.json` 定义样本来源、输入摘要、工具/运行时身份、规则依据、预期完整性和 finding、脱敏复核及裁定状态。`tests/fixtures/corpus/oracle.json` 是唯一语料索引；`accepted`、`pending_reproduction`、`disputed` 分开统计，未复现历史日志不进入已接受评测。

当前索引含 5 条 accepted、3 条 pending_reproduction、0 条 disputed：

- Ruff 0.16.8 的 F401 正例、无 finding 反例，以及冻结的真实 `codeguard-plugin@9e4adb1` 源码负例。使用 `--isolated --select F401 --output-format json`，核对二进制 SHA-256 与每份源码 SHA-256。
- Maven 3.9.16/JDK 26.0.1 对最小 POM 4.1 项目实际返回非零和 modelVersion 工具链错误；预期为 `incomplete`、零代码 finding。Maven 启动脚本、JDK 二进制和项目树均有摘要。
- 另一个最小 Java 项目在离线 `mvn -B -DskipTests verify` 下真实 `BUILD SUCCESS`，日志只有资源、编译、测试跳过和打包，没有 P3C/Javadoc 目标；对应质量义务仍未证明，构成旧 `verify=质量通过` 推断的假通过样本。
- 三条来自旧插件历史回归的 Maven/Javadoc/ShellCheck 诊断被脱敏保留；由于原始工具版本或项目源码身份缺失，标记 `pending_reproduction`，不作为精度统计的真值。

执行证据：`cargo run --offline -p codeguard-cli --example validate_corpus -- --verify-ruff /opt/anaconda3/bin/ruff --verify-maven /opt/homebrew/bin/mvn` 返回 `accepted=5,pending_reproduction=3,disputed=0`；`cargo test --offline -p codeguard-cli --test corpus_integrity` 对摘要篡改、未锁工具身份和分类隔离做反例检查。独立 holdout 和统计阈值冻结属于 12.2，尚未完成。
