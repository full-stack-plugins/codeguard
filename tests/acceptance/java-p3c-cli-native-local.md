# Java `lint` 的 Maven/P3C 单文件原生观察

当前命令：`codeguard lint java FILE --maven-tool ABS_PATH --java-home ABS_PATH --maven-repo ABS_PATH --repo-sha256 SHA256 --format json`。

Rust CLI 在私有临时目录复制一份 Java 文件，并写入固定 Maven PMD 3.11.0、P3C 2.1.1、PMD 6.15.0、十个 P3C `ali-*.xml` 声明规则集的受控 POM 和隔离 settings。调用 Maven 原生 `pmd` 目标，固定 `-o`、显式本地仓和总截止时间；执行前后复核原文件、复制文件、POM/settings、Maven 入口、JDK 入口和仓库目录树。原始 Maven 日志保存在临时私有目录，公开反馈只输出结构化规则、规则集、位置和摘要，不输出原生消息或任意 stdout。

正常执行并有违规文件节点时，返回 `findings_observed_untrusted`；空 XML 即使 Maven 退出 0，也返回 `clean_scope_unproven`。缺新报告、仓库摘要不符、执行失败、输入/配置被工具修改或报告路径越界均为 `incomplete`。所有情形 `coverage_proven=false`、`checker_identity=unverified`、`rulepack_approval=unverified`、`delivery_decision=not_evaluated`、退出 3。反馈 0.2 列出十个**声明的**规则集及受控 POM SHA-256；声明不等于每条规则都被证实运行。这个入口检查隔离单文件候选，不读取项目 effective POM、不证明所有 P3C 规则或 Java 全量义务、不写项目源码/任务，也不签发白名单。

普通契约测试：`cargo test -p codeguard-cli --test java_p3c_cli --offline`。本轮真实 Maven/JDK 26 测试使用从已缓存制品复制出的 340 MiB 离线仓，树摘要 `ead9fd901ba10e3bddf60cc3543b64839d7650f967326b2c94b9442d556cedef`；显式 `--ignored` 的违规 `Bad_Name.java` 返回 `ClassNamingShouldBeCamelRule`/`AlibabaJavaNaming`，干净 `GoodName.java` 返回 `clean_scope_unproven`，二者退出 3。该仓是本机测试材料，摘要不是可信发行批准；通用 4.4 GiB `~/.m2` 超出运行时 512 MiB 目录预算，不能直接充当这次可锁闭包。

同一仓的原有 `p3c_native_replay --ignored` 还验证了坏规则集失败；最终 human CLI 直接显示 `Bad_Name.java:1 ClassNamingShouldBeCamelRule`、未核验提示与退出 3。新增 Java 分派导致的 Python 参数回归已由全工作区测试发现并修复，`lint_python_cli` 回归通过。

正式验收仍需受保护工具/策略锁、规则实际启用与目标覆盖证明、项目多模块与 source set、`check java` 和宿主反馈。局部稳定 finding/task 同步及 `check all` 已另有验收，见 `check-all-java-p3c-partial.md`。此局部证据对应 OpenSpec 5.4、6.2、6.6、6.7 的进行中状态。

2026-09-25 扩展后，本机离线 P3C 2.1.1 JAR 确认含十个声明资源；真实公开类样本在 `check all` 同时产生 `ClassNamingShouldBeCamelRule` 和 `ClassMustHaveAuthorRule`，包内可见类不产生作者注释诊断。`lint java` 与 `check all` 共用此隔离探针。此正反例只验证两条实际触发的规则及范围差异，不扩大为完整规则覆盖证明。
