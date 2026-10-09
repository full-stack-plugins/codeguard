# CodeGuard 逐语言四能力实施矩阵

> 2026-10-07；基线 `a30f3efc32e69bd50d557ff3c7d26836d9a229ca`；从生产计划逐项展开，不另维护完成状态。正式状态请回读 [源计划](../../rulepacks/production_acceptance_plan_v1.json)。

执行方式：每条构建路径单独实施和验收，统一使用 [手册](Codeguard-Implementation-Handoff.zh_CN.md) 第8–11节工作包与A01–A12标准。下面的引用路径是仓库相对路径；空入口表示需要新实现，候选工具不代表已安装或已支持。

## java

- 构建生态：maven, gradle。grammar候选：java。

- 版本/方言约束：Javadoc用途/参数/返回/异常及继承/record/生成范围。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["mvn", "-B", "-DskipTests", "verify"]`。

### syntax — blocked

OpenSpec任务：6.1, 6.2, 6.3, 6.4, 6.8, 15.2, 15.6, 15.7。

#### 构建路径 maven / partial

现有代码入口：

- [crates/codeguard-cli/src/grammar_native_checker.rs](../../crates/codeguard-cli/src/grammar_native_checker.rs)


现有验收/样例：

- [tests/acceptance/java-native-differential.md](../../tests/acceptance/java-native-differential.md)


必须解决的登记缺口：

- java/maven:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- java/maven/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 gradle / partial

现有代码入口：

- [crates/codeguard-cli/src/grammar_native_checker.rs](../../crates/codeguard-cli/src/grammar_native_checker.rs)


现有验收/样例：

- [tests/acceptance/java-native-differential.md](../../tests/acceptance/java-native-differential.md)


必须解决的登记缺口：

- java/gradle:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- java/gradle/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：6.1, 6.2, 6.3, 6.4, 6.8, 15.3, 15.6, 15.7。

#### 构建路径 maven / partial

现有代码入口：

- [crates/codeguard-cli/src/maven_javadoc_task_recheck.rs](../../crates/codeguard-cli/src/maven_javadoc_task_recheck.rs)


现有验收/样例：

- [tests/acceptance/maven-javadoc-detailed-descriptions.md](../../tests/acceptance/maven-javadoc-detailed-descriptions.md)

- [tests/acceptance/java-native-tool-preparation-plan.md](../../tests/acceptance/java-native-tool-preparation-plan.md)

- [tests/fixtures/java_native_tool_bootstrap/pom.xml](../../tests/fixtures/java_native_tool_bootstrap/pom.xml)

- [tests/fixtures/java_native_tool_bootstrap/src/main/java/BootstrapDocs.java](../../tests/fixtures/java_native_tool_bootstrap/src/main/java/BootstrapDocs.java)

- [tests/acceptance/evidence/java-native-tool-source-inventory-2026-10-07.json](../../tests/acceptance/evidence/java-native-tool-source-inventory-2026-10-07.json)


必须解决的登记缺口：

- java/maven: Javadoc用途/参数/返回/异常及继承/record/生成范围的完整原生规则/合法反例/契约准确性未验收

- java/maven/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 gradle / partial

现有代码入口：

- [crates/codeguard-cli/src/gradle_javadoc_task_recheck.rs](../../crates/codeguard-cli/src/gradle_javadoc_task_recheck.rs)


现有验收/样例：

- [tests/acceptance/gradle-javadoc-public-task-recheck.md](../../tests/acceptance/gradle-javadoc-public-task-recheck.md)


必须解决的登记缺口：

- java/gradle: Javadoc用途/参数/返回/异常及继承/record/生成范围的完整原生规则/合法反例/契约准确性未验收

- java/gradle/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：6.1, 6.2, 6.3, 6.4, 6.8, 15.4, 15.6, 15.7。

#### 构建路径 maven / partial

现有代码入口：

- [crates/codeguard-cli/src/java_p3c_scan.rs](../../crates/codeguard-cli/src/java_p3c_scan.rs)


现有验收/样例：

- [tests/acceptance/java-p3c-cli-native-local.md](../../tests/acceptance/java-p3c-cli-native-local.md)


必须解决的登记缺口：

- java/maven:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- java/maven/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 gradle / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- java/gradle:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- java/gradle/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：6.1, 6.2, 6.3, 6.4, 6.8, 15.5, 15.6, 15.7。

#### 构建路径 maven / partial

现有代码入口：

- [crates/codeguard-cli/src/java_cve_scan.rs](../../crates/codeguard-cli/src/java_cve_scan.rs)


现有验收/样例：

- [tests/acceptance/owasp-maven-check-java.md](../../tests/acceptance/owasp-maven-check-java.md)


必须解决的登记缺口：

- java/maven:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- java/maven/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 gradle / partial

现有代码入口：

- [crates/codeguard-cli/src/java_gradle_cve_command.rs](../../crates/codeguard-cli/src/java_gradle_cve_command.rs)

- [crates/codeguard-cli/src/gradle_dependency_check_probe.rs](../../crates/codeguard-cli/src/gradle_dependency_check_probe.rs)

- [crates/codeguard-cli/src/gradle_module_cache.rs](../../crates/codeguard-cli/src/gradle_module_cache.rs)

- [crates/codeguard-adapters/src/gradle_dependency_check_plan.rs](../../crates/codeguard-adapters/src/gradle_dependency_check_plan.rs)

- [crates/codeguard-adapters/src/gradle_owasp_report_ownership_parser.rs](../../crates/codeguard-adapters/src/gradle_owasp_report_ownership_parser.rs)

- [crates/codeguard-cli/src/gradle_owasp_report_budget.rs](../../crates/codeguard-cli/src/gradle_owasp_report_budget.rs)

- [crates/codeguard-cli/src/gradle_cve_workbench.rs](../../crates/codeguard-cli/src/gradle_cve_workbench.rs)

- [crates/codeguard-cli/src/work_sync/gradle_cve_report.rs](../../crates/codeguard-cli/src/work_sync/gradle_cve_report.rs)

- [crates/codeguard-cli/src/work_sync.rs](../../crates/codeguard-cli/src/work_sync.rs)

- [crates/codeguard-cli/src/next_command.rs](../../crates/codeguard-cli/src/next_command.rs)

- [crates/codeguard-cli/src/task_verify_command.rs](../../crates/codeguard-cli/src/task_verify_command.rs)

- [crates/codeguard-cli/src/gradle_cve_task_recheck.rs](../../crates/codeguard-cli/src/gradle_cve_task_recheck.rs)

- [crates/codeguard-cli/src/task_attempt_command.rs](../../crates/codeguard-cli/src/task_attempt_command.rs)

- [crates/codeguard-cli/src/check_command.rs](../../crates/codeguard-cli/src/check_command.rs)

- [crates/codeguard-cli/src/partial_sarif_feedback.rs](../../crates/codeguard-cli/src/partial_sarif_feedback.rs)

- [crates/codeguard-cli/src/command_catalogue.rs](../../crates/codeguard-cli/src/command_catalogue.rs)


现有验收/样例：

- [tests/acceptance/gradle-dependency-check-probe.md](../../tests/acceptance/gradle-dependency-check-probe.md)

- [tests/acceptance/gradle-owasp-aggregate-budget.md](../../tests/acceptance/gradle-owasp-aggregate-budget.md)

- [tests/acceptance/gradle-cve-workbench.md](../../tests/acceptance/gradle-cve-workbench.md)

- [tests/acceptance/gradle-cve-task-recheck.md](../../tests/acceptance/gradle-cve-task-recheck.md)

- [tests/acceptance/gradle-cve-unified-check.md](../../tests/acceptance/gradle-cve-unified-check.md)


必须解决的登记缺口：

- 真实OWASP有/无漏洞原生扫描、插件缓存闭包、漏洞库时效和完整依赖归属仍待验收；受控报告和实际缺插件阻塞不能授予资格

- 自动配置发现与调度、可信工作闭环、可信关闭/复发、版本/平台/宿主/发行仍未完成

- java/gradle:脱敏稳定准备任务已接入显式cve；自动配置调度、真实OWASP复检、完整依赖/数据时效和可信关闭仍未验收



## rust

- 构建生态：cargo。grammar候选：rust。

- 版本/方言约束：Rustdoc用途/参数/返回/错误/Panics/Safety及目标特性。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["cargo", "clippy", "--all-targets", "--", "-D", "warnings"]`。

### syntax — blocked

OpenSpec任务：7.1, 15.2, 15.6, 15.7。

#### 构建路径 cargo / partial

现有代码入口：

- [crates/codeguard-cli/src/rust_project_syntax.rs](../../crates/codeguard-cli/src/rust_project_syntax.rs)


现有验收/样例：

- [tests/acceptance/rust-native-differential.md](../../tests/acceptance/rust-native-differential.md)


必须解决的登记缺口：

- rust/cargo:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- rust/cargo/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：7.1, 15.3, 15.6, 15.7。

#### 构建路径 cargo / partial

现有代码入口：

- [crates/codeguard-cli/src/rust_comments_command.rs](../../crates/codeguard-cli/src/rust_comments_command.rs)

- [crates/codeguard-cli/src/rust_lint_scan.rs](../../crates/codeguard-cli/src/rust_lint_scan.rs)

- [crates/codeguard-cli/src/next_command.rs](../../crates/codeguard-cli/src/next_command.rs)

- [crates/codeguard-adapters/src/cargo_documentation_config.rs](../../crates/codeguard-adapters/src/cargo_documentation_config.rs)

- [crates/codeguard-cli/src/discovery.rs](../../crates/codeguard-cli/src/discovery.rs)

- [crates/codeguard-adapters/src/cargo_documentation_workspace.rs](../../crates/codeguard-adapters/src/cargo_documentation_workspace.rs)

- [crates/codeguard-adapters/src/cargo_workspace_reference.rs](../../crates/codeguard-adapters/src/cargo_workspace_reference.rs)

- [crates/codeguard-cli/src/rust_documentation_command.rs](../../crates/codeguard-cli/src/rust_documentation_command.rs)


现有验收/样例：

- [tests/acceptance/rustdoc-task-recheck.md](../../tests/acceptance/rustdoc-task-recheck.md)

- [tests/acceptance/clippy-documentation-contract.md](../../tests/acceptance/clippy-documentation-contract.md)

- [crates/codeguard-cli/tests/rust_clippy_documentation.rs](../../crates/codeguard-cli/tests/rust_clippy_documentation.rs)

- [tests/acceptance/clippy-documentation-aggregate.md](../../tests/acceptance/clippy-documentation-aggregate.md)

- [crates/codeguard-cli/tests/check_all_rust_native.rs](../../crates/codeguard-cli/tests/check_all_rust_native.rs)

- [crates/codeguard-cli/tests/cargo_doc_config_cli.rs](../../crates/codeguard-cli/tests/cargo_doc_config_cli.rs)

- [tests/acceptance/cargo-documentation-declarations.md](../../tests/acceptance/cargo-documentation-declarations.md)

- [tests/acceptance/cargo-documentation-workspace.md](../../tests/acceptance/cargo-documentation-workspace.md)

- [tests/acceptance/cargo-documentation-explicit-workspace.md](../../tests/acceptance/cargo-documentation-explicit-workspace.md)

- [crates/codeguard-cli/tests/rust_comments_combined.rs](../../crates/codeguard-cli/tests/rust_comments_combined.rs)

- [schemas/rust-comments-feedback-v0.1.schema.json](../../schemas/rust-comments-feedback-v0.1.schema.json)

- [tests/acceptance/rust-comments-combined.md](../../tests/acceptance/rust-comments-combined.md)

- [tests/acceptance/evidence/rust-comments-combined-native.json](../../tests/acceptance/evidence/rust-comments-combined-native.json)

- [tests/acceptance/evidence/rust-comments-combined-native-wasm.json](../../tests/acceptance/evidence/rust-comments-combined-native-wasm.json)

- [tests/acceptance/evidence/rust-comments-combined-schema.json](../../tests/acceptance/evidence/rust-comments-combined-schema.json)


必须解决的登记缺口：

- rust/cargo: Rustdoc用途/参数/返回/错误/Panics/Safety及目标特性的完整原生规则/合法反例/契约准确性未验收

- rust/cargo/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：7.1, 15.4, 15.6, 15.7。

#### 构建路径 cargo / partial

现有代码入口：

- [crates/codeguard-cli/src/rust_lint_command.rs](../../crates/codeguard-cli/src/rust_lint_command.rs)


现有验收/样例：

- [tests/acceptance/rust-clippy-input-stability.md](../../tests/acceptance/rust-clippy-input-stability.md)


必须解决的登记缺口：

- rust/cargo:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- rust/cargo/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：7.1, 15.5, 15.6, 15.7。

#### 构建路径 cargo / partial

现有代码入口：

- [crates/codeguard-cli/src/cargo_audit_command.rs](../../crates/codeguard-cli/src/cargo_audit_command.rs)

- [crates/codeguard-cli/src/cargo_audit_database_snapshot.rs](../../crates/codeguard-cli/src/cargo_audit_database_snapshot.rs)

- [crates/codeguard-cli/src/work_sync/rust_cve_report.rs](../../crates/codeguard-cli/src/work_sync/rust_cve_report.rs)


现有验收/样例：

- [tests/acceptance/cargo-audit-native-observation.md](../../tests/acceptance/cargo-audit-native-observation.md)

- [tests/acceptance/cargo-audit-database-stability.md](../../tests/acceptance/cargo-audit-database-stability.md)

- [crates/codeguard-cli/tests/cargo_audit_cli.rs](../../crates/codeguard-cli/tests/cargo_audit_cli.rs)

- [tests/acceptance/evidence/cargo-audit-database-stability.json](../../tests/acceptance/evidence/cargo-audit-database-stability.json)

- [tests/acceptance/evidence/cargo-audit-database-native.json](../../tests/acceptance/evidence/cargo-audit-database-native.json)

- [tests/acceptance/evidence/cargo-audit-database-native-wasm.json](../../tests/acceptance/evidence/cargo-audit-database-native-wasm.json)

- [tests/acceptance/evidence/cargo-audit-database-stability-wasm.json](../../tests/acceptance/evidence/cargo-audit-database-stability-wasm.json)

- [tests/acceptance/evidence/cargo-audit-database-schema.json](../../tests/acceptance/evidence/cargo-audit-database-schema.json)


必须解决的登记缺口：

- rust/cargo:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- rust/cargo/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## typescript

- 构建生态：npm, deno。grammar候选：typescript, javascript, tsx。

- 版本/方言约束：JSDoc/TSDoc与TypeScript/JSX声明模式及重载契约。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["npx", "--no-install", "eslint", ".", "--max-warnings", "0"]`。

### syntax — blocked

OpenSpec任务：7.3, 15.2, 15.6, 15.7。

#### 构建路径 npm / partial

现有代码入口：

- [crates/codeguard-cli/src/javascript_syntax_probe.rs](../../crates/codeguard-cli/src/javascript_syntax_probe.rs)


现有验收/样例：

- [tests/acceptance/typescript-syntax-fallback-candidate.md](../../tests/acceptance/typescript-syntax-fallback-candidate.md)


必须解决的登记缺口：

- typescript/npm:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- typescript/npm/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 deno / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- typescript/deno:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- typescript/deno/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：7.3, 15.3, 15.6, 15.7。

#### 构建路径 npm / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- typescript/npm: JSDoc/TSDoc与TypeScript/JSX声明模式及重载契约的完整原生规则/合法反例/契约准确性未验收

- typescript/npm/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 deno / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- typescript/deno: JSDoc/TSDoc与TypeScript/JSX声明模式及重载契约的完整原生规则/合法反例/契约准确性未验收

- typescript/deno/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：7.3, 15.4, 15.6, 15.7。

#### 构建路径 npm / partial

现有代码入口：

- [crates/codeguard-cli/src/eslint_lint_command.rs](../../crates/codeguard-cli/src/eslint_lint_command.rs)


现有验收/样例：

- [tests/acceptance/eslint-directory-feedback.md](../../tests/acceptance/eslint-directory-feedback.md)


必须解决的登记缺口：

- typescript/npm:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- typescript/npm/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 deno / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- typescript/deno:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- typescript/deno/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：7.3, 15.5, 15.6, 15.7。

#### 构建路径 npm / partial

现有代码入口：

- [crates/codeguard-cli/src/npm_audit_command.rs](../../crates/codeguard-cli/src/npm_audit_command.rs)


现有验收/样例：

- [tests/acceptance/npm-partial-native.md](../../tests/acceptance/npm-partial-native.md)


必须解决的登记缺口：

- typescript/npm:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- typescript/npm/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 deno / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- typescript/deno:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- typescript/deno/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## python

- 构建生态：pip, poetry, uv, pdm。grammar候选：python。

- 版本/方言约束：docstring用途/参数/Returns/Yields/Raises及Google/NumPy约定。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["ruff", "check", "."]`。

### syntax — blocked

OpenSpec任务：7.2, 15.2, 15.6, 15.7。

#### 构建路径 pip / partial

现有代码入口：

- [crates/codeguard-cli/src/ruff_probe.rs](../../crates/codeguard-cli/src/ruff_probe.rs)


现有验收/样例：

- [tests/acceptance/python-syntax-fallback-candidate.md](../../tests/acceptance/python-syntax-fallback-candidate.md)


必须解决的登记缺口：

- python/pip:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- python/pip/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 poetry / partial

现有代码入口：

- [crates/codeguard-cli/src/ruff_probe.rs](../../crates/codeguard-cli/src/ruff_probe.rs)


现有验收/样例：

- [tests/acceptance/python-syntax-fallback-candidate.md](../../tests/acceptance/python-syntax-fallback-candidate.md)


必须解决的登记缺口：

- python/poetry:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- python/poetry/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 uv / partial

现有代码入口：

- [crates/codeguard-cli/src/ruff_probe.rs](../../crates/codeguard-cli/src/ruff_probe.rs)


现有验收/样例：

- [tests/acceptance/python-syntax-fallback-candidate.md](../../tests/acceptance/python-syntax-fallback-candidate.md)


必须解决的登记缺口：

- python/uv:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- python/uv/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 pdm / partial

现有代码入口：

- [crates/codeguard-cli/src/ruff_probe.rs](../../crates/codeguard-cli/src/ruff_probe.rs)


现有验收/样例：

- [tests/acceptance/python-syntax-fallback-candidate.md](../../tests/acceptance/python-syntax-fallback-candidate.md)


必须解决的登记缺口：

- python/pdm:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- python/pdm/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：7.2, 15.3, 15.6, 15.7。

#### 构建路径 pip / partial

现有代码入口：

- [crates/codeguard-cli/src/python_lint_scan.rs](../../crates/codeguard-cli/src/python_lint_scan.rs)

- [crates/codeguard-cli/src/python_comments_command.rs](../../crates/codeguard-cli/src/python_comments_command.rs)

- [crates/codeguard-cli/src/python_comments_arguments.rs](../../crates/codeguard-cli/src/python_comments_arguments.rs)

- [crates/codeguard-cli/src/python_lint_command.rs](../../crates/codeguard-cli/src/python_lint_command.rs)

- [crates/codeguard-cli/src/next_command.rs](../../crates/codeguard-cli/src/next_command.rs)

- [crates/codeguard-cli/src/python_documentation_configuration.rs](../../crates/codeguard-cli/src/python_documentation_configuration.rs)

- [crates/codeguard-adapters/src/ruff_settings.rs](../../crates/codeguard-adapters/src/ruff_settings.rs)


现有验收/样例：

