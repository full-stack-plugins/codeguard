# `check all` / `check java` Java/P3C 局部原生接线验收

状态：局部原生观察、稳定任务同步及原工具任务复检已验证；完整 Java lint、可信工具/策略及交付门禁未实现。

`check all` 发现 Java 源码后创建 `java.p3c` DAG 节点。每份源码只采用最近 Maven 构建根中直接确认的 P3C 配置；缺失、无效或未解析生效模型不会启动 Maven。显式 `--maven-tool`、`--java-home`、`--maven-repo` 和 `--repo-sha256` 传入原有隔离单文件探针，与其它节点共用截止时间和取消标志。反馈保留每份文件的配置状态、原生理由和规则位置。没有配置或运行前提时为 `native_incomplete`，不会伪造源码违规。

隔离探针内置 P3C 2.1.1 十个可用 `ali-*.xml` 规则集，但项目检查只复制该 POM 中可静态确认的精确子集。只声明 `ali-naming.xml` 时，真实 `public class Bad_Name` 只应产生 `ClassNamingShouldBeCamelRule`；显式同时声明 comment 规则后才可产生 `ClassMustHaveAuthorRule`。本地反馈列出本轮实际选择的规则集和受控 POM 摘要。项目 POM 在发现前后、扫描期间或同步前变化，或包含无法安全对齐的配置时，不能形成当前有效 finding。此证据不证明项目真实构建/全源集覆盖。即使每份 Java 源码获得局部报告，`category_candidates` 中的 Java lint 仍为 `native_incomplete/p3c_declared_rulesets_unverified_coverage`，`coverage_proven=false`，整轮 `delivery_decision=incomplete`、退出 3。干净单文件报告仍为 `clean_scope_unproven`。

原生 XML 中每条诊断还必须同时符合本轮选中的规则集、该规则集的 PMD `name` 和 P3C 2.1.1 制品内的规则 ID。十个规则集共 56 个规则 ID，来源制品本地 SHA-256 为 `e7afec9340a0f30f56f4fdcd3b9c49e79ed24253719fcb9f457f5bcb43e54860`。若 XML 声称未选择的 comment 规则或把 comment 规则伪装成 naming 规则，报告为 `native_rule_outside_selected_rulesets`、无 finding、未完成；此核对不是可信规则包批准。
`work sync` 再次核对当前 POM 的规则子集、隔离探针 POM 摘要以及每条诊断的规则归属；已保存报告的 ruleset、声明子集或探针摘要被改写时，整份报告导入失败，不能通过重放形成稳定修复任务。

已初始化工作区将本轮 Java 原生局部报告写入私有报告目录，`work sync` 按规则、目标路径及源码锚点生成稳定 finding，按 Maven 构建根和阻塞原因生成环境/配置任务。重复扫描更新本轮观察而不重复创建任务；坏报告不能成为修复事实。`check all` 在对话反馈中返回任务简报。`task verify` 复用 Maven/P3C 原生探针、任务租约和验证事件：原问题仍在为 `still_present`；配置恢复但覆盖未核验为 `environment_restored_unverified_policy`；单文件 XML 零诊断为 `rule_coverage_requires_review`。以上情形任务均保持 open，不能据此签发质量通过。复检前后源码身份变化使观察无效。

`check java [path]` 复用同一 Java 原生节点与任务同步；混合项目不启动 Ruff/Clippy，候选类别仅列 Java，下一步只选 Java/P3C 任务。公开 `check_feedback` 0.8 把 `selection=java` 与 `delivery_decision=not_evaluated` 绑定，仍固定退出 3、`command_status=incomplete`；全项目 `check all` 继续 `delivery_decision=incomplete`。无 Java 目标返回 `java_target_absent_or_unobserved`，不签发空项目通过。项目发现结构仍保留全项目上下文，不能将未选语言的发现误读成已执行检查。

验证：

- `cargo test -p codeguard-cli --test check_all_java_p3c --offline`：未配置不启动 Maven、配置后原生发现进入统一反馈、嵌套构建根不会授权父目录源码；稳定 finding/blocker 任务、原生复检及三种报告重放篡改反例。
- `cargo test -p codeguard-adapters --test p3c_rule_catalog_contract --offline`：已选命名规则通过，未选 comment、规则集/规则 ID 错配和未知规则拒绝；`check_all_java_p3c` 的伪原生 XML 反例不创建项目 finding。
- 显式真实 Maven 用例 `real_native_p3c_finding_reaches_check_all_without_quality_allow` 与 `real_native_p3c_finding_survives_task_verify`：在本机 `/opt/homebrew/bin/mvn`、JDK 21、340 MiB 固定离线依赖仓上运行，真实 `ClassNamingShouldBeCamelRule` 进入 `check all` 并由原工具复检为 `still_present`，fact 保持 open，仍不签发质量通过。仓树 SHA-256 为 `ead9fd901ba10e3bddf60cc3543b64839d7650f967326b2c94b9442d556cedef`；它是本地复检身份，不是受保护工具批准。
- 显式真实 `real_native_check_java_reports_two_rules_without_delivery_gate`：在 POM 同时声明 naming/comment 后，`check java` 返回命名与作者注释两条 P3C 原生诊断；只声明 naming 的用例不得报告作者注释。Python/Rust 原生结果为空，交付未评估。
- `cargo test --workspace --offline -q`、`cargo clippy --workspace --all-targets --offline -- -D warnings`、`cargo fmt --all --check`、JSON schema 解析和 OpenSpec strict 应作为合并前回归。

尚缺完整 `check java` 义务、P3C 全规则执行与生效配置证明、Checkstyle、Maven Javadoc 插件项目级执行、CVE/安全/依赖/构建检查、可信工具/规则包身份、原生 suppression 对照、真实多模块项目前后差异裁定、正式关闭/重开及完整门禁验收。另有 JDK Javadoc 单文件局部探针，但不证明 Maven 项目级检查。故 OpenSpec 2.3、3.4、6.2、6.7、9.10 与 9.13 均不能勾选。Java finding 即使被用户声明为误报，也必须沿用精确白名单候选、独立批准与原始 finding 保留的统一协议；本局部接线不生效白名单。
