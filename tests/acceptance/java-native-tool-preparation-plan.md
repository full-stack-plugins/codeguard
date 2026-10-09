# Java 文档原工具验收：隔离工具准备计划

状态：待用户允许下载验收制品。对应已有OpenSpec 6.3/15.3/15.6，不创建新change，不把本计划当真实插件运行或生产验收。

## 当前阻塞与冻结版本

现有Maven详细描述、Checkstyle三个详细模块已有解析/公开任务/原工具复检代码及受控运输回归；当前本机未发现所需完整插件缓存或Checkstyle JAR。受控输出不能证明原工具语义，已有独立JDK/Gradle证据不能替代Maven/Checkstyle正常运行。需准备现有适配契约固定的版本，不升级适配器去迎合最新工具。

| 制品 | 固定版本和官方来源 | 隔离位置 |
|---|---|---|
| Checkstyle自包含JAR | 10.21.4，[官方发布](https://github.com/checkstyle/checkstyle/releases/tag/checkstyle-10.21.4)，资产checkstyle-10.21.4-all.jar，API确认19631994字节 | /Users/wandl/Library/Caches/codeguard-native-tools/checkstyle/10.21.4/ |
| Maven Javadoc插件及必要运行依赖 | org.apache.maven.plugins:maven-javadoc-plugin:3.12.0，[官方文档](https://maven.apache.org/plugins/maven-javadoc-plugin/)，使用Maven Central和固定坐标 | /Users/wandl/Library/Caches/codeguard-native-tools/maven-javadoc/3.12.0/repository/ |

已有Maven3.9.16和Microsoft JDK21继续使用；不安装JDK/Maven、不全局安装或升级、不修改用户项目/POM/settings.xml/全局.m2库、不发布或合并。Maven准备只执行本计划受控最小项目的固定Javadoc goal，源文件和POM放在同一隔离缓存下的bootstrap目录；准备完成后所有验收使用offline/locked已有输入。下载阶段新增工具库/日志/制品清单；本仓库仅保存脱敏证据及文档。

Checkstyle官方Release API的digest为null，只证明名称/来源/尺寸，不能冒充已验证发布者摘要。允许下载后记录URL、实际字节SHA-256、工具版本与获取时间，核对官方尺寸/版本，再作为所选原工具候选；依赖按实际Maven解析闭包记录固定坐标/摘要，不把下载成功称工具或生产资格通过。若缺少制品或出现版本/来源异常，报告具体阻塞，不替换工具版本或注入规则。

## 已准备可审阅的bootstrap输入

`tests/fixtures/java_native_tool_bootstrap/pom.xml`只固定Javadoc3.12.0/doclint=all与原失败策略，不含业务依赖或额外goal；`src/main/java/BootstrapDocs.java`包含完整用途、构造器、参数/返回及允许null的实际契约。POM已解析核对固定坐标；已有JDK21直接javadoc -Xdoclint:all实测无诊断并退出0，这只证明bootstrap源码可用，不代表Maven插件已运行。允许准备后将这两份输入复制到隔离bootstrap目录，执行固定goal填充专用库，随后离线验收。

只读来源记录见[evidence](evidence/java-native-tool-source-inventory-2026-10-07.json)：官方Checkstyle资产元数据与Maven Central POM HEAD 200/21345字节；未下载JAR/POM正文或插件依赖，未运行真实Maven/Checkstyle插件。Web读取POM首遇403，直接只读HEAD复核得到200，未把403解释成工具缺失或源码违规。

## 必须执行的真实验收

```mermaid
flowchart LR
 A[用户允许隔离工具准备] --> B[官方固定制品及运行依赖]
 B --> C[记录字节身份与版本]
 C --> D[已有真实条件测试 默认及WASM]
 D --> E[原工具规则 正反例 任务复检]
 E --> F[实际反馈协议和独立范围核对]
 F --> G[记录通过/问题/缺口]
 G --> H[完整详细契约 独立精度 平台及可信关闭继续验收]
```

1. Checkstyle：`checkstyle_detailed_descriptions::actual_checkstyle_detailed_modules_preserve_source_tasks_and_repaired_absence`；启用三个固定官方模块，原生正反例→公开lint→稳定任务→next→仍存在/修复后未受信消失。明确提供CODEGUARD_JAVA_BIN及CODEGUARD_CHECKSTYLE_JAR。先检查测试本身是否符合原工具实际行为，失败时保留证据并修正实现或测试假设，不换受控运输程序通过。
2. Maven：`maven_javadoc_detailed_descriptions::actual_cached_maven_detailed_comments_preserve_original_tasks`；四组详细描述样例4/3/1/0，failOnWarnings两配置、原POM多文件→公开comments→稳定任务→原工具task verify。明确提供CODEGUARD_MAVEN_BIN、CODEGUARD_TEST_JAVA_HOME、CODEGUARD_JAVADOC_MAVEN_REPO。
3. 两组在默认和wasm-precheck构建下串行执行；Cargo一律offline/locked/CARGO_PROFILE_TEST_DEBUG=0，仅选择这两个测试目标，受保护Erlang草稿不编辑、执行或暂存。
4. 实际CodeGuard反馈按当前独立schema验证；旧schema及旧证据不覆盖。每份新证据绑定所选工具和CodeGuard制品字节。工具环境故障不生成源码违规；规则/范围未验证或零诊断不关闭任务、不授予生产资格。

以下命令仅在允许准备且制品身份核验后执行，当前未运行：

```bash
CARGO_PROFILE_TEST_DEBUG=0 /opt/homebrew/opt/rustup/bin/cargo test --offline --locked \
  -p codeguard-cli --test checkstyle_detailed_descriptions \
  actual_checkstyle_detailed_modules_preserve_source_tasks_and_repaired_absence -- --ignored --exact
CARGO_PROFILE_TEST_DEBUG=0 /opt/homebrew/opt/rustup/bin/cargo test --offline --locked \
  -p codeguard-cli --test maven_javadoc_detailed_descriptions \
  actual_cached_maven_detailed_comments_preserve_original_tasks -- --ignored --exact
```

正式验收仍涵盖57语言×四核心、全部32grammar、独立误报/漏报、完整配置/目标、可信关闭/复发、五平台与宿主/发布。此准备仅解锁两个Java原工具测试路径；父任务保持66完成/288待完成，grammar正式资格0/32。固定插件审计来源CI阻塞与本计划独立，不能通过下载验收工具绕过CI源审计。

本轮默认两个既有详细文档CLI目标7通过/3条件忽略，均为受控/协议回归，三个真实条件仍未执行；新的bootstrap仅做已有JDK直接文档验证。上一提交d9e7283的CI37515178587仍在固定插件源检出失败，MSRV单独状态以实际CI为准；保持正式资格未授予。

首次生产计划回归拒绝新增指纹：三个bootstrap/元数据文件没有对应引用，不能把孤立摘要当验收映射。已将准备输入显式关联到Maven文档路径，保持原引用/摘要一致性检查，修正后重新验证；不删除或绕过该校验。