- [tests/acceptance/ruff-documentation-contract.md](../../tests/acceptance/ruff-documentation-contract.md)

- [tests/acceptance/python-comments-cli.md](../../tests/acceptance/python-comments-cli.md)

- [crates/codeguard-cli/tests/python_comments_cli.rs](../../crates/codeguard-cli/tests/python_comments_cli.rs)

- [crates/codeguard-cli/tests/ruff_documentation_contract.rs](../../crates/codeguard-cli/tests/ruff_documentation_contract.rs)

- [schemas/python-comments-feedback-v0.1.schema.json](../../schemas/python-comments-feedback-v0.1.schema.json)

- [tests/acceptance/evidence/python-comments-native.json](../../tests/acceptance/evidence/python-comments-native.json)

- [tests/acceptance/evidence/python-comments-native-wasm.json](../../tests/acceptance/evidence/python-comments-native-wasm.json)

- [tests/acceptance/evidence/python-comments-native-boundaries.json](../../tests/acceptance/evidence/python-comments-native-boundaries.json)

- [tests/acceptance/evidence/python-comments-schema.json](../../tests/acceptance/evidence/python-comments-schema.json)

- [tests/acceptance/evidence/python-comments-native-boundaries-wasm.json](../../tests/acceptance/evidence/python-comments-native-boundaries-wasm.json)

- [tests/acceptance/python-documentation-configuration.md](../../tests/acceptance/python-documentation-configuration.md)

- [schemas/python-comments-feedback-v0.2.schema.json](../../schemas/python-comments-feedback-v0.2.schema.json)

- [crates/codeguard-adapters/tests/ruff_settings_contract.rs](../../crates/codeguard-adapters/tests/ruff_settings_contract.rs)

- [tests/acceptance/evidence/python-documentation-configuration-native.json](../../tests/acceptance/evidence/python-documentation-configuration-native.json)

- [tests/acceptance/evidence/python-documentation-configuration-native-wasm.json](../../tests/acceptance/evidence/python-documentation-configuration-native-wasm.json)

- [tests/acceptance/evidence/python-documentation-configuration-boundaries.json](../../tests/acceptance/evidence/python-documentation-configuration-boundaries.json)

- [tests/acceptance/evidence/python-documentation-configuration-boundaries-wasm.json](../../tests/acceptance/evidence/python-documentation-configuration-boundaries-wasm.json)

- [tests/acceptance/evidence/python-documentation-configuration-schema.json](../../tests/acceptance/evidence/python-documentation-configuration-schema.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/cancelled.json](../../tests/acceptance/evidence/python-documentation-configuration/default/cancelled.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/clean-selected.json](../../tests/acceptance/evidence/python-documentation-configuration/default/clean-selected.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/missing-configuration.json](../../tests/acceptance/evidence/python-documentation-configuration/default/missing-configuration.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/nested-observed.json](../../tests/acceptance/evidence/python-documentation-configuration/default/nested-observed.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/no-python-sources.json](../../tests/acceptance/evidence/python-documentation-configuration/default/no-python-sources.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/partial.json](../../tests/acceptance/evidence/python-documentation-configuration/default/partial.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/per-file-ignore.json](../../tests/acceptance/evidence/python-documentation-configuration/default/per-file-ignore.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/root-unavailable.json](../../tests/acceptance/evidence/python-documentation-configuration/default/root-unavailable.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/timeout.json](../../tests/acceptance/evidence/python-documentation-configuration/default/timeout.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/unselected.json](../../tests/acceptance/evidence/python-documentation-configuration/default/unselected.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/cancelled.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/cancelled.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/clean-selected.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/clean-selected.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/missing-configuration.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/missing-configuration.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/nested-observed.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/nested-observed.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/no-python-sources.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/no-python-sources.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/partial.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/partial.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/per-file-ignore.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/per-file-ignore.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/root-unavailable.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/root-unavailable.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/timeout.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/timeout.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/unselected.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/unselected.json)


必须解决的登记缺口：

- python/pip: docstring用途/参数/Returns/Yields/Raises及Google/NumPy约定的完整原生规则/合法反例/契约准确性未验收

- python/pip/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 poetry / partial

现有代码入口：

- [crates/codeguard-cli/src/python_lint_scan.rs](../../crates/codeguard-cli/src/python_lint_scan.rs)

- [crates/codeguard-cli/src/python_comments_command.rs](../../crates/codeguard-cli/src/python_comments_command.rs)

- [crates/codeguard-cli/src/python_comments_arguments.rs](../../crates/codeguard-cli/src/python_comments_arguments.rs)

- [crates/codeguard-cli/src/python_lint_command.rs](../../crates/codeguard-cli/src/python_lint_command.rs)

- [crates/codeguard-cli/src/next_command.rs](../../crates/codeguard-cli/src/next_command.rs)

- [crates/codeguard-cli/src/python_documentation_configuration.rs](../../crates/codeguard-cli/src/python_documentation_configuration.rs)

- [crates/codeguard-adapters/src/ruff_settings.rs](../../crates/codeguard-adapters/src/ruff_settings.rs)


现有验收/样例：

- [tests/acceptance/ruff-documentation-contract.md](../../tests/acceptance/ruff-documentation-contract.md)

- [tests/acceptance/python-comments-cli.md](../../tests/acceptance/python-comments-cli.md)

- [crates/codeguard-cli/tests/python_comments_cli.rs](../../crates/codeguard-cli/tests/python_comments_cli.rs)

- [crates/codeguard-cli/tests/ruff_documentation_contract.rs](../../crates/codeguard-cli/tests/ruff_documentation_contract.rs)

- [schemas/python-comments-feedback-v0.1.schema.json](../../schemas/python-comments-feedback-v0.1.schema.json)

- [tests/acceptance/evidence/python-comments-native.json](../../tests/acceptance/evidence/python-comments-native.json)

- [tests/acceptance/evidence/python-comments-native-wasm.json](../../tests/acceptance/evidence/python-comments-native-wasm.json)

- [tests/acceptance/evidence/python-comments-native-boundaries.json](../../tests/acceptance/evidence/python-comments-native-boundaries.json)

- [tests/acceptance/evidence/python-comments-schema.json](../../tests/acceptance/evidence/python-comments-schema.json)

- [tests/acceptance/evidence/python-comments-native-boundaries-wasm.json](../../tests/acceptance/evidence/python-comments-native-boundaries-wasm.json)

- [tests/acceptance/python-documentation-configuration.md](../../tests/acceptance/python-documentation-configuration.md)

- [schemas/python-comments-feedback-v0.2.schema.json](../../schemas/python-comments-feedback-v0.2.schema.json)

- [crates/codeguard-adapters/tests/ruff_settings_contract.rs](../../crates/codeguard-adapters/tests/ruff_settings_contract.rs)

- [tests/acceptance/evidence/python-documentation-configuration-native.json](../../tests/acceptance/evidence/python-documentation-configuration-native.json)

- [tests/acceptance/evidence/python-documentation-configuration-native-wasm.json](../../tests/acceptance/evidence/python-documentation-configuration-native-wasm.json)

- [tests/acceptance/evidence/python-documentation-configuration-boundaries.json](../../tests/acceptance/evidence/python-documentation-configuration-boundaries.json)

- [tests/acceptance/evidence/python-documentation-configuration-boundaries-wasm.json](../../tests/acceptance/evidence/python-documentation-configuration-boundaries-wasm.json)

- [tests/acceptance/evidence/python-documentation-configuration-schema.json](../../tests/acceptance/evidence/python-documentation-configuration-schema.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/cancelled.json](../../tests/acceptance/evidence/python-documentation-configuration/default/cancelled.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/clean-selected.json](../../tests/acceptance/evidence/python-documentation-configuration/default/clean-selected.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/missing-configuration.json](../../tests/acceptance/evidence/python-documentation-configuration/default/missing-configuration.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/nested-observed.json](../../tests/acceptance/evidence/python-documentation-configuration/default/nested-observed.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/no-python-sources.json](../../tests/acceptance/evidence/python-documentation-configuration/default/no-python-sources.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/partial.json](../../tests/acceptance/evidence/python-documentation-configuration/default/partial.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/per-file-ignore.json](../../tests/acceptance/evidence/python-documentation-configuration/default/per-file-ignore.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/root-unavailable.json](../../tests/acceptance/evidence/python-documentation-configuration/default/root-unavailable.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/timeout.json](../../tests/acceptance/evidence/python-documentation-configuration/default/timeout.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/unselected.json](../../tests/acceptance/evidence/python-documentation-configuration/default/unselected.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/cancelled.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/cancelled.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/clean-selected.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/clean-selected.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/missing-configuration.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/missing-configuration.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/nested-observed.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/nested-observed.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/no-python-sources.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/no-python-sources.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/partial.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/partial.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/per-file-ignore.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/per-file-ignore.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/root-unavailable.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/root-unavailable.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/timeout.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/timeout.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/unselected.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/unselected.json)


必须解决的登记缺口：

- python/poetry: docstring用途/参数/Returns/Yields/Raises及Google/NumPy约定的完整原生规则/合法反例/契约准确性未验收

- python/poetry/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 uv / partial

现有代码入口：

- [crates/codeguard-cli/src/python_lint_scan.rs](../../crates/codeguard-cli/src/python_lint_scan.rs)

- [crates/codeguard-cli/src/python_comments_command.rs](../../crates/codeguard-cli/src/python_comments_command.rs)

- [crates/codeguard-cli/src/python_comments_arguments.rs](../../crates/codeguard-cli/src/python_comments_arguments.rs)

- [crates/codeguard-cli/src/python_lint_command.rs](../../crates/codeguard-cli/src/python_lint_command.rs)

- [crates/codeguard-cli/src/next_command.rs](../../crates/codeguard-cli/src/next_command.rs)

- [crates/codeguard-cli/src/python_documentation_configuration.rs](../../crates/codeguard-cli/src/python_documentation_configuration.rs)

- [crates/codeguard-adapters/src/ruff_settings.rs](../../crates/codeguard-adapters/src/ruff_settings.rs)


现有验收/样例：

- [tests/acceptance/ruff-documentation-contract.md](../../tests/acceptance/ruff-documentation-contract.md)

- [tests/acceptance/python-comments-cli.md](../../tests/acceptance/python-comments-cli.md)

- [crates/codeguard-cli/tests/python_comments_cli.rs](../../crates/codeguard-cli/tests/python_comments_cli.rs)

- [crates/codeguard-cli/tests/ruff_documentation_contract.rs](../../crates/codeguard-cli/tests/ruff_documentation_contract.rs)

- [schemas/python-comments-feedback-v0.1.schema.json](../../schemas/python-comments-feedback-v0.1.schema.json)

- [tests/acceptance/evidence/python-comments-native.json](../../tests/acceptance/evidence/python-comments-native.json)

- [tests/acceptance/evidence/python-comments-native-wasm.json](../../tests/acceptance/evidence/python-comments-native-wasm.json)

- [tests/acceptance/evidence/python-comments-native-boundaries.json](../../tests/acceptance/evidence/python-comments-native-boundaries.json)

- [tests/acceptance/evidence/python-comments-schema.json](../../tests/acceptance/evidence/python-comments-schema.json)

- [tests/acceptance/evidence/python-comments-native-boundaries-wasm.json](../../tests/acceptance/evidence/python-comments-native-boundaries-wasm.json)

- [tests/acceptance/python-documentation-configuration.md](../../tests/acceptance/python-documentation-configuration.md)

- [schemas/python-comments-feedback-v0.2.schema.json](../../schemas/python-comments-feedback-v0.2.schema.json)

- [crates/codeguard-adapters/tests/ruff_settings_contract.rs](../../crates/codeguard-adapters/tests/ruff_settings_contract.rs)

- [tests/acceptance/evidence/python-documentation-configuration-native.json](../../tests/acceptance/evidence/python-documentation-configuration-native.json)

- [tests/acceptance/evidence/python-documentation-configuration-native-wasm.json](../../tests/acceptance/evidence/python-documentation-configuration-native-wasm.json)

- [tests/acceptance/evidence/python-documentation-configuration-boundaries.json](../../tests/acceptance/evidence/python-documentation-configuration-boundaries.json)

- [tests/acceptance/evidence/python-documentation-configuration-boundaries-wasm.json](../../tests/acceptance/evidence/python-documentation-configuration-boundaries-wasm.json)

- [tests/acceptance/evidence/python-documentation-configuration-schema.json](../../tests/acceptance/evidence/python-documentation-configuration-schema.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/cancelled.json](../../tests/acceptance/evidence/python-documentation-configuration/default/cancelled.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/clean-selected.json](../../tests/acceptance/evidence/python-documentation-configuration/default/clean-selected.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/missing-configuration.json](../../tests/acceptance/evidence/python-documentation-configuration/default/missing-configuration.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/nested-observed.json](../../tests/acceptance/evidence/python-documentation-configuration/default/nested-observed.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/no-python-sources.json](../../tests/acceptance/evidence/python-documentation-configuration/default/no-python-sources.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/partial.json](../../tests/acceptance/evidence/python-documentation-configuration/default/partial.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/per-file-ignore.json](../../tests/acceptance/evidence/python-documentation-configuration/default/per-file-ignore.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/root-unavailable.json](../../tests/acceptance/evidence/python-documentation-configuration/default/root-unavailable.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/timeout.json](../../tests/acceptance/evidence/python-documentation-configuration/default/timeout.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/unselected.json](../../tests/acceptance/evidence/python-documentation-configuration/default/unselected.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/cancelled.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/cancelled.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/clean-selected.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/clean-selected.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/missing-configuration.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/missing-configuration.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/nested-observed.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/nested-observed.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/no-python-sources.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/no-python-sources.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/partial.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/partial.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/per-file-ignore.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/per-file-ignore.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/root-unavailable.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/root-unavailable.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/timeout.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/timeout.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/unselected.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/unselected.json)


必须解决的登记缺口：

- python/uv: docstring用途/参数/Returns/Yields/Raises及Google/NumPy约定的完整原生规则/合法反例/契约准确性未验收

- python/uv/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 pdm / partial

现有代码入口：

- [crates/codeguard-cli/src/python_lint_scan.rs](../../crates/codeguard-cli/src/python_lint_scan.rs)

- [crates/codeguard-cli/src/python_comments_command.rs](../../crates/codeguard-cli/src/python_comments_command.rs)

- [crates/codeguard-cli/src/python_comments_arguments.rs](../../crates/codeguard-cli/src/python_comments_arguments.rs)

- [crates/codeguard-cli/src/python_lint_command.rs](../../crates/codeguard-cli/src/python_lint_command.rs)

- [crates/codeguard-cli/src/next_command.rs](../../crates/codeguard-cli/src/next_command.rs)

- [crates/codeguard-cli/src/python_documentation_configuration.rs](../../crates/codeguard-cli/src/python_documentation_configuration.rs)

- [crates/codeguard-adapters/src/ruff_settings.rs](../../crates/codeguard-adapters/src/ruff_settings.rs)


现有验收/样例：

- [tests/acceptance/ruff-documentation-contract.md](../../tests/acceptance/ruff-documentation-contract.md)

- [tests/acceptance/python-comments-cli.md](../../tests/acceptance/python-comments-cli.md)

- [crates/codeguard-cli/tests/python_comments_cli.rs](../../crates/codeguard-cli/tests/python_comments_cli.rs)

- [crates/codeguard-cli/tests/ruff_documentation_contract.rs](../../crates/codeguard-cli/tests/ruff_documentation_contract.rs)

- [schemas/python-comments-feedback-v0.1.schema.json](../../schemas/python-comments-feedback-v0.1.schema.json)

- [tests/acceptance/evidence/python-comments-native.json](../../tests/acceptance/evidence/python-comments-native.json)

- [tests/acceptance/evidence/python-comments-native-wasm.json](../../tests/acceptance/evidence/python-comments-native-wasm.json)

- [tests/acceptance/evidence/python-comments-native-boundaries.json](../../tests/acceptance/evidence/python-comments-native-boundaries.json)

- [tests/acceptance/evidence/python-comments-schema.json](../../tests/acceptance/evidence/python-comments-schema.json)

- [tests/acceptance/evidence/python-comments-native-boundaries-wasm.json](../../tests/acceptance/evidence/python-comments-native-boundaries-wasm.json)

- [tests/acceptance/python-documentation-configuration.md](../../tests/acceptance/python-documentation-configuration.md)

- [schemas/python-comments-feedback-v0.2.schema.json](../../schemas/python-comments-feedback-v0.2.schema.json)

- [crates/codeguard-adapters/tests/ruff_settings_contract.rs](../../crates/codeguard-adapters/tests/ruff_settings_contract.rs)

- [tests/acceptance/evidence/python-documentation-configuration-native.json](../../tests/acceptance/evidence/python-documentation-configuration-native.json)

- [tests/acceptance/evidence/python-documentation-configuration-native-wasm.json](../../tests/acceptance/evidence/python-documentation-configuration-native-wasm.json)

- [tests/acceptance/evidence/python-documentation-configuration-boundaries.json](../../tests/acceptance/evidence/python-documentation-configuration-boundaries.json)

- [tests/acceptance/evidence/python-documentation-configuration-boundaries-wasm.json](../../tests/acceptance/evidence/python-documentation-configuration-boundaries-wasm.json)

- [tests/acceptance/evidence/python-documentation-configuration-schema.json](../../tests/acceptance/evidence/python-documentation-configuration-schema.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/cancelled.json](../../tests/acceptance/evidence/python-documentation-configuration/default/cancelled.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/clean-selected.json](../../tests/acceptance/evidence/python-documentation-configuration/default/clean-selected.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/missing-configuration.json](../../tests/acceptance/evidence/python-documentation-configuration/default/missing-configuration.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/nested-observed.json](../../tests/acceptance/evidence/python-documentation-configuration/default/nested-observed.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/no-python-sources.json](../../tests/acceptance/evidence/python-documentation-configuration/default/no-python-sources.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/partial.json](../../tests/acceptance/evidence/python-documentation-configuration/default/partial.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/per-file-ignore.json](../../tests/acceptance/evidence/python-documentation-configuration/default/per-file-ignore.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/root-unavailable.json](../../tests/acceptance/evidence/python-documentation-configuration/default/root-unavailable.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/timeout.json](../../tests/acceptance/evidence/python-documentation-configuration/default/timeout.json)

- [tests/acceptance/evidence/python-documentation-configuration/default/unselected.json](../../tests/acceptance/evidence/python-documentation-configuration/default/unselected.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/cancelled.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/cancelled.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/clean-selected.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/clean-selected.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/missing-configuration.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/missing-configuration.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/nested-observed.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/nested-observed.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/no-python-sources.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/no-python-sources.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/partial.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/partial.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/per-file-ignore.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/per-file-ignore.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/root-unavailable.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/root-unavailable.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/timeout.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/timeout.json)

- [tests/acceptance/evidence/python-documentation-configuration/wasm/unselected.json](../../tests/acceptance/evidence/python-documentation-configuration/wasm/unselected.json)


必须解决的登记缺口：

- python/pdm: docstring用途/参数/Returns/Yields/Raises及Google/NumPy约定的完整原生规则/合法反例/契约准确性未验收

- python/pdm/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：7.2, 15.4, 15.6, 15.7。

#### 构建路径 pip / partial

现有代码入口：

- [crates/codeguard-cli/src/ruff_probe.rs](../../crates/codeguard-cli/src/ruff_probe.rs)


现有验收/样例：

- [tests/acceptance/python-lint-scan.md](../../tests/acceptance/python-lint-scan.md)


