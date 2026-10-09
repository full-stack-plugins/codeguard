# Java P3C 配置状态矩阵与规则加载反例（2026-10-07）

对应 OpenSpec 6.2 / 15.4（Java 开发规范纵向闭环）。本轮在既有 P3C 扫描链路
（`check all` → `java_p3c_scan::observe_project` → `java_p3c_command::observe` → 原生 Maven
`org.apache.maven.plugins:maven-pmd-plugin:3.11.0:pmd`）上补齐任务要求的必测反例，并把
「配置存在不等于生效」落到扫描输出本身。

## 缺口与实现（RED→GREEN）

此前 `java_p3c_project_observation.files[]` 只携带聚合状态 `configuration`
（configured/missing/invalid/unknown），不带发现器给出的底层原因；invalid（skip=true）、
management-only、profile 限定三种反例在我所有的测试文件中无覆盖，用户无法区分「配置为何无效」。

新增测试 `crates/codeguard-cli/tests/java_p3c_config_status_matrix.rs` 首跑按预期失败
（`configuration_reason` 为 `null`，例如 disabled 用例期望 `p3c_plugin_explicitly_skipped`），
随后在 `crates/codeguard-cli/src/java_p3c_scan.rs` 的两处 `files[]` 输出中新增
`configuration_reason` 与 `configuration_next_action`（来自 `CheckerConfiguration`，逐源码取
最近构建根），复跑全部通过。字段为增量输出：`work_sync`、`task verify`（`classify_java` 要求
`schema_version=="0.2.0"`，故不升版）与 workbench 均按键读取，不受影响；旧持久化报告中缺省为
`null`，消费方不依赖它们。

## 反例矩阵（全部实际执行）

| 反例 | 测试 | 关键断言 |
| --- | --- | --- |
| 禁用规则（skip=true） | `p3c_disabled_plugin_is_invalid_and_never_launches_native` | `configuration=invalid`、`configuration_reason=p3c_plugin_explicitly_skipped`、原生标记未触发、零 findings |
| management-only | `p3c_management_only_declaration_is_unknown_and_never_launches_native` | `unknown` / `effective_model_or_profile_not_resolved`、不启动原生 |
| profile 限定 | `p3c_profile_only_declaration_is_unknown_and_never_launches_native` | 同上 |
| 配置存在不等于生效 | `configured_p3c_with_profile_section_keeps_ruleset_selection_unresolved` | 直接插件 `configured` 但同 POM 含 `<profiles>` 段时 `reason=p3c_ruleset_selection_unresolved`、不启动原生 |
| missing 带底层原因 | `missing_p3c_declaration_keeps_specific_configuration_reason` | `missing` / `p3c_plugin_not_declared` |
| 正反例结果相反 | `clean_counterpart_opposes_the_violating_sentinel_under_one_probe` | 同一探针同一规则集：`Bad_Name.java`→`ClassNamingShouldBeCamelRule` 命中 1 条；`GoodName.java`→0 诊断、`clean_scope_unproven`、`clean_report_has_no_file_attestation`，两者均不签发质量通过 |
| 坏 XML（扫描层） | `malformed_native_report_keeps_scan_incomplete_without_findings` | 截断 XML → `native_report_invalid_or_out_of_scope`、零 findings |
| 超时 | `deadline_exhaustion_keeps_native_scan_incomplete` | `CODEGUARD_TIMEOUT=2s` + sleep 30 的替身 → 零诊断、`local_observation_complete=false`、退出 3 |

PMD 6 探针（`pmd6_probe_contract.rs`）新增三个报告级反例并全部通过：
`malformed_xml_report_is_incomplete_and_not_a_violation`（`invalid_xml_report`，两种坏输入）、
`version_mismatched_report_is_incomplete`（`pmd_version_mismatch`）、
`processing_error_report_retains_diagnostics_without_passing`（`native_processing_error`，
诊断保留但不签发局部一致）。

既有覆盖保持不变：未知规则集（`unknown_p3c_ruleset_never_starts_native_probe`）、陈旧报告
（`old_report_blocks_execution_and_is_preserved`、`changing_project_pom_during_native_scan_drops_findings`）。

## 实际运行命令与结果（本机，2026-10-07）

```
CARGO_PROFILE_TEST_DEBUG=0 cargo test --offline --locked -p codeguard-cli --features wasm-precheck \
  --test java_p3c_config_status_matrix   # 8 passed（GREEN 前 5 failed：configuration_reason 为 null）
CARGO_PROFILE_TEST_DEBUG=0 cargo test ... --test pmd6_probe_contract      # 11 passed
CARGO_PROFILE_TEST_DEBUG=0 cargo test ... --test check_all_java_p3c       # 26 passed, 6 ignored
CARGO_PROFILE_TEST_DEBUG=0 cargo test ... --test java_p3c_cli             # 6 passed, 2 ignored
CARGO_PROFILE_TEST_DEBUG=0 cargo test ... --test java_p3c_workbench       # 21 passed
CARGO_PROFILE_TEST_DEBUG=0 cargo test ... --lib                            # 144 passed, 6 ignored
```

以上均为替身 Maven/JDK/仓库的离线夹具执行，证明的是 Rust 链路对配置状态、报告有效性与
正反例区分的行为契约；ignored 的原生用例不计入通过。

## 未运行：原生规则加载哨兵（诚实记录）

「规则实际加载」的直接证据仍是 `p3c_native_replay.rs::native_p3c_2_1_1_rule_loading_and_failure_matrix`
（真实 Maven + P3C 2.1.1 + PMD 6.15.0，正例命中/反例干净/未知规则集非零退出）。该测试
`#[ignore]`，需要四个环境变量；本机 Maven 3.9.16 与 JDK 8/17/21/26 存在，但 2026-09-24 记录的
约 26 MiB 隔离依赖仓库（树摘要 `587618cd…13879`）已不在磁盘，默认 `~/.m2` 亦无
p3c-pmd/pmd-core/pmd-java/maven-pmd-plugin 制品。按约束本轮不自行下载，**该哨兵本轮未运行**，
6.2 不因本轮工作勾选。

恢复方案（需授权后执行）：在隔离目录（建议 `/tmp/codeguard-p3c-m2-<date>`）用
`mvn -o dependency:go-offline -Dmaven.repo.local=<隔离目录>` 或从内部镜像预取
`org.apache.maven.plugins:maven-pmd-plugin:3.11.0`、`com.alibaba.p3c:p3c-pmd:2.1.1`、
`net.sourceforge.pmd:pmd-core/pmd-java:6.15.0` 及其传递闭包；核对单件 SHA-256 与
`codeguard-bundle-tree-v1` 树摘要（上次观测 `587618cd1f2813bfe0c4a11c6ec167522ae61095abbdf2c060c3621bb6e13879`，
重新物化后以新观测值为准，不沿用旧值）；随后按
`tests/acceptance/p3c-pmd-research.md` 的复跑命令在 JDK 8/17/21/26 各跑一次正反例与未知规则集；
失败时删除隔离目录即可恢复，不留系统级变更。
