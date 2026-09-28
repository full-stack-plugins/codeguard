# Maven 检查器配置探测基线（2026-09-24）

`codeguard detect <path> --format=json` 现在在只读项目观察中返回 `checker_configurations`；发现报告协议为 `0.3.0`。Maven POM 使用 Rust `roxmltree` 有界解析，识别直接声明的 PMD、Checkstyle、Javadoc、Maven Dependency、OWASP Dependency-Check，以及显式依赖 FindSecBugs 的 SpotBugs。各条返回构建根、配置来源、`configured/missing/invalid/unknown`、原因与下一步。此查询不运行 Maven，也不表示检查通过。

后续增加独立 `java.maven.p3c` 项，避免把普通 Maven PMD 插件声明误认为阿里 P3C。当前静态 `configured` 仅在直接 PMD 插件依赖明确使用 `com.alibaba.p3c:p3c-pmd:2.1.1`、规则集引用该制品内已观察的 `rulesets/java/ali-*.xml` 路径、并显式设置 `skipPmdError=false` 时成立；通用 PMD 仍由 `java.maven.pmd` 单独报告。P3C 版本或规则路径动态、错误处理政策不明时为 `unknown`；缺 P3C 制品或规则集为 `missing`；明确跳过或吞掉 PMD 处理错误为 `invalid`。`configured` 只证明 POM 中的静态声明，不证明规则实际加载、有效模型、扫描范围或可交付。

`crates/codeguard-cli/tests/detect_cli.rs` 已覆盖直接插件声明、未声明插件、`pluginManagement`、父 POM、XML 命名空间、错误/DTD POM、Gradle 动态模型、FindSecBugs 依赖、显式 `skip=true` 及动态插件坐标识别。父 POM、仅在 profile/pluginManagement 声明、Gradle 动态构建及无法静态确定的插件坐标返回 `unknown`；坏 POM 与显式跳过的插件返回 `invalid`，避免误报“已配置”。既有只读测试确认不创建项目状态。完整 workspace 测试与 Clippy 的最终结果以本轮验收记录为准。

P3C 反例进一步覆盖：仅有 PMD、伪装成 P3C 的其他依赖、缺官方规则路径、未设置 `skipPmdError`、将其设为 `true`、动态 P3C 版本均不能被认成 P3C 已配置。`plan lint java` 可以显示 P3C 静态候选，但不会给出执行命令或质量通过。

Javadoc 注释配置新增反例：直接插件即使存在，只要 `<doclint>` 明确为 `none`、`all,-missing` 或只启用 `syntax`，`java.maven.javadoc` 就为 `invalid/javadoc_missing_check_disabled`；动态 doclint 为 `unknown`。`failOnError=false` 为 `invalid/javadoc_errors_ignored`；项目属性 `maven.javadoc.skip=true` 在没有插件内显式覆盖时也为 `invalid/plugin_explicitly_skipped`。`check java` 的 discovery 同样反馈该状态，但尚未执行项目级 Javadoc 检查。缺省 doclint 仍只表示插件静态声明，不能推断 Maven 生命周期确实运行或完整源集已检查。

观察层无法读取 POM 时，`detect` 返回退出码 3、`blocked_paths` 和检查器 `unknown/pom_unreadable`；它不会把 I/O 故障伪装成 XML 配置错误。此边界由 `discovery.rs` 的可注入观察端口测试覆盖。

边界：这只是 Java 构建根的**静态配置探测切片**。尚未解析 Maven effective model、执行生命周期、验证规则实际生效、探测其他语言检查器或把结果送入真实宿主对话。`configured` 只表示 POM 中有明确插件声明，不能转换为 `passed`；`missing` 也不能自动升级成代码违规。