必须解决的登记缺口：

- python/pip:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- python/pip/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 poetry / partial

现有代码入口：

- [crates/codeguard-cli/src/ruff_probe.rs](../../crates/codeguard-cli/src/ruff_probe.rs)


现有验收/样例：

- [tests/acceptance/python-lint-scan.md](../../tests/acceptance/python-lint-scan.md)


必须解决的登记缺口：

- python/poetry:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- python/poetry/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 uv / partial

现有代码入口：

- [crates/codeguard-cli/src/ruff_probe.rs](../../crates/codeguard-cli/src/ruff_probe.rs)


现有验收/样例：

- [tests/acceptance/python-lint-scan.md](../../tests/acceptance/python-lint-scan.md)


必须解决的登记缺口：

- python/uv:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- python/uv/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 pdm / partial

现有代码入口：

- [crates/codeguard-cli/src/ruff_probe.rs](../../crates/codeguard-cli/src/ruff_probe.rs)


现有验收/样例：

- [tests/acceptance/python-lint-scan.md](../../tests/acceptance/python-lint-scan.md)


必须解决的登记缺口：

- python/pdm:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- python/pdm/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：7.2, 15.5, 15.6, 15.7。

#### 构建路径 pip / partial

现有代码入口：

- [crates/codeguard-cli/src/python_cve_command.rs](../../crates/codeguard-cli/src/python_cve_command.rs)


现有验收/样例：

- [tests/acceptance/python-cve-partial-native.md](../../tests/acceptance/python-cve-partial-native.md)


必须解决的登记缺口：

- python/pip:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- python/pip/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 poetry / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- python/poetry:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- python/poetry/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 uv / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- python/uv:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- python/uv/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 pdm / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- python/pdm:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- python/pdm/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## go

- 构建生态：go-modules。grammar候选：go。

- 版本/方言约束：导出API的Go文档与参数/返回/错误及泛型契约。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["go", "vet", "./..."]`。

### syntax — blocked

OpenSpec任务：8.1, 8.2, 8.3, 15.2, 15.6, 15.7。

#### 构建路径 go-modules / partial

现有代码入口：

- [crates/codeguard-cli/src/go_lint_command.rs](../../crates/codeguard-cli/src/go_lint_command.rs)


现有验收/样例：

- [tests/acceptance/go-native-differential.md](../../tests/acceptance/go-native-differential.md)


必须解决的登记缺口：

- go/go-modules:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- go/go-modules/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.1, 8.2, 8.3, 15.3, 15.6, 15.7。

#### 构建路径 go-modules / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- go/go-modules: 导出API的Go文档与参数/返回/错误及泛型契约的完整原生规则/合法反例/契约准确性未验收

- go/go-modules/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.1, 8.2, 8.3, 15.4, 15.6, 15.7。

#### 构建路径 go-modules / partial

现有代码入口：

- [crates/codeguard-cli/src/go_lint_command.rs](../../crates/codeguard-cli/src/go_lint_command.rs)


现有验收/样例：

- [tests/acceptance/go-vet-json-local-probe.md](../../tests/acceptance/go-vet-json-local-probe.md)


必须解决的登记缺口：

- go/go-modules:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- go/go-modules/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.1, 8.2, 8.3, 15.5, 15.6, 15.7。

#### 构建路径 go-modules / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- go/go-modules:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- go/go-modules/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## csharp

- 构建生态：dotnet。grammar候选：csharp。

- 版本/方言约束：XML documentation与参数/返回/异常及生成代码。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["dotnet", "format", "--verify-no-changes"]`。

### syntax — blocked

OpenSpec任务：8.4, 8.5, 8.6, 15.2, 15.6, 15.7。

#### 构建路径 dotnet / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- csharp/dotnet:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- csharp/dotnet/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.4, 8.5, 8.6, 15.3, 15.6, 15.7。

#### 构建路径 dotnet / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- csharp/dotnet: XML documentation与参数/返回/异常及生成代码的完整原生规则/合法反例/契约准确性未验收

- csharp/dotnet/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.4, 8.5, 8.6, 15.4, 15.6, 15.7。

#### 构建路径 dotnet / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- csharp/dotnet:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- csharp/dotnet/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.4, 8.5, 8.6, 15.5, 15.6, 15.7。

#### 构建路径 dotnet / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- csharp/dotnet:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- csharp/dotnet/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## kotlin

- 构建生态：gradle, maven。grammar候选：kotlin。

- 版本/方言约束：KDoc与参数/返回/异常及Java互操作。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["./gradlew", "detekt"]`。

### syntax — blocked

OpenSpec任务：8.7, 8.8, 8.9, 15.2, 15.6, 15.7。

#### 构建路径 gradle / partial

现有代码入口：

- [crates/codeguard-cli/src/kotlin_lint_command.rs](../../crates/codeguard-cli/src/kotlin_lint_command.rs)


现有验收/样例：

- [tests/acceptance/kotlin-native-differential.md](../../tests/acceptance/kotlin-native-differential.md)


必须解决的登记缺口：

- kotlin/gradle:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- kotlin/gradle/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 maven / partial

现有代码入口：

- [crates/codeguard-cli/src/kotlin_lint_command.rs](../../crates/codeguard-cli/src/kotlin_lint_command.rs)


现有验收/样例：

- [tests/acceptance/kotlin-native-differential.md](../../tests/acceptance/kotlin-native-differential.md)


必须解决的登记缺口：

- kotlin/maven:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- kotlin/maven/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.7, 8.8, 8.9, 15.3, 15.6, 15.7。

#### 构建路径 gradle / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- kotlin/gradle: KDoc与参数/返回/异常及Java互操作的完整原生规则/合法反例/契约准确性未验收

- kotlin/gradle/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 maven / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- kotlin/maven: KDoc与参数/返回/异常及Java互操作的完整原生规则/合法反例/契约准确性未验收

- kotlin/maven/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.7, 8.8, 8.9, 15.4, 15.6, 15.7。

#### 构建路径 gradle / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- kotlin/gradle:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- kotlin/gradle/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 maven / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- kotlin/maven:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- kotlin/maven/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.7, 8.8, 8.9, 15.5, 15.6, 15.7。

#### 构建路径 gradle / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- kotlin/gradle:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- kotlin/gradle/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 maven / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- kotlin/maven:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- kotlin/maven/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## swift

- 构建生态：swift-package-manager, xcode。grammar候选：swift。

- 版本/方言约束：Swift API documentation及参数/返回/Throws与并发。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["swiftlint"]`。

### syntax — blocked

OpenSpec任务：8.10, 8.11, 8.12, 15.2, 15.6, 15.7。

#### 构建路径 swift-package-manager / partial

现有代码入口：

- [crates/codeguard-cli/src/swift_lint_command.rs](../../crates/codeguard-cli/src/swift_lint_command.rs)


现有验收/样例：

- [tests/acceptance/swift-native-project.md](../../tests/acceptance/swift-native-project.md)


必须解决的登记缺口：

- swift/swift-package-manager:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- swift/swift-package-manager/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 xcode / partial

现有代码入口：

- [crates/codeguard-cli/src/swift_lint_command.rs](../../crates/codeguard-cli/src/swift_lint_command.rs)


现有验收/样例：

- [tests/acceptance/swift-native-project.md](../../tests/acceptance/swift-native-project.md)


必须解决的登记缺口：

- swift/xcode:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- swift/xcode/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.10, 8.11, 8.12, 15.3, 15.6, 15.7。

#### 构建路径 swift-package-manager / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- swift/swift-package-manager: Swift API documentation及参数/返回/Throws与并发的完整原生规则/合法反例/契约准确性未验收

- swift/swift-package-manager/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 xcode / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- swift/xcode: Swift API documentation及参数/返回/Throws与并发的完整原生规则/合法反例/契约准确性未验收

- swift/xcode/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.10, 8.11, 8.12, 15.4, 15.6, 15.7。

#### 构建路径 swift-package-manager / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- swift/swift-package-manager:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- swift/swift-package-manager/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 xcode / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- swift/xcode:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- swift/xcode/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.10, 8.11, 8.12, 15.5, 15.6, 15.7。

#### 构建路径 swift-package-manager / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- swift/swift-package-manager:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- swift/swift-package-manager/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 xcode / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- swift/xcode:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- swift/xcode/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## php

- 构建生态：composer。grammar候选：php。

- 版本/方言约束：PHPDoc与参数/返回/throws及类型契约。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["php", "-l", "{file}"]`。

### syntax — blocked

OpenSpec任务：8.13, 8.14, 8.15, 15.2, 15.6, 15.7。

#### 构建路径 composer / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- php/composer:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- php/composer/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.13, 8.14, 8.15, 15.3, 15.6, 15.7。

#### 构建路径 composer / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- php/composer: PHPDoc与参数/返回/throws及类型契约的完整原生规则/合法反例/契约准确性未验收

- php/composer/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.13, 8.14, 8.15, 15.4, 15.6, 15.7。

#### 构建路径 composer / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- php/composer:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- php/composer/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.13, 8.14, 8.15, 15.5, 15.6, 15.7。

#### 构建路径 composer / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- php/composer:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- php/composer/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## ruby

- 构建生态：bundler, rubygems。grammar候选：ruby。

- 版本/方言约束：Ruby/YARD文档及参数/返回/raise与DSL。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["rubocop"]`。

### syntax — blocked

OpenSpec任务：8.16, 8.17, 8.18, 15.2, 15.6, 15.7。

#### 构建路径 bundler / partial

现有代码入口：

- [crates/codeguard-cli/src/ruby_lint_command.rs](../../crates/codeguard-cli/src/ruby_lint_command.rs)


现有验收/样例：

- [tests/acceptance/ruby-native-entry.md](../../tests/acceptance/ruby-native-entry.md)


必须解决的登记缺口：

- ruby/bundler:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- ruby/bundler/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 rubygems / partial

现有代码入口：

- [crates/codeguard-cli/src/ruby_lint_command.rs](../../crates/codeguard-cli/src/ruby_lint_command.rs)


现有验收/样例：

- [tests/acceptance/ruby-native-entry.md](../../tests/acceptance/ruby-native-entry.md)


必须解决的登记缺口：

- ruby/rubygems:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- ruby/rubygems/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.16, 8.17, 8.18, 15.3, 15.6, 15.7。

#### 构建路径 bundler / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- ruby/bundler: Ruby/YARD文档及参数/返回/raise与DSL的完整原生规则/合法反例/契约准确性未验收

- ruby/bundler/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 rubygems / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- ruby/rubygems: Ruby/YARD文档及参数/返回/raise与DSL的完整原生规则/合法反例/契约准确性未验收

- ruby/rubygems/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.16, 8.17, 8.18, 15.4, 15.6, 15.7。

#### 构建路径 bundler / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- ruby/bundler:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- ruby/bundler/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 rubygems / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- ruby/rubygems:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- ruby/rubygems/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.16, 8.17, 8.18, 15.5, 15.6, 15.7。

#### 构建路径 bundler / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- ruby/bundler:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- ruby/bundler/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 rubygems / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- ruby/rubygems:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- ruby/rubygems/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## scala

- 构建生态：sbt, scala-cli。grammar候选：scala。

- 版本/方言约束：Scaladoc与类型参数/返回/throws及Scala版本。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["scalafmt", "--check", "."]`。

### syntax — blocked

OpenSpec任务：8.19, 8.20, 8.21, 15.2, 15.6, 15.7。

#### 构建路径 sbt / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- scala/sbt:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- scala/sbt/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 scala-cli / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- scala/scala-cli:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- scala/scala-cli/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.19, 8.20, 8.21, 15.3, 15.6, 15.7。

#### 构建路径 sbt / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- scala/sbt: Scaladoc与类型参数/返回/throws及Scala版本的完整原生规则/合法反例/契约准确性未验收

- scala/sbt/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 scala-cli / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- scala/scala-cli: Scaladoc与类型参数/返回/throws及Scala版本的完整原生规则/合法反例/契约准确性未验收

- scala/scala-cli/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.19, 8.20, 8.21, 15.4, 15.6, 15.7。

#### 构建路径 sbt / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- scala/sbt:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- scala/sbt/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 scala-cli / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- scala/scala-cli:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- scala/scala-cli/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.19, 8.20, 8.21, 15.5, 15.6, 15.7。

#### 构建路径 sbt / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- scala/sbt:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- scala/sbt/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 scala-cli / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- scala/scala-cli:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- scala/scala-cli/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## shell

- 构建生态：standalone-shell, enclosing-project。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：shell函数/脚本用途、参数、退出码和环境副作用。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["shellcheck", "--severity=warning", "{file}"]`。

### syntax — blocked

OpenSpec任务：7.4, 15.2, 15.6, 15.7。

#### 构建路径 standalone-shell / partial

现有代码入口：

- [crates/codeguard-cli/src/shell_lint_command.rs](../../crates/codeguard-cli/src/shell_lint_command.rs)


现有验收/样例：

- [tests/acceptance/shellcheck-workbench-baseline.md](../../tests/acceptance/shellcheck-workbench-baseline.md)


必须解决的登记缺口：

- shell/standalone-shell:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- shell/standalone-shell/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 enclosing-project / partial

现有代码入口：

- [crates/codeguard-cli/src/shell_lint_command.rs](../../crates/codeguard-cli/src/shell_lint_command.rs)


现有验收/样例：

- [tests/acceptance/shellcheck-workbench-baseline.md](../../tests/acceptance/shellcheck-workbench-baseline.md)


必须解决的登记缺口：

- shell/enclosing-project:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- shell/enclosing-project/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：7.4, 15.3, 15.6, 15.7。

#### 构建路径 standalone-shell / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- shell/standalone-shell: shell函数/脚本用途、参数、退出码和环境副作用的完整原生规则/合法反例/契约准确性未验收

- shell/standalone-shell/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 enclosing-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- shell/enclosing-project: shell函数/脚本用途、参数、退出码和环境副作用的完整原生规则/合法反例/契约准确性未验收

- shell/enclosing-project/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：7.4, 15.4, 15.6, 15.7。

#### 构建路径 standalone-shell / partial

现有代码入口：

- [crates/codeguard-cli/src/shell_lint_command.rs](../../crates/codeguard-cli/src/shell_lint_command.rs)


现有验收/样例：

- [tests/acceptance/shellcheck-workbench-baseline.md](../../tests/acceptance/shellcheck-workbench-baseline.md)


必须解决的登记缺口：

- shell/standalone-shell:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- shell/standalone-shell/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 enclosing-project / partial

现有代码入口：

- [crates/codeguard-cli/src/shell_lint_command.rs](../../crates/codeguard-cli/src/shell_lint_command.rs)


现有验收/样例：

- [tests/acceptance/shellcheck-workbench-baseline.md](../../tests/acceptance/shellcheck-workbench-baseline.md)


必须解决的登记缺口：

- shell/enclosing-project:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- shell/enclosing-project/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：7.4, 15.5, 15.6, 15.7。

#### 构建路径 standalone-shell / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- shell/standalone-shell:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- shell/standalone-shell/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 enclosing-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- shell/enclosing-project:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- shell/enclosing-project/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## dockerfile

- 构建生态：container-build。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：构建阶段/镜像依赖/运行入口和安全假设说明。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["hadolint", "{file}"]`。

### syntax — blocked

OpenSpec任务：7.4, 15.2, 15.6, 15.7。

#### 构建路径 container-build / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- dockerfile/container-build:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- dockerfile/container-build/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：7.4, 15.3, 15.6, 15.7。

#### 构建路径 container-build / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- dockerfile/container-build: 构建阶段/镜像依赖/运行入口和安全假设说明的完整原生规则/合法反例/契约准确性未验收

- dockerfile/container-build/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：7.4, 15.4, 15.6, 15.7。

#### 构建路径 container-build / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- dockerfile/container-build:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- dockerfile/container-build/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：7.4, 15.5, 15.6, 15.7。

#### 构建路径 container-build / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- dockerfile/container-build:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- dockerfile/container-build/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## yaml

- 构建生态：enclosing-project。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：所属配置schema、字段用途、默认值和安全约束说明。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["yamllint", "."]`。

### syntax — blocked

OpenSpec任务：8.70, 8.71, 8.72, 15.2, 15.6, 15.7。

#### 构建路径 enclosing-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- yaml/enclosing-project:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- yaml/enclosing-project/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.70, 8.71, 8.72, 15.3, 15.6, 15.7。

#### 构建路径 enclosing-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- yaml/enclosing-project: 所属配置schema、字段用途、默认值和安全约束说明的完整原生规则/合法反例/契约准确性未验收

- yaml/enclosing-project/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.70, 8.71, 8.72, 15.4, 15.6, 15.7。

#### 构建路径 enclosing-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- yaml/enclosing-project:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- yaml/enclosing-project/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.70, 8.71, 8.72, 15.5, 15.6, 15.7。

#### 构建路径 enclosing-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- yaml/enclosing-project:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- yaml/enclosing-project/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## elixir

- 构建生态：mix。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：ExDoc/moduledoc/doc/spec与返回/异常契约。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["mix", "credo", "--strict"]`。

### syntax — blocked

OpenSpec任务：8.22, 8.23, 8.24, 15.2, 15.6, 15.7。

#### 构建路径 mix / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- elixir/mix:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- elixir/mix/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.22, 8.23, 8.24, 15.3, 15.6, 15.7。

#### 构建路径 mix / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- elixir/mix: ExDoc/moduledoc/doc/spec与返回/异常契约的完整原生规则/合法反例/契约准确性未验收

- elixir/mix/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.22, 8.23, 8.24, 15.4, 15.6, 15.7。

#### 构建路径 mix / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- elixir/mix:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- elixir/mix/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.22, 8.23, 8.24, 15.5, 15.6, 15.7。

#### 构建路径 mix / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- elixir/mix:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- elixir/mix/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## css

- 构建生态：node-stylesheet-project。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：样式公共接口、变量用途、约束及组件范围说明。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["npx", "--no-install", "stylelint", "**/*.css"]`。

### syntax — blocked

OpenSpec任务：8.46, 8.47, 8.48, 15.2, 15.6, 15.7。

#### 构建路径 node-stylesheet-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- css/node-stylesheet-project:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- css/node-stylesheet-project/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.46, 8.47, 8.48, 15.3, 15.6, 15.7。

#### 构建路径 node-stylesheet-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- css/node-stylesheet-project: 样式公共接口、变量用途、约束及组件范围说明的完整原生规则/合法反例/契约准确性未验收

- css/node-stylesheet-project/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.46, 8.47, 8.48, 15.4, 15.6, 15.7。

#### 构建路径 node-stylesheet-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- css/node-stylesheet-project:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- css/node-stylesheet-project/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.46, 8.47, 8.48, 15.5, 15.6, 15.7。

#### 构建路径 node-stylesheet-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- css/node-stylesheet-project:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- css/node-stylesheet-project/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## c

- 构建生态：standalone, cmake, conan, vcpkg。grammar候选：c。

