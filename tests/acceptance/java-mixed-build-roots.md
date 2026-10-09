# Maven/Gradle同根配置归属与局部漏洞保留

规格事实源仍为introduce-rust-codeguard-cli：native-tool-adapters / Maven and Gradle manifests coexist in the same physical build root，以及6.x/8.x/15.5。RED：新增三个用例均因Gradle路径丢失失败。修复后，pom.xml与任一Gradle DSL同根保留双方检查器；两个Gradle脚本保留两份configuration_ref；坏POM也保留Gradle未知义务。Java依赖/CVE/安全类别保持configuration_unresolved/checker_build_systems_mixed，不让已配置Maven或其局部结果覆盖Gradle。

受控Maven替身报告仍提供依赖图与CVE-2026-1234观察，同时项目类别不授予全范围Maven身份；这只验证报告归属与保留，不是实际Maven/Gradle漏洞引擎运行。check_feedback0.61提供独立封闭普通检查/lint-only分支，保留所有原生槽位；旧0.38不能消费已有Java注释not_configured原因，该原因在新协议严格绑定Java/comments/not_configured。没有降低报告精度或将未知配置改为缺配置。

```mermaid
flowchart LR
    A[同物理目录的构建文件] --> B[Maven直接声明观察]
    A --> C[每份Gradle脚本独立未知模型]
    B --> D[Maven局部依赖图和漏洞观察]
    B --> E[逐源码最近构建根归属]
    C --> E
    D --> F[保留原生局部反馈]
    E --> G[混合构建模型未解析 不授予全范围Maven身份]
    F --> G
```

以下是字段节选，非完整报告：

```json
{"schema_version":"0.61.0","report_type":"check_feedback","category_candidates":[{"language":"java","category":"cve","checker_id":null,"status":"configuration_unresolved","reason":"checker_build_systems_mixed"}],"delivery_decision":"not_evaluated"}
```

默认五目标50 passed/0 failed/0 ignored，WASM两目标7 passed/0 failed/0 ignored；WASM案例与默认重叠，不合计成独立样本数。导出混合原生结果另重跑1用例通过，不重复计数。实际21份发现/check报告通过对应封闭schema，409份schema元定义通过。最终Clippy、分层、OpenSpec及diff结果追加于下；受保护Erlang草稿未执行、未编辑。

本机PATH未提供Gradle，但缓存中找到8.10.2/8.10；绝对路径8.10.2 --offline --no-daemon --version实际执行成功，JVM为Microsoft21.0.12.1。检查的.gradle/caches/modules-2/files-2.1中org.owasp/dependency-check-gradle与插件marker目录不存在。此证据只证明本地版本命令和缓存观察，不证明所有缓存不存在或插件原生扫描成立。未安装、下载、修改插件规则或发布。

后续必须实现Groovy/Kotlin生效模型、原生dependencyCheckAnalyze/多项目范围、实际解析依赖图和新鲜漏洞库绑定、稳定任务及原工具复检关闭，才能满足15.5。上游[OWASP Gradle插件](https://github.com/dependency-check/dependency-check-gradle)和[Gradle命令文档](https://docs.gradle.org/current/userguide/command_line_interface.html)已核对；官方多项目扫描与可配置报告目录说明不能由本批静态发现替代。总体生产目标仍未完成。

补充统一lint/SARIF/语言选择回归18 passed/0 failed/1条件忽略，包含前述混合根三个用例，不重复合计。lint all仍只选择lint类别，0.61保留requested_categories=[lint]且java_cve为空。21份实际报告含两种构建profile、受控Maven局部漏洞和lint-only；四类伪造交付通过/未来版本/Gradle资格/错误类别配置原因被拒绝。

最终默认/WASM CLI全目标Clippy -D warnings、分层、OpenSpec strict和diff检查通过。最新默认源再跑五目标50通过；摘要和完整报告包见[evidence摘要](evidence/java-mixed-build-roots-2026-10-06.json)与[21份报告](evidence/java-mixed-build-roots-reports-2026-10-06.json)。Gradle版本重跑在入口字节摘要前后不变条件下成功，记录见[已有运行时观察](evidence/gradle-existing-runtime-2026-10-06.json)；整个Gradle制品包尚未验收，不伪称原生漏洞能力。远端d69076c的MSRV通过、gate仍失败于Check out corpus evidence source，不能借用到本批提交；本地验收不等于完整CI/生产验收。
