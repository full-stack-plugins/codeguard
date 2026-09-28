# OWASP Dependency-Check JSON 局部解析

对应 OpenSpec `introduce-rust-codeguard-cli` 6.4/6.8 的解析器前置工作。Rust `codeguard-adapters::parse_owasp_dependency_check_json` 只接受有 `reportSchema=1.1`、`scanInfo.engineVersion`、`projectInfo.reportDate` 与显式 `dependencies` 数组的有界 JSON；拒绝重复键、错误标志、非空 `analysisExceptions`、损坏的组件摘要和漏洞标识/分数。数组字段缺失与空数组不同，防止损坏报告被解释成零漏洞。官方 [JSON 输出模板](https://github.com/dependency-check/DependencyCheck/blob/main/core/src/main/resources/templates/jsonReport.vsl) 是字段依据。

结果分别保留活动 `vulnerabilities` 和 `suppressedVulnerabilities`，保存来源、advisory ID、可用 CVSS 分数、原生 package ID 与文件摘要；不保存原生 `filePath` 或自由文本。缺分数仍保留漏洞；空 `dataSource` 仍如实记录为空，不能据此声称漏洞库新鲜。解析器不运行 Dependency-Check，不把 package ID 与 Maven 依赖图自动匹配，也不签发清洁、白名单或交付结论。官方报告可能含虚拟依赖、缺分数及原生 suppression，均须在后续本轮执行/归属/时效核验中区别处理。

目标契约先因接口缺失失败；完成后 `cargo test -p codeguard-adapters --test owasp_dependency_check_contract --offline` 的 5 项通过，定向 Clippy `-D warnings` 通过。真实原生 Maven Dependency-Check 调用、漏洞库时效与正反例、项目覆盖和对话反馈仍未实施；OpenSpec 6.4/6.8 不勾选。