- 版本/方言约束：Doxygen/API用途/参数/返回/错误与内存所有权。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["clang-tidy", "--quiet", "{file}"]`。

### syntax — blocked

OpenSpec任务：8.25, 8.26, 8.27, 15.2, 15.6, 15.7。

#### 构建路径 standalone / partial

现有代码入口：

- [crates/codeguard-cli/src/clang_lint_feedback.rs](../../crates/codeguard-cli/src/clang_lint_feedback.rs)

- [crates/codeguard-adapters/src/c_family_preprocessor_guard.rs](../../crates/codeguard-adapters/src/c_family_preprocessor_guard.rs)

- [crates/codeguard-cli/src/clang_syntax_probe.rs](../../crates/codeguard-cli/src/clang_syntax_probe.rs)

- [crates/codeguard-cli/src/grammar_native_checker.rs](../../crates/codeguard-cli/src/grammar_native_checker.rs)

- [crates/codeguard-cli/src/grammar_native_differential.rs](../../crates/codeguard-cli/src/grammar_native_differential.rs)


现有验收/样例：

- [tests/acceptance/clang-standalone-native.md](../../tests/acceptance/clang-standalone-native.md)

- [tests/acceptance/clang-preprocessor-context.md](../../tests/acceptance/clang-preprocessor-context.md)

- [tests/acceptance/c-family-native-replay.md](../../tests/acceptance/c-family-native-replay.md)

- [tests/acceptance/c-family-native-punctuation.md](../../tests/acceptance/c-family-native-punctuation.md)

- [crates/codeguard-cli/tests/c_native_differential.rs](../../crates/codeguard-cli/tests/c_native_differential.rs)

- [tests/acceptance/c-native-differential.md](../../tests/acceptance/c-native-differential.md)

- [tests/acceptance/evidence/c11-native-wasm-differential.json](../../tests/acceptance/evidence/c11-native-wasm-differential.json)


必须解决的登记缺口：

- c/standalone:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- c/standalone/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 cmake / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- c/cmake:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- c/cmake/syntax:五平台、宿主反馈、可信关闭和复发重开未验收

- 尚未接通该构建器的编译上下文；关联的独立文件Clang证据仅为参考，不能证明项目配置、头文件、宏或依赖源集已检查



#### 构建路径 conan / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- c/conan:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- c/conan/syntax:五平台、宿主反馈、可信关闭和复发重开未验收

- 尚未接通该构建器的编译上下文；关联的独立文件Clang证据仅为参考，不能证明项目配置、头文件、宏或依赖源集已检查



#### 构建路径 vcpkg / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- c/vcpkg:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- c/vcpkg/syntax:五平台、宿主反馈、可信关闭和复发重开未验收

- 尚未接通该构建器的编译上下文；关联的独立文件Clang证据仅为参考，不能证明项目配置、头文件、宏或依赖源集已检查



### documentation — blocked

OpenSpec任务：8.25, 8.26, 8.27, 15.3, 15.6, 15.7。

#### 构建路径 standalone / partial

现有代码入口：

- [crates/codeguard-cli/src/native_clang_profile.rs](../../crates/codeguard-cli/src/native_clang_profile.rs)

- [crates/codeguard-cli/src/c_family_comments_arguments.rs](../../crates/codeguard-cli/src/c_family_comments_arguments.rs)

- [crates/codeguard-cli/src/c_family_comments_command.rs](../../crates/codeguard-cli/src/c_family_comments_command.rs)

- [crates/codeguard-cli/src/clang_syntax_probe.rs](../../crates/codeguard-cli/src/clang_syntax_probe.rs)

- [crates/codeguard-adapters/src/clang_documentation_rule.rs](../../crates/codeguard-adapters/src/clang_documentation_rule.rs)

- [crates/codeguard-cli/src/c_family_comments_workbench.rs](../../crates/codeguard-cli/src/c_family_comments_workbench.rs)

- [crates/codeguard-cli/src/work_sync/c_family_comments_report.rs](../../crates/codeguard-cli/src/work_sync/c_family_comments_report.rs)

- [crates/codeguard-cli/src/next_command/c_family_comments_candidate.rs](../../crates/codeguard-cli/src/next_command/c_family_comments_candidate.rs)

- [crates/codeguard-cli/src/lib.rs](../../crates/codeguard-cli/src/lib.rs)

- [crates/codeguard-cli/src/command_examples.rs](../../crates/codeguard-cli/src/command_examples.rs)

- [crates/codeguard-cli/src/work_sync.rs](../../crates/codeguard-cli/src/work_sync.rs)

- [crates/codeguard-cli/src/next_command.rs](../../crates/codeguard-cli/src/next_command.rs)

- [crates/codeguard-cli/src/task_attempt_command.rs](../../crates/codeguard-cli/src/task_attempt_command.rs)

- [crates/codeguard-cli/src/task_verify_command.rs](../../crates/codeguard-cli/src/task_verify_command.rs)

- [crates/codeguard-cli/src/c_family_comments_task_recheck.rs](../../crates/codeguard-cli/src/c_family_comments_task_recheck.rs)

- [crates/codeguard-cli/src/workspace_view_command.rs](../../crates/codeguard-cli/src/workspace_view_command.rs)

- [crates/codeguard-adapters/src/clang_documentation_ast.rs](../../crates/codeguard-adapters/src/clang_documentation_ast.rs)

- [crates/codeguard-adapters/src/lib.rs](../../crates/codeguard-adapters/src/lib.rs)

- [crates/codeguard-adapters/src/clang_documentation_structure.rs](../../crates/codeguard-adapters/src/clang_documentation_structure.rs)

- [crates/codeguard-cli/src/c_family_structure_workbench.rs](../../crates/codeguard-cli/src/c_family_structure_workbench.rs)

- [crates/codeguard-cli/src/work_sync/c_family_structure_report.rs](../../crates/codeguard-cli/src/work_sync/c_family_structure_report.rs)

- [crates/codeguard-cli/src/next_command/c_family_structure_candidate.rs](../../crates/codeguard-cli/src/next_command/c_family_structure_candidate.rs)

- [crates/codeguard-cli/src/work_sync/task_projection.rs](../../crates/codeguard-cli/src/work_sync/task_projection.rs)

- [crates/codeguard-cli/src/c_family_structure_task_recheck.rs](../../crates/codeguard-cli/src/c_family_structure_task_recheck.rs)

- [crates/codeguard-cli/src/check_c_family_comments.rs](../../crates/codeguard-cli/src/check_c_family_comments.rs)

- [crates/codeguard-cli/src/c_family_documentation_hook_feedback.rs](../../crates/codeguard-cli/src/c_family_documentation_hook_feedback.rs)

- [crates/codeguard-cli/src/hook_execute_command.rs](../../crates/codeguard-cli/src/hook_execute_command.rs)

- [crates/codeguard-cli/src/hook_fast_scan.rs](../../crates/codeguard-cli/src/hook_fast_scan.rs)

- [crates/codeguard-cli/src/hook_native_tools.rs](../../crates/codeguard-cli/src/hook_native_tools.rs)

- [crates/codeguard-cli/src/c_family_documentation_host_guidance.rs](../../crates/codeguard-cli/src/c_family_documentation_host_guidance.rs)

- [crates/codeguard-cli/src/claude_hook_command.rs](../../crates/codeguard-cli/src/claude_hook_command.rs)


现有验收/样例：

- [crates/codeguard-adapters/tests/clang_documentation_contract.rs](../../crates/codeguard-adapters/tests/clang_documentation_contract.rs)

- [crates/codeguard-cli/tests/c_family_comments_cli.rs](../../crates/codeguard-cli/tests/c_family_comments_cli.rs)

- [schemas/c-family-comments-feedback-v0.1.schema.json](../../schemas/c-family-comments-feedback-v0.1.schema.json)

- [tests/acceptance/c-family-comments-native.md](../../tests/acceptance/c-family-comments-native.md)

- [tests/acceptance/evidence/c-family-comments-native.json](../../tests/acceptance/evidence/c-family-comments-native.json)

- [tests/c_family_comments_schema.py](../../tests/c_family_comments_schema.py)

- [tests/acceptance/evidence/c-family-comments-native-wasm.json](../../tests/acceptance/evidence/c-family-comments-native-wasm.json)

- [tests/acceptance/evidence/c-family-comments-schema.json](../../tests/acceptance/evidence/c-family-comments-schema.json)

- [tests/acceptance/evidence/c-family-comments-transport.json](../../tests/acceptance/evidence/c-family-comments-transport.json)

- [crates/codeguard-cli/tests/c_family_comments_workbench.rs](../../crates/codeguard-cli/tests/c_family_comments_workbench.rs)

- [schemas/c-family-comments-feedback-v0.2.schema.json](../../schemas/c-family-comments-feedback-v0.2.schema.json)

- [schemas/clang-documentation-workbench-observation-v0.1.schema.json](../../schemas/clang-documentation-workbench-observation-v0.1.schema.json)

- [schemas/repair-brief-preview-v0.28.schema.json](../../schemas/repair-brief-preview-v0.28.schema.json)

- [tests/acceptance/c-family-comments-workbench.md](../../tests/acceptance/c-family-comments-workbench.md)

- [tests/c_family_comments_workbench_schema.py](../../tests/c_family_comments_workbench_schema.py)

- [tests/acceptance/evidence/c-family-comments-workbench-native.json](../../tests/acceptance/evidence/c-family-comments-workbench-native.json)

- [tests/acceptance/evidence/c-family-comments-workbench-native-wasm.json](../../tests/acceptance/evidence/c-family-comments-workbench-native-wasm.json)

- [tests/acceptance/evidence/c-family-comments-workbench-schema.json](../../tests/acceptance/evidence/c-family-comments-workbench-schema.json)

- [tests/acceptance/c-family-comments-task-recheck.md](../../tests/acceptance/c-family-comments-task-recheck.md)

- [tests/c_family_comments_task_recheck_schema.py](../../tests/c_family_comments_task_recheck_schema.py)

- [schemas/clang-documentation-task-recheck-v0.1.schema.json](../../schemas/clang-documentation-task-recheck-v0.1.schema.json)

- [schemas/task-verification-preview-v0.36.schema.json](../../schemas/task-verification-preview-v0.36.schema.json)

- [schemas/repair-brief-preview-v0.29.schema.json](../../schemas/repair-brief-preview-v0.29.schema.json)

- [schemas/c-family-comments-feedback-v0.3.schema.json](../../schemas/c-family-comments-feedback-v0.3.schema.json)

- [tests/acceptance/c-family-comments-attempt-history.md](../../tests/acceptance/c-family-comments-attempt-history.md)

- [tests/c_family_comments_attempt_schema.py](../../tests/c_family_comments_attempt_schema.py)

- [schemas/c-family-comments-feedback-v0.4.schema.json](../../schemas/c-family-comments-feedback-v0.4.schema.json)

- [schemas/repair-brief-preview-v0.30.schema.json](../../schemas/repair-brief-preview-v0.30.schema.json)

- [schemas/task-show-preview-v0.4.schema.json](../../schemas/task-show-preview-v0.4.schema.json)

- [crates/codeguard-adapters/tests/clang_documentation_ast_contract.rs](../../crates/codeguard-adapters/tests/clang_documentation_ast_contract.rs)

- [crates/codeguard-cli/tests/clang_documentation_structure_native.rs](../../crates/codeguard-cli/tests/clang_documentation_structure_native.rs)

- [schemas/clang-function-documentation-structure-v0.1.schema.json](../../schemas/clang-function-documentation-structure-v0.1.schema.json)

- [tests/clang_documentation_structure_schema.py](../../tests/clang_documentation_structure_schema.py)

- [tests/acceptance/clang-documentation-structure.md](../../tests/acceptance/clang-documentation-structure.md)

- [tests/acceptance/evidence/clang-documentation-structure-native.json](../../tests/acceptance/evidence/clang-documentation-structure-native.json)

- [tests/acceptance/evidence/clang-documentation-structure-native-wasm.json](../../tests/acceptance/evidence/clang-documentation-structure-native-wasm.json)

- [tests/acceptance/evidence/clang-documentation-structure-schema.json](../../tests/acceptance/evidence/clang-documentation-structure-schema.json)

- [crates/codeguard-cli/tests/c_family_comments_structure_cli.rs](../../crates/codeguard-cli/tests/c_family_comments_structure_cli.rs)

- [schemas/c-family-comments-feedback-v0.5.schema.json](../../schemas/c-family-comments-feedback-v0.5.schema.json)

- [schemas/c-family-comments-feedback-v0.6.schema.json](../../schemas/c-family-comments-feedback-v0.6.schema.json)

- [tests/c_family_comments_structure_schema.py](../../tests/c_family_comments_structure_schema.py)

- [tests/acceptance/c-family-comments-structure-cli.md](../../tests/acceptance/c-family-comments-structure-cli.md)

- [tests/acceptance/evidence/c-family-comments-structure-cli.json](../../tests/acceptance/evidence/c-family-comments-structure-cli.json)

- [tests/acceptance/evidence/c-family-comments-structure-cli-wasm.json](../../tests/acceptance/evidence/c-family-comments-structure-cli-wasm.json)

- [tests/acceptance/evidence/c-family-comments-structure-schema.json](../../tests/acceptance/evidence/c-family-comments-structure-schema.json)

- [crates/codeguard-adapters/tests/clang_documentation_structure_contract.rs](../../crates/codeguard-adapters/tests/clang_documentation_structure_contract.rs)

- [crates/codeguard-cli/tests/c_family_structure_workbench.rs](../../crates/codeguard-cli/tests/c_family_structure_workbench.rs)

- [schemas/clang-documentation-structure-workbench-observation-v0.1.schema.json](../../schemas/clang-documentation-structure-workbench-observation-v0.1.schema.json)

- [schemas/repair-brief-preview-v0.31.schema.json](../../schemas/repair-brief-preview-v0.31.schema.json)

- [schemas/task-show-preview-v0.5.schema.json](../../schemas/task-show-preview-v0.5.schema.json)

- [schemas/c-family-comments-feedback-v0.7.schema.json](../../schemas/c-family-comments-feedback-v0.7.schema.json)

- [tests/c_family_structure_workbench_schema.py](../../tests/c_family_structure_workbench_schema.py)

- [tests/acceptance/c-family-structure-workbench.md](../../tests/acceptance/c-family-structure-workbench.md)

- [tests/acceptance/evidence/c-family-structure-workbench-native.json](../../tests/acceptance/evidence/c-family-structure-workbench-native.json)

- [tests/acceptance/evidence/c-family-structure-workbench-native-wasm.json](../../tests/acceptance/evidence/c-family-structure-workbench-native-wasm.json)

- [tests/acceptance/evidence/c-family-structure-workbench-schema.json](../../tests/acceptance/evidence/c-family-structure-workbench-schema.json)

- [crates/codeguard-cli/src/c_family_structure_recheck_tests.rs](../../crates/codeguard-cli/src/c_family_structure_recheck_tests.rs)

- [schemas/clang-documentation-structure-task-recheck-v0.1.schema.json](../../schemas/clang-documentation-structure-task-recheck-v0.1.schema.json)

- [schemas/task-verification-preview-v0.37.schema.json](../../schemas/task-verification-preview-v0.37.schema.json)

- [schemas/repair-brief-preview-v0.32.schema.json](../../schemas/repair-brief-preview-v0.32.schema.json)

- [schemas/task-show-preview-v0.6.schema.json](../../schemas/task-show-preview-v0.6.schema.json)

- [schemas/c-family-comments-feedback-v0.8.schema.json](../../schemas/c-family-comments-feedback-v0.8.schema.json)

- [tests/acceptance/evidence/c-family-structure-recheck-native.json](../../tests/acceptance/evidence/c-family-structure-recheck-native.json)

- [tests/acceptance/evidence/c-family-structure-recheck-native-wasm.json](../../tests/acceptance/evidence/c-family-structure-recheck-native-wasm.json)

- [schemas/repair-brief-preview-v0.33.schema.json](../../schemas/repair-brief-preview-v0.33.schema.json)

- [schemas/task-show-preview-v0.7.schema.json](../../schemas/task-show-preview-v0.7.schema.json)

- [schemas/c-family-comments-feedback-v0.9.schema.json](../../schemas/c-family-comments-feedback-v0.9.schema.json)

- [crates/codeguard-cli/tests/check_c_family_comments.rs](../../crates/codeguard-cli/tests/check_c_family_comments.rs)

- [schemas/check-feedback-v0.72.schema.json](../../schemas/check-feedback-v0.72.schema.json)

- [schemas/check-aborted-v0.22.schema.json](../../schemas/check-aborted-v0.22.schema.json)

- [schemas/c-family-documentation-scan-v0.1.schema.json](../../schemas/c-family-documentation-scan-v0.1.schema.json)

- [schemas/c-family-documentation-scans-v0.1.schema.json](../../schemas/c-family-documentation-scans-v0.1.schema.json)

- [tests/check_c_family_documentation_schema.py](../../tests/check_c_family_documentation_schema.py)

- [tests/acceptance/check-c-family-documentation.md](../../tests/acceptance/check-c-family-documentation.md)

- [tests/acceptance/evidence/check-c-family-documentation-default.json](../../tests/acceptance/evidence/check-c-family-documentation-default.json)

- [tests/acceptance/evidence/check-c-family-documentation-wasm.json](../../tests/acceptance/evidence/check-c-family-documentation-wasm.json)

- [tests/acceptance/evidence/check-c-family-documentation-schema.json](../../tests/acceptance/evidence/check-c-family-documentation-schema.json)

- [crates/codeguard-cli/tests/c_family_documentation_hook.rs](../../crates/codeguard-cli/tests/c_family_documentation_hook.rs)

- [schemas/hook-execution-feedback-v0.30.schema.json](../../schemas/hook-execution-feedback-v0.30.schema.json)

- [tests/c_family_documentation_hook_schema.py](../../tests/c_family_documentation_hook_schema.py)

- [tests/acceptance/c-family-documentation-hook.md](../../tests/acceptance/c-family-documentation-hook.md)

- [tests/acceptance/evidence/c-family-documentation-hook-default.json](../../tests/acceptance/evidence/c-family-documentation-hook-default.json)

- [tests/acceptance/evidence/c-family-documentation-hook-wasm.json](../../tests/acceptance/evidence/c-family-documentation-hook-wasm.json)

- [tests/acceptance/evidence/c-family-documentation-hook-schema.json](../../tests/acceptance/evidence/c-family-documentation-hook-schema.json)

- [crates/codeguard-cli/tests/c_family_documentation_edit_hook.rs](../../crates/codeguard-cli/tests/c_family_documentation_edit_hook.rs)

- [schemas/hook-execution-feedback-v0.31.schema.json](../../schemas/hook-execution-feedback-v0.31.schema.json)

- [tests/c_family_documentation_edit_hook_schema.py](../../tests/c_family_documentation_edit_hook_schema.py)

- [tests/acceptance/c-family-documentation-edit-hook.md](../../tests/acceptance/c-family-documentation-edit-hook.md)

- [tests/acceptance/evidence/c-family-edit-schema.json](../../tests/acceptance/evidence/c-family-edit-schema.json)

- [tests/acceptance/evidence/c-family-edit-default-c.json](../../tests/acceptance/evidence/c-family-edit-default-c.json)

- [tests/acceptance/evidence/c-family-edit-default-cpp.json](../../tests/acceptance/evidence/c-family-edit-default-cpp.json)

- [tests/acceptance/evidence/c-family-edit-wasm-c.json](../../tests/acceptance/evidence/c-family-edit-wasm-c.json)

- [tests/acceptance/evidence/c-family-edit-wasm-cpp.json](../../tests/acceptance/evidence/c-family-edit-wasm-cpp.json)

- [crates/codeguard-cli/tests/hook_failed_write_options.rs](../../crates/codeguard-cli/tests/hook_failed_write_options.rs)

- [crates/codeguard-cli/tests/claude_hook_cli.rs](../../crates/codeguard-cli/tests/claude_hook_cli.rs)

- [tests/acceptance/c-family-documentation-host-guidance.md](../../tests/acceptance/c-family-documentation-host-guidance.md)

- [tests/c_family_documentation_host_evidence.py](../../tests/c_family_documentation_host_evidence.py)

- [tests/acceptance/evidence/c-family-host-validation.json](../../tests/acceptance/evidence/c-family-host-validation.json)

- [tests/acceptance/evidence/c-family-host-default-c.json](../../tests/acceptance/evidence/c-family-host-default-c.json)

- [tests/acceptance/evidence/c-family-host-default-cpp.json](../../tests/acceptance/evidence/c-family-host-default-cpp.json)

- [tests/acceptance/evidence/c-family-host-wasm-c.json](../../tests/acceptance/evidence/c-family-host-wasm-c.json)

- [tests/acceptance/evidence/c-family-host-wasm-cpp.json](../../tests/acceptance/evidence/c-family-host-wasm-cpp.json)

- [tests/acceptance/clang-documentation-engine-identity.md](../../tests/acceptance/clang-documentation-engine-identity.md)

- [crates/codeguard-cli/src/c_family_placeholder_workbench.rs](../../crates/codeguard-cli/src/c_family_placeholder_workbench.rs)

- [crates/codeguard-cli/src/work_sync/c_family_placeholder_report.rs](../../crates/codeguard-cli/src/work_sync/c_family_placeholder_report.rs)

- [crates/codeguard-adapters/src/clang_documentation_placeholders.rs](../../crates/codeguard-adapters/src/clang_documentation_placeholders.rs)

- [crates/codeguard-adapters/src/clang_placeholder_validation.rs](../../crates/codeguard-adapters/src/clang_placeholder_validation.rs)

- [crates/codeguard-cli/tests/c_family_placeholder_cli.rs](../../crates/codeguard-cli/tests/c_family_placeholder_cli.rs)

- [crates/codeguard-cli/tests/c_family_placeholder_import.rs](../../crates/codeguard-cli/tests/c_family_placeholder_import.rs)

- [crates/codeguard-adapters/tests/clang_documentation_placeholder_contract.rs](../../crates/codeguard-adapters/tests/clang_documentation_placeholder_contract.rs)

- [crates/codeguard-adapters/tests/clang_placeholder_validation.rs](../../crates/codeguard-adapters/tests/clang_placeholder_validation.rs)

- [schemas/clang-documentation-placeholder-v0.1.schema.json](../../schemas/clang-documentation-placeholder-v0.1.schema.json)

- [schemas/clang-documentation-placeholder-workbench-observation-v0.1.schema.json](../../schemas/clang-documentation-placeholder-workbench-observation-v0.1.schema.json)

- [schemas/c-family-comments-feedback-v0.10.schema.json](../../schemas/c-family-comments-feedback-v0.10.schema.json)

- [schemas/c-family-comments-feedback-v0.11.schema.json](../../schemas/c-family-comments-feedback-v0.11.schema.json)

- [tests/acceptance/clang-documentation-placeholders.md](../../tests/acceptance/clang-documentation-placeholders.md)

- [crates/codeguard-cli/src/next_command/c_family_placeholder_candidate.rs](../../crates/codeguard-cli/src/next_command/c_family_placeholder_candidate.rs)

- [schemas/repair-brief-preview-v0.34.schema.json](../../schemas/repair-brief-preview-v0.34.schema.json)

- [crates/codeguard-cli/src/c_family_placeholder_task_recheck.rs](../../crates/codeguard-cli/src/c_family_placeholder_task_recheck.rs)


必须解决的登记缺口：

- C/C++结构文件策略任务、原工具task verify、受控repair-source尝试及同一输入两次失败预算已接线；跨输入语义无进展、可信关闭/复发、check/Hook与详细语义/完整项目/独立精度仍未验收。

- c/standalone: Doxygen/API用途/参数/返回/错误与内存所有权的完整原生规则/合法反例/契约准确性未验收

- c/standalone/documentation:五平台、宿主反馈、可信关闭和复发重开未验收

- 统一check独立档案已partial；完整项目头文件/编译配置、Hook、详细准确性及正式生产覆盖仍未验收



#### 构建路径 cmake / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- c/cmake: Doxygen/API用途/参数/返回/错误与内存所有权的完整原生规则/合法反例/契约准确性未验收

- c/cmake/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 conan / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- c/conan: Doxygen/API用途/参数/返回/错误与内存所有权的完整原生规则/合法反例/契约准确性未验收

- c/conan/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 vcpkg / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- c/vcpkg: Doxygen/API用途/参数/返回/错误与内存所有权的完整原生规则/合法反例/契约准确性未验收

- c/vcpkg/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.25, 8.26, 8.27, 15.4, 15.6, 15.7。

#### 构建路径 standalone / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- c/standalone:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- c/standalone/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 cmake / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- c/cmake:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- c/cmake/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 conan / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- c/conan:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- c/conan/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 vcpkg / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- c/vcpkg:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- c/vcpkg/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.25, 8.26, 8.27, 15.5, 15.6, 15.7。

#### 构建路径 standalone / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- c/standalone:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- c/standalone/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 cmake / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- c/cmake:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- c/cmake/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 conan / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- c/conan:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- c/conan/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 vcpkg / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- c/vcpkg:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- c/vcpkg/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## cpp

- 构建生态：standalone, cmake, conan, vcpkg。grammar候选：cpp。

- 版本/方言约束：Doxygen/API用途/模板/异常/生命周期与线程安全。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["clang-tidy", "--quiet", "{file}"]`。

