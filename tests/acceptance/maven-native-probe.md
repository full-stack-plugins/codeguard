# Maven 原生调用局部验收（2026-09-24）

Rust `maven_probe` 使用统一 `ProcessSpec` 和私有原子日志执行 Maven 原生 `--version` 与 `-B -o -f <pom> validate`。版本探测和模型验证共用一个截止时间，Maven 启动脚本及 POM 在执行前后均复核；从工具锁构造时还要求 JDK 启动文件和 Maven bundle 目录树摘要在各阶段保持一致。真实 Maven 启动脚本默认读取 `mavenrc`，探测强制 `MAVEN_SKIP_RC=1`，缺失即启动前 incomplete。原生退出码保留为证据；非零、超时、日志失败、身份变化均不能变成源码违规或质量通过。此 probe 不由 `detect`、`plan`、`doctor` 隐式启动，调用 Maven 可能运行项目构建逻辑，只能用于明确授权的执行阶段。

运行 `cargo test -p codeguard-cli --test maven_probe_contract --offline`：工具摘要不匹配在执行前被拒绝，版本不匹配不会进入 validate，原生非零保留为 incomplete 并写私有证据。运行 `CODEGUARD_MAVEN_BIN=/opt/homebrew/Cellar/maven/3.9.16/bin/mvn cargo test -p codeguard-cli --test maven_probe_contract --offline -- --ignored`：Maven 3.9.16/JDK 26.0.1 对 POM 4.1 样本真实返回非零并保持 incomplete；对没有质量任务绑定的普通 POM 真实完成 validate。两例均在临时副本执行，不修改语料源文件。

`NativeValidateComplete` 只证明此模型阶段的原生命令执行证据。另有真实锁制品构造试验以测试现场生成的入口/JDK/libexec 目录树摘要核对了原生执行链；它不证明锁的批准来源、JDK 动态依赖或项目扩展已完整锁定，也不证明 P3C、Checkstyle/Javadoc、CVE、安全或完整 build 已运行。Maven/Gradle/JDK/wrapper/profile/module/source-set 的全面观察、官方 P3C 兼容组合与原生报告解析仍由 OpenSpec 6.1–6.7 和 5.9 跟踪；本局部证据不勾选这些任务。

## 版本阶段失败类别

Maven `--version` 阶段不再把所有异常终止压为 `version_execution_incomplete`。Rust 保留超时、启动前预算耗尽、无法启动、取消、输出超限、信号终止、无效请求、读写管道失败、清理失败、不支持平台和原生非零退出的独立原因。均保持 incomplete，不启动 validate，不推导源码违规。新增短预算 shell 睡眠与缺可执行权限两项反例先失败，再通过；普通 Maven 契约共 9 项，连续五轮通过（各忽略 3 项需显式本机工具的验收）。

此前全量回归有一次 bundle mutation 用例在版本阶段提前失败；具体该次进程终止原因已无法恢复，不能仅因单独重跑通过而声称根因解决。新类别使后续复现可定位，测试预算和产品执行限制未放宽。
