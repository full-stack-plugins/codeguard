# Maven Javadoc 多文件局部原生探针

`check java` / `check all` 对静态确认声明 Javadoc 插件的 Maven 构建根，在显式给出 Maven、JDK 21、离线仓库和仓库树摘要时，另行运行一次私有多文件 Maven Javadoc 探针。只有简单静态 POM（无父 POM、profile、依赖、模块、扩展或其它插件，直接固定 Javadoc Plugin 3.12.0 与 `doclint=missing`）才进入原 POM 重放；其它项目返回 `project_pom_replay_ineligible`，不启动固定 POM 替代探针。Rust 快照保留原 `pom.xml` 及当前 `src/main/java` 文件的相对路径和字节，使用隔离 settings、`-o` 和新目录，避免旧 `target` 文档掩盖警告。原生日志经严格解析，且输入、工具、仓库树和 `target/reports/apidocs/index.html` 在本轮复核后才显示局部诊断。

本机 Maven 3.9.16、JDK 21 和独立离线仓库约 19 MB 的真实 CLI 验收：两份缺注释源码产生多条 `JavadocMissingComment`，改为完整注释后新一轮私有快照为 `clean_log_unverified`、零诊断。两轮 `check java` 均退出 3、`delivery_decision=not_evaluated`。首次真实测试用空 settings 失败，因为离线制品缓存的镜像仓库 ID 与当前上下文不匹配；改用仓内既有镜像配置后通过。

真实简单样本的 `native_plan_sha256` 等于原项目 POM 字节摘要，`pom_mode=direct_pom_replay`。但 Maven 生效模型、生成源码、record/Lombok、项目 exclusion 和完整源集尚未验证；报告仍固定 `project_checker_attribution=unverified`、`coverage_proven=false`，不进入 finding/task 同步或白名单授权。更复杂的 POM 应接入独立受控执行器，不能通过扩大此资格集合或使用合成 POM 跳过配置归属。Maven `-o` 限制制品解析，但仍不是通用内核沙箱。

```bash
cargo test -p codeguard-cli --test check_all_java_p3c --offline
CODEGUARD_MAVEN_BIN=/absolute/path/to/mvn CODEGUARD_JAVA_HOME=/absolute/path/to/jdk-21 CODEGUARD_JAVADOC_MAVEN_REPO=/absolute/path/to/isolated-repo cargo test -p codeguard-cli --test check_all_java_p3c --offline real_maven_javadoc_multifile_probe_keeps_project_authority_unverified -- --ignored
```