### syntax — blocked

OpenSpec任务：8.28, 8.29, 8.30, 15.2, 15.6, 15.7。

#### 构建路径 standalone / partial

现有代码入口：

- [crates/codeguard-cli/src/clang_lint_feedback.rs](../../crates/codeguard-cli/src/clang_lint_feedback.rs)

- [crates/codeguard-adapters/src/c_family_preprocessor_guard.rs](../../crates/codeguard-adapters/src/c_family_preprocessor_guard.rs)

- [crates/codeguard-cli/src/clang_syntax_probe.rs](../../crates/codeguard-cli/src/clang_syntax_probe.rs)

- [crates/codeguard-cli/src/grammar_native_checker.rs](../../crates/codeguard-cli/src/grammar_native_checker.rs)

- [crates/codeguard-cli/src/grammar_native_differential.rs](../../crates/codeguard-cli/src/grammar_native_differential.rs)


现有验收/样例：

- [tests/acceptance/clang-standalone-native.md](../../tests/acceptance/clang-standalone-native.md)

- [tests/acceptance/clang-preprocessor-context.md](../../tests/acceptance/clang-preprocessor-context.md)

- [tests/acceptance/c-family-native-replay.md](../../tests/acceptance/c-family-native-replay.md)

- [tests/acceptance/c-family-native-punctuation.md](../../tests/acceptance/c-family-native-punctuation.md)

- [crates/codeguard-cli/tests/cpp_native_differential.rs](../../crates/codeguard-cli/tests/cpp_native_differential.rs)

- [tests/acceptance/cpp17-native-wasm-differential.md](../../tests/acceptance/cpp17-native-wasm-differential.md)

- [tests/acceptance/evidence/cpp17-native-wasm-differential.json](../../tests/acceptance/evidence/cpp17-native-wasm-differential.json)


必须解决的登记缺口：

- cpp/standalone:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- cpp/standalone/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 cmake / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cpp/cmake:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- cpp/cmake/syntax:五平台、宿主反馈、可信关闭和复发重开未验收

- 尚未接通该构建器的编译上下文；关联的独立文件Clang证据仅为参考，不能证明项目配置、头文件、宏或依赖源集已检查



#### 构建路径 conan / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cpp/conan:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- cpp/conan/syntax:五平台、宿主反馈、可信关闭和复发重开未验收

- 尚未接通该构建器的编译上下文；关联的独立文件Clang证据仅为参考，不能证明项目配置、头文件、宏或依赖源集已检查



#### 构建路径 vcpkg / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cpp/vcpkg:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- cpp/vcpkg/syntax:五平台、宿主反馈、可信关闭和复发重开未验收

- 尚未接通该构建器的编译上下文；关联的独立文件Clang证据仅为参考，不能证明项目配置、头文件、宏或依赖源集已检查



### documentation — blocked

OpenSpec任务：8.28, 8.29, 8.30, 15.3, 15.6, 15.7。

#### 构建路径 standalone / partial

现有代码入口：

- [crates/codeguard-cli/src/native_clang_profile.rs](../../crates/codeguard-cli/src/native_clang_profile.rs)

- [crates/codeguard-cli/src/c_family_comments_arguments.rs](../../crates/codeguard-cli/src/c_family_comments_arguments.rs)

- [crates/codeguard-cli/src/c_family_comments_command.rs](../../crates/codeguard-cli/src/c_family_comments_command.rs)

- [crates/codeguard-cli/src/clang_syntax_probe.rs](../../crates/codeguard-cli/src/clang_syntax_probe.rs)

- [crates/codeguard-adapters/src/clang_documentation_rule.rs](../../crates/codeguard-adapters/src/clang_documentation_rule.rs)

- [crates/codeguard-cli/src/c_family_comments_workbench.rs](../../crates/codeguard-cli/src/c_family_comments_workbench.rs)

- [crates/codeguard-cli/src/work_sync/c_family_comments_report.rs](../../crates/codeguard-cli/src/work_sync/c_family_comments_report.rs)

- [crates/codeguard-cli/src/next_command/c_family_comments_candidate.rs](../../crates/codeguard-cli/src/next_command/c_family_comments_candidate.rs)

- [crates/codeguard-cli/src/lib.rs](../../crates/codeguard-cli/src/lib.rs)

- [crates/codeguard-cli/src/command_examples.rs](../../crates/codeguard-cli/src/command_examples.rs)

- [crates/codeguard-cli/src/work_sync.rs](../../crates/codeguard-cli/src/work_sync.rs)

- [crates/codeguard-cli/src/next_command.rs](../../crates/codeguard-cli/src/next_command.rs)

- [crates/codeguard-cli/src/task_attempt_command.rs](../../crates/codeguard-cli/src/task_attempt_command.rs)

- [crates/codeguard-cli/src/task_verify_command.rs](../../crates/codeguard-cli/src/task_verify_command.rs)

- [crates/codeguard-cli/src/c_family_comments_task_recheck.rs](../../crates/codeguard-cli/src/c_family_comments_task_recheck.rs)

- [crates/codeguard-cli/src/workspace_view_command.rs](../../crates/codeguard-cli/src/workspace_view_command.rs)

- [crates/codeguard-adapters/src/clang_documentation_ast.rs](../../crates/codeguard-adapters/src/clang_documentation_ast.rs)

- [crates/codeguard-adapters/src/lib.rs](../../crates/codeguard-adapters/src/lib.rs)

- [crates/codeguard-adapters/src/clang_documentation_structure.rs](../../crates/codeguard-adapters/src/clang_documentation_structure.rs)

- [crates/codeguard-cli/src/c_family_structure_workbench.rs](../../crates/codeguard-cli/src/c_family_structure_workbench.rs)

- [crates/codeguard-cli/src/work_sync/c_family_structure_report.rs](../../crates/codeguard-cli/src/work_sync/c_family_structure_report.rs)

- [crates/codeguard-cli/src/next_command/c_family_structure_candidate.rs](../../crates/codeguard-cli/src/next_command/c_family_structure_candidate.rs)

- [crates/codeguard-cli/src/work_sync/task_projection.rs](../../crates/codeguard-cli/src/work_sync/task_projection.rs)

- [crates/codeguard-cli/src/c_family_structure_task_recheck.rs](../../crates/codeguard-cli/src/c_family_structure_task_recheck.rs)

- [crates/codeguard-cli/src/check_c_family_comments.rs](../../crates/codeguard-cli/src/check_c_family_comments.rs)

- [crates/codeguard-cli/src/c_family_documentation_hook_feedback.rs](../../crates/codeguard-cli/src/c_family_documentation_hook_feedback.rs)

- [crates/codeguard-cli/src/hook_execute_command.rs](../../crates/codeguard-cli/src/hook_execute_command.rs)

- [crates/codeguard-cli/src/hook_fast_scan.rs](../../crates/codeguard-cli/src/hook_fast_scan.rs)

- [crates/codeguard-cli/src/hook_native_tools.rs](../../crates/codeguard-cli/src/hook_native_tools.rs)

- [crates/codeguard-cli/src/c_family_documentation_host_guidance.rs](../../crates/codeguard-cli/src/c_family_documentation_host_guidance.rs)

- [crates/codeguard-cli/src/claude_hook_command.rs](../../crates/codeguard-cli/src/claude_hook_command.rs)


现有验收/样例：

- [crates/codeguard-adapters/tests/clang_documentation_contract.rs](../../crates/codeguard-adapters/tests/clang_documentation_contract.rs)

- [crates/codeguard-cli/tests/c_family_comments_cli.rs](../../crates/codeguard-cli/tests/c_family_comments_cli.rs)

- [schemas/c-family-comments-feedback-v0.1.schema.json](../../schemas/c-family-comments-feedback-v0.1.schema.json)

- [tests/acceptance/c-family-comments-native.md](../../tests/acceptance/c-family-comments-native.md)

- [tests/acceptance/evidence/c-family-comments-native.json](../../tests/acceptance/evidence/c-family-comments-native.json)

- [tests/c_family_comments_schema.py](../../tests/c_family_comments_schema.py)

- [tests/acceptance/evidence/c-family-comments-native-wasm.json](../../tests/acceptance/evidence/c-family-comments-native-wasm.json)

- [tests/acceptance/evidence/c-family-comments-schema.json](../../tests/acceptance/evidence/c-family-comments-schema.json)

- [tests/acceptance/evidence/c-family-comments-transport.json](../../tests/acceptance/evidence/c-family-comments-transport.json)

- [crates/codeguard-cli/tests/c_family_comments_workbench.rs](../../crates/codeguard-cli/tests/c_family_comments_workbench.rs)

- [schemas/c-family-comments-feedback-v0.2.schema.json](../../schemas/c-family-comments-feedback-v0.2.schema.json)

- [schemas/clang-documentation-workbench-observation-v0.1.schema.json](../../schemas/clang-documentation-workbench-observation-v0.1.schema.json)

- [schemas/repair-brief-preview-v0.28.schema.json](../../schemas/repair-brief-preview-v0.28.schema.json)

- [tests/acceptance/c-family-comments-workbench.md](../../tests/acceptance/c-family-comments-workbench.md)

- [tests/c_family_comments_workbench_schema.py](../../tests/c_family_comments_workbench_schema.py)

- [tests/acceptance/evidence/c-family-comments-workbench-native.json](../../tests/acceptance/evidence/c-family-comments-workbench-native.json)

- [tests/acceptance/evidence/c-family-comments-workbench-native-wasm.json](../../tests/acceptance/evidence/c-family-comments-workbench-native-wasm.json)

- [tests/acceptance/evidence/c-family-comments-workbench-schema.json](../../tests/acceptance/evidence/c-family-comments-workbench-schema.json)

- [tests/acceptance/c-family-comments-task-recheck.md](../../tests/acceptance/c-family-comments-task-recheck.md)

- [tests/c_family_comments_task_recheck_schema.py](../../tests/c_family_comments_task_recheck_schema.py)

- [schemas/clang-documentation-task-recheck-v0.1.schema.json](../../schemas/clang-documentation-task-recheck-v0.1.schema.json)

- [schemas/task-verification-preview-v0.36.schema.json](../../schemas/task-verification-preview-v0.36.schema.json)

- [schemas/repair-brief-preview-v0.29.schema.json](../../schemas/repair-brief-preview-v0.29.schema.json)

- [schemas/c-family-comments-feedback-v0.3.schema.json](../../schemas/c-family-comments-feedback-v0.3.schema.json)

- [tests/acceptance/c-family-comments-attempt-history.md](../../tests/acceptance/c-family-comments-attempt-history.md)

- [tests/c_family_comments_attempt_schema.py](../../tests/c_family_comments_attempt_schema.py)

- [schemas/c-family-comments-feedback-v0.4.schema.json](../../schemas/c-family-comments-feedback-v0.4.schema.json)

- [schemas/repair-brief-preview-v0.30.schema.json](../../schemas/repair-brief-preview-v0.30.schema.json)

- [schemas/task-show-preview-v0.4.schema.json](../../schemas/task-show-preview-v0.4.schema.json)

- [crates/codeguard-adapters/tests/clang_documentation_ast_contract.rs](../../crates/codeguard-adapters/tests/clang_documentation_ast_contract.rs)

- [crates/codeguard-cli/tests/clang_documentation_structure_native.rs](../../crates/codeguard-cli/tests/clang_documentation_structure_native.rs)

- [schemas/clang-function-documentation-structure-v0.1.schema.json](../../schemas/clang-function-documentation-structure-v0.1.schema.json)

- [tests/clang_documentation_structure_schema.py](../../tests/clang_documentation_structure_schema.py)

- [tests/acceptance/clang-documentation-structure.md](../../tests/acceptance/clang-documentation-structure.md)

- [tests/acceptance/evidence/clang-documentation-structure-native.json](../../tests/acceptance/evidence/clang-documentation-structure-native.json)

- [tests/acceptance/evidence/clang-documentation-structure-native-wasm.json](../../tests/acceptance/evidence/clang-documentation-structure-native-wasm.json)

- [tests/acceptance/evidence/clang-documentation-structure-schema.json](../../tests/acceptance/evidence/clang-documentation-structure-schema.json)

- [crates/codeguard-cli/tests/c_family_comments_structure_cli.rs](../../crates/codeguard-cli/tests/c_family_comments_structure_cli.rs)

- [schemas/c-family-comments-feedback-v0.5.schema.json](../../schemas/c-family-comments-feedback-v0.5.schema.json)

- [schemas/c-family-comments-feedback-v0.6.schema.json](../../schemas/c-family-comments-feedback-v0.6.schema.json)

- [tests/c_family_comments_structure_schema.py](../../tests/c_family_comments_structure_schema.py)

- [tests/acceptance/c-family-comments-structure-cli.md](../../tests/acceptance/c-family-comments-structure-cli.md)

- [tests/acceptance/evidence/c-family-comments-structure-cli.json](../../tests/acceptance/evidence/c-family-comments-structure-cli.json)

- [tests/acceptance/evidence/c-family-comments-structure-cli-wasm.json](../../tests/acceptance/evidence/c-family-comments-structure-cli-wasm.json)

- [tests/acceptance/evidence/c-family-comments-structure-schema.json](../../tests/acceptance/evidence/c-family-comments-structure-schema.json)

- [crates/codeguard-adapters/tests/clang_documentation_structure_contract.rs](../../crates/codeguard-adapters/tests/clang_documentation_structure_contract.rs)

- [crates/codeguard-cli/tests/c_family_structure_workbench.rs](../../crates/codeguard-cli/tests/c_family_structure_workbench.rs)

- [schemas/clang-documentation-structure-workbench-observation-v0.1.schema.json](../../schemas/clang-documentation-structure-workbench-observation-v0.1.schema.json)

- [schemas/repair-brief-preview-v0.31.schema.json](../../schemas/repair-brief-preview-v0.31.schema.json)

- [schemas/task-show-preview-v0.5.schema.json](../../schemas/task-show-preview-v0.5.schema.json)

- [schemas/c-family-comments-feedback-v0.7.schema.json](../../schemas/c-family-comments-feedback-v0.7.schema.json)

- [tests/c_family_structure_workbench_schema.py](../../tests/c_family_structure_workbench_schema.py)

