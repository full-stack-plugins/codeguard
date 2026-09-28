# OWASP Maven 原生 CVE 局部反馈

`check java`/`check all` 对静态识别为直接配置 `org.owasp:dependency-check-maven` 的构建根建立 `java.cve` 任务，包括无 Java 源文件的 Maven 根。原 POM 必须有固定项目与插件版本、直接且固定的依赖、无父 POM/profile/插件配置或外部仓库；除该 OWASP 插件外，只允许并列声明固定版本的 `maven-dependency-plugin:3.8.1`，其它插件仍拒绝。资格不足时反馈 `project_pom_replay_ineligible`，不会替项目生成一个简化 POM。可执行探针需要显式 Maven、JDK、离线仓库及其树摘要、CVE 数据目录及其树摘要；缺一项返回 `prerequisites_missing`。

Rust runtime 在私有 POM 与数据库副本中调用原生 `org.owasp:dependency-check-maven:<POM 版本>:check`，指定 JSON 格式、私有报告目录、`autoUpdate=false` 和常见远程分析器关闭参数；参数逐项传递，不经 Shell。原 POM、Maven/JDK、离线依赖闭包、数据库原件与副本在执行前后核对；私有报告按 OWASP JSON 1.1 严格解析，核对引擎版本和项目名。human/JSON 反馈保留安全格式的 advisory 来源、ID、可用分数、标准 Maven PURL 和原生 suppression；含私服 `repository_url` 或未知限定符的包标识只显示 `redacted-sha256` 摘要，异常来源/时间戳也不原样回显。原生失败、坏报告、项目错配或数据库字节变化均为 `incomplete`，不能变成空漏洞结论。

数据库摘要仅说明**本地字节一致**，不能证明来源、签发时间或更新时效；零漏洞报告标 `empty_report_unverified`，有漏洞标 `findings_observed_untrusted`，两者固定 `database_freshness=unverified`、`coverage_proven=false`、交付未评估。此阶段的 CVE 观察不具备误报白名单的可信身份与门禁前提，不能通过候选白名单放行。当前没有系统调用级网络隔离、真实 OWASP 插件与合格漏洞库验收、同轮 Maven 图/制品 SHA 绑定或可信策略，因此 6.4/6.8 仍未完成。参数依据：[OWASP Maven check goal](https://dependency-check.github.io/DependencyCheck/dependency-check-maven/check-mojo.html) 和 [官方配置说明](https://dependency-check.github.io/DependencyCheck/dependency-check-maven/configuration.html)。

定向测试：`cargo test -p codeguard-adapters --test owasp_maven_pom_contract --offline -q` 与 `cargo test -p codeguard-cli --test check_java_cve_native --offline -q`。测试使用模拟 Maven/JDK/数据库，覆盖已配置原生结果进入对话反馈、缺数据库、错误项目名、数据库摘要变化和动态/复杂 POM 拒绝。即使根 Maven 报告已观察到漏洞，加入 Gradle 子构建根后，项目级候选仍为混合构建系统未解析，不冒称全项目已配置或已扫描。模拟工具不作为真实 OWASP 执行验收。

同一简单 POM 同时声明依赖插件和 OWASP 插件时，两项原生任务在同一 `check` 调用中执行。CLI 仅在构建根、POM、Maven/JDK 和离线仓库摘要相同且两份报告均为本轮原生局部观察时，使用已有 Rust 归属器逐条比较 Maven PURL、依赖树节点与制品 SHA-256；`check_feedback` 0.14 的 `java_cve.attribution_probes` 显示候选或具体失配。私服限定符在公开反馈中已脱敏，不能作为可归属的原始坐标。对等摘要只是本地候选，不证明数据库时效、工具锁批准、跨进程执行隔离或完整依赖覆盖，交付决策仍为 `not_evaluated`。`cargo test -p codeguard-cli --test check_java_cve_attribution --offline -q` 用模拟双插件项目覆盖精确候选与摘要失配；真实 OWASP/数据库验收仍缺。

`check_feedback` 0.15 将 CVE 局部结果升级为可在已初始化工作区保存的 0.3 本地报告。`check java` 自动保存、幂等 `work sync` 并返回 `java_cve.next`：原生报告中有 advisory 或零漏洞但库时效未核验时，以构建根和 `cve_database_freshness_unverified` 形成一张稳定**环境/证据阻塞任务**；缺工具/库、坏报告等原生未完成按结构化原因形成阻塞任务。任务包含报告摘要、允许范围、原工具复检参数和关闭条件，明确不是已确认源码漏洞或白名单批准。重复扫描更新观察但不新增同身份任务；POM 扫描后改变会转为配置变更阻塞。未初始化时只反馈观察，不写 `codeguard/`。修改已消费的本地报告后再次同步会出现摘要冲突，不默默改写已建立的任务。

`task verify <CG-B-id> .` 现按 `java.maven.dependency_check` 任务身份重新调用原生 OWASP Maven 探针，接受与 `check java` 相同的 Maven/JDK/仓库/漏洞库显式路径和摘要参数。复检报告经 `work sync` 保留新阻塞，并追加与报告摘要绑定的 `verification_observed`；缺前置条件为 `still_blocked`，原漏洞库时效任务即使再次观察到 advisory 仍是 `still_blocked`。本地报告始终 `local_unverified`、退出 3、事实保持 `open`。复检后 POM 改变须重新扫描；尝试历史和 `next` 会校验 CVE 复检事件，不能把其错认成 Ruff 报告。

重复 CVE blocker 扫描现在只在首次发现写 tracked `observed` 事件，后续每轮报告摘要与受影响路径写入默认忽略的 `state/observations/`；复检事件之后再次出现才补一条 tracked 事件，继续重复扫描不增加 Git 噪声。`check_java_cve_native` 中连续扫描与复检后的再现矩阵验证这一点。

本切片尚未把未经核验的 advisory 写成 `finding`，也未实现 CVE 任务的正式关闭/重开或可信漏洞库验收；故此任务始终不能用于交付通过。模拟原生与缺数据库的持久化正反例见 `check_java_cve_native` 测试。
