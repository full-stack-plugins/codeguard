# Gradle OWASP 原任务执行与公开CVE入口局部验收

对应原change的6.4/15.5，不完成逐语言生产验收。任务计划/报告归属测试先因入口不存在失败；公开cve java先误路由npm并退出2，接通后通过。缓存隔离测试先因请求无module_cache字段编译失败，再补齐有界只读复制。原Javadoc执行与协议没有修改。

同一私有输入/Gradle调用观察模型、原任务、唯一报告；显式Analyze/Aggregate路径、官方类型、所属插件及enabled严格绑定。JSON格式保持原配置；抑制/阈值/扫描/数据库设置未替换。报告协议JSON1.1/engine12.1.0和任务显示名验证后保留活动/原生抑制漏洞。退出0/1都能保留有效观察，原生失败不签发清洁。预存或共用报告、越界/分析异常/错项目、原源码/私有副本/工具/缓存变化均撤回观察。数据库、依赖归属、coverage与delivery固定未验证。

已有模块缓存显式输入复制到私有Gradlehome；读取范围排除用户配置、锁文件和符号链接。4096文件/128MiB单文件/512MiB总计/20000目录项。复制成功的受控哨兵验证私有路径，并核对源缓存不变；源缓存变化拒绝。此用例不是OWASP依赖解析成功的原生证明。

实际已有Gradle8.10.2与JDK21运行两类样例：普通同名dependencyCheckAnalyze任务、离线缺失OWASP12.1.0插件。内部服务与公开cve java共四次实际原生调用，均退出1且报告incomplete/无advisory。普通任务返回受管task_unavailable；不安装插件、不将缺插件误判无漏洞。源码及两份内部/两份公开反馈见[native evidence](evidence/gradle-dependency-check/native-unavailable-2026-10-06.json)。真实OWASP有/无漏洞扫描仍缺，不能以受控JSON替代。

完整受控公开反馈：[public controlled](evidence/gradle-dependency-check/public-controlled-2026-10-06.json)。最终内部受控反馈在controlled-current；开发中的过渡输出未作为最终协议证据入库。独立封闭原生/公开schema核对最终报文并拒绝资格升级。全量测试/Clippy终态将在任务日志同步。

复现真实环境条件用例，显式提供已有工具，不安装：

```bash
CARGO_PROFILE_TEST_DEBUG=0 cargo test --offline --locked -p codeguard-cli \
  --test gradle_dependency_check_probe \
  actual_gradle_rejects_imitation_task_and_missing_offline_plugin_without_advisories \
  -- --ignored --exact
```

当前普通check调度、work/next/task verify、依赖图及数据源可信时效、真实OWASP正反例/独立精度、多平台、完整宿主及发行仍未完成；原工作台不变，用户Erlang草稿不执行或修改。全部父任务保持开放，正式grammar资格0/32。

最终回归：默认CLI五目标29通过/3条件忽略，WASM三目标19通过/1条件忽略；新增公开SIGINT默认/WASM各1通过。适配器五目标17通过。原生环境条件用例另1通过/0忽略，四次真实Gradle调用；不与受控报告计数合并为原生精度。29份最终受控原生、2份实际原生、3份公开报文及实际SIGINT反馈通过两份schema，五类资格伪造拒绝。默认/WASM全工作区all-targets严格Clippy通过。计划视图更新为Gradle局部执行、资格blocked；初次回归因旧configuration_only断言失败后按实际新增能力修正并添加不得升级原生资格的反例，默认/WASM各4通过。

## 2026-10-07 真实 Gradle 阻塞路径复验

同日 Maven OWASP 探针修复（见 `owasp-maven-check-java.md`）后，用本机真实 Gradle 8.10.2 分发（`~/.gradle/wrapper/dists`，未安装/下载任何插件）与 Microsoft OpenJDK 21.0.12.1 重跑两条环境条件用例并全部通过（零联网）：

```
CARGO_PROFILE_TEST_DEBUG=0 CODEGUARD_TEST_GRADLE_BUNDLE=<分发路径> \
CODEGUARD_TEST_JAVA_HOME=<JDK21> cargo test --offline --locked -p codeguard-cli \
  --features wasm-precheck --test gradle_dependency_check_probe actual_ -- --ignored
# actual_gradle_rejects_imitation_task_and_missing_offline_plugin_without_advisories ... ok
# actual_unified_gradle_cve_environment_blockers_and_original_task_recheck_stay_open ... ok
```

普通任务假冒 OWASP 名、`--offline` 下插件缺失仍分别为 `native_exit_code=1` 的诚实未完成，无 advisory、无验收；统一 check 与 task verify 后任务保持 open。该复验只覆盖真实工具的阻塞/拒绝路径；真实 Gradle OWASP 插件正反例（需要下载 `org.owasp.dependencycheck` 12.x 及其模块缓存进隔离目录，或提供已含该插件的宿主缓存）仍未运行，15.5 的 Gradle 正例保持未完成。