- [tests/acceptance/c-family-structure-workbench.md](../../tests/acceptance/c-family-structure-workbench.md)

- [tests/acceptance/evidence/c-family-structure-workbench-native.json](../../tests/acceptance/evidence/c-family-structure-workbench-native.json)

- [tests/acceptance/evidence/c-family-structure-workbench-native-wasm.json](../../tests/acceptance/evidence/c-family-structure-workbench-native-wasm.json)

- [tests/acceptance/evidence/c-family-structure-workbench-schema.json](../../tests/acceptance/evidence/c-family-structure-workbench-schema.json)

- [crates/codeguard-cli/src/c_family_structure_recheck_tests.rs](../../crates/codeguard-cli/src/c_family_structure_recheck_tests.rs)

- [schemas/clang-documentation-structure-task-recheck-v0.1.schema.json](../../schemas/clang-documentation-structure-task-recheck-v0.1.schema.json)

- [schemas/task-verification-preview-v0.37.schema.json](../../schemas/task-verification-preview-v0.37.schema.json)

- [schemas/repair-brief-preview-v0.32.schema.json](../../schemas/repair-brief-preview-v0.32.schema.json)

- [schemas/task-show-preview-v0.6.schema.json](../../schemas/task-show-preview-v0.6.schema.json)

- [schemas/c-family-comments-feedback-v0.8.schema.json](../../schemas/c-family-comments-feedback-v0.8.schema.json)

- [tests/acceptance/evidence/c-family-structure-recheck-native.json](../../tests/acceptance/evidence/c-family-structure-recheck-native.json)

- [tests/acceptance/evidence/c-family-structure-recheck-native-wasm.json](../../tests/acceptance/evidence/c-family-structure-recheck-native-wasm.json)

- [schemas/repair-brief-preview-v0.33.schema.json](../../schemas/repair-brief-preview-v0.33.schema.json)

- [schemas/task-show-preview-v0.7.schema.json](../../schemas/task-show-preview-v0.7.schema.json)

- [schemas/c-family-comments-feedback-v0.9.schema.json](../../schemas/c-family-comments-feedback-v0.9.schema.json)

- [crates/codeguard-cli/tests/check_c_family_comments.rs](../../crates/codeguard-cli/tests/check_c_family_comments.rs)

- [schemas/check-feedback-v0.72.schema.json](../../schemas/check-feedback-v0.72.schema.json)

- [schemas/check-aborted-v0.22.schema.json](../../schemas/check-aborted-v0.22.schema.json)

- [schemas/c-family-documentation-scan-v0.1.schema.json](../../schemas/c-family-documentation-scan-v0.1.schema.json)

- [schemas/c-family-documentation-scans-v0.1.schema.json](../../schemas/c-family-documentation-scans-v0.1.schema.json)

- [tests/check_c_family_documentation_schema.py](../../tests/check_c_family_documentation_schema.py)

- [tests/acceptance/check-c-family-documentation.md](../../tests/acceptance/check-c-family-documentation.md)

- [tests/acceptance/evidence/check-c-family-documentation-default.json](../../tests/acceptance/evidence/check-c-family-documentation-default.json)

- [tests/acceptance/evidence/check-c-family-documentation-wasm.json](../../tests/acceptance/evidence/check-c-family-documentation-wasm.json)

- [tests/acceptance/evidence/check-c-family-documentation-schema.json](../../tests/acceptance/evidence/check-c-family-documentation-schema.json)

- [crates/codeguard-cli/tests/c_family_documentation_hook.rs](../../crates/codeguard-cli/tests/c_family_documentation_hook.rs)

- [schemas/hook-execution-feedback-v0.30.schema.json](../../schemas/hook-execution-feedback-v0.30.schema.json)

- [tests/c_family_documentation_hook_schema.py](../../tests/c_family_documentation_hook_schema.py)

- [tests/acceptance/c-family-documentation-hook.md](../../tests/acceptance/c-family-documentation-hook.md)

- [tests/acceptance/evidence/c-family-documentation-hook-default.json](../../tests/acceptance/evidence/c-family-documentation-hook-default.json)

- [tests/acceptance/evidence/c-family-documentation-hook-wasm.json](../../tests/acceptance/evidence/c-family-documentation-hook-wasm.json)

- [tests/acceptance/evidence/c-family-documentation-hook-schema.json](../../tests/acceptance/evidence/c-family-documentation-hook-schema.json)

- [crates/codeguard-cli/tests/c_family_documentation_edit_hook.rs](../../crates/codeguard-cli/tests/c_family_documentation_edit_hook.rs)

- [schemas/hook-execution-feedback-v0.31.schema.json](../../schemas/hook-execution-feedback-v0.31.schema.json)

- [tests/c_family_documentation_edit_hook_schema.py](../../tests/c_family_documentation_edit_hook_schema.py)

- [tests/acceptance/c-family-documentation-edit-hook.md](../../tests/acceptance/c-family-documentation-edit-hook.md)

- [tests/acceptance/evidence/c-family-edit-schema.json](../../tests/acceptance/evidence/c-family-edit-schema.json)

- [tests/acceptance/evidence/c-family-edit-default-c.json](../../tests/acceptance/evidence/c-family-edit-default-c.json)

- [tests/acceptance/evidence/c-family-edit-default-cpp.json](../../tests/acceptance/evidence/c-family-edit-default-cpp.json)

- [tests/acceptance/evidence/c-family-edit-wasm-c.json](../../tests/acceptance/evidence/c-family-edit-wasm-c.json)

- [tests/acceptance/evidence/c-family-edit-wasm-cpp.json](../../tests/acceptance/evidence/c-family-edit-wasm-cpp.json)

- [crates/codeguard-cli/tests/hook_failed_write_options.rs](../../crates/codeguard-cli/tests/hook_failed_write_options.rs)

- [crates/codeguard-cli/tests/claude_hook_cli.rs](../../crates/codeguard-cli/tests/claude_hook_cli.rs)

- [tests/acceptance/c-family-documentation-host-guidance.md](../../tests/acceptance/c-family-documentation-host-guidance.md)

- [tests/c_family_documentation_host_evidence.py](../../tests/c_family_documentation_host_evidence.py)

- [tests/acceptance/evidence/c-family-host-validation.json](../../tests/acceptance/evidence/c-family-host-validation.json)

- [tests/acceptance/evidence/c-family-host-default-c.json](../../tests/acceptance/evidence/c-family-host-default-c.json)

- [tests/acceptance/evidence/c-family-host-default-cpp.json](../../tests/acceptance/evidence/c-family-host-default-cpp.json)

- [tests/acceptance/evidence/c-family-host-wasm-c.json](../../tests/acceptance/evidence/c-family-host-wasm-c.json)

- [tests/acceptance/evidence/c-family-host-wasm-cpp.json](../../tests/acceptance/evidence/c-family-host-wasm-cpp.json)

- [tests/acceptance/cpp17-method-documentation.md](../../tests/acceptance/cpp17-method-documentation.md)

- [tests/acceptance/evidence/cpp17-method-documentation.json](../../tests/acceptance/evidence/cpp17-method-documentation.json)

- [tests/acceptance/evidence/cpp17-free-operator-documentation.json](../../tests/acceptance/evidence/cpp17-free-operator-documentation.json)

- [crates/codeguard-cli/tests/cpp_documentation_workbench.rs](../../crates/codeguard-cli/tests/cpp_documentation_workbench.rs)

- [tests/cpp_documentation_workbench_schema.py](../../tests/cpp_documentation_workbench_schema.py)

- [tests/acceptance/cpp17-member-workbench.md](../../tests/acceptance/cpp17-member-workbench.md)

- [tests/acceptance/evidence/cpp17-member-workbench-default.json](../../tests/acceptance/evidence/cpp17-member-workbench-default.json)

- [tests/acceptance/evidence/cpp17-member-workbench-wasm.json](../../tests/acceptance/evidence/cpp17-member-workbench-wasm.json)

- [tests/acceptance/evidence/cpp17-member-workbench-schema.json](../../tests/acceptance/evidence/cpp17-member-workbench-schema.json)

- [tests/acceptance/clang-documentation-engine-identity.md](../../tests/acceptance/clang-documentation-engine-identity.md)

- [crates/codeguard-cli/src/c_family_placeholder_workbench.rs](../../crates/codeguard-cli/src/c_family_placeholder_workbench.rs)

- [crates/codeguard-cli/src/work_sync/c_family_placeholder_report.rs](../../crates/codeguard-cli/src/work_sync/c_family_placeholder_report.rs)

- [crates/codeguard-adapters/src/clang_documentation_placeholders.rs](../../crates/codeguard-adapters/src/clang_documentation_placeholders.rs)

- [crates/codeguard-adapters/src/clang_placeholder_validation.rs](../../crates/codeguard-adapters/src/clang_placeholder_validation.rs)

- [crates/codeguard-cli/tests/c_family_placeholder_cli.rs](../../crates/codeguard-cli/tests/c_family_placeholder_cli.rs)

- [crates/codeguard-cli/tests/c_family_placeholder_import.rs](../../crates/codeguard-cli/tests/c_family_placeholder_import.rs)

- [crates/codeguard-adapters/tests/clang_documentation_placeholder_contract.rs](../../crates/codeguard-adapters/tests/clang_documentation_placeholder_contract.rs)

- [crates/codeguard-adapters/tests/clang_placeholder_validation.rs](../../crates/codeguard-adapters/tests/clang_placeholder_validation.rs)

- [schemas/clang-documentation-placeholder-v0.1.schema.json](../../schemas/clang-documentation-placeholder-v0.1.schema.json)

- [schemas/clang-documentation-placeholder-workbench-observation-v0.1.schema.json](../../schemas/clang-documentation-placeholder-workbench-observation-v0.1.schema.json)

- [schemas/c-family-comments-feedback-v0.10.schema.json](../../schemas/c-family-comments-feedback-v0.10.schema.json)

- [schemas/c-family-comments-feedback-v0.11.schema.json](../../schemas/c-family-comments-feedback-v0.11.schema.json)

- [tests/acceptance/clang-documentation-placeholders.md](../../tests/acceptance/clang-documentation-placeholders.md)

- [crates/codeguard-cli/src/next_command/c_family_placeholder_candidate.rs](../../crates/codeguard-cli/src/next_command/c_family_placeholder_candidate.rs)

- [schemas/repair-brief-preview-v0.34.schema.json](../../schemas/repair-brief-preview-v0.34.schema.json)

- [crates/codeguard-cli/src/c_family_placeholder_task_recheck.rs](../../crates/codeguard-cli/src/c_family_placeholder_task_recheck.rs)


必须解决的登记缺口：

- C/C++结构文件策略任务、原工具task verify、受控repair-source尝试及同一输入两次失败预算已接线；跨输入语义无进展、可信关闭/复发、check/Hook与详细语义/完整项目/独立精度仍未验收。

- cpp/standalone: Doxygen/API用途/模板/异常/生命周期与线程安全的完整原生规则/合法反例/契约准确性未验收

- cpp/standalone/documentation:五平台、宿主反馈、可信关闭和复发重开未验收

- 统一check独立档案已partial；完整项目头文件/编译配置、Hook、详细准确性及正式生产覆盖仍未验收



#### 构建路径 cmake / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cpp/cmake: Doxygen/API用途/模板/异常/生命周期与线程安全的完整原生规则/合法反例/契约准确性未验收

- cpp/cmake/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 conan / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cpp/conan: Doxygen/API用途/模板/异常/生命周期与线程安全的完整原生规则/合法反例/契约准确性未验收

- cpp/conan/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 vcpkg / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cpp/vcpkg: Doxygen/API用途/模板/异常/生命周期与线程安全的完整原生规则/合法反例/契约准确性未验收

- cpp/vcpkg/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.28, 8.29, 8.30, 15.4, 15.6, 15.7。

#### 构建路径 standalone / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cpp/standalone:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- cpp/standalone/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 cmake / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cpp/cmake:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- cpp/cmake/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 conan / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cpp/conan:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- cpp/conan/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 vcpkg / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cpp/vcpkg:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- cpp/vcpkg/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.28, 8.29, 8.30, 15.5, 15.6, 15.7。

#### 构建路径 standalone / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cpp/standalone:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- cpp/standalone/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 cmake / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cpp/cmake:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- cpp/cmake/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 conan / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cpp/conan:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- cpp/conan/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 vcpkg / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cpp/vcpkg:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- cpp/vcpkg/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## objc

- 构建生态：xcode, clang-project。grammar候选：objc。

- 版本/方言约束：Objective-C API文档、NSError、ownership与nullability。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["clang-tidy", "--quiet", "{file}"]`。

### syntax — blocked

OpenSpec任务：8.31, 8.32, 8.33, 15.2, 15.6, 15.7。

#### 构建路径 xcode / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- objc/xcode:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- objc/xcode/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 clang-project / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- objc/clang-project:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- objc/clang-project/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.31, 8.32, 8.33, 15.3, 15.6, 15.7。

#### 构建路径 xcode / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- objc/xcode: Objective-C API文档、NSError、ownership与nullability的完整原生规则/合法反例/契约准确性未验收

- objc/xcode/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 clang-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- objc/clang-project: Objective-C API文档、NSError、ownership与nullability的完整原生规则/合法反例/契约准确性未验收

- objc/clang-project/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.31, 8.32, 8.33, 15.4, 15.6, 15.7。

#### 构建路径 xcode / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- objc/xcode:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- objc/xcode/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 clang-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- objc/clang-project:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- objc/clang-project/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.31, 8.32, 8.33, 15.5, 15.6, 15.7。

#### 构建路径 xcode / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- objc/xcode:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- objc/xcode/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 clang-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- objc/clang-project:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- objc/clang-project/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## dart

- 构建生态：pub, flutter。grammar候选：dart。

- 版本/方言约束：Dartdoc用途/参数/返回/异常与Flutter公共API。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["dart", "analyze"]`。

### syntax — blocked

OpenSpec任务：8.34, 8.35, 8.36, 15.2, 15.6, 15.7。

#### 构建路径 pub / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- dart/pub:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- dart/pub/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 flutter / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- dart/flutter:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- dart/flutter/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.34, 8.35, 8.36, 15.3, 15.6, 15.7。

#### 构建路径 pub / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- dart/pub: Dartdoc用途/参数/返回/异常与Flutter公共API的完整原生规则/合法反例/契约准确性未验收

- dart/pub/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 flutter / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- dart/flutter: Dartdoc用途/参数/返回/异常与Flutter公共API的完整原生规则/合法反例/契约准确性未验收

- dart/flutter/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.34, 8.35, 8.36, 15.4, 15.6, 15.7。

#### 构建路径 pub / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- dart/pub:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- dart/pub/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 flutter / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- dart/flutter:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- dart/flutter/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.34, 8.35, 8.36, 15.5, 15.6, 15.7。

#### 构建路径 pub / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- dart/pub:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- dart/pub/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 flutter / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- dart/flutter:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- dart/flutter/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## vue

- 构建生态：npm-vue。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：Vue组件props/events/slots及脚本公共API。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["npx", "--no-install", "eslint", "--ext", ".vue", "."]`。

### syntax — blocked

OpenSpec任务：8.37, 8.38, 8.39, 15.2, 15.6, 15.7。

#### 构建路径 npm-vue / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- vue/npm-vue:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- vue/npm-vue/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.37, 8.38, 8.39, 15.3, 15.6, 15.7。

#### 构建路径 npm-vue / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- vue/npm-vue: Vue组件props/events/slots及脚本公共API的完整原生规则/合法反例/契约准确性未验收

- vue/npm-vue/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.37, 8.38, 8.39, 15.4, 15.6, 15.7。

#### 构建路径 npm-vue / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- vue/npm-vue:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- vue/npm-vue/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.37, 8.38, 8.39, 15.5, 15.6, 15.7。

#### 构建路径 npm-vue / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- vue/npm-vue:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- vue/npm-vue/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## svelte

- 构建生态：npm-svelte。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：Svelte组件props/events及生成代码边界。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["npx", "--no-install", "eslint", "."]`。

### syntax — blocked

OpenSpec任务：8.40, 8.41, 8.42, 15.2, 15.6, 15.7。

#### 构建路径 npm-svelte / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- svelte/npm-svelte:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- svelte/npm-svelte/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.40, 8.41, 8.42, 15.3, 15.6, 15.7。

#### 构建路径 npm-svelte / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- svelte/npm-svelte: Svelte组件props/events及生成代码边界的完整原生规则/合法反例/契约准确性未验收

- svelte/npm-svelte/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.40, 8.41, 8.42, 15.4, 15.6, 15.7。

#### 构建路径 npm-svelte / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- svelte/npm-svelte:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- svelte/npm-svelte/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.40, 8.41, 8.42, 15.5, 15.6, 15.7。

#### 构建路径 npm-svelte / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- svelte/npm-svelte:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- svelte/npm-svelte/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## astro

- 构建生态：npm-astro。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：Astro组件props/slots与客户端服务端行为。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["npx", "--no-install", "eslint", "."]`。

### syntax — blocked

OpenSpec任务：8.43, 8.44, 8.45, 15.2, 15.6, 15.7。

#### 构建路径 npm-astro / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- astro/npm-astro:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- astro/npm-astro/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.43, 8.44, 8.45, 15.3, 15.6, 15.7。

#### 构建路径 npm-astro / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- astro/npm-astro: Astro组件props/slots与客户端服务端行为的完整原生规则/合法反例/契约准确性未验收

- astro/npm-astro/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.43, 8.44, 8.45, 15.4, 15.6, 15.7。

#### 构建路径 npm-astro / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- astro/npm-astro:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- astro/npm-astro/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.43, 8.44, 8.45, 15.5, 15.6, 15.7。

#### 构建路径 npm-astro / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- astro/npm-astro:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- astro/npm-astro/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## solidity

- 构建生态：foundry, hardhat。grammar候选：solidity。

- 版本/方言约束：NatSpec notice/dev/param/return及权限/失败条件。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["solhint", "**/*.sol"]`。

### syntax — blocked

OpenSpec任务：8.55, 8.56, 8.57, 15.2, 15.6, 15.7。

#### 构建路径 foundry / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- solidity/foundry:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- solidity/foundry/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 hardhat / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- solidity/hardhat:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- solidity/hardhat/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.55, 8.56, 8.57, 15.3, 15.6, 15.7。

#### 构建路径 foundry / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- solidity/foundry: NatSpec notice/dev/param/return及权限/失败条件的完整原生规则/合法反例/契约准确性未验收

- solidity/foundry/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 hardhat / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- solidity/hardhat: NatSpec notice/dev/param/return及权限/失败条件的完整原生规则/合法反例/契约准确性未验收

- solidity/hardhat/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.55, 8.56, 8.57, 15.4, 15.6, 15.7。

#### 构建路径 foundry / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- solidity/foundry:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- solidity/foundry/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 hardhat / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- solidity/hardhat:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- solidity/hardhat/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.55, 8.56, 8.57, 15.5, 15.6, 15.7。

#### 构建路径 foundry / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- solidity/foundry:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- solidity/foundry/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 hardhat / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- solidity/hardhat:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- solidity/hardhat/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## terraform

- 构建生态：terraform-providers-modules。grammar候选：terraform。

- 版本/方言约束：模块inputs/outputs/providers及副作用与敏感字段。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["tflint"]`。

### syntax — blocked

OpenSpec任务：8.58, 8.59, 8.60, 15.2, 15.6, 15.7。

#### 构建路径 terraform-providers-modules / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- terraform/terraform-providers-modules:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- terraform/terraform-providers-modules/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.58, 8.59, 8.60, 15.3, 15.6, 15.7。

#### 构建路径 terraform-providers-modules / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- terraform/terraform-providers-modules: 模块inputs/outputs/providers及副作用与敏感字段的完整原生规则/合法反例/契约准确性未验收

- terraform/terraform-providers-modules/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.58, 8.59, 8.60, 15.4, 15.6, 15.7。

#### 构建路径 terraform-providers-modules / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- terraform/terraform-providers-modules:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- terraform/terraform-providers-modules/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.58, 8.59, 8.60, 15.5, 15.6, 15.7。

#### 构建路径 terraform-providers-modules / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- terraform/terraform-providers-modules:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- terraform/terraform-providers-modules/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## nix

- 构建生态：nix-flakes, nix-expression-project。grammar候选：nix。

- 版本/方言约束：Nix函数/选项、输入输出及求值/构建约束。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["deadnix", "{file}"]`。

### syntax — blocked

OpenSpec任务：8.61, 8.62, 8.63, 15.2, 15.6, 15.7。

#### 构建路径 nix-flakes / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- nix/nix-flakes:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- nix/nix-flakes/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 nix-expression-project / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- nix/nix-expression-project:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- nix/nix-expression-project/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.61, 8.62, 8.63, 15.3, 15.6, 15.7。

#### 构建路径 nix-flakes / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- nix/nix-flakes: Nix函数/选项、输入输出及求值/构建约束的完整原生规则/合法反例/契约准确性未验收

- nix/nix-flakes/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 nix-expression-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- nix/nix-expression-project: Nix函数/选项、输入输出及求值/构建约束的完整原生规则/合法反例/契约准确性未验收

- nix/nix-expression-project/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.61, 8.62, 8.63, 15.4, 15.6, 15.7。

#### 构建路径 nix-flakes / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- nix/nix-flakes:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- nix/nix-flakes/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 nix-expression-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- nix/nix-expression-project:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- nix/nix-expression-project/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.61, 8.62, 8.63, 15.5, 15.6, 15.7。

#### 构建路径 nix-flakes / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- nix/nix-flakes:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- nix/nix-flakes/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 nix-expression-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- nix/nix-expression-project:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- nix/nix-expression-project/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## html

- 构建生态：web-project。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：组件/模板用途、属性、可访问性与嵌入脚本边界。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["npx", "--no-install", "htmlhint", "{file}"]`。

### syntax — blocked

OpenSpec任务：8.49, 8.50, 8.51, 15.2, 15.6, 15.7。

#### 构建路径 web-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- html/web-project:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- html/web-project/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.49, 8.50, 8.51, 15.3, 15.6, 15.7。

#### 构建路径 web-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- html/web-project: 组件/模板用途、属性、可访问性与嵌入脚本边界的完整原生规则/合法反例/契约准确性未验收

- html/web-project/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.49, 8.50, 8.51, 15.4, 15.6, 15.7。

#### 构建路径 web-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- html/web-project:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- html/web-project/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.49, 8.50, 8.51, 15.5, 15.6, 15.7。

#### 构建路径 web-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- html/web-project:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- html/web-project/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## sql

- 构建生态：database-dialect-project。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：schema/过程/查询用途、参数、事务与错误条件。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["sqlfluff", "lint", "{file}"]`。

### syntax — blocked

OpenSpec任务：8.64, 8.65, 8.66, 15.2, 15.6, 15.7。

#### 构建路径 database-dialect-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- sql/database-dialect-project:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- sql/database-dialect-project/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.64, 8.65, 8.66, 15.3, 15.6, 15.7。

#### 构建路径 database-dialect-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- sql/database-dialect-project: schema/过程/查询用途、参数、事务与错误条件的完整原生规则/合法反例/契约准确性未验收

- sql/database-dialect-project/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.64, 8.65, 8.66, 15.4, 15.6, 15.7。

#### 构建路径 database-dialect-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- sql/database-dialect-project:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- sql/database-dialect-project/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.64, 8.65, 8.66, 15.5, 15.6, 15.7。

#### 构建路径 database-dialect-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- sql/database-dialect-project:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- sql/database-dialect-project/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## graphql

- 构建生态：schema-client-project。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：schema description与字段/参数/返回/弃用约束。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["npx", "--no-install", "eslint", "--ext", ".graphql", "."]`。

### syntax — blocked

OpenSpec任务：8.52, 8.53, 8.54, 15.2, 15.6, 15.7。

#### 构建路径 schema-client-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- graphql/schema-client-project:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- graphql/schema-client-project/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.52, 8.53, 8.54, 15.3, 15.6, 15.7。

#### 构建路径 schema-client-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- graphql/schema-client-project: schema description与字段/参数/返回/弃用约束的完整原生规则/合法反例/契约准确性未验收

- graphql/schema-client-project/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.52, 8.53, 8.54, 15.4, 15.6, 15.7。

#### 构建路径 schema-client-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- graphql/schema-client-project:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- graphql/schema-client-project/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.52, 8.53, 8.54, 15.5, 15.6, 15.7。

#### 构建路径 schema-client-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- graphql/schema-client-project:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- graphql/schema-client-project/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## protobuf

- 构建生态：buf, protoc-build。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：message/field/service/rpc契约及兼容性。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["buf", "lint"]`。

### syntax — blocked

OpenSpec任务：8.67, 8.68, 8.69, 15.2, 15.6, 15.7。

#### 构建路径 buf / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- protobuf/buf:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- protobuf/buf/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 protoc-build / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- protobuf/protoc-build:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- protobuf/protoc-build/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.67, 8.68, 8.69, 15.3, 15.6, 15.7。

#### 构建路径 buf / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- protobuf/buf: message/field/service/rpc契约及兼容性的完整原生规则/合法反例/契约准确性未验收

- protobuf/buf/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 protoc-build / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- protobuf/protoc-build: message/field/service/rpc契约及兼容性的完整原生规则/合法反例/契约准确性未验收

- protobuf/protoc-build/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.67, 8.68, 8.69, 15.4, 15.6, 15.7。

#### 构建路径 buf / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- protobuf/buf:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- protobuf/buf/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 protoc-build / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- protobuf/protoc-build:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- protobuf/protoc-build/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.67, 8.68, 8.69, 15.5, 15.6, 15.7。

#### 构建路径 buf / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- protobuf/buf:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- protobuf/buf/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 protoc-build / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- protobuf/protoc-build:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- protobuf/protoc-build/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## markdown

- 构建生态：documentation-project。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：文档结构、示例有效性和事实时效，不冒充API源码检查。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["npx", "--no-install", "markdownlint-cli2", "**/*.md", "#node_modules"]`。

### syntax — blocked

OpenSpec任务：8.73, 8.74, 8.75, 15.2, 15.6, 15.7。

#### 构建路径 documentation-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- markdown/documentation-project:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- markdown/documentation-project/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.73, 8.74, 8.75, 15.3, 15.6, 15.7。

#### 构建路径 documentation-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- markdown/documentation-project: 文档结构、示例有效性和事实时效，不冒充API源码检查的完整原生规则/合法反例/契约准确性未验收

- markdown/documentation-project/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.73, 8.74, 8.75, 15.4, 15.6, 15.7。

#### 构建路径 documentation-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- markdown/documentation-project:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- markdown/documentation-project/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.73, 8.74, 8.75, 15.5, 15.6, 15.7。

#### 构建路径 documentation-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- markdown/documentation-project:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- markdown/documentation-project/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## toml

- 构建生态：enclosing-project。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：所属manifest/config字段、默认值及依赖范围说明。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["taplo", "lint", "."]`。

### syntax — blocked

OpenSpec任务：8.76, 8.77, 8.78, 15.2, 15.6, 15.7。

#### 构建路径 enclosing-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- toml/enclosing-project:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- toml/enclosing-project/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.76, 8.77, 8.78, 15.3, 15.6, 15.7。

#### 构建路径 enclosing-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- toml/enclosing-project: 所属manifest/config字段、默认值及依赖范围说明的完整原生规则/合法反例/契约准确性未验收

- toml/enclosing-project/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.76, 8.77, 8.78, 15.4, 15.6, 15.7。

#### 构建路径 enclosing-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- toml/enclosing-project:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- toml/enclosing-project/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.76, 8.77, 8.78, 15.5, 15.6, 15.7。

#### 构建路径 enclosing-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- toml/enclosing-project:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- toml/enclosing-project/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## haskell

- 构建生态：cabal, stack。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：Haddock与类型/错误/效果契约。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["hlint", "."]`。

### syntax — blocked

OpenSpec任务：8.79, 8.80, 8.81, 15.2, 15.6, 15.7。

#### 构建路径 cabal / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- haskell/cabal:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- haskell/cabal/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 stack / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- haskell/stack:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- haskell/stack/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.79, 8.80, 8.81, 15.3, 15.6, 15.7。

#### 构建路径 cabal / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- haskell/cabal: Haddock与类型/错误/效果契约的完整原生规则/合法反例/契约准确性未验收

- haskell/cabal/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 stack / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- haskell/stack: Haddock与类型/错误/效果契约的完整原生规则/合法反例/契约准确性未验收

- haskell/stack/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.79, 8.80, 8.81, 15.4, 15.6, 15.7。

#### 构建路径 cabal / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- haskell/cabal:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- haskell/cabal/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 stack / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- haskell/stack:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- haskell/stack/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.79, 8.80, 8.81, 15.5, 15.6, 15.7。

#### 构建路径 cabal / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- haskell/cabal:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- haskell/cabal/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 stack / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- haskell/stack:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- haskell/stack/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## ocaml

- 构建生态：dune, opam。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：odoc与参数/返回/异常和模块签名。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["ocamlformat", "--check", "."]`。

### syntax — blocked

OpenSpec任务：8.82, 8.83, 8.84, 15.2, 15.6, 15.7。

#### 构建路径 dune / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- ocaml/dune:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- ocaml/dune/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 opam / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- ocaml/opam:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- ocaml/opam/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.82, 8.83, 8.84, 15.3, 15.6, 15.7。

#### 构建路径 dune / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- ocaml/dune: odoc与参数/返回/异常和模块签名的完整原生规则/合法反例/契约准确性未验收

- ocaml/dune/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 opam / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- ocaml/opam: odoc与参数/返回/异常和模块签名的完整原生规则/合法反例/契约准确性未验收

- ocaml/opam/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.82, 8.83, 8.84, 15.4, 15.6, 15.7。

#### 构建路径 dune / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- ocaml/dune:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- ocaml/dune/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 opam / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- ocaml/opam:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- ocaml/opam/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.82, 8.83, 8.84, 15.5, 15.6, 15.7。

#### 构建路径 dune / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- ocaml/dune:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- ocaml/dune/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 opam / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- ocaml/opam:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- ocaml/opam/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## fsharp

- 构建生态：dotnet。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：XML/API文档与参数/返回/异常契约。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["dotnet", "fantomas", "--check"]`。

### syntax — blocked

OpenSpec任务：8.85, 8.86, 8.87, 15.2, 15.6, 15.7。

#### 构建路径 dotnet / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- fsharp/dotnet:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- fsharp/dotnet/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.85, 8.86, 8.87, 15.3, 15.6, 15.7。

#### 构建路径 dotnet / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- fsharp/dotnet: XML/API文档与参数/返回/异常契约的完整原生规则/合法反例/契约准确性未验收

- fsharp/dotnet/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.85, 8.86, 8.87, 15.4, 15.6, 15.7。

#### 构建路径 dotnet / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- fsharp/dotnet:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- fsharp/dotnet/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.85, 8.86, 8.87, 15.5, 15.6, 15.7。

#### 构建路径 dotnet / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- fsharp/dotnet:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- fsharp/dotnet/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## perl

- 构建生态：cpan, perl-project。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：POD公共接口、参数/返回/错误及上下文。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["perlcritic", "{file}"]`。

### syntax — blocked

OpenSpec任务：8.88, 8.89, 8.90, 15.2, 15.6, 15.7。

#### 构建路径 cpan / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- perl/cpan:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- perl/cpan/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 perl-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- perl/perl-project:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- perl/perl-project/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.88, 8.89, 8.90, 15.3, 15.6, 15.7。

#### 构建路径 cpan / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- perl/cpan: POD公共接口、参数/返回/错误及上下文的完整原生规则/合法反例/契约准确性未验收

- perl/cpan/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 perl-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- perl/perl-project: POD公共接口、参数/返回/错误及上下文的完整原生规则/合法反例/契约准确性未验收

- perl/perl-project/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.88, 8.89, 8.90, 15.4, 15.6, 15.7。

#### 构建路径 cpan / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- perl/cpan:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- perl/cpan/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 perl-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- perl/perl-project:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- perl/perl-project/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.88, 8.89, 8.90, 15.5, 15.6, 15.7。

#### 构建路径 cpan / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- perl/cpan:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- perl/cpan/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 perl-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- perl/perl-project:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- perl/perl-project/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## groovy

- 构建生态：gradle, maven。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：Groovydoc/DSL用途和Java互操作契约。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["npm-groovy-lint", "{file}"]`。

### syntax — blocked

OpenSpec任务：8.91, 8.92, 8.93, 15.2, 15.6, 15.7。

#### 构建路径 gradle / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- groovy/gradle:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- groovy/gradle/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 maven / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- groovy/maven:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- groovy/maven/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.91, 8.92, 8.93, 15.3, 15.6, 15.7。

#### 构建路径 gradle / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- groovy/gradle: Groovydoc/DSL用途和Java互操作契约的完整原生规则/合法反例/契约准确性未验收

- groovy/gradle/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 maven / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- groovy/maven: Groovydoc/DSL用途和Java互操作契约的完整原生规则/合法反例/契约准确性未验收

- groovy/maven/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.91, 8.92, 8.93, 15.4, 15.6, 15.7。

#### 构建路径 gradle / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- groovy/gradle:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- groovy/gradle/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 maven / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- groovy/maven:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- groovy/maven/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.91, 8.92, 8.93, 15.5, 15.6, 15.7。

#### 构建路径 gradle / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- groovy/gradle:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- groovy/gradle/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 maven / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- groovy/maven:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- groovy/maven/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## clojure

- 构建生态：tools-deps, leiningen。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：docstring公共var、参数/返回及异常/副作用。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["clj-kondo", "--lint", "src"]`。

### syntax — blocked

OpenSpec任务：8.94, 8.95, 8.96, 15.2, 15.6, 15.7。

#### 构建路径 tools-deps / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- clojure/tools-deps:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- clojure/tools-deps/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 leiningen / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- clojure/leiningen:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- clojure/leiningen/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.94, 8.95, 8.96, 15.3, 15.6, 15.7。

#### 构建路径 tools-deps / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- clojure/tools-deps: docstring公共var、参数/返回及异常/副作用的完整原生规则/合法反例/契约准确性未验收

- clojure/tools-deps/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 leiningen / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- clojure/leiningen: docstring公共var、参数/返回及异常/副作用的完整原生规则/合法反例/契约准确性未验收

- clojure/leiningen/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.94, 8.95, 8.96, 15.4, 15.6, 15.7。

#### 构建路径 tools-deps / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- clojure/tools-deps:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- clojure/tools-deps/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 leiningen / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- clojure/leiningen:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- clojure/leiningen/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.94, 8.95, 8.96, 15.5, 15.6, 15.7。

#### 构建路径 tools-deps / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- clojure/tools-deps:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- clojure/tools-deps/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 leiningen / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- clojure/leiningen:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- clojure/leiningen/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## powershell

- 构建生态：powershell-modules。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：comment-based help参数/输出/错误和权限。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["pwsh", "-NoProfile", "-Command", "Invoke-ScriptAnalyzer -Path . -Severity Error"]`。

### syntax — blocked

OpenSpec任务：8.97, 8.98, 8.99, 15.2, 15.6, 15.7。

#### 构建路径 powershell-modules / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- powershell/powershell-modules:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- powershell/powershell-modules/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.97, 8.98, 8.99, 15.3, 15.6, 15.7。

#### 构建路径 powershell-modules / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- powershell/powershell-modules: comment-based help参数/输出/错误和权限的完整原生规则/合法反例/契约准确性未验收

- powershell/powershell-modules/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.97, 8.98, 8.99, 15.4, 15.6, 15.7。

#### 构建路径 powershell-modules / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- powershell/powershell-modules:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- powershell/powershell-modules/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.97, 8.98, 8.99, 15.5, 15.6, 15.7。

#### 构建路径 powershell-modules / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- powershell/powershell-modules:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- powershell/powershell-modules/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## zig

- 构建生态：zig-build。grammar候选：zig。

- 版本/方言约束：Zig doc comments用途/参数/返回/error set及分配器。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["zig", "fmt", "--check", "."]`。

### syntax — blocked

OpenSpec任务：8.100, 8.101, 8.102, 15.2, 15.6, 15.7。

#### 构建路径 zig-build / partial

现有代码入口：

- [crates/codeguard-cli/src/zig_lint_command.rs](../../crates/codeguard-cli/src/zig_lint_command.rs)


现有验收/样例：

- [tests/acceptance/zig-native-differential.md](../../tests/acceptance/zig-native-differential.md)


必须解决的登记缺口：

- zig/zig-build:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- zig/zig-build/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.100, 8.101, 8.102, 15.3, 15.6, 15.7。

#### 构建路径 zig-build / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- zig/zig-build: Zig doc comments用途/参数/返回/error set及分配器的完整原生规则/合法反例/契约准确性未验收

- zig/zig-build/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.100, 8.101, 8.102, 15.4, 15.6, 15.7。

#### 构建路径 zig-build / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- zig/zig-build:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- zig/zig-build/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.100, 8.101, 8.102, 15.5, 15.6, 15.7。

#### 构建路径 zig-build / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- zig/zig-build:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- zig/zig-build/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## nim

- 构建生态：nimble。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：Nim doc comments与参数/返回/raises/effects。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["nim", "check", "src"]`。

### syntax — blocked

OpenSpec任务：8.103, 8.104, 8.105, 15.2, 15.6, 15.7。

#### 构建路径 nimble / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- nim/nimble:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- nim/nimble/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.103, 8.104, 8.105, 15.3, 15.6, 15.7。

#### 构建路径 nimble / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- nim/nimble: Nim doc comments与参数/返回/raises/effects的完整原生规则/合法反例/契约准确性未验收

- nim/nimble/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.103, 8.104, 8.105, 15.4, 15.6, 15.7。

#### 构建路径 nimble / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- nim/nimble:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- nim/nimble/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.103, 8.104, 8.105, 15.5, 15.6, 15.7。

#### 构建路径 nimble / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- nim/nimble:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- nim/nimble/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## crystal

- 构建生态：shards。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：Crystal API文档与参数/返回/异常。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["ameba"]`。

### syntax — blocked

OpenSpec任务：8.106, 8.107, 8.108, 15.2, 15.6, 15.7。

#### 构建路径 shards / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- crystal/shards:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- crystal/shards/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.106, 8.107, 8.108, 15.3, 15.6, 15.7。

#### 构建路径 shards / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- crystal/shards: Crystal API文档与参数/返回/异常的完整原生规则/合法反例/契约准确性未验收

- crystal/shards/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.106, 8.107, 8.108, 15.4, 15.6, 15.7。

#### 构建路径 shards / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- crystal/shards:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- crystal/shards/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.106, 8.107, 8.108, 15.5, 15.6, 15.7。

#### 构建路径 shards / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- crystal/shards:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- crystal/shards/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## julia

- 构建生态：julia-project。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：Julia docstrings与方法签名/参数/返回/异常。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`null`。

### syntax — blocked

OpenSpec任务：8.109, 8.110, 8.111, 15.2, 15.6, 15.7。

#### 构建路径 julia-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- julia/julia-project:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- julia/julia-project/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.109, 8.110, 8.111, 15.3, 15.6, 15.7。

#### 构建路径 julia-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- julia/julia-project: Julia docstrings与方法签名/参数/返回/异常的完整原生规则/合法反例/契约准确性未验收

- julia/julia-project/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.109, 8.110, 8.111, 15.4, 15.6, 15.7。

#### 构建路径 julia-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- julia/julia-project:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- julia/julia-project/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.109, 8.110, 8.111, 15.5, 15.6, 15.7。

#### 构建路径 julia-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- julia/julia-project:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- julia/julia-project/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## elm

- 构建生态：elm-packages。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：Elm module/value文档与类型/失败建模。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["elm-review"]`。

### syntax — blocked

OpenSpec任务：8.112, 8.113, 8.114, 15.2, 15.6, 15.7。

#### 构建路径 elm-packages / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- elm/elm-packages:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- elm/elm-packages/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.112, 8.113, 8.114, 15.3, 15.6, 15.7。

#### 构建路径 elm-packages / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- elm/elm-packages: Elm module/value文档与类型/失败建模的完整原生规则/合法反例/契约准确性未验收

- elm/elm-packages/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.112, 8.113, 8.114, 15.4, 15.6, 15.7。

#### 构建路径 elm-packages / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- elm/elm-packages:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- elm/elm-packages/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.112, 8.113, 8.114, 15.5, 15.6, 15.7。

#### 构建路径 elm-packages / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- elm/elm-packages:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- elm/elm-packages/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## lua

- 构建生态：luarocks, embedded-host。grammar候选：lua。

- 版本/方言约束：Lua API文档与宿主参数/返回/错误/栈约束。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["luacheck", "."]`。

### syntax — blocked

OpenSpec任务：8.115, 8.116, 8.117, 15.2, 15.6, 15.7。

#### 构建路径 luarocks / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- lua/luarocks:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- lua/luarocks/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 embedded-host / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- lua/embedded-host:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- lua/embedded-host/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.115, 8.116, 8.117, 15.3, 15.6, 15.7。

#### 构建路径 luarocks / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- lua/luarocks: Lua API文档与宿主参数/返回/错误/栈约束的完整原生规则/合法反例/契约准确性未验收

- lua/luarocks/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 embedded-host / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- lua/embedded-host: Lua API文档与宿主参数/返回/错误/栈约束的完整原生规则/合法反例/契约准确性未验收

- lua/embedded-host/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.115, 8.116, 8.117, 15.4, 15.6, 15.7。

#### 构建路径 luarocks / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- lua/luarocks:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- lua/luarocks/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 embedded-host / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- lua/embedded-host:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- lua/embedded-host/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.115, 8.116, 8.117, 15.5, 15.6, 15.7。

#### 构建路径 luarocks / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- lua/luarocks:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- lua/luarocks/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 embedded-host / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- lua/embedded-host:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- lua/embedded-host/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## luau

- 构建生态：luau-project, embedded-host。grammar候选：luau。

- 版本/方言约束：Luau API文档与类型/宿主返回/错误约束。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["luau-analyze", "{file}"]`。

### syntax — blocked

OpenSpec任务：8.118, 8.119, 8.120, 15.2, 15.6, 15.7。

#### 构建路径 luau-project / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- luau/luau-project:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- luau/luau-project/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 embedded-host / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- luau/embedded-host:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- luau/embedded-host/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.118, 8.119, 8.120, 15.3, 15.6, 15.7。

#### 构建路径 luau-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- luau/luau-project: Luau API文档与类型/宿主返回/错误约束的完整原生规则/合法反例/契约准确性未验收

- luau/luau-project/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 embedded-host / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- luau/embedded-host: Luau API文档与类型/宿主返回/错误约束的完整原生规则/合法反例/契约准确性未验收

- luau/embedded-host/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.118, 8.119, 8.120, 15.4, 15.6, 15.7。

#### 构建路径 luau-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- luau/luau-project:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- luau/luau-project/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 embedded-host / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- luau/embedded-host:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- luau/embedded-host/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.118, 8.119, 8.120, 15.5, 15.6, 15.7。

#### 构建路径 luau-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- luau/luau-project:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- luau/luau-project/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 embedded-host / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- luau/embedded-host:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- luau/embedded-host/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## pascal

- 构建生态：freepascal, delphi。grammar候选：pascal。

- 版本/方言约束：Pascal API文档与参数/返回/异常及编译器方言。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`null`。

### syntax — blocked

OpenSpec任务：8.121, 8.122, 8.123, 15.2, 15.6, 15.7。

#### 构建路径 freepascal / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- pascal/freepascal:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- pascal/freepascal/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 delphi / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- pascal/delphi:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- pascal/delphi/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.121, 8.122, 8.123, 15.3, 15.6, 15.7。

#### 构建路径 freepascal / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- pascal/freepascal: Pascal API文档与参数/返回/异常及编译器方言的完整原生规则/合法反例/契约准确性未验收

- pascal/freepascal/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 delphi / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- pascal/delphi: Pascal API文档与参数/返回/异常及编译器方言的完整原生规则/合法反例/契约准确性未验收

- pascal/delphi/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.121, 8.122, 8.123, 15.4, 15.6, 15.7。

#### 构建路径 freepascal / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- pascal/freepascal:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- pascal/freepascal/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 delphi / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- pascal/delphi:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- pascal/delphi/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.121, 8.122, 8.123, 15.5, 15.6, 15.7。

#### 构建路径 freepascal / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- pascal/freepascal:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- pascal/freepascal/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 delphi / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- pascal/delphi:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- pascal/delphi/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## r

- 构建生态：renv, r-package。grammar候选：r。

- 版本/方言约束：roxygen2/R API文档与参数/value/错误/副作用。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["Rscript", "-e", "lintr::lint_dir('.')"]`。

### syntax — blocked

OpenSpec任务：8.124, 8.125, 8.126, 15.2, 15.6, 15.7。

#### 构建路径 renv / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- r/renv:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- r/renv/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 r-package / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- r/r-package:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- r/r-package/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.124, 8.125, 8.126, 15.3, 15.6, 15.7。

#### 构建路径 renv / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- r/renv: roxygen2/R API文档与参数/value/错误/副作用的完整原生规则/合法反例/契约准确性未验收

- r/renv/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 r-package / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- r/r-package: roxygen2/R API文档与参数/value/错误/副作用的完整原生规则/合法反例/契约准确性未验收

- r/r-package/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.124, 8.125, 8.126, 15.4, 15.6, 15.7。

#### 构建路径 renv / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- r/renv:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- r/renv/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 r-package / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- r/r-package:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- r/r-package/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.124, 8.125, 8.126, 15.5, 15.6, 15.7。

#### 构建路径 renv / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- r/renv:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- r/renv/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 r-package / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- r/r-package:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- r/r-package/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## cfml

- 构建生态：cfml-engine-project。grammar候选：cfml, cfquery, cfscript。

- 版本/方言约束：CFML函数/组件文档与参数/返回/异常及嵌入SQL。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["cflint", "{file}"]`。

### syntax — blocked

OpenSpec任务：8.127, 8.128, 8.129, 15.2, 15.6, 15.7。

#### 构建路径 cfml-engine-project / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cfml/cfml-engine-project:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- cfml/cfml-engine-project/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.127, 8.128, 8.129, 15.3, 15.6, 15.7。

#### 构建路径 cfml-engine-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cfml/cfml-engine-project: CFML函数/组件文档与参数/返回/异常及嵌入SQL的完整原生规则/合法反例/契约准确性未验收

- cfml/cfml-engine-project/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.127, 8.128, 8.129, 15.4, 15.6, 15.7。

#### 构建路径 cfml-engine-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cfml/cfml-engine-project:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- cfml/cfml-engine-project/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.127, 8.128, 8.129, 15.5, 15.6, 15.7。

#### 构建路径 cfml-engine-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cfml/cfml-engine-project:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- cfml/cfml-engine-project/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## cobol

- 构建生态：gnucobol, enterprise-cobol。grammar候选：cobol。

- 版本/方言约束：COBOL程序/数据接口与返回状态及运行时约束。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`null`。

### syntax — blocked

OpenSpec任务：8.145, 15.2, 15.6, 15.7。

#### 构建路径 gnucobol / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cobol/gnucobol:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- cobol/gnucobol/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 enterprise-cobol / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cobol/enterprise-cobol:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- cobol/enterprise-cobol/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.145, 15.3, 15.6, 15.7。

#### 构建路径 gnucobol / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cobol/gnucobol: COBOL程序/数据接口与返回状态及运行时约束的完整原生规则/合法反例/契约准确性未验收

- cobol/gnucobol/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 enterprise-cobol / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cobol/enterprise-cobol: COBOL程序/数据接口与返回状态及运行时约束的完整原生规则/合法反例/契约准确性未验收

- cobol/enterprise-cobol/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.145, 15.4, 15.6, 15.7。

#### 构建路径 gnucobol / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cobol/gnucobol:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- cobol/gnucobol/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 enterprise-cobol / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cobol/enterprise-cobol:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- cobol/enterprise-cobol/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.145, 15.5, 15.6, 15.7。

#### 构建路径 gnucobol / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cobol/gnucobol:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- cobol/gnucobol/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 enterprise-cobol / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cobol/enterprise-cobol:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- cobol/enterprise-cobol/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## vbnet

- 构建生态：dotnet。grammar候选：vbnet。

- 版本/方言约束：XML API文档与参数/返回/异常及生成代码。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["dotnet", "format", "--verify-no-changes"]`。

### syntax — blocked

OpenSpec任务：8.130, 8.131, 8.132, 15.2, 15.6, 15.7。

#### 构建路径 dotnet / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- vbnet/dotnet:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- vbnet/dotnet/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.130, 8.131, 8.132, 15.3, 15.6, 15.7。

#### 构建路径 dotnet / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- vbnet/dotnet: XML API文档与参数/返回/异常及生成代码的完整原生规则/合法反例/契约准确性未验收

- vbnet/dotnet/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.130, 8.131, 8.132, 15.4, 15.6, 15.7。

#### 构建路径 dotnet / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- vbnet/dotnet:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- vbnet/dotnet/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.130, 8.131, 8.132, 15.5, 15.6, 15.7。

#### 构建路径 dotnet / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- vbnet/dotnet:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- vbnet/dotnet/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## erlang

- 构建生态：rebar3, otp-application。grammar候选：erlang。

- 版本/方言约束：EEP-48/EDoc/spec与参数/返回/异常/进程行为。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["elvis", "rock"]`。

### syntax — blocked

OpenSpec任务：8.133, 8.134, 8.135, 15.2, 15.6, 15.7。

#### 构建路径 rebar3 / partial

现有代码入口：

- [crates/codeguard-cli/src/erlang_lint_command.rs](../../crates/codeguard-cli/src/erlang_lint_command.rs)


现有验收/样例：

- [tests/acceptance/erlang-native-first-workbench.md](../../tests/acceptance/erlang-native-first-workbench.md)


必须解决的登记缺口：

- erlang/rebar3:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- erlang/rebar3/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 otp-application / partial

现有代码入口：

- [crates/codeguard-cli/src/erlang_lint_command.rs](../../crates/codeguard-cli/src/erlang_lint_command.rs)


现有验收/样例：

- [tests/acceptance/erlang-native-first-workbench.md](../../tests/acceptance/erlang-native-first-workbench.md)


必须解决的登记缺口：

- erlang/otp-application:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- erlang/otp-application/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.133, 8.134, 8.135, 15.3, 15.6, 15.7。

#### 构建路径 rebar3 / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- erlang/rebar3: EEP-48/EDoc/spec与参数/返回/异常/进程行为的完整原生规则/合法反例/契约准确性未验收

- erlang/rebar3/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 otp-application / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- erlang/otp-application: EEP-48/EDoc/spec与参数/返回/异常/进程行为的完整原生规则/合法反例/契约准确性未验收

- erlang/otp-application/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.133, 8.134, 8.135, 15.4, 15.6, 15.7。

#### 构建路径 rebar3 / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- erlang/rebar3:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- erlang/rebar3/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 otp-application / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- erlang/otp-application:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- erlang/otp-application/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.133, 8.134, 8.135, 15.5, 15.6, 15.7。

#### 构建路径 rebar3 / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- erlang/rebar3:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- erlang/rebar3/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 otp-application / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- erlang/otp-application:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- erlang/otp-application/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## arkts

- 构建生态：hvigor-ohpm。grammar候选：arkts。

- 版本/方言约束：ArkTS公共API文档与声明限制/参数/返回/错误。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`null`。

### syntax — blocked

OpenSpec任务：8.146, 15.2, 15.6, 15.7。

#### 构建路径 hvigor-ohpm / wasm_candidate_only

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- arkts/hvigor-ohpm:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- arkts/hvigor-ohpm/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.146, 15.3, 15.6, 15.7。

#### 构建路径 hvigor-ohpm / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- arkts/hvigor-ohpm: ArkTS公共API文档与声明限制/参数/返回/错误的完整原生规则/合法反例/契约准确性未验收

- arkts/hvigor-ohpm/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.146, 15.4, 15.6, 15.7。

#### 构建路径 hvigor-ohpm / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- arkts/hvigor-ohpm:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- arkts/hvigor-ohpm/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.146, 15.5, 15.6, 15.7。

#### 构建路径 hvigor-ohpm / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- arkts/hvigor-ohpm:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- arkts/hvigor-ohpm/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## metal

- 构建生态：xcode-metal。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：Metal函数参数、资源、线程组与运行约束说明。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`null`。

### syntax — blocked

OpenSpec任务：8.147, 15.2, 15.6, 15.7。

#### 构建路径 xcode-metal / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- metal/xcode-metal:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- metal/xcode-metal/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.147, 15.3, 15.6, 15.7。

#### 构建路径 xcode-metal / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- metal/xcode-metal: Metal函数参数、资源、线程组与运行约束说明的完整原生规则/合法反例/契约准确性未验收

- metal/xcode-metal/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.147, 15.4, 15.6, 15.7。

#### 构建路径 xcode-metal / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- metal/xcode-metal:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- metal/xcode-metal/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.147, 15.5, 15.6, 15.7。

#### 构建路径 xcode-metal / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- metal/xcode-metal:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- metal/xcode-metal/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## liquid

- 构建生态：shopify-theme, embedded-template-host。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：Liquid模板/过滤器接口及宿主输入输出约束。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["theme-check", "."]`。

### syntax — blocked

OpenSpec任务：8.136, 8.137, 8.138, 15.2, 15.6, 15.7。

#### 构建路径 shopify-theme / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- liquid/shopify-theme:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- liquid/shopify-theme/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 embedded-template-host / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- liquid/embedded-template-host:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- liquid/embedded-template-host/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.136, 8.137, 8.138, 15.3, 15.6, 15.7。

#### 构建路径 shopify-theme / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- liquid/shopify-theme: Liquid模板/过滤器接口及宿主输入输出约束的完整原生规则/合法反例/契约准确性未验收

- liquid/shopify-theme/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 embedded-template-host / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- liquid/embedded-template-host: Liquid模板/过滤器接口及宿主输入输出约束的完整原生规则/合法反例/契约准确性未验收

- liquid/embedded-template-host/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.136, 8.137, 8.138, 15.4, 15.6, 15.7。

#### 构建路径 shopify-theme / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- liquid/shopify-theme:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- liquid/shopify-theme/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 embedded-template-host / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- liquid/embedded-template-host:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- liquid/embedded-template-host/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.136, 8.137, 8.138, 15.5, 15.6, 15.7。

#### 构建路径 shopify-theme / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- liquid/shopify-theme:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- liquid/shopify-theme/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 embedded-template-host / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- liquid/embedded-template-host:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- liquid/embedded-template-host/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## cuda

- 构建生态：cmake-cuda, nvcc-project。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：CUDA API文档、内存、kernel参数与同步/错误条件。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["clang-tidy", "--quiet", "{file}"]`。

### syntax — blocked

OpenSpec任务：8.139, 8.140, 8.141, 15.2, 15.6, 15.7。

#### 构建路径 cmake-cuda / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cuda/cmake-cuda:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- cuda/cmake-cuda/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 nvcc-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cuda/nvcc-project:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- cuda/nvcc-project/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.139, 8.140, 8.141, 15.3, 15.6, 15.7。

#### 构建路径 cmake-cuda / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cuda/cmake-cuda: CUDA API文档、内存、kernel参数与同步/错误条件的完整原生规则/合法反例/契约准确性未验收

- cuda/cmake-cuda/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 nvcc-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cuda/nvcc-project: CUDA API文档、内存、kernel参数与同步/错误条件的完整原生规则/合法反例/契约准确性未验收

- cuda/nvcc-project/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.139, 8.140, 8.141, 15.4, 15.6, 15.7。

#### 构建路径 cmake-cuda / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cuda/cmake-cuda:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- cuda/cmake-cuda/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 nvcc-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cuda/nvcc-project:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- cuda/nvcc-project/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.139, 8.140, 8.141, 15.5, 15.6, 15.7。

#### 构建路径 cmake-cuda / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cuda/cmake-cuda:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- cuda/cmake-cuda/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



#### 构建路径 nvcc-project / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- cuda/nvcc-project:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- cuda/nvcc-project/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收



## ansible

- 构建生态：ansible-collections。grammar候选：无；不得虚构WASM支持。

- 版本/方言约束：Ansible模块/角色/collection参数/返回/权限和副作用。qualification=unqualified。

- 历史lint候选（须核实，非最终命令）：`["ansible-lint"]`。

### syntax — blocked

OpenSpec任务：8.142, 8.143, 8.144, 15.2, 15.6, 15.7。

#### 构建路径 ansible-collections / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- ansible/ansible-collections:完整原生语法版本/方言、源集、WASM联合路径及独立精度未验收

- ansible/ansible-collections/syntax:五平台、宿主反馈、可信关闭和复发重开未验收



### documentation — blocked

OpenSpec任务：8.142, 8.143, 8.144, 15.3, 15.6, 15.7。

#### 构建路径 ansible-collections / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- ansible/ansible-collections: Ansible模块/角色/collection参数/返回/权限和副作用的完整原生规则/合法反例/契约准确性未验收

- ansible/ansible-collections/documentation:五平台、宿主反馈、可信关闭和复发重开未验收



### conventions — blocked

OpenSpec任务：8.142, 8.143, 8.144, 15.4, 15.6, 15.7。

#### 构建路径 ansible-collections / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- ansible/ansible-collections:原项目规范规则及配置/真实诊断/误报裁定未完整验收；旧命令不能代替适配

- ansible/ansible-collections/conventions:五平台、宿主反馈、可信关闭和复发重开未验收



### vulnerabilities — blocked

OpenSpec任务：8.142, 8.143, 8.144, 15.5, 15.6, 15.7。

#### 构建路径 ansible-collections / not_integrated

现有代码入口：

- 无已登记入口；先确认共享适配可能性，再按工作包模板实现。


现有验收/样例：

- 无已登记证据；须补真实工具正反例、故障与闭环验收。


必须解决的登记缺口：

- ansible/ansible-collections:实际依赖图/锁、漏洞来源及时效、原生扫描与修复复检未完整验收

- ansible/ansible-collections/vulnerabilities:五平台、宿主反馈、可信关闭和复发重开未验收
